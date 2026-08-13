//! Right side panel view — sidebar v2 (mockup → layout, flagship rsx sections).
//!
//! ## `on_hover` / animation split (fork rule)
//! Our gpui fork stores a **single** `Option` hover handler per element and
//! `debug_assert!`s if `.on_hover` is set twice. Consequences:
//! - Root node: **only** the peek close-debounce `on_hover` (this file).
//! - Children: **no** extra root hover.
//! - Peek motion: state-driven `.transition_when` on an **inner** wrapper.

use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

use chronos_services::net_stats::{self, NetState};
use gpui::{
    AnimationExt, AsyncApp, IntoElement, Render,
    Window, div, prelude::*, px,
};

use crate::agent_follow::AgentFollowState;
use crate::edit_mode;
use crate::motion;
use crate::side_panel_right::power_row::{
    ARM_TIMEOUT, ArmState, PowerAction, is_confirming_click, on_click as arm_on_click, on_timeout,
    render_footer,
};
use crate::side_panel_right::preview_target::PreviewTarget;
use crate::side_panel_right::surfaces;
use crate::side_panel_right::tab::TabContent;
use crate::side_panel_right::tab::system::format_net_pair;
use crate::side_panel_right::panels_config;
use crate::side_panel_right::tabs::PanelTab;
use crate::side_panel_right::{
    HANDLE_WIDTH, MAX_WIDTH, RAIL_ONLY_WIDTH, RightPanelResize, SidePanelRightState,
};
use crate::state::AppState;
use crate::{scene, workspace_mode};

use chronos_ui::{Theme, WindowRootExt, elevation_glow_bar};

/// Delay before peek-close after mouse leaves panel (or strip).
const PEEK_LEAVE_DEBOUNCE: Duration = Duration::from_millis(280);

