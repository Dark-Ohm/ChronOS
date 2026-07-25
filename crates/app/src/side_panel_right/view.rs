//! Right side panel view — sidebar v2 (mockup → layout, flagship rsx sections).
//!
//! ## `on_hover` / animation split (fork rule)
//! Our gpui fork stores a **single** `Option` hover handler per element and
//! `debug_assert!`s if `.on_hover` is set twice. Consequences:
//! - Root node: **only** the peek close-debounce `on_hover` (this file).
//! - Children: **no** extra root hover.
//! - Peek motion: state-driven `.transition_when` on an **inner** wrapper.

use std::{
    rc::Rc,
    time::{Duration, Instant},
};

use chronos_services::net_stats::{self, NetState};
use chronos_services::{DiskInfo, MprisState, Service, SystemResourcesState};
use gpui::{
    App, AsyncApp, Context, IntoElement, Render, ScrollHandle, Window, div, layer_shell::*,
    prelude::*, px, rgb,
};
use gpui_animation::animation::TransitionExt;
use gpui_animation::transition::general::Linear;

use crate::side_panel_right::disks::render_disks_section;
use crate::side_panel_right::header::render_header;
use crate::side_panel_right::mpris_card::render_mpris_card;
use crate::side_panel_right::permission::render_permission_card;
use crate::side_panel_right::power_row::{
    ARM_TIMEOUT, ArmState, PowerAction, is_confirming_click, on_click as arm_on_click, on_timeout,
    render_footer,
};
use crate::side_panel_right::rail::render_rail;
use crate::side_panel_right::spectrum_row::{
    H_CPU, H_GPU, H_NET, H_RAM, SpectrumHistory, color_cpu, color_gpu, color_net, color_ram,
    color_value_default, push_sample, render_spectrum_row,
};
use crate::side_panel_right::tabs::PanelTab;
use crate::side_panel_right::{
    DEFAULT_CONTENT_WIDTH, HANDLE_WIDTH, RAIL_ONLY_WIDTH, RightPanelResize, SidePanelRightState,
};
use crate::state::{self, AppState};

/// Delay before peek-close after mouse leaves panel (or strip).
const PEEK_LEAVE_DEBOUNCE: Duration = Duration::from_millis(280);

const REVEAL_MS: u64 = 180;

pub struct SidePanelRightView {
    mpris: MprisState,
    system: SystemResourcesState,
    disks: Vec<DiskInfo>,
    cpu_history: SpectrumHistory,
    ram_history: SpectrumHistory,
    gpu_history: SpectrumHistory,
    net_state: NetState,
    net_dl_history: SpectrumHistory,
    net_ul_history: SpectrumHistory,
    power_arm: ArmState,
    /// State-driven reveal for `transition_when` (not hover-driven).
    revealed: bool,
    scroll: ScrollHandle,
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
}

impl SidePanelRightView {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let mpris_signal = AppState::mpris(cx).subscribe();
        state::watch(cx, mpris_signal, |this: &mut Self, data: MprisState, cx| {
            this.mpris = data;
            cx.notify();
        });

        let sys_signal = AppState::system_resources(cx).subscribe();
        state::watch(
            cx,
            sys_signal,
            |this: &mut Self, data: SystemResourcesState, cx| {
                push_sample(&mut this.cpu_history, data.cpu_percent);
                push_sample(&mut this.ram_history, data.ram_percent);
                if let Some(gpu) = data.gpu_percent {
                    push_sample(&mut this.gpu_history, gpu);
                }
                this.system = data;
                cx.notify();
            },
        );

        let disks_signal = AppState::disks(cx).subscribe();
        state::watch(
            cx,
            disks_signal,
            |this: &mut Self, data: Vec<DiskInfo>, cx| {
                this.disks = data;
                cx.notify();
            },
        );

        cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(Duration::from_millis(16))
                .await;
            match this.update(cx, |this, cx| {
                this.revealed = true;
                cx.notify();
            }) {
                Ok(()) => {}
                Err(e) => tracing::debug!("side_panel_right: reveal skipped (view gone): {e}"),
            }
        })
        .detach();

        Self {
            mpris: AppState::mpris(cx).get(),
            system: AppState::system_resources(cx).get(),
            disks: AppState::disks(cx).get(),
            cpu_history: SpectrumHistory::default(),
            ram_history: SpectrumHistory::default(),
            gpu_history: SpectrumHistory::default(),
            net_state: NetState::default(),
            net_dl_history: SpectrumHistory::default(),
            net_ul_history: SpectrumHistory::default(),
            power_arm: ArmState::default(),
            revealed: false,
            scroll: ScrollHandle::new(),
            active_tab: PanelTab::default(),
            last_resized_width: crate::side_panel_right::RAIL_ONLY_WIDTH,
            last_exclusive_zone: None,
            resize_start_x: None,
            resize_start_width: None,
        }
    }

    fn start_resize(&mut self, start_x: f32, cx: &mut Context<Self>) {
        let w = cx.global::<SidePanelRightState>().width;
        self.resize_start_x = Some(start_x);
        self.resize_start_width = Some(w);
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
        let speed = net_stats::update_speed(
            &mut self.net_state,
            rx,
            tx,
            Instant::now(),
            net_stats::SAMPLE_INTERVAL,
        );
        let new_t = self.net_state.sample.as_ref().map(|s| s.time);
        if prev_t != new_t {
            push_sample(&mut self.net_dl_history, speed.dl as f32);
            push_sample(&mut self.net_ul_history, speed.ul as f32);
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
        // Opening a tab from rail-only pulls content out (overlay).
        {
            let state = cx.global_mut::<SidePanelRightState>();
            state.ensure_content_width();
        }
        cx.notify();
    }
}

