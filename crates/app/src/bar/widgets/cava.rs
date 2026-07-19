//! Cava visualizer widget — 24 vertical bars in the bar center.

use gpui::{AnyElement, App, SharedString, Window, div, prelude::*, px};

use chronos_luau::bar::{BarSection, BarWidget};
use chronos_services::{Service, cava::BAR_COUNT};
use chronos_ui::Theme;

use crate::state::AppState;

/// Max bar height in px (bar itself is 32px tall).
const MAX_BAR_H: f32 = 18.;
/// Minimum visible stub so idle frame is not invisible.
const MIN_BAR_H: f32 = 2.;
const BAR_W: f32 = 3.;
const BAR_GAP: f32 = 2.5;

/// Map a 0..=100 level to pixel height.
fn level_to_height(level: u8) -> f32 {
    let t = f32::from(level).clamp(0.0, 100.0) / 100.0;
    MIN_BAR_H + t * (MAX_BAR_H - MIN_BAR_H)
}

pub struct CavaWidget;

impl BarWidget for CavaWidget {
    fn name(&self) -> &str {
        "cava"
    }

    fn section(&self) -> BarSection {
        BarSection::Center
    }

    fn render(&self, _window: &mut Window, cx: &App) -> AnyElement {
        let bars = AppState::cava(cx).get();
        let theme = Theme::global(cx);
        let color = theme.accent.primary;

        // Pad/truncate to BAR_COUNT so layout stays stable if cava is down.
        let mut levels = bars;
        levels.resize(BAR_COUNT, 0);

        let strips: Vec<AnyElement> = levels
            .iter()
            .enumerate()
            .map(|(i, &level)| {
                let h = level_to_height(level);
                div()
                    .id(SharedString::from(format!("cava-bar-{i}")))
                    .w(px(BAR_W))
                    .h(px(h))
                    .rounded(px(1.))
                    .bg(color)
                    .into_any_element()
            })
            .collect();

        div()
            .id("bar-cava")
            .flex()
            .items_end()
            .gap(px(BAR_GAP))
            .h(px(MAX_BAR_H))
            .px(px(4.))
            .children(strips)
            .into_any_element()
    }
}

/// Register the cava visualizer with the global bar registry.
pub fn register(cx: &mut App) {
    cx.global_mut::<chronos_luau::bar::BarWidgetRegistry>()
        .register(Box::new(CavaWidget));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn level_zero_is_min() {
        assert!((level_to_height(0) - MIN_BAR_H).abs() < f32::EPSILON);
    }

    #[test]
    fn level_max_is_max() {
        assert!((level_to_height(100) - MAX_BAR_H).abs() < f32::EPSILON);
    }

    #[test]
    fn level_mid_in_range() {
        let h = level_to_height(50);
        assert!(h > MIN_BAR_H && h < MAX_BAR_H);
    }
}
