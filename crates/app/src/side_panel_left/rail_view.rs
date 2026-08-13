//! T278 / Slice A1 — the rail window's root view.
//!
//! Mirrors `side_panel_right::rail_view::RailView` (T276): the entity
//! that owns the standalone 40 px rail surface, sets the exclusive zone,
//! and renders the icon column + dock toggle. All mutable panel state
//! (active tab, width, dock) lives in `SidePanelLeftState_`; this view
//! only renders and forwards input via the weak content handle.
//!
//! Hover contract: hover-enter/leave bumps `peek_generation` so the
//! debounce in `mod.rs::schedule_release_peek` cannot close the panel
//! while the cursor is on the rail. (Hover-peek OPEN is disabled per
//! 2026-07-23 — the strip is dormant. The generation guard stays alive
//! for cross-surface continuity: leaving the rail for the content
//! canvas must not fire a stale close.)

use std::rc::Rc;

use gpui::{
    App, Context, IntoElement, Render, Subscription, WeakEntity, Window, div, prelude::*, px,
};

use chronos_ui::{Theme, WindowRootExt};

use crate::side_panel_left::SidePanelLeftState_;
use crate::side_panel_left::tabs::{
    BOTTOM_TAB, LeftTab, PRIMARY_TABS, RAIL_WIDTH, width_for_open,
};
use crate::side_panel_left::workspace_view::WorkspaceView;

pub struct RailView {
    /// Weak handle to the live `WorkspaceView` (lives in the content
    /// window). Used for tab switches and dock toggles triggered from
    /// this window.
    content: WeakEntity<WorkspaceView>,
    /// Held only to keep the subscription alive.
    _content_sub: Subscription,
}

impl RailView {
    pub fn new(content: WeakEntity<WorkspaceView>, cx: &mut Context<Self>) -> Self {
        let sub = cx.observe_global::<SidePanelLeftState_>(|_, cx| cx.notify());
        Self {
            content,
            _content_sub: sub,
        }
    }
}

fn rail_button_bg(is_active: bool, theme: &Theme) -> gpui::Hsla {
    if is_active {
        theme.interactive.hover
    } else {
        gpui::transparent_black()
    }
}

/// Single rail icon button. Returns the element tree; caller wraps it in
/// the rail column. The active accent strip sits on the rail's right
/// edge (the side that meets content), so the visual focus is on the
/// panel's working surface, not the rail itself.
fn render_rail_button(
    tab: LeftTab,
    is_active: bool,
    content: WeakEntity<WorkspaceView>,
    theme: &Theme,
) -> impl IntoElement {
    let icon = div()
        .id(("rail-tab", tab as usize))
        .relative()
        .flex()
        .items_center()
        .justify_center()
        .size(px(28.))
        .rounded(theme.radius)
        .bg(rail_button_bg(is_active, theme))
        .on_click({
            let content = content.clone();
            move |_ev, _window, cx| {
                if let Some(view) = content.upgrade() {
                    view.update(cx, |view, cx| view.on_rail_tab_select(tab, cx));
                }
            }
        })
        .child(
            gpui::svg()
                .path(tab.icon_path())
                .size(px(18.))
                .text_color(if is_active {
                    theme.text.primary
                } else {
                    theme.text.muted
                }),
        )
        .when(is_active, |el| {
            // Active indicator bar — flush against the rail's right edge
            // (the side facing content).
            el.child(
                div()
                    .absolute()
                    .right(px(-4.))
                    .top(px(4.))
                    .w(px(3.))
                    .h(px(20.))
                    .rounded(px(2.))
                    .bg(theme.accent.primary),
            )
        });
    icon.into_any_element()
}