impl Render for SidePanelRightView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.sample_network();
        let revealed = self.revealed;
        let power_arm = self.power_arm;
        let gpu = self.system.gpu_percent;

        let dl = format_bytes_per_sec(self.net_state.cached_dl);
        let ul = format_bytes_per_sec(self.net_state.cached_ul);
        let net_summary = format!("↓ {dl}  ↑ {ul}");

        // --- Exclusive zone & width sync (mirror T126 left panel) ---
        let panel_state = cx.global::<crate::side_panel_right::SidePanelRightState>();
        let dock_content = panel_state.dock_content;
        let panel_width = panel_state.width;

        // Calculate exclusive zone: dock ON → full width, dock OFF → rail only
        let rail_only_width = crate::side_panel_right::RAIL_ONLY_WIDTH;
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

        // Resize window if panel width changed (e.g., dock toggle)
        if self.last_resized_width != panel_width {
            if let Some(display) = window.display(cx) {
                let bounds = display.bounds();
                let new_h = (f32::from(bounds.size.height)
                    - crate::side_panel_right::PANEL_EDGE_GAP)
                    .max(100.);
                window.resize(gpui::Size::new(px(panel_width), px(new_h)));
            }
            self.last_resized_width = panel_width;
        }

        // Content open when dock is ON OR user dragged past rail+handle threshold
        let content_open = dock_content || panel_width > rail_only_width + 1.0;

        // Resize handlers before any RPIT that captures cx (Rust 2024).
        let resize_drag_handler = cx.listener(
            |this, ev: &gpui::DragMoveEvent<RightPanelResize>, _window, cx| {
                this.update_resize(f32::from(ev.event.position.x), cx);
            },
        );
        let resize_mouse_handler = cx.listener(|this, ev: &gpui::MouseDownEvent, _w, cx| {
            this.start_resize(f32::from(ev.position.x), cx);
        });

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
                    .bg(rgb(0x18_18_25))
                    .border_r_1()
                    .border_color(rgb(0x23_23_36))
                    .on_mouse_down(gpui::MouseButton::Left, resize_mouse_handler)
                    .on_drag(RightPanelResize, |_, _, _, cx| cx.new(|_| gpui::EmptyView))
                    .on_drag_move(resize_drag_handler)
                    .child(div().w(px(1.)).h_full().bg(rgb(0x45_47_5a))),
            )
            .child(
                div()
                    .id("side-panel-body")
                    .with_transition("side-panel-body")
                    .flex_1()
                    .min_w(px(0.))
                    .h_full()
                    .bg(rgb(0x18_18_25))
                    .border_l_1()
                    .border_color(rgb(0x31_32_44))
                    .flex()
                    .flex_row() // content first, rail last — rail flush against the screen's right edge
                    .overflow_hidden()
                    .opacity(if revealed { 1.0 } else { 0.0 })
                    .transition_when(revealed, Duration::from_millis(REVEAL_MS), Linear, |s| {
                        s.opacity(1.0)
                    })
                    .when(content_open, |body| {
                        body.child(
                            div()
                                .id("side-panel-content-column")
                                .flex_1()
                                .min_w(px(0.))
                                .flex()
                                .flex_col()
                                .overflow_hidden()
                                .when(self.active_tab == PanelTab::System, |col| {
                                    col
                                        // 1. Header (flex:none) — rsx
                                        .child(render_header())
                                        // 2. Permission card (flex:none) — rsx
                                        .child(render_permission_card())
                                        // 3. Scrollable middle — UNCHANGED body
                                        .child(
                                            div()
                                                .id("side-panel-scroll")
                                                .flex_1()
                                                .min_h(px(0.))
                                                .overflow_y_scroll()
                                                .track_scroll(&self.scroll)
                                                .flex()
                                                .flex_col()
                                                .gap(px(14.))
                                                .p(px(14.))
                                                .child(render_mpris_card(&self.mpris, cx))
                                                .child(
                                                    div()
                                                        .flex()
                                                        .flex_col()
                                                        .gap(px(10.))
                                                        .child(render_spectrum_row(
                                                            "CPU",
                                                            &self.cpu_history,
                                                            &format!(
                                                                "{:.0}%",
                                                                self.system.cpu_percent
                                                            ),
                                                            color_cpu(),
                                                            color_cpu(),
                                                            H_CPU,
                                                        ))
                                                        .child(render_spectrum_row(
                                                            "RAM",
                                                            &self.ram_history,
                                                            &format!(
                                                                "{:.0}%",
                                                                self.system.ram_percent
                                                            ),
                                                            color_ram(),
                                                            color_ram(),
                                                            H_RAM,
                                                        ))
                                                        .when_some(gpu, |d, gpu_pct| {
                                                            d.child(render_spectrum_row(
                                                                "GPU",
                                                                &self.gpu_history,
                                                                &format!("{gpu_pct:.0}%"),
                                                                color_gpu(),
                                                                color_gpu(),
                                                                H_GPU,
                                                            ))
                                                        }),
                                                )
                                                .child(
                                                    div()
                                                        .flex()
                                                        .flex_col()
                                                        .gap(px(10.))
                                                        .child(render_spectrum_row(
                                                            "↓ down",
                                                            &self.net_dl_history,
                                                            &dl,
                                                            color_net(),
                                                            color_value_default(),
                                                            H_NET,
                                                        ))
                                                        .child(render_spectrum_row(
                                                            "↑ up",
                                                            &self.net_ul_history,
                                                            &ul,
                                                            color_net(),
                                                            color_value_default(),
                                                            H_NET,
                                                        )),
                                                )
                                                .child(render_disks_section(&self.disks, cx)),
                                        )
                                        // 4. Footer (flex:none)
                                        .child(render_footer(&net_summary, power_arm, cx))
                                })
                                .when(self.active_tab != PanelTab::System, |col| {
                                    col.child(
                                        div()
                                            .size_full()
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .child(div().text_color(rgb(0x6c_70_86)).child(
                                                format!(
                                                    "{} — coming soon",
                                                    self.active_tab.label()
                                                ),
                                            )),
                                    )
                                }),
                        )
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
                                this_for_dock.update(cx, |_, cx| {
                                    let state = cx.global_mut::<SidePanelRightState>();
                                    state.dock_content = !state.dock_content;
                                    if state.dock_content {
                                        state.ensure_content_width();
                                    } else {
                                        state.last_exclusive_zone = None;
                                    }
                                    cx.notify();
                                });
                            });
                        crate::side_panel_right::rail::render_rail(
                            cx,
                            active,
                            on_select,
                            dock_content,
                            on_dock_toggle,
                        )
                    }),
            )
    }
}

fn format_bytes_per_sec(bps: f64) -> String {
    if bps >= 1_000_000.0 {
        format!("{:.1} MB/s", bps / 1_000_000.0)
    } else if bps >= 1_000.0 {
        format!("{:.0} KB/s", bps / 1_000.0)
    } else {
        format!("{bps:.0} B/s")
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
