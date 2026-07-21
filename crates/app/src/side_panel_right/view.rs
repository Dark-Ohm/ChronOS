//! Right side panel view — MPRIS (T9), spectrum meters (T10), power row (T11).
//!
//! ## `on_hover` / animation split (fork rule)
//! Our gpui fork stores a **single** `Option` hover handler per element and
//! `debug_assert!`s if `.on_hover` is set twice. Consequences:
//! - Root node: **only** the peek close-debounce `on_hover` (this file).
//! - Spectrum / power-row are children — they get **no** extra root hover.
//! - Peek motion: state-driven `.transition_when` on an **inner** wrapper.

use std::time::{Duration, Instant};

use chronos_services::net_stats::{self, NetState};
use chronos_services::{MprisState, Service, SystemResourcesState};
use chronos_ui::Theme;
use gpui::{
    AsyncApp, Context, IntoElement, Render, Window, div, prelude::*, px, rgb,
};
use gpui_animation::animation::TransitionExt;
use gpui_animation::transition::general::Linear;

use crate::side_panel_right::mpris_card::render_mpris_card;
use crate::side_panel_right::power_row::{
    is_confirming_click, on_click as arm_on_click, on_timeout, render_power_row, ArmState,
    PowerAction, ARM_TIMEOUT,
};
use crate::side_panel_right::spectrum_row::{
    push_sample, render_spectrum_row, SpectrumHistory,
};
use crate::state::{self, AppState};

/// Delay before peek-close after mouse leaves panel (or strip).
const PEEK_LEAVE_DEBOUNCE: Duration = Duration::from_millis(280);

const REVEAL_MS: u64 = 180;

// Plan palette — blue-cyan only (no rainbow).
fn color_cpu() -> gpui::Hsla {
    rgb(0x5f_d3_e8).into()
}
fn color_ram() -> gpui::Hsla {
    rgb(0x4f_a3_c9).into()
}
fn color_gpu() -> gpui::Hsla {
    rgb(0x33_63_8a).into()
}
fn color_dn() -> gpui::Hsla {
    rgb(0x7c_c4_e8).into()
}
fn color_up() -> gpui::Hsla {
    rgb(0x3d_6d_94).into()
}

pub struct SidePanelRightView {
    mpris: MprisState,
    system: SystemResourcesState,
    cpu_history: SpectrumHistory,
    ram_history: SpectrumHistory,
    gpu_history: SpectrumHistory,
    net_state: NetState,
    net_dl_history: SpectrumHistory,
    net_ul_history: SpectrumHistory,
    power_arm: ArmState,
    /// State-driven reveal for `transition_when` (not hover-driven).
    revealed: bool,
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

        cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(Duration::from_millis(16))
                .await;
            match this.update(cx, |this, cx| {
                this.revealed = true;
                cx.notify();
            }) {
                Ok(()) => {}
                Err(e) => tracing::debug!(
                    "side_panel_right: reveal skipped (view gone): {e}"
                ),
            }
        })
        .detach();

        Self {
            mpris: AppState::mpris(cx).get(),
            system: AppState::system_resources(cx).get(),
            cpu_history: SpectrumHistory::default(),
            ram_history: SpectrumHistory::default(),
            gpu_history: SpectrumHistory::default(),
            net_state: NetState::default(),
            net_dl_history: SpectrumHistory::default(),
            net_ul_history: SpectrumHistory::default(),
            power_arm: ArmState::default(),
            revealed: false,
        }
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
            // NOT `let _ = view.update(..)` — swallowed Err hid ghost windows.
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
}

impl Render for SidePanelRightView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.sample_network();
        let theme = *Theme::global(cx);
        let revealed = self.revealed;
        let power_arm = self.power_arm;
        let gpu = self.system.gpu_percent;

        // OUTER: sole window-level `on_hover` (debounce). No transition_on_hover.
        div()
            .id("side-panel-right-root")
            .size_full()
            .on_hover(|hovered, _window, cx| {
                if *hovered {
                    crate::side_panel_right::hold_peek(cx);
                } else {
                    crate::side_panel_right::schedule_release_peek(cx);
                }
            })
            .child(
                div()
                    .id("side-panel-body")
                    .with_transition("side-panel-body")
                    .size_full()
                    .bg(theme.bg.secondary)
                    .border_l_1()
                    .border_color(theme.border.default)
                    .p(px(16.))
                    .flex()
                    .flex_col()
                    .gap(px(14.))
                    .opacity(if revealed { 1.0 } else { 0.0 })
                    .transition_when(
                        revealed,
                        Duration::from_millis(REVEAL_MS),
                        Linear,
                        |s| s.opacity(1.0),
                    )
                    .child(render_mpris_card(&self.mpris, &theme, cx))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .child(render_spectrum_row(
                                "CPU",
                                &self.cpu_history,
                                &format!("{:.0}%", self.system.cpu_percent),
                                color_cpu(),
                                &theme,
                            ))
                            .child(render_spectrum_row(
                                "RAM",
                                &self.ram_history,
                                &format!("{:.0}%", self.system.ram_percent),
                                color_ram(),
                                &theme,
                            ))
                            .when_some(gpu, |d, gpu_pct| {
                                d.child(render_spectrum_row(
                                    "GPU",
                                    &self.gpu_history,
                                    &format!("{gpu_pct:.0}%"),
                                    color_gpu(),
                                    &theme,
                                ))
                            }),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .child(render_spectrum_row(
                                "dn",
                                &self.net_dl_history,
                                &format_bytes_per_sec(self.net_state.cached_dl),
                                color_dn(),
                                &theme,
                            ))
                            .child(render_spectrum_row(
                                "up",
                                &self.net_ul_history,
                                &format_bytes_per_sec(self.net_state.cached_ul),
                                color_up(),
                                &theme,
                            )),
                    )
                    .child(render_power_row(&theme, power_arm, cx)),
            )
    }
}

fn format_bytes_per_sec(bps: f64) -> String {
    if bps >= 1_000_000.0 {
        format!("{:.1}M", bps / 1_000_000.0)
    } else if bps >= 1_000.0 {
        format!("{:.0}K", bps / 1_000.0)
    } else {
        format!("{bps:.0}")
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