pub struct SidePanelRightView {
    power_arm: ArmState,
    net_state: NetState,
    net_dl_history: crate::side_panel_right::spectrum_row::SpectrumHistory,
    net_ul_history: crate::side_panel_right::spectrum_row::SpectrumHistory,
    active_tab: PanelTab,
    /// Width the platform window was last physically resized to. `render`
    /// only issues `window.resize()` when `state.width` has drifted from
    /// this, avoiding redundant Wayland round-trips.
    last_resized_width: f32,
    /// Last exclusive zone value we pushed to the compositor. Only
    /// `window.set_exclusive_zone()` when it changes.
    last_exclusive_zone: Option<f32>,
    resize_start_x: Option<f32>,
    /// Width at drag start (T214: anchor model, not frame-to-frame).
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
            power_arm: ArmState::default(),
            net_state: NetState::default(),
            net_dl_history: Default::default(),
            net_ul_history: Default::default(),
            active_tab: PanelTab::default(),
            last_resized_width: RAIL_ONLY_WIDTH,
            last_exclusive_zone: None,
            resize_start_x: None,
            resize_start_width: None,
            tab_views: HashMap::new(),
            tab_resize_memory: HashMap::new(),
            _preview_target_subscription: preview_target_subscription,
            _follow_subscription: cx.observe_global::<AgentFollowState>(|_, cx| cx.notify()),
        }
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
        let content_open = state.dock_content || state.width > RAIL_ONLY_WIDTH + 1.0;
        if content_open {
            let changed = state.width != target;
            state.ensure_content_width(target);
            if changed {
                self.last_resized_width = f32::NAN;
            }
        } else {
            state.last_exclusive_zone = None;
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

    fn start_resize(&mut self, start_x: f32, cx: &mut Context<Self>) {
        // T210: suppress peek-close for the press lifetime.
        cx.global_mut::<SidePanelRightState>().resizing = true;
        let w = cx.global::<SidePanelRightState>().width;
        let tab = self.active_tab;
        let resizable = tab.resizable();

        // Pressing the handle rail-only opens the tab at its natural width. This
        // happens for EVERY tab, fixed-width ones included — it is the affordance
        // that opens content, not a resize.
        //
        // It is also what makes a drag possible at all: rail-only the window is
        // 40px wide, so the first pointer motion leaves the surface before GPUI
        // has started a drag session and `on_drag_move` never fires (measured:
        // `resize drag started` with zero `resize drag move` when the expand was
        // removed). Expanding first gives the gesture a surface to be born on.
        if w <= RAIL_ONLY_WIDTH + 1.0 {
            let target = self.active_tab_width(tab, cx);
            let state = cx.global_mut::<SidePanelRightState>();
            state.width = target;
            state.last_exclusive_zone = None;
            self.last_resized_width = f32::NAN;
            if resizable {
                self.tab_resize_memory.insert(tab, target);
            }
            tracing::info!(
                width = target,
                tab = tab.label(),
                "side_panel_right: handle grab expanded rail → content"
            );
            cx.notify();
        }

        // T218: arm the drag only for tabs the user may resize. Fixed-width tabs
        // leave the anchors unset, so `update_resize` returns immediately and the
        // width stays exactly `preferred_content_width`.
        if !resizable {
            self.resize_start_x = None;
            self.resize_start_width = None;
            tracing::debug!(tab = tab.label(), "side_panel_right: tab is fixed width, drag ignored");
            return;
        }

        // T216 grab offset. After a rail→content expand the left edge slid left by
        // (target - w) while the cursor stayed put, so the same physical point now
        // sits that much deeper into the window; without re-expressing it the first
        // event would read the cursor as deep in the body and snap back to rail.
        let width_now = cx.global::<SidePanelRightState>().width;
        self.resize_start_x = Some(start_x + (width_now - w));
        self.resize_start_width = Some(width_now);
        tracing::debug!(
            grab_x = ?self.resize_start_x,
            start_w = w,
            tab = tab.label(),
            "side_panel_right: resize drag started"
        );
    }

    fn update_resize(&mut self, current_x: f32, window: &Window, cx: &mut Context<Self>) {
        let Some(grab) = self.resize_start_x else {
            return;
        };
        // T216 — self-correcting absolute model. The panel is anchored to the RIGHT
        // screen edge, so width growth drags the window's LEFT edge leftward, and
        // `current_x` (window-local) is measured from that moving edge.
        //
        //   pointer_screen = window_left + current_x,  window_left = screen_right - actual_w
        //   keep the grabbed point under the cursor  =>  desired_left = pointer_screen - grab
        //   desired_w = screen_right - desired_left  =  actual_w - current_x + grab
        //
        // `actual_w` MUST come from the surface the pointer coords are relative to
        // (window.bounds()), not from our own state: `window.resize()` is async on
        // Wayland, so state runs ahead of the compositor between configure acks.
        // Feeding state width back in is what made T210/T214 accumulate error and
        // thrash to both clamps; reading live geometry makes a stale frame merely
        // repeat the same target instead of compounding it.
        let actual_w = window.bounds().size.width.as_f32();
        let new_w = crate::side_panel_right::resize_target_width(actual_w, current_x, grab);
        let state = cx.global_mut::<SidePanelRightState>();
        state.resize(new_w);
        state.last_exclusive_zone = None;
        self.tab_resize_memory.insert(self.active_tab, state.width);
        crate::side_panel_right::hold_peek(cx);
        tracing::trace!(current_x, actual_w, new_w, "side_panel_right: resize drag move");
        cx.notify();
    }

    /// Sample network speed on every render. Time-gated by
    /// `update_speed`'s `SAMPLE_INTERVAL` — history only advances when a
    /// real sample lands (not every paint with a cached value).
    fn sample_network(&mut self) {
        let Ok((rx, tx)) = net_stats::read_interface_bytes() else {
            return;
        };
        let prev_t = self.net_state.sample.as_ref().map(|s| s.time);
        let _speed = net_stats::update_speed(
            &mut self.net_state,
            rx,
            tx,
            Instant::now(),
            net_stats::SAMPLE_INTERVAL,
        );
        let new_t = self.net_state.sample.as_ref().map(|s| s.time);
        if prev_t != new_t {
            crate::side_panel_right::spectrum_row::push_sample(
                &mut self.net_dl_history,
                self.net_state.cached_dl as f32,
            );
            crate::side_panel_right::spectrum_row::push_sample(
                &mut self.net_ul_history,
                self.net_state.cached_ul as f32,
            );
        }
    }

    pub(crate) fn on_power_click(&mut self, action: PowerAction, cx: &mut Context<Self>) {
        if is_confirming_click(&self.power_arm, action) {
            match action {
                PowerAction::LogOut => AppState::power(cx).log_out(),
                PowerAction::Restart => AppState::power(cx).restart(),
                PowerAction::Shutdown => AppState::power(cx).shutdown(),
            }
            tracing::info!("side_panel_right: power confirmed {action:?}");
            self.power_arm = ArmState::Idle;
            cx.notify();
            return;
        }

        let armed = arm_on_click(self.power_arm, action);
        self.power_arm = armed;
        tracing::info!("side_panel_right: power armed {action:?}");
        cx.notify();

        cx.spawn(async move |view, cx| {
            cx.background_executor().timer(ARM_TIMEOUT).await;
            match view.update(cx, |view, cx| {
                if view.power_arm == armed {
                    view.power_arm = on_timeout(armed);
                    tracing::info!("side_panel_right: power arm timeout → Idle");
                    cx.notify();
                }
            }) {
                Ok(()) => {}
                Err(e) => tracing::warn!(
                    "side_panel_right: power arm timeout could not disarm ({e}) — \
                     a power button may still read 'Confirm?'"
                ),
            }
        })
        .detach();
    }

    pub(crate) fn on_tab_select(&mut self, tab: PanelTab, cx: &mut Context<Self>) {
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

        let (dock_content, content_open) = {
            let state = cx.global::<SidePanelRightState>();
            (
                state.dock_content,
                state.dock_content || state.width > RAIL_ONLY_WIDTH + 1.0,
            )
        };

        if tab != self.active_tab {
            // Branch 4 — different tab.
            //
            // Under dock: only switch `active_tab`. The dock button ⊞/⊟
            // and the resize handle are the only knobs for width when
            // content is always-visible; switching tabs in dock mode must
            // not undo a pinned width. (Otherwise clicking another icon
            // would collapse from 700 to 440 — surprising for any tab
            // whose `preferred_content_width` differs from the docked
            // panel width.)
            //
            // Off dock: switch AND force-open at the new tab's natural /
            // remembered width. We bypass `apply_active_tab_width` because
            // it is a no-op when content is collapsed (the T171 "trap #3"
            // — only mode-fallback needs to re-cover the width silently).
            // A user clicking a *different* rail icon requires the new
            // tab to be visible — that's the whole affordance, and a click
            // while collapsed that only updates `active_tab` would be
            // invisible.
            self.active_tab = tab;
            self.ensure_tab_view(self.active_tab, cx);
            if dock_content {
                self.last_resized_width = f32::NAN;
                cx.notify();
                tracing::info!(
                    tab = tab.label(),
                    "side_panel_right: switched tab under dock (width pinned)"
                );
                return;
            }
            let target = self.active_tab_width(self.active_tab, cx);
            cx.global_mut::<SidePanelRightState>().ensure_content_width(target);
            self.last_resized_width = f32::NAN;
            cx.notify();
            tracing::info!(
                tab = tab.label(),
                width = target,
                "side_panel_right: switched tab → opened at per-tab width"
            );
            return;
        }

        // Same tab clicked.
        if dock_content {
            // Branch 1.
            tracing::debug!(
                tab = tab.label(),
                "side_panel_right: same active tab click while docked → no-op (⊞/⊟ is the dock knob)"
            );
            return;
        }

        if content_open {
            // Branch 2 — collapse. `tab_resize_memory` stays intact because
            // we only touch `state.width` and `state.last_exclusive_zone`.
            cx.global_mut::<SidePanelRightState>().width = RAIL_ONLY_WIDTH;
            cx.global_mut::<SidePanelRightState>().last_exclusive_zone = None;
            self.last_resized_width = f32::NAN;
            cx.notify();
            tracing::info!(
                tab = tab.label(),
                "side_panel_right: same tab → collapsed to rail (memory preserved)"
            );
        } else {
            // Branch 3 — re-open at the tab's stored width.
            let target = self.active_tab_width(self.active_tab, cx);
            cx.global_mut::<SidePanelRightState>().ensure_content_width(target);
            self.last_resized_width = f32::NAN;
            cx.notify();
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
    pub(crate) fn ensure_tab_view(
        &mut self,
        tab: PanelTab,
        cx: &mut Context<Self>,
    ) -> TabContent {
        self.tab_views
            .entry(tab)
            .or_insert_with(|| TabContent::create(tab, cx))
            .clone()
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
        self.sample_network();
        // T219: resolve rail groups from panels.toml (two groups: top + bottom).
        // Falls back to panels_config defaults which mirror the old for_mode.
        let current_mode = workspace_mode::current(cx);
        let panel_cfg = panels_config::cached();
        let (top_tabs, bottom_tabs) = panels_config::resolve_grouped(current_mode, &panel_cfg);
        // Flatten for active-tab validation.
        let mut all_tabs: Vec<PanelTab> = top_tabs.iter().chain(bottom_tabs.iter()).copied().collect();
        all_tabs.dedup();
        // Active tab left the set after a mode switch — land on System, keep
        // the panel open (§5: must not discard panel state / close on mode change).
        // Resolve the fallback and apply System's per-tab width. The resolver
        // is a no-op on the next render once the active tab belongs to the rail.
        if self.resolve_active_tab(&all_tabs, cx) {
            // System may not have been visited yet — ensure the entry exists
            // before the render path reads it via get().
            self.ensure_tab_view(PanelTab::System, cx);
        }
        let power_arm = self.power_arm;

        let dl = format_net_pair(self.net_state.cached_dl, 0.0);
        let ul = format_net_pair(0.0, self.net_state.cached_ul);
        let net_summary = format!("↓ {dl}  ↑ {ul}");

        // --- Exclusive zone & width sync (mirror T126 left panel) ---
        let panel_state = cx.global::<SidePanelRightState>();
        let dock_content = panel_state.dock_content;
        let panel_width = panel_state.width;

        // Calculate exclusive zone: dock ON → full width, dock OFF → rail only
        let rail_only_width = RAIL_ONLY_WIDTH;
        let new_zone = if dock_content {
            panel_width
        } else {
            rail_only_width
        };

        // Update exclusive zone only when it changes (avoid redundant syscalls)
        if self.last_exclusive_zone != Some(new_zone) {
            window.set_exclusive_edge(gpui::layer_shell::Anchor::RIGHT);
            window.set_exclusive_zone(px(new_zone));
            self.last_exclusive_zone = Some(new_zone);
        }

        // T243 trace: record the width-sync inputs on EVERY render so a live
        // repro (toggle/select-tab ×10) can be read frame-by-frame. Key
        // questions the log must answer: does `window.resize()` fire at all,
        // and does `actual_width` ever converge to `panel_width` (compositor
        // ack) or stay at the rail width (async resize never committing).
        let actual_width_pre = window.bounds().size.width.as_f32();
        tracing::debug!(
            last_resized_width = self.last_resized_width,
            panel_width,
            actual_width = actual_width_pre,
            dock_content,
            "T243 render: width sync check"
        );

        // Resize window if the compositor's ACTUAL width hasn't caught up to
        // the target (`panel_width`). T243: `window.resize()` is async on
        // Wayland — the old guard compared `last_resized_width` (a state
        // copy) against `panel_width` (also state), so a configure that
        // never landed left last==target while the surface sat at rail
        // width: the guard read "already resized" and never retried (the
        // w=40 stick, caught live in the T243 trace: last=320 panel=320
        // actual=40 for seconds). Gating on `window.bounds()` (live
        // geometry) re-issues the resize on every render until the
        // compositor acks — self-healing, same principle as `update_resize`
        // (T216: "state runs ahead of the compositor between configure
        // acks"). Once acked, actual==target and the re-issue stops.
        let actual_width = window.bounds().size.width.as_f32();
        if needs_width_resize(actual_width, panel_width) {
            let display_h = crate::monitor::pult_display_info(cx)
                .map(|d| f32::from(d.bounds().size.height))
                .or_else(|| window.display(cx).map(|d| f32::from(d.bounds().size.height)))
                .unwrap_or(1080.);
            let panel_h =
                (display_h - crate::side_panel_right::panel_edge_gap()).max(100.);
            window.resize(gpui::Size::new(px(panel_width), px(panel_h)));
            self.last_resized_width = panel_width;
            // T216: no drag-anchor fixup here. `update_resize` re-derives the target
            // from live window geometry each event, so shifting the grab offset by Δw
            // (T214) would double-count the very same edge movement.
            tracing::debug!(
                panel_width,
                panel_h,
                actual_width,
                "T243 render: window.resize after width change"
            );
        }

        // Content open when dock is ON OR the panel is past rail+handle
        // threshold. Gated on the ACTUAL window width (`window.bounds()`),
        // not `panel_width` (the target state) — `window.resize()` is
        // async on Wayland (T216, see `update_resize` above: "state runs
        // ahead of the compositor between configure acks"). Gating on the
        // target let the content column vanish/appear a frame or two
        // before the compositor actually committed the new width: the
        // rail visibly reflowed inside a still-stale-sized window (the
        // "wobble" on close), and the old content's hit-test region
        // could still catch a click meant for the rail during that gap.
        let actual_width = window.bounds().size.width.as_f32();
        let content_open = dock_content || actual_width > rail_only_width + 1.0;
        // T243 trace: content_open transition (wobble symptom on close).
        tracing::debug!(
            content_open,
            actual_width,
            panel_width,
            dock_content,
            "T243 render: content_open computed from live bounds"
        );

        // T217 — top-corner radius where the panel meets the bar. Each corner
        // is decided independently: one the bar sits above stays square (the
        // panel tucks under the bar's bottom edge — rounding would seam), a
        // free one rhymes with the bar's pill radius. Re-evaluated every
        // render, so bar hot-reload (radius / extent) and panel resizes take
        // effect live without a window recreate.
        let display_w = crate::monitor::pult_display_info(cx)
            .map(|d| f32::from(d.bounds().size.width))
            .or_else(|| window.display(cx).map(|d| f32::from(d.bounds().size.width)))
            .unwrap_or(1920.);
        let corner_tl = crate::state::panel_corner_radius(display_w - panel_width);
        let corner_tr = crate::state::panel_corner_radius(display_w);

        // Elevated chrome на content-колонке (не rail-only) — общий язык
        // глубины из `theme.elevation_popup()` (T128).
        let theme = *Theme::global(cx);
        let elev = theme.elevation_popup();

        // Resize handlers before any RPIT that captures cx (Rust 2024).
        let resize_drag_handler = cx.listener(
            |this, ev: &gpui::DragMoveEvent<RightPanelResize>, window, cx| {
                this.update_resize(f32::from(ev.event.position.x), window, cx);
            },
        );
        let resize_mouse_handler = cx.listener(|this, ev: &gpui::MouseDownEvent, _w, cx| {
            this.start_resize(f32::from(ev.position.x), cx);
        });
        // T210: mouse-up on the handle means the drag ended. Expansion already
        // happened on mouse-down (see `start_resize`), so this is pure cleanup.
        let resize_mouse_up_handler = cx.listener(|this, _ev: &gpui::MouseUpEvent, _w, cx| {
            cx.global_mut::<SidePanelRightState>().resizing = false;
            this.resize_start_x = None;
            this.resize_start_width = None;
            tracing::info!("side_panel_right: resize drag ended (mouse-up)");
            cx.notify();
        });

        // Lazy tab view — created on first paint, cached thereafter.
        // ensure_tab_view() avoids expect-panic on the very first render
        // (before any on_tab_select has fired). T168 errata 3.
        let active = self.active_tab;
        let tab_entry = self.ensure_tab_view(active, cx);

        // OUTER: sole window-level `on_hover` (debounce). No transition_on_hover.
        // Layout: [handle | content? | rail] — flex handle so drag hits work
        // (absolute overlay was unhittable / broke resize entirely).
        //
        // T217: round the top corners + clip when either corner is free (bar
        // does not reach it). Root's own background is transparent (the
        // gpui-component Root base was overridden to transparent in
        // `open_window`), so the rounded cutouts show the desktop behind, not
        // a square chrome corner. When both corners are covered (full-width
        // bar) no rounding and no clip — keeps the elevation shadows intact.
        div()
            .id("side-panel-right-root")
            .window_font(&theme)
            .size_full()
            .flex()
            .flex_row()
            .when(corner_tl > 0.0 || corner_tr > 0.0, |d| {
                d.rounded_tl(px(corner_tl))
                    .rounded_tr(px(corner_tr))
                    .overflow_hidden()
            })
            .on_hover(|hovered, _window, cx| {
                if *hovered {
                    crate::side_panel_right::hold_peek(cx);
                } else {
                    crate::side_panel_right::schedule_release_peek(cx);
                }
            })
            .child(
                // 4px ghost hit strip — transparent (no gray lip). Real flex
                // column so on_drag receives events.
                //
                // T218: the strip stays in the layout for every tab (rail-only
                // width is rail + handle, and mouse-down here is what opens the
                // content), but on a fixed-width tab it advertises no resize —
                // no col-resize cursor, no drag session to start.
                div()
                    .id("side-panel-right-resize-handle")
                    .flex_none()
                    .w(px(HANDLE_WIDTH))
                    .h_full()
                    .bg(gpui::transparent_black())
                    .on_mouse_down(gpui::MouseButton::Left, resize_mouse_handler)
                    .on_mouse_up(gpui::MouseButton::Left, resize_mouse_up_handler)
                    .when(active.resizable(), |h| {
                        h.cursor_col_resize()
                            .on_drag(RightPanelResize, |_, _, _, cx| cx.new(|_| gpui::EmptyView))
                            .on_drag_move(resize_drag_handler)
                    }),
            )
            .child(
                div()
                    .id("side-panel-body")
                    .relative()
                    .flex_1()
                    .min_w(px(0.))
                    .h_full()
                    // Chrome bg only when content is open — in rail-only mode
                    // the rail provides its own background. Transparent body
                    // eliminates the gray lip next to the handle.
                    .when(content_open, |b| {
                        b.bg(surfaces::chrome(&theme))
                            .border_l_1()
                            .border_color(theme.border.subtle)
                    })
                    .flex()
                    .flex_row() // content first, rail last — rail flush against the screen's right edge
                    .overflow_hidden()
                    .when(content_open, |body| {
                        body.child({
                            let col = div()
                                .id("side-panel-content-column")
                                .flex_1()
                                .min_w(px(0.))
                                .flex()
                                .flex_col()
                                .overflow_hidden()
                                .bg(surfaces::content(&theme))
                                .shadow(elev.shadows.to_vec());
                            // Light-C glow-ребро на верхней кромке content-колонки.
                            let col = match elev.glow {
                                Some(glow) => col.child(elevation_glow_bar(glow)),
                                None => col,
                            };
                            // --- Tab content ---
                            let content_el = match tab_entry {
                                TabContent::System(entity) => {
                                    col.child(entity.clone())
                                        // Footer: power + net summary (stays on
                                        // SidePanelRightView because render_footer
                                        // takes Context<SidePanelRightView>).
                                        .child(render_footer(
                                            &net_summary,
                                            power_arm,
                                            cx,
                                        ))
                                }
                                TabContent::Files(entity) => {
                                    col.child(entity.clone())
                                }
                                TabContent::Terminal(entity) => {
                                    col.child(entity.clone())
                                }
                                TabContent::Build(entity) => {
                                    col.child(entity.clone())
                                }
                                // T179: minimum addition to keep the enum
                                // exhaustive; pairs with the same one-line match
                                // arm in `tab_entity_id` below. View body itself
                                // stays outside T179's zone.
                                TabContent::Preview(entity) => {
                                    col.child(entity.clone())
                                }
                                // T188: Library is a real entity (Gamer hub).
                                TabContent::Library(entity) => {
                                    col.child(entity.clone())
                                }
                                // T193: Hyprland binds (read-only list).
                                TabContent::HyprBinds(entity) => {
                                    col.child(entity.clone())
                                }
                                // T202: System settings «Bar» page.
                                TabContent::BarSettings(entity) => {
                                    col.child(entity.clone())
                                }
                                TabContent::AcpSettings(entity) => {
                                    col.child(entity.clone())
                                }
                                TabContent::Placeholder(entity) => {
                                    col.child(entity.clone())
                                }
                            };
                            // Enter animation belongs to the content column
                            // alone — it must NOT wrap the rail (rail is a
                            // permanent sibling, not something that enters).
                            // Previously `.with_animation` sat on the outer
                            // `side-panel-body` div below, which included the
                            // rail as a child; every content_open toggle (e.g.
                            // closing a tab) replayed the EaseOutBack
                            // slide-and-overshoot on the rail too, reading as
                            // an unwanted "wobbly stretch" on close.
                            content_el.with_animation(
                                "side-panel-content-enter",
                                motion::enter_animation(),
                                motion::apply_enter_from_right,
                            )
                        })
                    })
                    .child({
                        let active = self.active_tab;
                        let this = cx.entity();
                        let editing = edit_mode::is_active(cx);
                        let this_for_select = this.clone();
                        let on_select = std::rc::Rc::new(
                            move |tab: PanelTab, _window: &mut Window, cx: &mut gpui::App| {
                                this_for_select.update(cx, |this, cx| {
                                    this.on_tab_select(tab, cx);
                                });
                            },
                        );
                        let this_for_dock = this.clone();
                        let on_dock_toggle =
                            std::rc::Rc::new(move |_window: &mut Window, cx: &mut gpui::App| {
                                this_for_dock.update(cx, |this, cx| {
                                    let target = this.active_tab_width(this.active_tab, cx);
                                    let state = cx.global_mut::<SidePanelRightState>();
                                    state.dock_content = !state.dock_content;
                                    state.ensure_content_width(target);
                                    this.last_resized_width = f32::NAN;
                                    tracing::info!(
                                        dock = state.dock_content,
                                        width = state.width,
                                        "side_panel_right: dock toggle"
                                    );
                                    cx.notify();
                                });
                            });
                        // T219: on_move callback. The closure is intentionally
                        // trivial: it looks up the current workspace mode and
                        // delegates to `panels_config::move_tab`, which owns
                        // the cache → disk → refresh_windows pipeline. Putting
                        // all the IO in one place means the unit tests in
                        // `panels_config::tests` exercise *the* code that ships
                        // — re-writing the same glue inside a test (the T164
                        // anti-pattern) is no longer a temptation.
                        let on_move: std::rc::Rc<dyn Fn(PanelTab, isize, &mut gpui::App)> =
                            std::rc::Rc::new(move |tab: PanelTab, delta: isize, cx: &mut gpui::App| {
                                let mode = workspace_mode::current(cx);
                                panels_config::move_tab(cx, mode, tab, delta);
                            });
                        crate::side_panel_right::rail::render_rail(
                            cx,
                            &top_tabs,
                            &bottom_tabs,
                            active,
                            on_select,
                            dock_content,
                            on_dock_toggle,
                            content_open,
                            editing,
                            on_move,
                        )
                    }),
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
    // correct via its single caller in `view.rs`.

    #[test]
    fn width_resize_guard_retries_until_compositor_acks() {
        // T243: the old guard compared state-to-state (last_resized_width vs
        // panel_width) and never retried a lost async configure. The new guard
        // compares live geometry: it re-issues while actual != target and
        // stops once acked.
        // Stuck state caught live: last=320 panel=320 actual=40 — must retry.
        assert!(needs_width_resize(40.0, 320.0));
        // Acked: actual caught up — stop.
        assert!(!needs_width_resize(320.0, 320.0));
        // Sub-pixel drift — don't thrash set_size.
        assert!(!needs_width_resize(320.4, 320.0));
        // Shrink direction (close): window still wide, target rail — retry.
        assert!(needs_width_resize(320.0, 40.0));
        assert!(!needs_width_resize(40.0, 40.0));
    }

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

        let before = cx.read(|cx| panels_config::cached());
        cx.update(|cx| {
            assert!(
                !panels_config::move_tab(cx, WorkspaceMode::Developer, PanelTab::Library, -1),
                "Library is not in the Developer rail — helper must report no-op"
            );
        });
        let after = cx.read(|cx| panels_config::cached());
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
    // resize — `state.width` is the truth under test, the Wayland surface
    // tracks it on the next paint (T216/T218 already cover that contract).

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
                cx.global::<SidePanelRightState>().width, N,
                "Editor: re-open phase must restore remembered width N, \
                 not the tab's preferred 560"
            );
        });
    }

    /// (5) Dock wins: with `dock_content = true`, clicking the active tab
    /// is a no-op. The dock button ⊞/⊟ is the only knob for docked mode —
    /// mixing rail-icon-toggling into dock would create the inconsistent
    /// «always-visible but rail-only» state we explicitly forbid.
    #[gpui::test]
    async fn on_tab_select_active_tab_while_docked_is_noop(cx: &mut TestAppContext) {
        const PINNED: f32 = 700.0;
        let mut state = SidePanelRightState::default();
        state.width = PINNED;
        state.dock_content = true;
        let view = boot_view(cx, state, WorkspaceMode::Developer);
        cx.update_entity(&view, |this, _| {
            this.active_tab = PanelTab::Preview;
        });

        cx.update_entity(&view, |this, cx| {
            this.on_tab_select(PanelTab::Preview, cx);
        });

        cx.update(|cx| {
            let state = cx.global::<SidePanelRightState>();
            assert_eq!(
                state.width, PINNED,
                "dock + active-tab click must NOT resize"
            );
            assert!(
                state.dock_content,
                "dock mode must remain unchanged (rail-icon click cannot un-dock)"
            );
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
                this.active_tab, PanelTab::Files,
                "different-tab click must still switch the active tab"
            );
        });
    }
}

/// T243: pure decision for the render resize guard — re-issue `window.resize`
/// while the compositor's actual width has not caught up to the target
/// (`panel_width`). Comparing state-to-state (`last_resized_width` vs
/// `panel_width`) let a lost async configure stick the surface at rail width
/// forever; comparing live geometry makes the guard self-healing.
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
