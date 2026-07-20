//! Volume widget for the bar — sink icon + percent, click opens volume popup,
//! scroll ±5%.

use gpui::{AnyElement, App, ScrollDelta, ScrollWheelEvent, Window, div, prelude::*, px, svg};

use chronos_luau::bar::{BarSection, BarWidget};
use chronos_services::{AudioCommand, EndpointState, Service, audio::clamp_volume};
use chronos_ui::Theme;

use crate::state::AppState;

/// Step applied on each scroll tick (±5%).
const SCROLL_STEP: f64 = 0.05;

/// Pure description of what the widget should display (unit-testable).
#[derive(Debug, PartialEq, Eq)]
struct VolumeView {
    icon: &'static str,
    /// Formatted percent label without `%` suffix (e.g. `"42"`).
    percent: String,
    muted: bool,
}

fn describe(sink: &EndpointState) -> VolumeView {
    VolumeView {
        icon: volume_icon(sink.muted, sink.volume),
        percent: format_percent(sink.volume),
        muted: sink.muted,
    }
}

/// Icon buckets matching OSD (`osd/view.rs`): mute / near-zero / mid / high.
fn volume_icon(muted: bool, volume: f64) -> &'static str {
    if muted {
        "icons/speaker-mute.svg"
    } else if volume < 0.01 {
        "icons/speaker-none.svg"
    } else if volume < 0.5 {
        "icons/speaker-low.svg"
    } else {
        "icons/speaker-high.svg"
    }
}

/// Whole-percent string for the sink volume (0.0 → `"0"`, 1.0 → `"100"`).
fn format_percent(volume: f64) -> String {
    let pct = if volume.is_finite() {
        (volume * 100.0).round().clamp(0.0, 150.0) as i32
    } else {
        0
    };
    pct.to_string()
}

/// Map scroll delta to a volume step. Scroll up (negative y) → +5%.
fn scroll_volume_delta(delta: &ScrollDelta) -> f64 {
    let y = match delta {
        ScrollDelta::Lines(p) => p.y as f64,
        ScrollDelta::Pixels(p) => f64::from(p.y),
    };
    if y < 0.0 {
        SCROLL_STEP
    } else if y > 0.0 {
        -SCROLL_STEP
    } else {
        0.0
    }
}

pub struct VolumeWidget;

impl BarWidget for VolumeWidget {
    fn name(&self) -> &str {
        "volume"
    }

    fn section(&self) -> BarSection {
        BarSection::Right
    }

    fn render(&self, _window: &mut Window, cx: &App) -> AnyElement {
        let audio = AppState::audio(cx);
        let sink = audio.get().sink;
        let theme = Theme::global(cx);
        let view = describe(&sink);

        let color = if view.muted {
            theme.text.muted
        } else {
            theme.text.secondary
        };

        div()
            .id("bar-volume")
            .flex()
            .items_center()
            .gap(px(4.))
            .cursor_pointer()
            .px(px(6.))
            .py(px(2.))
            .rounded(theme.radius)
            .hover(|s| s.bg(theme.interactive.hover))
            .child(svg().path(view.icon).size(px(13.)).text_color(color))
            .child(
                div()
                    .child(format!("{}%", view.percent))
                    .text_color(color)
                    .font_family(theme.font_mono)
                    .text_size(theme.font_sizes.sm),
            )
            .on_click(|_event, window, cx: &mut App| {
                crate::volume_popup::toggle(window, cx);
            })
            .on_scroll_wheel(|event: &ScrollWheelEvent, _window, cx: &mut App| {
                let step = scroll_volume_delta(&event.delta);
                if step == 0.0 {
                    return;
                }
                let audio = AppState::audio(cx);
                let current = audio.get().sink.volume;
                let next = clamp_volume(current + step);
                audio.dispatch(AudioCommand::SetSinkVolume(next));
            })
            .into_any_element()
    }
}

/// Register the volume widget with the global bar registry.
pub fn register(cx: &mut App) {
    cx.global_mut::<chronos_luau::bar::BarWidgetRegistry>()
        .register(Box::new(VolumeWidget));
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{point, px};

    fn sink(volume: f64, muted: bool) -> EndpointState {
        EndpointState {
            volume,
            muted,
            name: "Speakers".into(),
            available: Vec::new(),
        }
    }

    #[test]
    fn icon_mute() {
        assert_eq!(volume_icon(true, 0.8), "icons/speaker-mute.svg");
        assert_eq!(volume_icon(true, 0.0), "icons/speaker-mute.svg");
    }

    #[test]
    fn icon_level_buckets() {
        assert_eq!(volume_icon(false, 0.0), "icons/speaker-none.svg");
        assert_eq!(volume_icon(false, 0.009), "icons/speaker-none.svg");
        assert_eq!(volume_icon(false, 0.01), "icons/speaker-low.svg");
        assert_eq!(volume_icon(false, 0.49), "icons/speaker-low.svg");
        assert_eq!(volume_icon(false, 0.5), "icons/speaker-high.svg");
        assert_eq!(volume_icon(false, 1.0), "icons/speaker-high.svg");
        assert_eq!(volume_icon(false, 1.5), "icons/speaker-high.svg");
    }

    #[test]
    fn format_percent_rounds() {
        assert_eq!(format_percent(0.0), "0");
        assert_eq!(format_percent(0.35), "35");
        assert_eq!(format_percent(1.0), "100");
        assert_eq!(format_percent(1.25), "125");
        assert_eq!(format_percent(f64::NAN), "0");
    }

    #[test]
    fn describe_muted() {
        let v = describe(&sink(0.4, true));
        assert_eq!(v.icon, "icons/speaker-mute.svg");
        assert_eq!(v.percent, "40");
        assert!(v.muted);
    }

    #[test]
    fn describe_loud() {
        let v = describe(&sink(0.8, false));
        assert_eq!(v.icon, "icons/speaker-high.svg");
        assert_eq!(v.percent, "80");
        assert!(!v.muted);
    }

    #[test]
    fn scroll_up_raises_volume() {
        assert_eq!(
            scroll_volume_delta(&ScrollDelta::Lines(point(0.0, -1.0))),
            SCROLL_STEP
        );
        assert_eq!(
            scroll_volume_delta(&ScrollDelta::Pixels(point(px(0.), px(-10.)))),
            SCROLL_STEP
        );
    }

    #[test]
    fn scroll_down_lowers_volume() {
        assert_eq!(
            scroll_volume_delta(&ScrollDelta::Lines(point(0.0, 1.0))),
            -SCROLL_STEP
        );
        assert_eq!(
            scroll_volume_delta(&ScrollDelta::Pixels(point(px(0.), px(10.)))),
            -SCROLL_STEP
        );
    }

    #[test]
    fn scroll_zero_is_noop() {
        assert_eq!(
            scroll_volume_delta(&ScrollDelta::Lines(point(0.0, 0.0))),
            0.0
        );
    }
}
