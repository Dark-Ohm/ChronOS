//! Right side panel content view — lives in the fixed-canvas `content`
//! window (T276). Renders the active tab plus the (right-aligned) visible
//! slice of the canvas; the icon rail itself lives in a separate window
//! (`rail_view::RailView`) and reaches this view through the shared weak
//! entity in `SidePanelRightState`, the same pattern `mod.rs::select_tab`
//! already used from an `App`-only IPC context.
//!
//! ## `on_hover` / animation split (fork rule)
//! Our gpui fork stores a **single** `Option` hover handler per element and
//! `debug_assert!`s if `.on_hover` is set twice. Consequences:
//! - Root node: **only** the peek close-debounce `on_hover` (this file).
//! - Children: **no** extra root hover.
//! - Peek motion: state-driven `.transition_when` on an **inner** wrapper.
//!
//! ## T276 — no window resize left in this file
//! The `content` window's `WindowBounds` are fixed at open
//! (`CONTENT_CANVAS_WIDTH` px) and never change again — `render()` never
//! calls `window.resize()`. The only per-frame work here is (a) which
//! right-aligned rectangle of the canvas is painted (`visible_w`) and (b)
//! keeping `Window::set_input_region` in sync with it, so the empty part of
//! the canvas passes clicks through to whatever is behind it. This retires
//! the entire T210/T214/T216/T243 family of async-resize-race bugs — there
//! is no configure to race against anymore.

use std::{
    collections::HashMap,
    time::Duration,
};

use chronos_services::NotificationCommand;
use gpui::{AnimationExt, AsyncApp, IntoElement, Render, Window, div, prelude::*, px};

use crate::agent_follow::AgentFollowState;
use crate::motion;
use crate::side_panel_right::panels_config;
use crate::side_panel_right::preview_target::PreviewTarget;
use crate::side_panel_right::surfaces;
use crate::side_panel_right::tab::TabContent;
use crate::side_panel_right::tabs::PanelTab;
use crate::side_panel_right::{
    CONTENT_CANVAS_WIDTH, HANDLE_WIDTH, MAX_WIDTH, RAIL_ONLY_WIDTH, RightPanelResize,
    SidePanelRightState, content_input_region, content_interactive_width, content_resize_handle_x,
    visible_content_width,
};
use crate::state::AppState;
use crate::workspace_mode;

use chronos_ui::{Theme, WindowRootExt, elevation_glow_bar};

/// Delay before peek-close after mouse leaves panel (or strip).
const PEEK_LEAVE_DEBOUNCE: Duration = Duration::from_millis(280);

pub struct SidePanelRightView {
    active_tab: PanelTab,
    /// T276: last visible-width value pushed to `Window::set_input_region`.
    /// Only re-issued when it changes, avoiding a Wayland round-trip on
    /// every render (mirrors the old `last_exclusive_zone` cache, now on
    /// the rail side).
    last_visible_width: Option<f32>,
    /// Pointer x in the fixed content-canvas frame at drag start.
    resize_start_x: Option<f32>,
    /// Panel width at drag start (T276: pure delta model — the drag no
    /// longer chases a resizing surface, so this plus `resize_start_x` is
    /// the entire state needed to compute the target on every move).
    resize_start_width: Option<f32>,
    /// Lazy, cached tab views — one per visited tab. Created on first
    /// activation, retained across switches and mode changes.
    tab_views: HashMap<PanelTab, TabContent>,
    /// Per-tab user-resized widths (session-only, not persisted to disk).
    /// When a tab is selected, its width here (or `preferred_content_width`
    /// if never resized) is applied to `SidePanelRightState.width`.
    tab_resize_memory: HashMap<PanelTab, f32>,
    /// T194: opening a file (Files click, or a future agent-follow path —
    /// T195) switches the panel to the Editor tab. Kept alive only to hold
    /// the `observe_global` subscription; dropping it would silently stop
    /// the switch.
    _preview_target_subscription: gpui::Subscription,
    /// T195: observes `AgentFollowState` — reserved for activity strip UI.
    #[allow(dead_code)]
    _follow_subscription: gpui::Subscription,
}

impl SidePanelRightView {
    pub fn new(cx: &mut Context<Self>) -> Self {
        // Defensive default — mirrors `PreviewTab::new`'s guard: tests and
        // early wiring must not race with `side_panel_right::init`, and
        // `cx.observe_global` requires the global to already exist.
        if !cx.has_global::<PreviewTarget>() {
            cx.set_global(PreviewTarget::default());
        }
        if !cx.has_global::<AgentFollowState>() {
            cx.set_global(AgentFollowState::default());
        }
        let preview_target_subscription = cx.observe_global::<PreviewTarget>(|this, cx| {
            // A file was opened (path went from None to Some, or a new file
            // was clicked) — switch to Editor so the user sees it land
            // without a second click. `Files → Editor` is the wire T194
            // asks for; `resolve_for_mode`/`for_mode` already put both in
            // the same (Developer) rail, so switching cannot land on a tab
            // absent from the current rail.
            if cx.global::<PreviewTarget>().path.is_some() {
                this.on_tab_select(PanelTab::Preview, cx);
            }
        });
        Self {
            active_tab: PanelTab::default(),
            last_visible_width: None,
            resize_start_x: None,
            resize_start_width: None,
            tab_views: HashMap::new(),
            tab_resize_memory: HashMap::new(),
            _preview_target_subscription: preview_target_subscription,
            _follow_subscription: cx.observe_global::<AgentFollowState>(|_, cx| cx.notify()),
        }
    }

    /// T276: currently active tab. Read by `rail_view::RailView` (a
    /// different window) to highlight the matching rail icon.
    pub(crate) fn active_tab(&self) -> PanelTab {
        self.active_tab
    }

    /// Return the effective width for `tab`: user-resized width if the tab is
    /// draggable at all, otherwise its `preferred_content_width`. Clamped to
    /// `RAIL_ONLY_WIDTH .. MAX_WIDTH`.
    ///
    /// T218: a fixed-width tab ignores `tab_resize_memory` outright, so a width
    /// recorded before the tab was frozen (or by a stray drag) can never keep it
    /// off its natural size.
    fn active_tab_width(&self, tab: PanelTab, _cx: &Context<Self>) -> f32 {
        let preferred = tab.preferred_content_width();
        let w = if tab.resizable() {
            self.tab_resize_memory
                .get(&tab)
                .copied()
                .unwrap_or(preferred)
        } else {
            preferred
        };
        w.clamp(RAIL_ONLY_WIDTH, MAX_WIDTH)
    }

