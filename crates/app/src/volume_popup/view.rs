//! Volume popup view — Speakers + Microphone fill-bars, ±5%/mute, device picker.
//!
//! Fill-bars are **visual only** (no drag). Device list expands under the
//! section title inside the same window (no second layer-shell surface).

use gpui::{
    AnyElement, App, Context, InteractiveElement, IntoElement, Render, SharedString, Styled,
    Window, div, prelude::*, px,
};

use chronos_services::{
    AudioCommand, AudioDevice, EndpointState, Service, audio::clamp_volume,
};
use chronos_ui::Theme;

use crate::state::AppState;
use crate::volume_popup::{close_this, resize_to_fit};

const PAD: f32 = 12.;
const TRACK_W: f32 = 160.;
const TRACK_H: f32 = 8.;
const STEP: f64 = 0.05;
const MAX_DEVICE_ROWS: usize = 8;

/// Which endpoint's device list is expanded (if any).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EndpointKind {
    Sink,
    Source,
}

impl EndpointKind {
    fn id_prefix(self) -> &'static str {
        match self {
            Self::Sink => "sink",
            Self::Source => "source",
        }
    }

    fn is_source(self) -> bool {
        matches!(self, Self::Source)
    }
}

pub struct VolumePopupView {
    /// Open device picker under Speakers / Microphone (or neither).
    expanded: Option<EndpointKind>,
}

impl VolumePopupView {
    pub fn new(_cx: &mut App) -> Self {
        Self { expanded: None }
    }

    pub(crate) fn expanded(&self) -> Option<EndpointKind> {
        self.expanded
    }
}

impl Render for VolumePopupView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let audio = AppState::audio(cx).get();
        let theme = Theme::global(cx);
        let expanded = self.expanded;

        let bg = theme.bg.elevated;
        let text_primary = theme.text.primary;
        let text_muted = theme.text.muted;
        let text_secondary = theme.text.secondary;
        let divider = theme.bg.secondary;
        let radius = theme.radius;
        let radius_lg = theme.radius_lg;
        let hover = theme.interactive.hover;
        let accent = theme.accent.primary;
        let bar_track = theme.bg.secondary;
        let border_subtle = theme.border.subtle;

        let header = div()
            .w_full()
            .flex()
            .items_center()
            .justify_between()
            .px(px(PAD))
            .py(px(8.))
            .child(div().text_color(text_primary).child("Volume"))
            .child(
                div()
                    .id("volume-popup-close")
                    .cursor_pointer()
                    .px(px(6.))
                    .rounded(radius)
                    .text_color(text_muted)
                    .hover(|s| s.bg(hover))
                    .child("✕")
                    .on_click(|_event, window, cx: &mut App| {
                        close_this(window, cx);
                    }),
            );

        let divider_line = div().w_full().h(px(1.)).bg(divider);

        div()
            .flex_col()
            .w(px(300.))
            .rounded(radius_lg)
            .bg(bg)
            .border_1()
            .border_color(border_subtle)
            .overflow_hidden()
            .child(header)
            .child(divider_line)
            .child(endpoint_block(
                "Speakers",
                EndpointKind::Sink,
                &audio.sink,
                expanded,
                text_primary,
                text_secondary,
                text_muted,
                accent,
                bar_track,
                radius,
                hover,
                cx,
            ))
            .child(div().w_full().h(px(1.)).bg(divider))
            .child(endpoint_block(
                "Microphone",
                EndpointKind::Source,
                &audio.source,
                expanded,
                text_primary,
                text_secondary,
                text_muted,
                accent,
                bar_track,
                radius,
                hover,
                cx,
            ))
    }
}