pub fn render_rail(
    cx: &App,
    content: WeakEntity<WorkspaceView>,
) -> impl IntoElement {
    let theme = Theme::global(cx);
    let state = cx.global::<SidePanelLeftState_>();
    let active = state.active_tab;
    let dock_content = state.dock_content;
    drop(state);

    // Project selector (Slice A: top button, fixed). Then PRIMARY_TABS
    // in fixed order, a flex_1 spacer, then BOTTOM_TAB (Archive), then
    // the dock toggle.
    let project = LeftTab::Project;

    div()
        .id("side-panel-left-rail")
        .flex()
        .flex_col()
        .items_center()
        .gap(px(4.))
        .py(px(8.))
        .w(px(RAIL_WIDTH))
        .h_full()
        .bg(theme.bg.primary)
        .border_r_1()
        .border_color(theme.border.subtle)
        .on_hover(|hovered, _window, cx| {
            if *hovered {
                crate::side_panel_left::hold_peek(cx);
            } else {
                crate::side_panel_left::schedule_release_peek(cx);
            }
        })
        // Project selector — sits above PRIMARY_TABS so it can carry a
        // project sigil/initials in Slice A2. A1 keeps it as the same
        // generic rail button.
        .child(render_rail_button(project, active == project, content.clone(), theme))
        // PRIMARY_TABS
        .children(PRIMARY_TABS.iter().copied().filter(|t| *t != project).map({
            let content = content.clone();
            move |tab| render_rail_button(tab, active == tab, content.clone(), theme)
        }))
        // Spacer — pushes Archive down regardless of badge counts.
        .child(div().flex_1())
        // Archive
        .child(render_rail_button(BOTTOM_TAB, active == BOTTOM_TAB, content.clone(), theme))
        // Dock toggle — always last, below Archive.
        .child({
            let on_click_dock = {
                let content = content.clone();
                move |_ev: &gpui::MouseUpEvent, _window: &mut Window, cx: &mut App| {
                    if let Some(view) = content.upgrade() {
                        view.update(cx, |view, cx| view.on_dock_toggle(cx));
                    }
                }
            };
            // Icon convention: the icon shows the ACTION that the click
            // performs. `⊞` enables dock (shown when currently undocked),
            // `⊟` disables dock (shown when currently docked). This is
            // Material Design's affordance-shows-action convention and
            // matches the spec wording `⊞ включает dock, ⊟ выключает`.
            div()
                .id("dock-toggle-left")
                .w(px(28.))
                .h(px(28.))
                .rounded(px(6.))
                .flex()
                .items_center()
                .justify_center()
                .text_size(px(11.))
                .text_color(if dock_content {
                    theme.accent.primary
                } else {
                    theme.text.muted
                })
                .cursor_pointer()
                .hover(|s| s.bg(theme.border.subtle))
                .on_mouse_up(gpui::MouseButton::Left, on_click_dock)
                .child(if dock_content { "⊟" } else { "⊞" })
        })
}

impl Render for RailView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = *Theme::global(cx);
        // T276 mirror: rail owns the exclusive zone. Cache against the
        // shared global field — `mod.rs::ensure_content_width` invalidates
        // it on width changes; we re-derive and push when it actually
        // moves.
        let zone = cx.global::<SidePanelLeftState_>().exclusive_px();
        let cached = cx.global::<SidePanelLeftState_>().last_exclusive_zone;
        if cached != Some(zone) {
            _window.set_exclusive_edge(gpui::layer_shell::Anchor::LEFT);
            _window.set_exclusive_zone(px(zone));
            cx.global_mut::<SidePanelLeftState_>().last_exclusive_zone = Some(zone);
        }

        // T217: round the rail's top-left corner where it meets the bar
        // (mirrors the right panel's top-right rule). Free corners rhyme
        // with the bar; under-the-bar corners square off.
        let display_w = crate::monitor::pult_display_info(cx)
            .map(|d| f32::from(d.bounds().size.width))
            .unwrap_or(1920.);
        let corner_tl = crate::state::panel_corner_radius(0.0);

        div()
            .id("side-panel-left-rail-root")
            .window_font(&theme)
            .size_full()
            .flex()
            .flex_row()
            .when(corner_tl > 0.0, |d| {
                d.rounded_tl(px(corner_tl)).overflow_hidden()
            })
            .child(render_rail(cx, self.content.clone()))
    }
}

