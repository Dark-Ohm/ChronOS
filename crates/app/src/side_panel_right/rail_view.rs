//! T276: the `rail` window's own root view — the standalone icon rail plus
//! its 4px drag handle, living in the fixed `RAIL_ONLY_WIDTH` surface. All
//! mutable panel state (`active_tab`, resize bookkeeping, per-tab width
//! memory) still belongs to `SidePanelRightView` in the `content` window;
//! this view only renders and forwards input to it via the shared weak
//! entity, exactly the way `mod.rs::select_tab` already reaches the content
//! view from an `App`-only IPC context.
//!
//! Repaint on cross-window state changes: `cx.observe(&content, ...)`
//! piggybacks on every `cx.notify()` the content view already calls after
//! mutating shared state (tab switch, dock toggle, resize). Workspace-mode
//! and edit-mode changes reach this window the same way they reach every
//! other window — `workspace_mode::set` / `edit_mode::toggle` /
//! `panels_config::apply` all call `cx.refresh_windows()` already.

use std::rc::Rc;

use gpui::{
    App, Bounds, Context, Entity, IntoElement, Pixels, Render, Subscription, WeakEntity, Window,
    div, prelude::*, px,
};

use chronos_ui::{Theme, WindowRootExt};

use crate::edit_mode;
use crate::side_panel_right::view::SidePanelRightView;
use crate::side_panel_right::{SidePanelRightState, panels_config, rail, tabs::PanelTab};
use crate::workspace_mode;

pub struct RailView {
    content: WeakEntity<SidePanelRightView>,
    // Held only to keep the subscription alive — repaints via cx.notify().
    _content_sub: Subscription,
}

impl RailView {
    pub fn new(content: Entity<SidePanelRightView>, cx: &mut Context<Self>) -> Self {
        let sub = cx.observe(&content, |_, _, cx| cx.notify());
        Self {
            content: content.downgrade(),
            _content_sub: sub,
        }
    }
}

impl Render for RailView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = *Theme::global(cx);
        let current_mode = workspace_mode::current(cx);
        let panel_cfg = panels_config::cached();
        let (top_tabs, bottom_tabs) = panels_config::resolve_grouped(current_mode, &panel_cfg);
        let editing = edit_mode::is_active(cx);
        let dock_content = cx.global::<SidePanelRightState>().dock_content;
        // T305: while the control-center popup is open its tab owns the
        // highlight; otherwise the panel's active tab does.
        let active = crate::side_panel_right::control_center::active_tab(cx)
            .or_else(|| {
                self.content
                    .upgrade()
                    .map(|v| v.read(cx).active_tab())
            })
            .unwrap_or_default();

        // T276: rail owns the exclusive zone — content's own is pinned at
        // the wlr-layer-shell opt-out value `-1` (see `content_window_options`
        // in mod.rs). Cached against the SHARED global field
        // (`SidePanelRightState.last_exclusive_zone`) rather than a local
        // one, since `mod.rs::SidePanelRightState::ensure_content_width`
        // already resets it to force a recompute on width changes — that
        // invalidation must reach whichever surface actually owns the zone.
        let zone = cx.global::<SidePanelRightState>().exclusive_px();
        if cx.global::<SidePanelRightState>().last_exclusive_zone != Some(zone) {
            window.set_exclusive_edge(gpui::layer_shell::Anchor::RIGHT);
            window.set_exclusive_zone(px(zone));
            cx.global_mut::<SidePanelRightState>().last_exclusive_zone = Some(zone);
        }

        // T217: round the rail's own top-right corner (the display's actual
        // corner) when the bar doesn't stretch all the way to it — mirrors
        // the pre-T276 combined-window corner_tr computation, now scoped to
        // rail's own surface only (content owns the OTHER free corner).
        let display_w = crate::monitor::pult_display_info(cx)
            .map(|d| f32::from(d.bounds().size.width))
            .unwrap_or(1920.);
        let corner_tr = crate::state::panel_corner_radius(display_w);

        let content_for_select = self.content.clone();
        let on_select = Rc::new(
            move |tab: PanelTab, bounds: Bounds<Pixels>, _window: &mut Window, cx: &mut App| {
                // T305: settings tabs never touch the panel content — the
                // click opens (or remaps/closes) the control-center popup
                // anchored to this icon's live bounds.
                if crate::side_panel_right::control_center::is_popup_tab(tab) {
                    crate::side_panel_right::control_center::toggle(bounds, tab, cx);
                    return;
                }
                // A work-tool click dismisses an open popup so it cannot
                // linger over the newly opened panel content.
                crate::side_panel_right::control_center::close(cx);
                if let Some(view) = content_for_select.upgrade() {
                    view.update(cx, |view, cx| view.on_tab_select(tab, cx));
                }
            },
        );

        let content_for_dock = self.content.clone();
        let on_dock_toggle = Rc::new(move |_window: &mut Window, cx: &mut App| {
            if let Some(view) = content_for_dock.upgrade() {
                view.update(cx, |view, cx| view.toggle_dock(cx));
            }
        });

        let on_move: Rc<dyn Fn(PanelTab, isize, &mut App)> =
            Rc::new(move |tab: PanelTab, delta: isize, cx: &mut App| {
                let mode = workspace_mode::current(cx);
                panels_config::move_tab(cx, mode, tab, delta);
            });

        div()
            .id("side-panel-right-rail-root")
            .window_font(&theme)
            .size_full()
            .flex()
            .flex_row()
            .when(corner_tr > 0.0, |d| {
                d.rounded_tr(px(corner_tr)).overflow_hidden()
            })
            .on_hover(|hovered, _window, cx| {
                if *hovered {
                    crate::side_panel_right::hold_peek(cx);
                } else {
                    crate::side_panel_right::schedule_release_peek(cx);
                }
            })
            .child(rail::render_rail(
                cx,
                &top_tabs,
                &bottom_tabs,
                active,
                on_select,
                dock_content,
                on_dock_toggle,
                editing,
                on_move,
            ))
    }
}