    fn apply_active_tab_width(&mut self, cx: &mut Context<Self>) {
        let target = self.active_tab_width(self.active_tab, cx);
        let state = cx.global_mut::<SidePanelRightState>();
        let before = state.width;
        let content_open = state.width > RAIL_ONLY_WIDTH + 1.0;
        if content_open {
            state.ensure_content_width(target);
        }
        tracing::info!(
            before,
            after = state.width,
            content_open,
            tab = self.active_tab.label(),
            "side_panel_right: apply per-tab width"
        );
    }

    fn resolve_active_tab(&mut self, all_tabs: &[PanelTab], cx: &mut Context<Self>) -> bool {
        if all_tabs.contains(&self.active_tab) {
            return false;
        }
        tracing::info!(
            was = self.active_tab.label(),
            "side_panel_right: active tab not in mode set → System"
        );
        self.active_tab = PanelTab::System;
        self.apply_active_tab_width(cx);
        true
    }

    /// T276: called by the transparent handle on the screen-inward left edge
    /// of this fixed content canvas. All resize bookkeeping stays here, the
    /// single owner of `tab_resize_memory`.
    pub(crate) fn start_resize(&mut self, start_x: f32, cx: &mut Context<Self>) {
        // T210: suppress peek-close for the press lifetime.
        cx.global_mut::<SidePanelRightState>().resizing = true;
        let w = cx.global::<SidePanelRightState>().width;
        let tab = self.active_tab;
        let resizable = tab.resizable();

        // Defensive rail-only path: an in-flight drag may reach the clamp while
        // the pointer is still captured. A fresh open normally comes from the
        // standalone rail icon, not from this content-owned handle.
        if w <= RAIL_ONLY_WIDTH + 1.0 {
            let target = self.active_tab_width(tab, cx);
            let state = cx.global_mut::<SidePanelRightState>();
            state.width = target;
            if resizable {
                self.tab_resize_memory.insert(tab, target);
            }
            tracing::info!(
                width = target,
                tab = tab.label(),
                "side_panel_right: handle grab expanded rail → content"
            );
            cx.refresh_windows();
        }

        // T218: arm the drag only for tabs the user may resize. Fixed-width tabs
        // leave the anchors unset, so `update_resize` returns immediately and the
        // width stays exactly `preferred_content_width`.
        if !resizable {
            self.resize_start_x = None;
            self.resize_start_width = None;
            tracing::debug!(
                tab = tab.label(),
                "side_panel_right: tab is fixed width, drag ignored"
            );
            return;
        }

        // T276: the content window frame is immobile (it is never resized),
        // so unlike the old T216 grab-offset fixup there is no
        // edge sliding out from under the cursor to compensate for — the
        // handle-local press point IS the anchor for the whole drag.
        let width_now = cx.global::<SidePanelRightState>().width;
        self.resize_start_x = Some(start_x);
        self.resize_start_width = Some(width_now);
        tracing::debug!(
            grab_x = ?self.resize_start_x,
            start_w = width_now,
            tab = tab.label(),
            "side_panel_right: resize drag started"
        );
    }

    /// T276: pure delta from `resize_start_x`/`resize_start_width` — no
    /// `Window` parameter needed anymore since neither surface this drag
    /// touches is ever resized. See `resize_target_width`'s doc for the
    /// coordinate-frame reasoning.
    pub(crate) fn update_resize(&mut self, current_x: f32, cx: &mut Context<Self>) {
        let (Some(start_x), Some(start_width)) = (self.resize_start_x, self.resize_start_width)
        else {
            return;
        };
        let new_w = crate::side_panel_right::resize_target_width(start_width, start_x, current_x);
        let state = cx.global_mut::<SidePanelRightState>();
        let old_w = state.width;
        state.resize(new_w);
        self.tab_resize_memory.insert(self.active_tab, state.width);
        crate::side_panel_right::hold_peek(cx);
        tracing::trace!(
            current_x,
            start_x,
            start_width,
            old_w,
            new_w,
            "side_panel_right: resize drag move"
        );
        cx.refresh_windows();
    }

    /// T210: mouse-up on the handle means the drag ended. Expansion already
    /// happened on mouse-down (see `start_resize`), so this is pure cleanup.
    pub(crate) fn end_resize(&mut self, cx: &mut Context<Self>) {
        cx.global_mut::<SidePanelRightState>().resizing = false;
        self.resize_start_x = None;
        self.resize_start_width = None;
        tracing::info!("side_panel_right: resize drag ended (mouse-up)");
        cx.refresh_windows();
    }

    /// T276: dock ⊞/⊟ toggle, called from `rail_view::RailView` via the
    /// shared weak entity (the button itself renders in the rail window).
    pub(crate) fn toggle_dock(&mut self, cx: &mut Context<Self>) {
        let state = cx.global_mut::<SidePanelRightState>();
        state.dock_content = !state.dock_content;
        state.last_exclusive_zone = None;
        tracing::info!(
            dock = state.dock_content,
            width = state.width,
            "side_panel_right: dock toggle"
        );
        cx.refresh_windows();
    }

