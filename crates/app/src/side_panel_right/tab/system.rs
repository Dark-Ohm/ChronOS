//! System tab — hardware monitor and system controls.
//!
//! Owns all service subscriptions and rendering for the System panel tab.
//! The footer (power buttons + network summary) stays on `SidePanelRightView`
//! because `power_row::render_footer` takes `Context<SidePanelRightView>` —
//! that file is outside the task zone. The footer renders below this view.

use std::time::Instant;

use chronos_services::net_stats::{self, NetState};
use chronos_services::{
    ActiveWindow, CompositorState, DiskInfo, MprisState, Service, SystemResourcesState,
};
use gpui::{AnyElement, Context, IntoElement, Render, ScrollHandle, Window, div, prelude::*, px};

use crate::side_panel_right::disks::render_disks_section;
use crate::side_panel_right::tab::ui::{elevated_card, section_header};
use crate::side_panel_right::header::render_header;
use crate::side_panel_right::mpris_card::render_mpris_card;
use crate::side_panel_right::spectrum_row::{
    H_CPU, H_GPU, H_NET, H_RAM, SpectrumHistory, color_cpu, color_gpu, color_net, color_ram,
    color_value_default, push_sample, render_spectrum_row,
};
use crate::side_panel_right::wallpaper_card::render_wallpaper_card;
use crate::state::{self, AppState};

use chronos_ui::Theme;

pub struct SystemTab {
    mpris: MprisState,
    system: SystemResourcesState,
    disks: Vec<DiskInfo>,
    wallpaper: chronos_services::WallpaperState,
    /// T256: real title of the Hyprland active window, fed by the compositor
    /// service. Rendered in the header instead of a hardcoded string.
    active_window: Option<ActiveWindow>,
    waytrogen_available: bool,
    cpu_history: SpectrumHistory,
    ram_history: SpectrumHistory,
    gpu_history: SpectrumHistory,
    net_state: NetState,
    net_dl_history: SpectrumHistory,
    net_ul_history: SpectrumHistory,
    scroll: ScrollHandle,
}

impl SystemTab {
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

        let wallpaper_signal = AppState::wallpaper(cx).subscribe();
        state::watch(
            cx,
            wallpaper_signal,
            |this: &mut Self, data: chronos_services::WallpaperState, cx| {
                this.wallpaper = data;
                cx.notify();
            },
        );

        // T256: subscribe to compositor for the header title. The signal
        // carries the full `CompositorState` (workspaces / keyboard / monitors
        // / active_window); we ignore every field except active_window and
        // even re-paint only when that field actually changes — workspace
        // switches, keyboard layout swaps, monitor hotplugs all carry a
        // CompositorState signal but produce zero rendered effect here.
        let compositor_signal = AppState::compositor(cx).subscribe();
        state::watch(
            cx,
            compositor_signal,
            |this: &mut Self, data: CompositorState, cx| {
                // Only re-paint on real active-window changes — the full
                // CompositorState fires on every workspace / keyboard / monitor
                // event too, and the rest of this tab doesn't read those.
                if data.active_window != this.active_window {
                    this.active_window = data.active_window;
                    cx.notify();
                }
            },
        );

        Self {
            mpris: AppState::mpris(cx).get(),
            system: AppState::system_resources(cx).get(),
            disks: AppState::disks(cx).get(),
            wallpaper: AppState::wallpaper(cx).get(),
            active_window: AppState::compositor(cx).get().active_window,
            waytrogen_available: crate::wallpaper_ctl::waytrogen_available(),
            cpu_history: SpectrumHistory::default(),
            ram_history: SpectrumHistory::default(),
            gpu_history: SpectrumHistory::default(),
            net_state: NetState::default(),
            net_dl_history: SpectrumHistory::default(),
            net_ul_history: SpectrumHistory::default(),
            scroll: ScrollHandle::new(),
        }
    }

    /// Sample network speed on every render. Time-gated by `update_speed`'s
    /// `SAMPLE_INTERVAL` — history only advances when a real sample lands.
    fn sample_network(&mut self) {
        let Ok((rx, tx)) = net_stats::read_interface_bytes() else {
            return;
        };
        let prev_t = self.net_state.sample.as_ref().map(|s| s.time);
        net_stats::update_speed(
            &mut self.net_state,
            rx,
            tx,
            Instant::now(),
            net_stats::SAMPLE_INTERVAL,
        );
        let new_t = self.net_state.sample.as_ref().map(|s| s.time);
        if prev_t != new_t {
            push_sample(&mut self.net_dl_history, self.net_state.cached_dl as f32);
            push_sample(&mut self.net_ul_history, self.net_state.cached_ul as f32);
        }
    }
}

impl Render for SystemTab {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.sample_network();
        let theme = *Theme::global(cx);
        let gpu = self.system.gpu_percent;

        let dl = format_net_pair(self.net_state.cached_dl, 0.0);
        let ul = format_net_pair(0.0, self.net_state.cached_ul);

