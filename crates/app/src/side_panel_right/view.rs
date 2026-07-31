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

use crate::motion;
use crate::side_panel_right::power_row::{
    ARM_TIMEOUT, ArmState, PowerAction, is_confirming_click, on_click as arm_on_click, on_timeout,
    render_footer,
};
use crate::side_panel_right::surfaces;
use crate::side_panel_right::tab::TabContent;
use crate::side_panel_right::tab::system::format_net_pair;
use crate::side_panel_right::tabs::PanelTab;
use crate::side_panel_right::{
    HANDLE_WIDTH, RAIL_ONLY_WIDTH, RightPanelResize, SidePanelRightState,
};
use crate::state::AppState;
use crate::{scene, workspace_mode};

use chronos_ui::{Theme, elevation_glow_bar};

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
    resize_start_width: Option<f32>,
    /// Lazy, cached tab views — one per visited tab. Created on first
    /// activation, retained across switches and mode changes.
    tab_views: HashMap<PanelTab, TabContent>,
}

impl SidePanelRightView {
    pub fn new(_cx: &mut Context<Self>) -> Self {
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
        }
    }

    fn start_resize(&mut self, start_x: f32, cx: &mut Context<Self>) {
        let w = cx.global::<SidePanelRightState>().width;
        self.resize_start_x = Some(start_x);
        self.resize_start_width = Some(w);
        // Rail-only: first grab on the handle pops content to default width
        // (user "pulls the panel out of the bar"). Further drag adjusts freely.
        if w <= RAIL_ONLY_WIDTH + 1.0 {
            let state = cx.global_mut::<SidePanelRightState>();
            state.ensure_content_width();
            self.resize_start_width = Some(state.width);
            self.last_resized_width = f32::NAN;
            tracing::info!(
                width = state.width,
                "side_panel_right: handle grab expanded rail → content"
            );
            cx.notify();
        }
    }

    fn update_resize(&mut self, current_x: f32, cx: &mut Context<Self>) {
        let (start_x, start_w) = match (self.resize_start_x, self.resize_start_width) {
            (Some(x), Some(w)) => (x, w),
            _ => return,
        };
        // Right-anchored: pointer left → wider. new = start - (current - start_x)
        let delta = current_x - start_x;
        let state = cx.global_mut::<SidePanelRightState>();
        state.resize(start_w - delta);
        state.last_exclusive_zone = None;
        crate::side_panel_right::hold_peek(cx);
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

    /// Pure: clicking a rail button always makes that tab active — no toggle,
    /// no special-case for re-clicking the already-active tab.
    fn next_active_tab(_current: PanelTab, clicked: PanelTab) -> PanelTab {
        clicked
    }

    pub(crate) fn on_tab_select(&mut self, tab: PanelTab, cx: &mut Context<Self>) {
        self.active_tab = Self::next_active_tab(self.active_tab, tab);
        // Lazy-create the tab view on first activation. Also called from
        // render() for the very-first-paint case. T168 errata 3.
        self.ensure_tab_view(self.active_tab, cx);
        // Opening a tab from rail-only pulls content out (overlay).
        {
            let state = cx.global_mut::<SidePanelRightState>();
            let before = state.width;
            state.ensure_content_width();
            tracing::info!(
                before,
                after = state.width,
                tab = tab.label(),
                "side_panel_right: tab select → ensure content width"
            );
        }
        // Force next paint to re-apply resize even if a previous failed attempt
        // had advanced last_resized_width without a real set_size.
        self.last_resized_width = f32::NAN;
        cx.notify();
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
}

impl Render for SidePanelRightView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.sample_network();
        // Mode/scene composition for the rail (scene override > mode default).
        let rail_tabs = PanelTab::resolve_for_mode(
            workspace_mode::current(cx),
            scene::rail_tabs_override(cx).as_deref(),
        );
        // Active tab left the set after a mode switch — land on System, keep
        // the panel open (§5: must not discard panel state / close on mode change).
        if !rail_tabs.contains(&self.active_tab) {
            tracing::info!(
                was = self.active_tab.label(),
                "side_panel_right: active tab not in mode set → System"
            );
            self.active_tab = PanelTab::System;
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

        // Resize window if panel width changed (tab open / dock / drag).
        if self.last_resized_width != panel_width {
            let display_h = crate::monitor::pult_display_info(cx)
                .map(|d| f32::from(d.bounds().size.height))
                .or_else(|| window.display(cx).map(|d| f32::from(d.bounds().size.height)))
                .unwrap_or(1080.);
            let panel_h =
                (display_h - crate::side_panel_right::PANEL_EDGE_GAP).max(100.);
            window.resize(gpui::Size::new(px(panel_width), px(panel_h)));
            self.last_resized_width = panel_width;
            tracing::debug!(
                panel_width,
                panel_h,
                "side_panel_right: window.resize after width change"
            );
        }

        // Content open when dock is ON OR user dragged past rail+handle threshold
        let content_open = dock_content || panel_width > rail_only_width + 1.0;

        // Elevated chrome на content-колонке (не rail-only) — общий язык
        // глубины из `theme.elevation_popup()` (T128).
        let theme = *Theme::global(cx);
        let elev = theme.elevation_popup();

        // Resize handlers before any RPIT that captures cx (Rust 2024).
        let resize_drag_handler = cx.listener(
            |this, ev: &gpui::DragMoveEvent<RightPanelResize>, _window, cx| {
                this.update_resize(f32::from(ev.event.position.x), cx);
            },
        );
        let resize_mouse_handler = cx.listener(|this, ev: &gpui::MouseDownEvent, _w, cx| {
            this.start_resize(f32::from(ev.position.x), cx);
        });

        // Lazy tab view — created on first paint, cached thereafter.
        // ensure_tab_view() avoids expect-panic on the very first render
        // (before any on_tab_select has fired). T168 errata 3.
        let active = self.active_tab;
        let tab_entry = self.ensure_tab_view(active, cx);

        // OUTER: sole window-level `on_hover` (debounce). No transition_on_hover.
        // Layout: [handle | content? | rail] — rail flush right; handle is inner edge.
        div()
            .id("side-panel-right-root")
            .size_full()
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
                div()
                    .id("side-panel-right-resize-handle")
                    .flex_none()
                    .w(px(HANDLE_WIDTH))
                    .h_full()
                    .cursor_col_resize()
                    .flex()
                    .items_center()
                    .justify_center()
                    .bg(surfaces::chrome(&theme))
                    .border_r_1()
                    .border_color(theme.border.subtle)
                    .on_mouse_down(gpui::MouseButton::Left, resize_mouse_handler)
                    .on_drag(RightPanelResize, |_, _, _, cx| cx.new(|_| gpui::EmptyView))
                    .on_drag_move(resize_drag_handler)
                    .child(div().w(px(1.)).h_full().bg(theme.text.disabled)),
            )
            .child(
                div()
                    .id("side-panel-body")
                    .relative()
                    .flex_1()
                    .min_w(px(0.))
                    .h_full()
                    .bg(surfaces::chrome(&theme))
                    .border_l_1()
                    .border_color(theme.border.default)
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
                            match tab_entry {
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
                                TabContent::Placeholder(entity) => {
                                    col.child(entity.clone())
                                }
                            }
                        })
                    })
                    .child({
                        let active = self.active_tab;
                        let this = cx.entity();
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
                                    let state = cx.global_mut::<SidePanelRightState>();
                                    state.dock_content = !state.dock_content;
                                    if state.dock_content {
                                        state.ensure_content_width();
                                    } else {
                                        state.last_exclusive_zone = None;
                                    }
                                    this.last_resized_width = f32::NAN;
                                    tracing::info!(
                                        dock = state.dock_content,
                                        width = state.width,
                                        "side_panel_right: dock toggle"
                                    );
                                    cx.notify();
                                });
                            });
                        crate::side_panel_right::rail::render_rail(
                            cx,
                            &rail_tabs,
                            active,
                            on_select,
                            dock_content,
                            on_dock_toggle,
                        )
                    })
                    .with_animation(
                        "side-panel-body-enter",
                        motion::enter_animation(),
                        motion::apply_enter_from_right,
                    ),
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
            TabContent::Placeholder(e) => e.entity_id(),
        })
    }
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