    pub(crate) fn on_tab_select(&mut self, tab: PanelTab, cx: &mut Context<Self>) {
        // T293: when the Notifications tab is selected, mark the history
        // read so the bell's unread dot clears the moment the inbox is
        // viewed — same behavior as the former history popup.
        if tab == PanelTab::Notifications {
            let svc = AppState::notification(cx).clone();
            cx.background_spawn(async move {
                let _ = svc.dispatch(NotificationCommand::MarkAllRead).await;
            })
            .detach();
        }

        // T221 — rail icon is the single affordance. Three actions, in order:
        //
        //   1. Same tab, `dock_content = true`  → no-op. Dock keeps content
        //      always-visible; a rail icon cannot shrink a docked panel
        //      without contradicting dock state. ⊞/⊟ is the dock knob.
        //   2. Same tab, content open           → collapse to rail.
        //      `tab_resize_memory` is NOT touched — the view owns it, not
        //      `SidePanelRightState.width`, so a future re-open restores
        //      the remembered width (T218).
        //   3. Same tab, content closed          → open at `active_tab_width`
        //      (T218: preferred for fixed-width tabs, remembered for
        //      Editor / System settings).
        //   4. Different tab                    → switch AND open (a click
        //      somewhere else cannot be a clamp-to-rail action — it has to
        //      show the user the new tab).
        //
        // Width arithmetic goes through `active_tab_width`, the same path
        // T171/T218 wired. We deliberately bypass `apply_active_tab_width`'s
        // «skip when collapsed» optimisation in the re-open branch, because
        // that branch IS the act of opening.
        //
        // T276: every branch calls `cx.refresh_windows()` instead of plain
        // `cx.notify()` — the rail's active-tab highlight and dock icon live
        // in a *different* window (`rail_view::RailView`) that only repaints
        // when something marks it dirty explicitly. `refresh_windows()` is
        // the same idiom `workspace_mode::set`/`edit_mode::toggle` already
        // use for cross-window state changes.

        let (dock_content, content_open) = {
            let state = cx.global::<SidePanelRightState>();
            (state.dock_content, state.width > RAIL_ONLY_WIDTH + 1.0)
        };

        if tab != self.active_tab {
            // Branch 4 — different tab.
            //
            // Under dock: only switch `active_tab`. The dock button ⊞/⊟
            // and the resize handle are the only knobs for width — switching
            // tabs in dock mode must not undo a pinned width.
            //
            // Off dock: switch AND force-open at the new tab's natural /
            // remembered width.
            self.active_tab = tab;
            self.ensure_tab_view(self.active_tab, cx);
            if dock_content {
                cx.refresh_windows();
                tracing::info!(
                    tab = tab.label(),
                    "side_panel_right: switched tab under dock (width pinned)"
                );
                return;
            }
            let target = self.active_tab_width(self.active_tab, cx);
            cx.global_mut::<SidePanelRightState>()
                .ensure_content_width(target);
            cx.refresh_windows();
            tracing::info!(
                tab = tab.label(),
                width = target,
                "side_panel_right: switched tab → opened at per-tab width"
            );
            return;
        }

        // Same tab clicked — collapse if open, re-open if collapsed.
        // (T289) Dock is an exclusive-zone flag only — it does NOT guard
        // this path, so same-tab clicks under dock behave identically to
        // under dock-off (collapse if open, re-open if collapsed).
        if content_open {
            // Branch 2 — collapse. `tab_resize_memory` stays intact because
            // we only touch `state.width`.
            cx.global_mut::<SidePanelRightState>().width = RAIL_ONLY_WIDTH;
            cx.refresh_windows();
            tracing::info!(
                tab = tab.label(),
                "side_panel_right: same tab → collapsed to rail (memory preserved)"
            );
        } else {
            // Branch 3 — re-open at the tab's stored width.
            let target = self.active_tab_width(self.active_tab, cx);
            cx.global_mut::<SidePanelRightState>()
                .ensure_content_width(target);
            cx.refresh_windows();
            tracing::info!(
                tab = tab.label(),
                width = target,
                "side_panel_right: same tab → re-opened at stored width"
            );
        }
    }

    /// Lazily create the tab view if not already cached. Called from both
    /// `on_tab_select` and `render()` — the single source of creation.
    ///
    /// Returns the cached handle so callers never need to look it up again
    /// (and never need an `unwrap` on a key that was just inserted).
    pub(crate) fn ensure_tab_view(&mut self, tab: PanelTab, cx: &mut Context<Self>) -> TabContent {
        self.tab_views
            .entry(tab)
            .or_insert_with(|| TabContent::create(tab, cx))
            .clone()
    }

    /// T279 — point the Files tab at `path`, creating the tab lazily if it
    /// was never opened. Called by `side_panel_right::open_files_at` (the
    /// left workspace Project "Files" action).
    pub(crate) fn set_files_root(&mut self, path: std::path::PathBuf, cx: &mut Context<Self>) {
        if let TabContent::Files(files) = self.ensure_tab_view(PanelTab::Files, cx) {
            files.update(cx, |tab, cx| tab.set_root(path, cx));
        }
    }

    /// T279 — respawn the Terminal tab's shell at `path`, creating the tab
    /// lazily if needed. Called by `side_panel_right::open_terminal_at`
    /// (the left workspace Project "Terminal" action).
    pub(crate) fn open_terminal_at(&mut self, path: std::path::PathBuf, cx: &mut Context<Self>) {
        if let TabContent::Terminal(term) = self.ensure_tab_view(PanelTab::Terminal, cx) {
            term.update(cx, |tab, cx| tab.open_at(path, cx));
        }
    }

    /// T226 tooling: focus handle of the currently active tab, when that tab
    /// is keyboard-focusable. Synthetic mouse clicks do not focus GPUI
    /// layer-shell windows, so `select_tab` re-focuses the window itself to
    /// let external input (wtype/ydotool) reach the newly active tab.
    pub(crate) fn active_tab_focus(&self, cx: &gpui::App) -> Option<gpui::FocusHandle> {
        match self.tab_views.get(&self.active_tab)? {
            TabContent::Terminal(entity) => Some(
                <crate::side_panel_right::tab::terminal::TerminalTab as gpui::Focusable>::focus_handle(
                    entity.read(cx),
                    cx,
                ),
            ),
            TabContent::Preview(entity) => entity.read(cx).editor_focus_handle(cx),
            _ => None,
        }
    }
}