fn endpoint_block(
    title: &'static str,
    kind: EndpointKind,
    ep: &EndpointState,
    expanded: Option<EndpointKind>,
    text_primary: gpui::Hsla,
    text_secondary: gpui::Hsla,
    text_muted: gpui::Hsla,
    accent: gpui::Hsla,
    bar_track: gpui::Hsla,
    radius: gpui::Pixels,
    hover: gpui::Hsla,
    cx: &mut Context<VolumePopupView>,
) -> AnyElement {
    let muted = ep.muted;
    let volume = ep.volume;
    let fraction = volume.clamp(0.0, 1.0) as f32;
    let fill_w = TRACK_W * fraction;
    let percent = format_percent(volume);
    let mute_label = mute_icon(kind.is_source(), muted);
    let bar_fill = if muted { text_muted } else { accent };
    let title_color = if muted { text_muted } else { text_primary };
    let prefix = kind.id_prefix();
    let is_open = expanded == Some(kind);
    let chevron = if is_open { "▾" } else { "▸" };
    // Prefer live device description when available.
    let device_label = if ep.name.is_empty() {
        title.to_string()
    } else {
        ep.name.clone()
    };
    let title_id: SharedString = format!("{prefix}-title").into();

    let title_row = div()
        .id(title_id)
        .w_full()
        .flex()
        .items_center()
        .justify_between()
        .cursor_pointer()
        .rounded(radius)
        .hover(move |s| s.bg(hover))
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(6.))
                .child(div().text_color(title_color).child(format!("{chevron} {title}")))
                .child(
                    div()
                        .text_color(text_muted)
                        .text_xs()
                        .child(truncate_label(&device_label, 28)),
                ),
        )
        .on_click(cx.listener(move |this, _event, window, cx| {
            this.expanded = if this.expanded == Some(kind) {
                None
            } else {
                Some(kind)
            };
            resize_to_fit(window, this.expanded, cx);
            cx.notify();
        }));

    let device_list: AnyElement = if is_open {
        let devices = &ep.available;
        let shown = devices.len().min(MAX_DEVICE_ROWS);
        let mut rows: Vec<AnyElement> = devices[..shown]
            .iter()
            .map(|d| device_row(kind, d, text_primary, text_muted, accent, radius, hover))
            .collect();
        if devices.len() > shown {
            let hidden = devices.len() - shown;
            rows.push(
                div()
                    .w_full()
                    .px(px(4.))
                    .py(px(2.))
                    .text_color(text_muted)
                    .text_xs()
                    .child(format!("+{hidden} more"))
                    .into_any_element(),
            );
        }
        if devices.is_empty() {
            rows.push(
                div()
                    .w_full()
                    .px(px(4.))
                    .py(px(4.))
                    .text_color(text_muted)
                    .text_xs()
                    .child("No devices found")
                    .into_any_element(),
            );
        }
        div()
            .w_full()
            .flex_col()
            .gap(px(2.))
            .mt(px(4.))
            .pl(px(8.))
            .children(rows)
            .into_any_element()
    } else {
        div().into_any_element()
    };

    div()
        .w_full()
        .flex_col()
        .gap(px(6.))
        .px(px(PAD))
        .py(px(10.))
        .child(title_row)
        .child(device_list)
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(10.))
                .child(
                    div()
                        .w(px(TRACK_W))
                        .h(px(TRACK_H))
                        .rounded(radius)
                        .bg(bar_track)
                        .overflow_hidden()
                        .child(
                            div()
                                .h_full()
                                .w(px(fill_w))
                                .rounded(radius)
                                .bg(bar_fill),
                        ),
                )
                .child(
                    div()
                        .text_color(if muted { text_muted } else { text_secondary })
                        .min_w(px(40.))
                        .child(if muted {
                            "mute".to_string()
                        } else {
                            format!("{percent}%")
                        }),
                ),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(6.))
                .child(
                    div()
                        .id(SharedString::from(format!("{prefix}-minus")))
                        .cursor_pointer()
                        .px(px(8.))
                        .py(px(2.))
                        .rounded(radius)
                        .text_color(text_secondary)
                        .hover(move |s| s.bg(hover))
                        .child("−5%")
                        .on_click(move |_event, _window, cx: &mut App| {
                            step_volume(kind, -STEP, cx);
                        }),
                )
                .child(
                    div()
                        .id(SharedString::from(format!("{prefix}-mute")))
                        .cursor_pointer()
                        .px(px(8.))
                        .py(px(2.))
                        .rounded(radius)
                        .text_color(text_secondary)
                        .hover(move |s| s.bg(hover))
                        .child(mute_label.to_string())
                        .on_click(move |_event, _window, cx: &mut App| {
                            toggle_mute(kind, cx);
                        }),
                )
                .child(
                    div()
                        .id(SharedString::from(format!("{prefix}-plus")))
                        .cursor_pointer()
                        .px(px(8.))
                        .py(px(2.))
                        .rounded(radius)
                        .text_color(text_secondary)
                        .hover(move |s| s.bg(hover))
                        .child("+5%")
                        .on_click(move |_event, _window, cx: &mut App| {
                            step_volume(kind, STEP, cx);
                        }),
                ),
        )
        .into_any_element()
}

