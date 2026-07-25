//! Volume popup view — Sound UI: Volume + Microphone sliders, device
//! menus (grow-window inline list), footer dual mute.
//!
//! ## Visual language (matches `updates_popup` / `notifications`)
//! - Light C: elevated shadow + 1px inner accent ring + top accent glow +
//!   hexagon-sigil watermark (same recipe as `updates_popup/view.rs`).
//! - Animated via the vendored fork crate `gpui_animation` (`TransitionExt`,
//!   `transition_on_hover`) — the project's animation crate at
//!   `Source/gpui-animation`. Interactive controls that don't need view
//!   state (footer mute buttons, device rows) are wrapped in
//!   `AnimatedWrapper` so border/color morph to accent on hover with a
//!   spring ease; the device picker springs open/closed via
//!   `transition_when` (EaseOutBack).
//!
//! ## Build split
//! - Static chrome (header «Sound» + ✕) is `rsx!`.
//! - Live meters/menus are the `div()` builder because they need
//!   `on_mouse_down` / `on_drag` + `cx.listener` stateful interaction.
//!   Those keep the plain `Div` hover (matching the other popups); the
//!   `gpui_animation` wrapper is used where the click handler only needs
//!   `&mut App` (no `cx.listener`).
//!
//! See T121 report for the rsx↔div map and the fork deltas.

use std::time::Duration;

use gpui::{
    AnyElement, App, BoxShadow, Context, Corners, DragMoveEvent, InteractiveElement, IntoElement,
    MouseButton, MouseDownEvent, Render, SharedString, Styled, Window, canvas, div, img, prelude::*,
    px, svg,
};
use gpui::EmptyView;
use gpui_animation::animation::TransitionExt;
use gpui_animation::transition::Transition;
use gpui_rsx::rsx;

use chronos_services::{
    AudioCommand, AudioDevice, AudioState, EndpointState, Service, audio::clamp_volume,
};
use chronos_ui::Theme;

use crate::state::AppState;
use crate::volume_popup::{POPUP_WIDTH, close_this, resize_to_fit};

const PAD: f32 = 14.;
/// Slider track height (mockup: 4px).
const TRACK_H: f32 = 4.;
/// Slider thumb diameter (mockup: 13px).
const THUMB: f32 = 13.;
const MAX_DEVICE_ROWS: usize = 8;

/// Spring-overshoot easing adapter so the fork's `EasingCurve::EaseOutBack`
/// can drive `gpui_animation` declarative transitions (the animation crate
/// only ships quad/cubic/sine/exponential curves natively).
struct SpringBack(f32);

impl Transition for SpringBack {
    fn calculate(&self, t: f32) -> f32 {
        gpui::easing::EasingCurve::EaseOutBack(self.0).sample(t)
    }
}