impl Render for SidePanelRightView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // T219: resolve rail groups from panels.toml (two groups: top + bottom).
        // Falls back to panels_config defaults which mirror the old for_mode.
        let current_mode = workspace_mode::current(cx);
        let panel_cfg = panels_config::cached();
        let (top_tabs, bottom_tabs) = panels_config::resolve_grouped(current_mode, &panel_cfg);
        // Flatten for active-tab validation.
        let mut all_tabs: Vec<PanelTab> =
            top_tabs.iter().chain(bottom_tabs.iter()).copied().collect();
        all_tabs.dedup();
        // Active tab left the set after a mode switch — land on System, keep
        // the panel open (§5: must not discard panel state / close on mode change).
        if self.resolve_active_tab(&all_tabs, cx) {
            // System may not have been visited yet — ensure the entry exists
            // before the render path reads it via get().
            self.ensure_tab_view(PanelTab::System, cx);
        }

        let panel_state = cx.global::<SidePanelRightState>();
        let panel_width = panel_state.width;
        let resizing = panel_state.resizing;

        // T276: `content`'s WindowBounds are fixed forever — `visible_w` is
        // the only thing that moves. It drives both the painted rectangle
        // below and the Wayland input region (so the empty part of the
        // canvas passes clicks through instead of eating them).
        let visible_w = visible_content_width(panel_width);
        // T289: content_open is purely width-driven — dock_content only
        // affects the exclusive zone (exclusive_px), not visibility.
        let content_open = visible_w > 1.0;
        let interactive_w = content_interactive_width(visible_w, resizing);

        if self.last_visible_width != Some(interactive_w) {
            let canvas_h = f32::from(window.bounds().size.height);
            let regions = content_input_region(CONTENT_CANVAS_WIDTH, canvas_h, interactive_w);
            window.set_input_region(Some(&regions));
            self.last_visible_width = Some(interactive_w);
        }

        // T217 — top-left corner radius where the visible content column
        // meets the bar. The rail's own top-right (display) corner is
        // rounded independently in `rail_view::RailView::render`.
        // Elevated chrome на content-колонке (не rail-only) — общий язык
        // глубины из `theme.elevation_popup()` (T128).
        let theme = *Theme::global(cx);
        let elev = theme.elevation_popup();

        let active = self.active_tab;
        let resize_mouse_down = cx.listener(|this, ev: &gpui::MouseDownEvent, _window, cx| {
            this.start_resize(f32::from(ev.position.x), cx);
        });
        let resize_drag_move = cx.listener(
            |this, ev: &gpui::DragMoveEvent<RightPanelResize>, _window, cx| {
                this.update_resize(f32::from(ev.event.position.x), cx);
            },
        );
        let resize_mouse_up =
            cx.listener(|this, _ev: &gpui::MouseUpEvent, _window, cx| this.end_resize(cx));

        // Lazy tab view — created on first paint, cached thereafter.
        // ensure_tab_view() avoids expect-panic on the very first render
        // (before any on_tab_select has fired). T168 errata 3.
        let tab_entry = self.ensure_tab_view(active, cx);
        let handle_x = content_resize_handle_x(CONTENT_CANVAS_WIDTH, visible_w);

        // ROOT: the whole fixed canvas. `on_hover` here still matters for
        // peek even while collapsed (regions is empty, but GPUI delivers
        // hover to the *element tree*, not gated on the Wayland input
        // region — the input region only decides who receives pointer
        // *events*, not who observes hover state changes from this
        // process's own compositor-agnostic hit testing... T276 note:
        // this must be verified live, since a fully input-transparent
        // window CANNOT receive a real pointer enter/leave from the
        // compositor either. If peek-open regresses when the panel starts
        // collapsed, this is the first place to look — the strip window
        // (`hover_strip.rs`) is the actual peek trigger in that state, so
        // it should be unaffected, but the panel's own re-collapse debounce
        // (`on_hover` below) depends on this surface catching mouse-leave
        // while it still has *some* visible width.
        div()
            .id("side-panel-right-content-root")
            .window_font(&theme)
            .size_full()
            .relative()
            .flex()
            .flex_row()
            .on_hover(|hovered, _window, cx| {
                if *hovered {
                    crate::side_panel_right::hold_peek(cx);
                } else {
                    crate::side_panel_right::schedule_release_peek(cx);
                }
            })
            .child(
                // Empty, transparent slice of the fixed canvas to the left
                // of the visible content — never painted, never receives
                // input (excluded from the Wayland input region above).
                div()
                    .id("side-panel-content-void")
                    .flex_1()
                    .min_w(px(0.))
                    .h_full(),
            )
            .when(content_open, |root| {
                root.child({
                    let col = div()
                        .id("side-panel-content-column")
                        .flex_none()
                        .w(px(visible_w))
                        .h_full()
                        .flex()
                        .flex_col()
                        .overflow_hidden()
                        // T266: the content column is the visible plate —
                        // surface alpha applies here, not in `surfaces::`
                        // helpers (nested cards stay opaque).
                        .bg(theme.surface_color(surfaces::content(&theme)))
                        // T315: near-side border removed — inside continuous
                        // chrome the seam reads as two objects.
                        .shadow(elev.shadows.to_vec())
                        // T315: far-side corners (left side of right content)
                        // get r=8 rounding. The near side (right, adjacent to
                        // rail) stays straight.
                        .rounded_tl(px(8.))
                        .rounded_bl(px(8.));
                    // Light-C glow-ребро на верхней кромке content-колонки.
                    let col = match elev.glow {
                        Some(glow) => col.child(elevation_glow_bar(glow)),
                        None => col,
                    };
                    // --- Tab content ---
                    let content_el = match tab_entry {
                        // T305: System's content lives in the control-center
                        // popup now — the rail never creates it here, but the
                        // arm stays for IPC/fallback paths that may.
                        TabContent::System(entity) => col.child(entity.clone()),
                        TabContent::Files(entity) => col.child(entity.clone()),
                        TabContent::Terminal(entity) => col.child(entity.clone()),
                        TabContent::Build(entity) => col.child(entity.clone()),
                        // T179: minimum addition to keep the enum
                        // exhaustive; pairs with the same one-line match
                        // arm in `tab_entity_id` below.
                        TabContent::Preview(entity) => col.child(entity.clone()),
                        // T188: Library is a real entity (Gamer hub).
                        TabContent::Library(entity) => col.child(entity.clone()),
                        // T193: Hyprland binds (read-only list).
                        TabContent::HyprBinds(entity) => col.child(entity.clone()),
                        // T202: System settings «Bar» page.
                        TabContent::BarSettings(entity) => col.child(entity.clone()),
                        TabContent::AcpSettings(entity) => col.child(entity.clone()),
                        // T296: Display settings (brightness + wallpaper).
                        TabContent::Display(entity) => col.child(entity.clone()),
                        // T294: Updates list (pacman-only apply, AUR display-only).
                        TabContent::Updates(entity) => col.child(entity.clone()),
                        // T293: Notifications history list.
                        TabContent::Notifications(entity) => col.child(entity.clone()),
                        // T265-G: Launcher settings page.
                        TabContent::LauncherSettings(entity) => col.child(entity.clone()),
                        // T305: Media — control-center popup tab.
                        TabContent::Media(entity) => col.child(entity.clone()),
                        TabContent::Placeholder(entity) => col.child(entity.clone()),
                    };
                    // Enter animation belongs to the content column alone.
                    content_el.with_animation(
                        "side-panel-content-enter",
                        motion::enter_animation(),
                        motion::apply_enter_from_right,
                    )
                })
            })
            .when(
                (visible_w > 1.0 || resizing) && active.resizable(),
                |root| {
                    root.child(
                        // A right panel resizes from its screen-inward LEFT
                        // edge. The hit strip moves inside the fixed content
                        // canvas; it never belongs to the standalone rail.
                        div()
                            .id("side-panel-right-resize-handle")
                            .absolute()
                            .left(px(handle_x))
                            .top(px(0.))
                            .w(px(HANDLE_WIDTH))
                            .h_full()
                            .cursor_col_resize()
                            .on_mouse_down(gpui::MouseButton::Left, resize_mouse_down)
                            .on_mouse_up(gpui::MouseButton::Left, resize_mouse_up)
                            .on_drag(RightPanelResize, |_, _, _, cx| cx.new(|_| gpui::EmptyView))
                            .on_drag_move(resize_drag_move),
                    )
                },
            )
    }
}

