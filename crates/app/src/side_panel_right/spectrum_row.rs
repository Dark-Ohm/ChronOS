//! Reusable spectrum-bar row: label, N thin vertical bars (history), and
//! a right-aligned formatted value. Used for CPU/RAM/GPU and network
//! down/up — one component, five call sites.

use std::collections::VecDeque;

use chronos_ui::Theme;
use gpui::{Hsla, IntoElement, div, prelude::*, px};

const HISTORY_LEN: usize = 14;
const BAR_HEIGHT_PX: f32 = 52.;

#[derive(Default)]
pub struct SpectrumHistory {
    pub samples: VecDeque<f32>,
}

/// Push a new sample, dropping the oldest once the buffer exceeds
/// `HISTORY_LEN`.
pub fn push_sample(history: &mut SpectrumHistory, value: f32) {
    history.samples.push_back(value);
    while history.samples.len() > HISTORY_LEN {
        history.samples.pop_front();
    }
}

pub fn render_spectrum_row(
    label: &str,
    history: &SpectrumHistory,
    value_text: &str,
    color: Hsla,
    theme: &Theme,
) -> impl IntoElement {
    let max = history.samples.iter().cloned().fold(1.0_f32, f32::max);
    let bars: Vec<_> = history
        .samples
        .iter()
        .map(|&v| {
            let height_pct = (v / max).clamp(0.0, 1.0);
            // Floor of 2px so a zero-ish sample still reads as a tick.
            let h = (BAR_HEIGHT_PX * height_pct).max(if history.samples.is_empty() {
                0.0
            } else {
                2.0
            });
            div().flex_1().h(px(h)).bg(color)
        })
        .collect();

    div()
        .flex()
        .items_center()
        .gap(px(12.))
        .py(px(10.))
        .child(
            div()
                .w(px(34.))
                .font_family(theme.font_mono)
                .text_size(px(10.))
                .text_color(theme.text.secondary)
                .child(label.to_string()),
        )
        .child(
            div()
                .flex_1()
                .flex()
                .items_end()
                .gap(px(2.))
                .h(px(BAR_HEIGHT_PX))
                .children(bars),
        )
        .child(
            div()
                .w(px(48.))
                .font_family(theme.font_mono)
                .text_size(px(12.))
                .text_color(theme.text.primary)
                .child(value_text.to_string()),
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ring_buffer_holds_at_most_14_samples_oldest_dropped_first() {
        let mut history = SpectrumHistory::default();
        for i in 0..20 {
            push_sample(&mut history, i as f32);
        }
        assert_eq!(history.samples.len(), 14);
        assert_eq!(history.samples.front().copied(), Some(6.0));
        assert_eq!(history.samples.back().copied(), Some(19.0));
    }

    #[test]
    fn empty_history_has_no_samples() {
        let history = SpectrumHistory::default();
        assert!(history.samples.is_empty());
    }
}