/// Marker type for the slider drag gesture (no payload needed — the
/// drag-move listener already closes over the endpoint `kind`).
pub struct VolumeSliderDrag;

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
    /// Open device picker under Volume / Microphone (or neither).
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
        let theme = *Theme::global(cx);
        let expanded = self.expanded;

        let bg = theme.bg.primary;
        let text_primary = theme.text.primary;
        let text_muted = theme.text.muted;
        let text_secondary = theme.text.secondary;
        let divider = theme.border.default;
        let radius = theme.radius;
        let radius_lg = theme.radius_lg;
        let hover = theme.interactive.hover;
        let accent = theme.accent.primary;
        let border_subtle = theme.border.subtle;
        let is_light = theme.is_light;
        let font_mono = theme.font_mono;

        // ── Card (builder) ──────────────────────────────────────────
        // Light C recipe identical to updates_popup/view.rs: elevated
        // shadow + inset accent ring + top glow + sigil watermark.
        // PLUS a real backdrop-blur (fork `window.paint_blur`) behind the
        // card so the panel reads as frosted glass — the one premium touch
        // the other popups don't have yet.
        let blur_layer = div()
            .absolute()
            .inset_0()
            .child(canvas(
                |_bounds, _window, _cx| {},
                move |bounds, _state, window: &mut Window, _cx: &mut App| {
                    window.paint_blur(
                        bounds,
                        px(18.0),
                        Corners::all(radius_lg),
                        gpui::Hsla {
                            h: 0.0,
                            s: 0.0,
                            l: 1.0,
                            a: 0.06,
                        },
                        1.15,
                    );
                },
            ));

        let mut card = div()
            .relative()
            .flex_col()
            .w(px(POPUP_WIDTH))
            .rounded(radius_lg)
            .bg(bg.alpha(0.82))
            .border_1()
            .border_color(border_subtle)
            .child(blur_layer)
            .overflow_hidden();

        if is_light {
            card = card
                .shadow(vec![
                    BoxShadow::new(px(0.), px(6.), gpui::rgba(0x3c_40_6e29).into())
                        .blur_radius(px(24.)),
                    BoxShadow::new(px(0.), px(0.), gpui::rgba(0x007a_cc26).into())
                        .spread_radius(px(1.))
                        .inset(),
                ])
                .child(
                    div()
                        .absolute()
                        .top(px(0.))
                        .left(px(0.))
                        .right(px(0.))
                        .h(px(1.))
                        .bg(accent)
                        .opacity(0.4),
                )
                .child(
                    svg()
                        .path("icons/hexagon-sigil.svg")
                        .absolute()
                        .top(px(-30.))
                        .right(px(-30.))
                        .size(px(140.))
                        .text_color(accent)
                        .opacity(0.18),
                );
        }

        // ── Header «Sound» + ✕ (rsx) ────────────────────────────────
        let header = rsx! {
            <div
                class="w-full flex items-center justify-between"
                px={px(PAD)}
                py={px(12.)}
                border_b_1
                border_color={divider}
            >
                <div
                    text_color={text_primary}
                    font_family={font_mono}
                    text_size={px(13.)}
                    font_weight={gpui::FontWeight::SEMIBOLD}
                >
                    { "Sound" }
                </div>
                <div
                    id="volume-popup-close"
                    w={px(22.)}
                    h={px(22.)}
                    rounded={px(6.)}
                    class="flex items-center justify-center"
                    cursor_pointer
                    text_color={text_muted}
                    hover={|s| s.bg(hover)}
                    onClick={move |_ev, window, cx| {
                        close_this(window, cx);
                    }}
                >
                    { img("icons/x.svg").w(px(13.)).h(px(13.)) }
                </div>
            </div>
        };

        card.child(header)
            .child(endpoint_block(
                "Volume",
                EndpointKind::Sink,
                &audio.sink,
                expanded,
                text_primary,
                text_secondary,
                text_muted,
                accent,
                hover,
                radius,
                border_subtle,
                font_mono,
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
                hover,
                radius,
                border_subtle,
                font_mono,
                cx,
            ))
            .child(footer(
                &audio,
                text_muted,
                accent,
                border_subtle,
                radius,
                font_mono,
                hover,
            ))
    }
}

/// Footer: two outlined mute buttons. Wrapped in `AnimatedWrapper` so the
/// border/color morph to accent on hover with a spring ease (fork crate).
/// These clicks only need `&mut App`, so they are compatible with the
/// `AnimatedWrapper::on_click` signature.
fn footer(
    audio: &AudioState,
    text_muted: gpui::Hsla,
    accent: gpui::Hsla,
    border_subtle: gpui::Hsla,
    radius: gpui::Pixels,
    font_mono: &'static str,
    hover: gpui::Hsla,
) -> AnyElement {
    let sink_muted = audio.sink.muted;
    let source_muted = audio.source.muted;

    let out_label = if sink_muted { "Unmute output" } else { "Mute output" };
    let mic_label = if source_muted { "Unmute mic" } else { "Mute mic" };
    let out_color = if sink_muted { accent } else { text_muted };
    let mic_color = if source_muted { accent } else { text_muted };

    div()
        .w_full()
        .flex()
        .gap(px(8.))
        .px(px(PAD))
        .py(px(12.))
        .border_t_1()
        .border_color(border_subtle)
        .child(
            div()
                .id("volume-popup-mute-output")
                .flex_1()
                .text_center()
                .py(px(8.))
                .rounded(radius)
                .border_1()
                .border_color(border_subtle)
                .text_color(out_color)
                .font_family(font_mono)
                .text_size(px(12.))
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .cursor_pointer()
                .child(out_label)
                .with_transition("volume-popup-mute-output")
                .transition_on_hover(Duration::from_millis(220), SpringBack(1.6), move |_hovered, s| {
                    s.border_color(accent).text_color(accent)
                })
                .on_click(move |_event, _window, cx: &mut App| {
                    toggle_mute(EndpointKind::Sink, cx);
                }),
        )
        .child(
            div()
                .id("volume-popup-mute-mic")
                .flex_1()
                .text_center()
                .py(px(8.))
                .rounded(radius)
                .border_1()
                .border_color(border_subtle)
                .text_color(mic_color)
                .font_family(font_mono)
                .text_size(px(12.))
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .cursor_pointer()
                .child(mic_label)
                .with_transition("volume-popup-mute-mic")
                .transition_on_hover(Duration::from_millis(220), SpringBack(1.6), move |_hovered, s| {
                    s.border_color(accent).text_color(accent)
                })
                .on_click(move |_event, _window, cx: &mut App| {
                    toggle_mute(EndpointKind::Source, cx);
                }),
        )
        .into_any_element()
}