#[cfg(test)]
impl SidePanelRightView {
    pub(crate) fn tab_count(&self) -> usize {
        self.tab_views.len()
    }

    pub(crate) fn tab_entity_id(&self, tab: PanelTab) -> Option<gpui::EntityId> {
        self.tab_views.get(&tab).map(|tc| match tc {
            TabContent::System(e) => e.entity_id(),
            TabContent::Files(e) => e.entity_id(),
            TabContent::Terminal(e) => e.entity_id(),
            TabContent::Build(e) => e.entity_id(),
            // T179: minimum addition to keep the enum exhaustive; precedent
            // set by T176 (Files) and T177 (Terminal). View body itself stays
            // outside T179's zone — this is a one-line structural match.
            TabContent::Preview(e) => e.entity_id(),
            // T188: Library is a real entity (Gamer hub).
            TabContent::Library(e) => e.entity_id(),
            // T193: Hyprland binds (read-only list).
            TabContent::HyprBinds(e) => e.entity_id(),
            // T202: System settings «Bar» page.
            TabContent::BarSettings(e) => e.entity_id(),
            TabContent::AcpSettings(e) => e.entity_id(),
            // T296: Display settings (brightness + wallpaper).
            TabContent::Display(e) => e.entity_id(),
            // T294: Updates list (pacman-only apply, AUR display-only).
            TabContent::Updates(e) => e.entity_id(),
            // T293: Notifications history list.
            TabContent::Notifications(e) => e.entity_id(),
            // T265-G: Launcher settings page.
            TabContent::LauncherSettings(e) => e.entity_id(),
            // T305: Media — control-center popup tab.
            TabContent::Media(e) => e.entity_id(),
            TabContent::Placeholder(e) => e.entity_id(),
        })
    }