fn device_row(
    kind: EndpointKind,
    device: &AudioDevice,
    text_primary: gpui::Hsla,
    text_muted: gpui::Hsla,
    accent: gpui::Hsla,
    radius: gpui::Pixels,
    hover: gpui::Hsla,
) -> AnyElement {
    let id = device.id;
    let mark = if device.is_default { "●" } else { "○" };
    let label = truncate_label(&device.name, 34);
    let color = if device.is_default {
        accent
    } else {
        text_primary
    };
    let row_id: SharedString = format!("{}-dev-{id}", kind.id_prefix()).into();

    div()
        .id(row_id)
        .w_full()
        .flex()
        .items_center()
        .gap(px(6.))
        .px(px(4.))
        .py(px(3.))
        .rounded(radius)
        .cursor_pointer()
        .hover(move |s| s.bg(hover))
        .child(div().text_color(if device.is_default { accent } else { text_muted }).child(mark))
        .child(div().text_color(color).text_xs().child(label))
        .on_click(move |_event, _window, cx: &mut App| {
            set_default_device(kind, id, cx);
        })
        .into_any_element()
}

fn step_volume(kind: EndpointKind, delta: f64, cx: &mut App) {
    let audio = AppState::audio(cx);
    let current = match kind {
        EndpointKind::Sink => audio.get().sink.volume,
        EndpointKind::Source => audio.get().source.volume,
    };
    let next = clamp_volume(current + delta);
    match kind {
        EndpointKind::Sink => audio.dispatch(AudioCommand::SetSinkVolume(next)),
        EndpointKind::Source => audio.dispatch(AudioCommand::SetSourceVolume(next)),
    }
}

fn toggle_mute(kind: EndpointKind, cx: &mut App) {
    let audio = AppState::audio(cx);
    match kind {
        EndpointKind::Sink => audio.dispatch(AudioCommand::ToggleSinkMute),
        EndpointKind::Source => audio.dispatch(AudioCommand::ToggleSourceMute),
    }
}

fn set_default_device(kind: EndpointKind, id: u32, cx: &mut App) {
    let audio = AppState::audio(cx);
    match kind {
        EndpointKind::Sink => audio.dispatch(AudioCommand::SetDefaultSink(id)),
        EndpointKind::Source => audio.dispatch(AudioCommand::SetDefaultSource(id)),
    }
    tracing::info!(
        "volume_popup: set default {} id={id}",
        kind.id_prefix()
    );
}

fn mute_icon(is_source: bool, muted: bool) -> &'static str {
    if is_source {
        if muted { "🎤̸" } else { "🎤" }
    } else if muted {
        "🔇"
    } else {
        "🔊"
    }
}

fn format_percent(volume: f64) -> i32 {
    if volume.is_finite() {
        (volume * 100.0).round().clamp(0.0, 150.0) as i32
    } else {
        0
    }
}

fn truncate_label(s: &str, max_chars: usize) -> String {
    let count = s.chars().count();
    if count <= max_chars {
        return s.to_string();
    }
    let take = max_chars.saturating_sub(1);
    let mut out: String = s.chars().take(take).collect();
    out.push('…');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_percent_rounds() {
        assert_eq!(format_percent(0.0), 0);
        assert_eq!(format_percent(0.35), 35);
        assert_eq!(format_percent(1.0), 100);
        assert_eq!(format_percent(1.25), 125);
        assert_eq!(format_percent(f64::NAN), 0);
    }

    #[test]
    fn mute_icons() {
        assert_eq!(mute_icon(false, true), "🔇");
        assert_eq!(mute_icon(false, false), "🔊");
        assert_eq!(mute_icon(true, true), "🎤̸");
        assert_eq!(mute_icon(true, false), "🎤");
    }

    #[test]
    fn truncate_short_unchanged() {
        assert_eq!(truncate_label("Built-in", 28), "Built-in");
    }

    #[test]
    fn truncate_long() {
        let s = "GA104 High Definition Audio Controller Digital Stereo (HDMI)";
        let t = truncate_label(s, 28);
        assert!(t.ends_with('…'));
        assert!(t.chars().count() <= 28);
    }
}