        div()
            .size_full()
            .flex()
            .flex_col()
            // 1. Header (flex:none) — title comes from Hyprland active
            // window (T256); was historically hardcoded "kitty".
            .child(render_header(cx, self.active_window.as_ref()))
            // 2. Scrollable middle (flex:1)
            .child(
                div()
                    .id("system-tab-scroll")
                    .flex_1()
                    .min_h(px(0.))
                    .overflow_y_scroll()
                    .track_scroll(&self.scroll)
                    .flex()
                    .flex_col()
                    .gap(px(14.))
                    .p(px(14.))
                    .child(render_mpris_card(&self.mpris, cx))
                    .child(render_wallpaper_card(
                        &self.wallpaper,
                        self.waytrogen_available,
                        cx,
                    ))
                    // T259: «+ Add terminal» — only while edit mode (Super+Shift+E)
                    // is active, same gating as the bar's reorder affordances.
                    .when(crate::edit_mode::is_active(cx), |d| {
                        d.child(render_add_terminal_card(cx))
                    })
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(10.))
                            .child(render_spectrum_row(
                                "CPU",
                                &self.cpu_history,
                                &format!("{:.0}%", self.system.cpu_percent),
                                color_cpu(&theme),
                                color_cpu(&theme),
                                H_CPU,
                                &theme,
                            ))
                            .child(render_spectrum_row(
                                "RAM",
                                &self.ram_history,
                                &format!("{:.0}%", self.system.ram_percent),
                                color_ram(&theme),
                                color_ram(&theme),
                                H_RAM,
                                &theme,
                            ))
                            .when_some(gpu, |d, gpu_pct| {
                                d.child(render_spectrum_row(
                                    "GPU",
                                    &self.gpu_history,
                                    &format!("{gpu_pct:.0}%"),
                                    color_gpu(&theme),
                                    color_gpu(&theme),
                                    H_GPU,
                                    &theme,
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
                                color_net(&theme),
                                color_value_default(&theme),
                                H_NET,
                                &theme,
                            ))
                            .child(render_spectrum_row(
                                "↑ up",
                                &self.net_ul_history,
                                &ul,
                                color_net(&theme),
                                color_value_default(&theme),
                                H_NET,
                                &theme,
                            )),
                    )
                    .child(render_disks_section(&self.disks, cx)),
            )
    }
}

/// T259: edit-mode-only card to spawn a new desktop-terminal widget.
///
/// Visual language: the T231 `elevated_card` + `section_header` pattern from
/// `tab/ui.rs` (the same one bar-settings and sibling tabs use) — no new
/// design vocabulary. The click creates a spec (offset from the last widget),
/// persists it to `desktop_terminal.toml`, and opens the window.
fn render_add_terminal_card(cx: &mut Context<SystemTab>) -> AnyElement {
    let theme = *Theme::global(cx);
    let add = cx.listener(|_this, _e: &gpui::ClickEvent, _window, cx| {
        crate::desktop_terminal::add_widget(cx);
    });
    elevated_card(theme)
        // NOTE: `.id()` must come after `elevated_card` — it upgrades the
        // bare Div into a Stateful element (see tab/ui.rs doc comment).
        .id("system-terminal-card")
        .child(section_header(
            theme,
            "Desktop Terminal",
            "desktop_terminal.toml · widgets",
        ))
        .child(
            div()
                .id("system-add-terminal")
                .w_full()
                .cursor_pointer()
                .px(px(12.))
                .py(px(9.))
                .rounded_md()
                .border_1()
                .border_color(theme.border.subtle)
                .bg(theme.bg.primary)
                .hover(|s| s.bg(theme.interactive.hover))
                .flex()
                .items_center()
                .justify_center()
                .gap(px(6.))
                .text_size(px(12.))
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .text_color(theme.accent.primary)
                .on_click(add)
                .child("＋ Add terminal"),
        )
        .into_any_element()
}

pub(crate) fn format_net_pair(dl: f64, ul: f64) -> String {
    fn one(bps: f64) -> String {
        if bps >= 1_000_000.0 {
            format!("{:.1} MB/s", bps / 1_000_000.0)
        } else if bps >= 1_000.0 {
            format!("{:.0} KB/s", bps / 1_000.0)
        } else {
            format!("{bps:.0} B/s")
        }
    }
    format!("↓ {}  ↑ {}", one(dl), one(ul))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_net_pair_zero() {
        let s = format_net_pair(0.0, 0.0);
        assert_eq!(s, "↓ 0 B/s  ↑ 0 B/s");
    }

    #[test]
    fn format_net_pair_kilobytes() {
        let s = format_net_pair(1_500.0, 500.0);
        assert_eq!(s, "↓ 2 KB/s  ↑ 500 B/s");
    }

    #[test]
    fn format_net_pair_megabytes() {
        let s = format_net_pair(2_500_000.0, 1_000_000.0);
        assert_eq!(s, "↓ 2.5 MB/s  ↑ 1.0 MB/s");
    }
}