    /// Simulate a user resize for testing: stores width in both the
    /// global state and per-tab memory.
    ///
    /// T218: mirrors the real drag path — a fixed-width tab refuses the resize
    /// outright, so a test cannot record a width the UI would never produce.
    pub(crate) fn sim_resize(&mut self, width: f32, cx: &mut Context<Self>) {
        if !self.active_tab.resizable() {
            return;
        }
        let state = cx.global_mut::<SidePanelRightState>();
        state.resize(width);
        self.tab_resize_memory.insert(self.active_tab, state.width);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workspace_mode::WorkspaceMode;
    use gpui::TestAppContext;

    #[gpui::test]
    async fn mode_fallback_applies_system_preferred_width(cx: &mut TestAppContext) {
        cx.update(|cx| {
            let mut state = SidePanelRightState::default();
            state.dock_content = true;
            state.width = 480.0; // T289: dock ON alone no longer opens content;
            // width must be > RAIL_ONLY_WIDTH for content_open
            cx.set_global(state);
        });
        let view = cx.new(|cx| SidePanelRightView::new(cx));

        cx.update_entity(&view, |this, cx| {
            this.active_tab = PanelTab::Editor;
            this.resolve_active_tab(&[PanelTab::System], cx);
        });

        cx.update(|cx| {
            assert_eq!(
                cx.global::<SidePanelRightState>().width,
                PanelTab::System.preferred_content_width()
            );
        });
        cx.update_entity(&view, |this, _cx| {
            assert_eq!(this.active_tab, PanelTab::System);
        });
    }

    #[gpui::test]
    async fn mode_fallback_applies_fixed_system_width(cx: &mut TestAppContext) {
        // T218: System is fixed width now, so the mode fallback must land on its
        // preferred 400 and ignore any recorded width. The memory path itself is
        // covered for a resizable tab by `switch_tab_restores_per_tab_resize_memory`.
        cx.update(|cx| {
            let mut state = SidePanelRightState::default();
            state.dock_content = true;
            state.width = 480.0; // T289: dock ON alone no longer opens content;
            // width must be > RAIL_ONLY_WIDTH for content_open
            cx.set_global(state);
        });
        let view = cx.new(|cx| SidePanelRightView::new(cx));

        cx.update_entity(&view, |this, cx| {
            this.active_tab = PanelTab::Editor;
            this.tab_resize_memory.insert(PanelTab::System, 480.);
            this.resolve_active_tab(&[PanelTab::System], cx);
        });

        cx.update(|cx| {
            assert_eq!(
                cx.global::<SidePanelRightState>().width,
                PanelTab::System.preferred_content_width(),
                "fixed-width System must ignore recorded width on mode fallback"
            );
        });
    }

    #[gpui::test]
    async fn mode_fallback_keeps_rail_only_width_closed(cx: &mut TestAppContext) {
        cx.update(|cx| {
            cx.set_global(SidePanelRightState::default());
        });
        let view = cx.new(|cx| SidePanelRightView::new(cx));

        cx.update_entity(&view, |this, cx| {
            this.active_tab = PanelTab::Editor;
            this.resolve_active_tab(&[PanelTab::System], cx);
        });

        cx.update(|cx| {
            let state = cx.global::<SidePanelRightState>();
            assert_eq!(state.width, RAIL_ONLY_WIDTH);
            assert!(!state.dock_content);
        });
    }

    // ── T219 regression: on_move callback wires through to panels_config ──
    //
    // These tests call the *same* helper the production closure delegates to
    // (`panels_config::move_tab`). They are NOT a re-implementation of the
    // closure body — that was the T164 anti-pattern the spec warned about.
    // If `move_tab` ever grows a new step (e.g. a server sync), these tests
    // reflect that automatically because they drive the real entry point.
    //
    // Bonus: visiting `panels_config::move_tab` directly from the test side
    // confirms the helper is independently callable, not just accidentally
    // correct via its single caller (now `rail_view.rs`, not this file).

    #[gpui::test]
    async fn move_tab_helper_persists_reorder_and_updates_cache(cx: &mut TestAppContext) {
        // No panels.toml on disk → cached() returns fresh defaults. We do
        // NOT call `apply()` here because that pulls from the real config
        // path; we want the test to start from a known sanitized default.
        cx.update(|cx| {
            cx.set_global(crate::workspace_mode::WorkspaceModeState::default());
        });
        cx.update_global::<crate::workspace_mode::WorkspaceModeState, _>(|s, _| {
            s.mode = WorkspaceMode::Developer;
        });

        // Move system (idx 0 in dev top) up by -1 → crosses to end of bottom.
        // This is exactly what the rail's ▲ handler fires against the same tab.
        cx.update(|cx| {
            assert!(
                panels_config::move_tab(cx, WorkspaceMode::Developer, PanelTab::System, -1),
                "`move_tab` must report success"
            );
        });

        // The next render reads `cached()` via `resolve_grouped`. system must
        // have left the dev top group and joined the dev bottom group's tail.
        cx.update(|_cx| {
            let cfg = panels_config::cached();
            let dev = &cfg.right.rail.developer;
            assert!(
                !dev.top.contains(&"system".to_string()),
                "system must leave dev top after the move: {:?}",
                dev.top
            );
            assert_eq!(
                dev.bottom.last().expect("non-empty bottom"),
                "system",
                "system must land at the tail of dev bottom"
            );
        });
    }

    #[gpui::test]
    async fn move_tab_helper_noop_leaves_cache_and_disk_untouched(cx: &mut TestAppContext) {
        // Library is a Gamer-hub tab, not in the Developer rail.
        cx.update(|cx| {
            cx.set_global(crate::workspace_mode::WorkspaceModeState::default());
        });
        cx.update_global::<crate::workspace_mode::WorkspaceModeState, _>(|s, _| {
            s.mode = WorkspaceMode::Developer;
        });

        let before = panels_config::cached();
        cx.update(|cx| {
            assert!(
                !panels_config::move_tab(cx, WorkspaceMode::Developer, PanelTab::Library, -1),
                "Library is not in the Developer rail — helper must report no-op"
            );
        });
        let after = panels_config::cached();
        assert_eq!(
            before, after,
            "no-op must not perturb the cache (save/update_cache skipped)"
        );
    }

    // ── T221 regression: rail icon toggles panel content —─────────────────
    //
    // The contract is tested by calling the real `on_tab_select` (not by
    // reading the corresponding state-assembly code back into the test, which
    // is the anti-T164 shortcut). When `on_tab_select` grows a fourth branch,
    // these tests fail in proportion.
    //
    // State is constructed directly (not via render) so the assertions aren't
    // coupled to layer-shell geometry, width rounding, or platform-window
    // surface size — `state.width` is the truth under test; T276 removed the
    // platform-resize step this contract used to depend on entirely.

    /// Helper: stand up view + a given initial `SidePanelRightState`.
    fn boot_view(
        cx: &mut TestAppContext,
        state: SidePanelRightState,
        mode: WorkspaceMode,
    ) -> gpui::Entity<SidePanelRightView> {
        cx.update(|cx| {
            cx.set_global(state);
            cx.set_global(crate::workspace_mode::WorkspaceModeState::default());
        });
        cx.update_global::<crate::workspace_mode::WorkspaceModeState, _>(|s, _| {
            s.mode = mode;
        });
        cx.new(|cx| SidePanelRightView::new(cx))
    }

    /// (1) Click a **different** tab → switches AND opens content at that
    /// tab's natural width. Catches the regression where a click on another
    /// icon would do nothing or stay at rail-only.
    #[gpui::test]
    async fn on_tab_select_different_tab_opens_at_natural_width(cx: &mut TestAppContext) {
        let view = boot_view(
            cx,
            SidePanelRightState::default(), // rail-only
            WorkspaceMode::Developer,
        );
        let target = PanelTab::Files; // natural width 440, fixed (T218)
        assert_ne!(target, PanelTab::System); // sanity: this test starts non-active

        cx.update_entity(&view, |this, cx| {
            this.on_tab_select(target, cx);
        });

        cx.update_entity(&view, |this, _| {
            assert_eq!(this.active_tab, target);
        });
        cx.update(|cx| {
            assert_eq!(
                cx.global::<SidePanelRightState>().width,
                PanelTab::Files.preferred_content_width(),
                "different-tab click must open content at the new tab's natural width"
            );
        });
    }

    /// (2) Same tab, **content open** → collapse to rail-only. Companion of
    /// case (3) below.
    #[gpui::test]
    async fn on_tab_select_same_tab_open_collapses_to_rail(cx: &mut TestAppContext) {
        let mut state = SidePanelRightState::default();
        state.width = PanelTab::System.preferred_content_width();
        let view = boot_view(cx, state, WorkspaceMode::Developer);

        cx.update_entity(&view, |this, cx| {
            this.on_tab_select(PanelTab::System, cx);
        });

        cx.update(|cx| {
            assert_eq!(
                cx.global::<SidePanelRightState>().width,
                RAIL_ONLY_WIDTH,
                "open + active-tab click must collapse to rail-only"
            );
        });
    }

    /// (3) Same tab, **content collapsed** → re-open at the tab's
    /// `active_tab_width`. System is fixed-width so it lands on its
    /// preferred 400; the “re-opens at remembered width” case for
    /// resizable tabs is covered separately below.
    #[gpui::test]
    async fn on_tab_select_same_tab_collapsed_reopens_at_natural_width(cx: &mut TestAppContext) {
        let view = boot_view(cx, SidePanelRightState::default(), WorkspaceMode::Developer);
        // active_tab is System by default; System is fixed-width 400 (T218).

        cx.update_entity(&view, |this, cx| {
            this.on_tab_select(PanelTab::System, cx);
        });

        cx.update(|cx| {
            assert_eq!(
                cx.global::<SidePanelRightState>().width,
                PanelTab::System.preferred_content_width(),
                "collapsed + active-tab click must re-open at the tab's natural width, \
                 not the panel default"
            );
        });
    }

    /// (4) Editor round-trip: collapse must NOT erase remembered resize.
    /// This is the contract §2 of the task: “Editor: раскрыть → перетянуть
    /// до N → свернуть → раскрыть = N.” Drives the real `update_resize`
    /// path (the test harness `sim_resize` mirrors it) to record N.
    #[gpui::test]
    async fn on_tab_select_collapse_preserves_editor_resize_memory(cx: &mut TestAppContext) {
        const N: f32 = 720.0;
        let mut state = SidePanelRightState::default();
        state.width = N;
        let view = boot_view(cx, state, WorkspaceMode::Developer);
        cx.update_entity(&view, |this, cx| {
            this.active_tab = PanelTab::Preview; // the “Editor” tab (T192 product cut)
            // The drag-vs-memory path (T218) writes through the view's
            // `update_resize`, which `sim_resize` mirrors; that is what
            // populates `tab_resize_memory`. Direct field assignments
            // skip this contract.
            this.sim_resize(N, cx);
        });

        // Collapse.
        cx.update_entity(&view, |this, cx| {
            this.on_tab_select(PanelTab::Preview, cx);
        });
        cx.update(|cx| {
            assert_eq!(
                cx.global::<SidePanelRightState>().width,
                RAIL_ONLY_WIDTH,
                "Editor: collapse phase must shrink to rail"
            );
        });
        cx.update_entity(&view, |this, _| {
            assert_eq!(
                this.tab_resize_memory.get(&PanelTab::Preview).copied(),
                Some(N),
                "collapse must NOT erase resize memory (T221 §2)"
            );
        });

        // Re-open.
        cx.update_entity(&view, |this, cx| {
            this.on_tab_select(PanelTab::Preview, cx);
        });
        cx.update(|cx| {
            assert_eq!(
                cx.global::<SidePanelRightState>().width,
                N,
                "Editor: re-open phase must restore remembered width N, \
                 not the tab's preferred 560"
            );
        });
    }

    /// (5) Dock ON + same-tab click: collapse to rail-only, dock stays ON.
    /// Repeated click re-opens at the remembered width. T289: dock is an
    /// exclusive-zone flag — it does NOT make content permanently visible,
    /// so same-tab clicks under dock behave identically to under dock-off
    /// (collapse if open, re-open if collapsed), with `dock_content` untouched.
    #[gpui::test]
    async fn on_tab_select_same_tab_while_docked_collapses_then_reopens(cx: &mut TestAppContext) {
        const PINNED: f32 = 700.0;
        let mut state = SidePanelRightState::default();
        state.dock_content = true;
        state.width = PINNED;
        let view = boot_view(cx, state, WorkspaceMode::Developer);
        cx.update_entity(&view, |this, _| {
            this.active_tab = PanelTab::Preview; // resizable → has resize memory
        });
        // Record the pinned width through the drag path (T218).
        cx.update_entity(&view, |this, cx| {
            this.sim_resize(PINNED, cx);
        });

        // Click 1: dock ON + same tab → collapse to rail-only, dock stays ON.
        cx.update_entity(&view, |this, cx| {
            this.on_tab_select(PanelTab::Preview, cx);
        });
        cx.update(|cx| {
            let s = cx.global::<SidePanelRightState>();
            assert_eq!(
                s.width, RAIL_ONLY_WIDTH,
                "dock ON + same-tab click must collapse to rail-only"
            );
            assert!(s.dock_content, "dock must remain ON after collapse");
        });

        // Click 2: dock ON + same tab again → re-open at remembered width.
        cx.update_entity(&view, |this, cx| {
            this.on_tab_select(PanelTab::Preview, cx);
        });
        cx.update(|cx| {
            let s = cx.global::<SidePanelRightState>();
            assert_eq!(
                s.width, PINNED,
                "dock ON + same-tab re-click must re-open at remembered width"
            );
            assert!(s.dock_content, "dock must remain ON after re-open");
        });
    }

    /// (6) Belt-and-braces: under dock, a **different** tab click still
    /// works (it's just a tab switch — content stays because dock keeps it).
    /// Defends the spec's “click on another icon is a tab switch” contract
    /// from drifting into “dock forbids all rail clicks”.
    #[gpui::test]
    async fn on_tab_select_different_tab_while_docked_still_switches(cx: &mut TestAppContext) {
        const PINNED: f32 = 700.0;
        let mut state = SidePanelRightState::default();
        state.width = PINNED;
        state.dock_content = true;
        let view = boot_view(cx, state, WorkspaceMode::Developer);

        cx.update_entity(&view, |this, cx| {
            this.on_tab_select(PanelTab::Files, cx);
        });

        cx.update(|cx| {
            let state = cx.global::<SidePanelRightState>();
            assert_eq!(
                state.width, PINNED,
                "different-tab click under dock must not resize"
            );
            assert!(
                state.dock_content,
                "dock mode must be unchanged on tab switch"
            );
        });
        cx.update_entity(&view, |this, _| {
            assert_eq!(
                this.active_tab,
                PanelTab::Files,
                "different-tab click must still switch the active tab"
            );
        });
    }

    // ── T276: resize bookkeeping moved to plain methods (rail window calls
    // these through the shared weak entity — see rail_view.rs) ──

    #[gpui::test]
    async fn start_resize_arms_the_drag_for_a_resizable_tab(cx: &mut TestAppContext) {
        let mut state = SidePanelRightState::default();
        state.width = 400.0;
        let view = boot_view(cx, state, WorkspaceMode::Developer);
        cx.update_entity(&view, |this, cx| {
            this.active_tab = PanelTab::Preview; // resizable (T218)
            this.start_resize(2.0, cx);
        });
        cx.update_entity(&view, |this, _| {
            assert_eq!(this.resize_start_x, Some(2.0));
            assert_eq!(this.resize_start_width, Some(400.0));
        });
        cx.update(|cx| {
            assert!(cx.global::<SidePanelRightState>().resizing);
        });
    }

    #[gpui::test]
    async fn start_resize_from_rail_only_expands_first(cx: &mut TestAppContext) {
        let view = boot_view(cx, SidePanelRightState::default(), WorkspaceMode::Developer);
        cx.update_entity(&view, |this, cx| {
            this.active_tab = PanelTab::Preview; // preferred 560 (DEFAULT_CONTENT_WIDTH)
            this.start_resize(2.0, cx);
        });
        cx.update(|cx| {
            assert_eq!(
                cx.global::<SidePanelRightState>().width,
                PanelTab::Preview.preferred_content_width(),
                "handle press at rail-only must expand to the tab's natural width first"
            );
        });
    }

    #[gpui::test]
    async fn start_resize_ignores_a_fixed_width_tab(cx: &mut TestAppContext) {
        let view = boot_view(cx, SidePanelRightState::default(), WorkspaceMode::Developer);
        cx.update_entity(&view, |this, cx| {
            this.active_tab = PanelTab::System; // fixed-width (T218)
            this.start_resize(2.0, cx);
        });
        cx.update_entity(&view, |this, _| {
            assert_eq!(
                this.resize_start_x, None,
                "fixed-width tab must not arm a drag"
            );
        });
    }

    #[gpui::test]
    async fn update_resize_moves_width_by_the_drag_delta(cx: &mut TestAppContext) {
        let mut state = SidePanelRightState::default();
        state.width = 400.0;
        let view = boot_view(cx, state, WorkspaceMode::Developer);
        cx.update_entity(&view, |this, cx| {
            this.active_tab = PanelTab::Preview;
            this.start_resize(2.0, cx);
            this.update_resize(2.0 - 50.0, cx); // moved 50px left → grow
        });
        cx.update(|cx| {
            assert_eq!(cx.global::<SidePanelRightState>().width, 450.0);
        });
    }

    #[gpui::test]
    async fn end_resize_clears_bookkeeping_and_resizing_flag(cx: &mut TestAppContext) {
        let mut state = SidePanelRightState::default();
        state.width = 400.0;
        let view = boot_view(cx, state, WorkspaceMode::Developer);
        cx.update_entity(&view, |this, cx| {
            this.active_tab = PanelTab::Preview;
            this.start_resize(2.0, cx);
            this.end_resize(cx);
        });
        cx.update_entity(&view, |this, _| {
            assert_eq!(this.resize_start_x, None);
            assert_eq!(this.resize_start_width, None);
        });
        cx.update(|cx| {
            assert!(!cx.global::<SidePanelRightState>().resizing);
        });
    }

    #[test]
    fn needs_width_resize_still_serves_side_panel_left() {
        // T276 retired this panel's own use of the guard (fixed-size
        // surfaces), but `side_panel_left::mod::render` still calls it —
        // regression coverage kept alive here.
        assert!(needs_width_resize(40.0, 320.0));
        assert!(!needs_width_resize(320.0, 320.0));
        assert!(!needs_width_resize(320.4, 320.0));
    }

    /// T289: dock toggle is a pure flag flip — it never calls
    /// `ensure_content_width`. Width is unchanged regardless of dock state.
    #[gpui::test]
    async fn toggle_dock_flips_flag_without_changing_width(cx: &mut TestAppContext) {
        // (1) Collapsed + dock toggle ON → dock=true, width stays rail-only.
        let view = boot_view(cx, SidePanelRightState::default(), WorkspaceMode::Developer);
        cx.update_entity(&view, |this, cx| {
            this.active_tab = PanelTab::Files;
            this.toggle_dock(cx);
        });
        cx.update(|cx| {
            let state = cx.global::<SidePanelRightState>();
            assert!(state.dock_content);
            assert_eq!(
                state.width, RAIL_ONLY_WIDTH,
                "dock toggle from collapsed must not auto-open content"
            );
        });

        // (2) Open (dock OFF) + dock toggle ON → dock=true, width does not jump.
        let mut state = SidePanelRightState::default();
        state.width = 480.0;
        let view = boot_view(cx, state, WorkspaceMode::Developer);
        cx.update_entity(&view, |this, cx| {
            this.toggle_dock(cx);
        });
        cx.update(|cx| {
            let s = cx.global::<SidePanelRightState>();
            assert!(s.dock_content);
            assert_eq!(
                s.width, 480.0,
                "dock toggle ON from open must not change width"
            );
        });
    }
}

/// Pure decision: re-issue `window.resize` while the compositor's actual
/// width has not caught up to the target. T276 removed this panel's own
/// need for it (`content`/`rail` are fixed-size surfaces now — see the
/// module doc), but `side_panel_left::mod::render` still calls it for its
/// own (unrelated, still-resizing) surface — kept here rather than deleted
/// out from under that caller.
pub(crate) fn needs_width_resize(actual: f32, target: f32) -> bool {
    (actual - target).abs() > 1.0
}

#[allow(dead_code)]
pub(crate) fn peek_leave_debounce() -> Duration {
    PEEK_LEAVE_DEBOUNCE
}

pub(crate) fn schedule_release_from_app(cx: &mut gpui::App, generation: u64) {
    cx.spawn(async move |app_cx: &mut AsyncApp| {
        app_cx
            .background_executor()
            .timer(PEEK_LEAVE_DEBOUNCE)
            .await;
        app_cx.update(|app_cx| {
            if app_cx
                .global::<crate::side_panel_right::SidePanelRightState>()
                .peek_generation
                != generation
            {
                return;
            }
            crate::side_panel_right::close_peek_if_not_pinned(app_cx);
        });
    })
    .detach();
}