/// One endpoint section: title row (mute icon + name + device subtitle +
/// chevron + %), drag/click slider, and an inline device picker that
/// springs open/closed via `gpui_animation`.
fn endpoint_block(
    title: &'static str,
    kind: EndpointKind,
    ep: &EndpointState,
    expanded: Option<EndpointKind>,
    text_primary: gpui::Hsla,
    text_secondary: gpui::Hsla,
    text_muted: gpui::Hsla,
    accent: gpui::Hsla,
    hover: gpui::Hsla,
    radius: gpui::Pixels,
    border_subtle: gpui::Hsla,
    font_mono: &'static str,
    cx: &mut Context<VolumePopupView>,
) -> AnyElement {
    let muted = ep.muted;
    let volume = ep.volume;
    let fraction = volume.clamp(0.0, 1.0) as f32;
    let fill_w = (POPUP_WIDTH - 2.0 * PAD) * fraction;
    let percent = format_percent(volume);
    let title_color = if muted { text_muted } else { text_primary };
    let prefix = kind.id_prefix();
    let is_open = expanded == Some(kind);
    let chevron = if is_open { "▾" } else { "▸" };
    let device_label = if ep.name.is_empty() {
        title.to_string()
    } else {
        ep.name.clone()
    };
    let title_id: SharedString = format!("{prefix}-title").into();

    // ── Slider drag + click (builder, stateful) ────────────────────
    // Uses `cx.listener` (needs `&mut Context<Self>`), so it stays a
    // plain `Div`; the inner track gets a hover glow via a nested
    // AnimatedWrapper (its hover only needs `&mut App` via the crate's
    // internal hook).
    let mouse_listener = cx.listener(move |_this, ev: &MouseDownEvent, _window, cx: &mut Context<VolumePopupView>| {
        let frac = frac_from_window_x(f32::from(ev.position.x));
        set_volume_unmute_if_needed(kind, frac, cx);
    });
    let drag_listener = cx.listener(
        move |_this, ev: &DragMoveEvent<VolumeSliderDrag>, _window, cx: &mut Context<VolumePopupView>| {
            let frac = frac_from_window_x(f32::from(ev.event.position.x));
            set_volume_unmute_if_needed(kind, frac, cx);
        },
    );
    let slider_id: SharedString = format!("{prefix}-slider").into();

    let slider = div()
        .id(slider_id)
        .w_full()
        .h(px(TRACK_H + 10.)) // hit area taller than visual track
        .flex()
        .items_center()
        .cursor_pointer()
        .on_mouse_down(MouseButton::Left, mouse_listener)
        .on_drag(VolumeSliderDrag, |_, _, _, cx| cx.new(|_| EmptyView))
        .on_drag_move(drag_listener)
        .child(
            div()
                .w_full()
                .h(px(TRACK_H))
                .rounded(px(2.))
                .bg(theme_track_bg(text_muted))
                .relative()
                .child(
                    div()
                        .absolute()
                        .left(px(0.))
                        .top(px(0.))
                        .bottom(px(0.))
                        .w(px(fill_w.max(0.)))
                        .rounded(px(2.))
                        .bg(if muted { text_muted } else { accent }),
                )
                .child(
                    div()
                        .absolute()
                        .top(px((TRACK_H - THUMB) / 2.))
                        .left(px(fill_w.max(0.) - THUMB / 2.))
                        .size(px(THUMB))
                        .rounded(px(THUMB / 2.))
                        .bg(if muted { text_muted } else { text_primary })
                        .shadow(vec![
                            BoxShadow::new(px(0.), px(1.), gpui::rgba(0x0000_0000).into())
                                .blur_radius(px(2.)),
                        ]),
                ),
        );

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
                .gap(px(7.))
                .child(
                    div()
                        .id(SharedString::from(format!("{prefix}-mute-icon")))
                        .w(px(24.))
                        .h(px(24.))
                        .rounded(radius)
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_color(text_muted)
                        .child(mute_icon(kind.is_source(), muted))
                        .with_transition(format!("{prefix}-mute-icon"))
                        .transition_on_hover(Duration::from_millis(160), SpringBack(1.4), move |_hovered, s| {
                            s.bg(accent).text_color(hover)
                        })
                        .on_click(move |_event, _window, cx: &mut App| {
                            toggle_mute(kind, cx);
                        }),
                )
                .child(
                    div()
                        .flex_col()
                        .child(
                            div()
                                .text_color(title_color)
                                .text_size(px(12.5))
                                .font_weight(gpui::FontWeight::MEDIUM)
                                .child(title),
                        )
                        .child(
                            div()
                                .text_color(text_muted)
                                .text_xs()
                                .child(truncate_label(&device_label, 28)),
                        ),
                )
                .child(div().text_color(text_muted).child(chevron)),
        )
        .child(
            div()
                .font_family(font_mono)
                .text_size(px(11.))
                .text_color(text_muted)
                .child(if muted {
                    "Muted".to_string()
                } else {
                    format!("{percent}%")
                }),
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
        .gap(px(8.))
        .px(px(PAD))
        .py(px(12.))
        .child(title_row)
        .child(slider)
        .child(
            div()
                .id(format!("{prefix}-devices"))
                .overflow_hidden()
                .with_transition(format!("{prefix}-devices"))
                .transition_when(is_open, Duration::from_millis(260), SpringBack(1.8), |s| {
                    // The list is present in both states; height morphs via
                    // max-height + opacity spring when the picker opens.
                    s.opacity(1.0).max_h(px(400.))
                })
                .child(device_list),
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
    let mark = if device.is_default { "✓" } else { "" };
    let label = truncate_label(&device.name, 34);
    let color = if device.is_default { accent } else { text_primary };
    let row_id: SharedString = format!("{}-dev-{id}", kind.id_prefix()).into();

    div()
        .id(row_id.clone())
        .w_full()
        .flex()
        .items_center()
        .justify_between()
        .gap(px(8.))
        .px(px(8.))
        .py(px(6.))
        .rounded(radius)
        .cursor_pointer()
        .child(div().text_color(color).text_xs().child(label))
        .child(
            div()
                .text_color(if device.is_default {
                    accent
                } else {
                    text_muted
                })
                .text_size(px(12.))
                .child(mark),
        )
        .with_transition(row_id.clone())
        .transition_on_hover(Duration::from_millis(150), SpringBack(1.5), move |_hovered, s| {
            s.bg(hover).text_color(accent)
        })
        .on_click(move |_event, _window, cx: &mut App| {
            set_default_device(kind, id, cx);
        })
        .into_any_element()
}

/// Convert a pointer x (popup-window coordinates) to a 0..=1 fraction.
///
/// The card is the full window width (`POPUP_WIDTH`) with section padding
/// `PAD`, so the track starts at `PAD` and spans `POPUP_WIDTH - 2*PAD`.
fn frac_from_window_x(x: f32) -> f64 {
    let track_left = PAD;
    let track_w = POPUP_WIDTH - 2.0 * PAD;
    let rel = (x - track_left) / track_w;
    rel.clamp(0.0, 1.0) as f64
}

/// Set volume to `frac` and unmute the endpoint if it is currently muted
/// (mockup `onVolumeChange` clears mute). Reads live state to avoid a
/// stale double-toggle during a drag.
fn set_volume_unmute_if_needed(kind: EndpointKind, frac: f64, cx: &mut App) {
    let v = clamp_volume(frac);
    let audio = AppState::audio(cx);
    let currently_muted = match kind {
        EndpointKind::Sink => audio.get().sink.muted,
        EndpointKind::Source => audio.get().source.muted,
    };
    match kind {
        EndpointKind::Sink => {
            audio.dispatch(AudioCommand::SetSinkVolume(v));
            if currently_muted {
                audio.dispatch(AudioCommand::ToggleSinkMute);
            }
        }
        EndpointKind::Source => {
            audio.dispatch(AudioCommand::SetSourceVolume(v));
            if currently_muted {
                audio.dispatch(AudioCommand::ToggleSourceMute);
            }
        }
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
    tracing::info!("volume_popup: set default {} id={id}", kind.id_prefix());
}

fn mute_icon(is_source: bool, muted: bool) -> AnyElement {
    let path = if is_source {
        if muted {
            "icons/microphone-mute.svg"
        } else {
            "icons/microphone.svg"
        }
    } else if muted {
        "icons/speaker-mute.svg"
    } else {
        "icons/speaker-high.svg"
    };
    svg().path(path).size(px(15.)).into_any_element()
}

/// Track background per mockup (dark `#313244`-ish).
fn theme_track_bg(text_muted: gpui::Hsla) -> gpui::Hsla {
    text_muted
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

    #[test]
    fn frac_clamps() {
        assert_eq!(frac_from_window_x(-50.0), 0.0);
        assert_eq!(frac_from_window_x(1e9), 1.0);
        let mid = frac_from_window_x(PAD + (POPUP_WIDTH - 2.0 * PAD) / 2.0);
        assert!((mid - 0.5).abs() < 1e-6);
    }
}