impl WorkspaceView {
    /// Called by `RailView` when an icon is clicked. Same three-action
    /// policy as T276 / T221:
    ///
    /// 1. Same tab, dock on → no-op (dock wins, can't collapse).
    /// 2. Same tab, content open → collapse to rail-only.
    /// 3. Same tab, content closed → open at `width_for_open`.
    /// 4. Different tab → switch and open.
    pub fn on_rail_tab_select(&mut self, tab: LeftTab, cx: &mut Context<Self>) {
        let state = cx.global::<crate::side_panel_left::SidePanelLeftState_>();
        let active = state.active_tab;
        let dock = state.dock_content;
        let panel_w = state.panel_width;
        let visible_w = crate::side_panel_left::state::geometry::visible_content_width(panel_w);
        let content_open = dock || visible_w > 1.0;
        drop(state);

        let new_w = width_for_open(tab, &cx.global::<SidePanelLeftState_>().remembered_widths);
        let state = cx.global_mut::<crate::side_panel_left::SidePanelLeftState_>();

        match (tab == active, content_open, dock) {
            (true, true, true) => {
                // 1. Dock wins — no-op.
                tracing::debug!(tab = tab.label(), "side_panel_left: rail click while docked — no-op");
                return;
            }
            (true, true, false) => {
                // 2. Collapse to rail-only.
                state.panel_width = RAIL_WIDTH;
                state.dock_content = false;
                state.last_exclusive_zone = None;
                state.remembered_widths.set(active, panel_w);
                tracing::info!(tab = active.label(), "side_panel_left: rail click collapsed to rail-only");
            }
            _ => {
                // 3 + 4. Select and open (or re-open).
                state.active_tab = tab;
                state.ensure_content_width(new_w);
                state.last_exclusive_zone = None;
                tracing::info!(
                    tab = tab.label(),
                    width = state.panel_width,
                    "side_panel_left: rail click opened tab"
                );
            }
        }
        cx.notify();
    }

    /// Called by `RailView`'s dock toggle button. T278 architect round 3:
    /// the reducer is a pure `tabs::dock_transition` (spec §4.1):
    /// - rail-only + dock on → expand to `width_for_open(active, remembered)`.
    /// - overlay + dock on → preserve width, flip flag.
    /// - docked + dock off → preserve width, flip flag.
    ///
    /// The next regular tab switch applies the new tab's policy — dock
    /// toggle never pre-bakes a future tab's width.
    pub fn on_dock_toggle(&mut self, cx: &mut Context<Self>) {
        // Compute the next state via the pure helper BEFORE taking the
        // global mutably, so the helper stays a pure function and we
        // get the same answer as a unit test would.
        let (next_width, next_dock) = {
            let state = cx.global::<crate::side_panel_left::SidePanelLeftState_>();
            crate::side_panel_left::tabs::dock_transition(
                state.panel_width,
                state.dock_content,
                state.active_tab,
                &state.remembered_widths,
            )
        };
        let state = cx.global_mut::<crate::side_panel_left::SidePanelLeftState_>();
        let was_docked = state.dock_content;
        let was_width = state.panel_width;
        state.panel_width = next_width;
        state.dock_content = next_dock;
        // Invalidate the rail's exclusive_zone cache only if the value
        // actually moved (the rail re-pushes whenever the cached value
        // differs from `exclusive_px()`).
        let new_zone = state.exclusive_px();
        if state.last_exclusive_zone != Some(new_zone) {
            state.last_exclusive_zone = None;
        }
        tracing::info!(
            was_docked,
            was_width,
            now_dock = state.dock_content,
            now_width = state.panel_width,
            exclusive_px = new_zone,
            "side_panel_left: dock toggle"
        );
        cx.notify();
    }
}

/// Silence unused — `Rc` is referenced through `std::rc::Rc` paths in
/// the closure set, but Rust 2024 doesn't see the through-closure
/// borrow. Keeping the import list explicit would re-introduce a
/// warning without value.
#[allow(dead_code)]
fn _rc_anchor() {
    let _ = Rc::new(0);
}