//! Spectrum-bar row for sidebar meters (CPU/RAM/GPU/net).
//! Mockup layout: label row above, then N thin vertical bars.
//! `HISTORY_LEN` = 24 (System Sidebar.dc.html).

use std::collections::VecDeque;

use gpui::{Hsla, IntoElement, div, prelude::*, px, rgb};
use chronos_ui::Theme;

/// Ring depth — mockup renders 24 bars.
pub const HISTORY_LEN: usize = 24;

/// Mockup track heights (px).
pub const H_CPU: f32 = 38.;
pub const H_RAM: f32 = 34.;
pub const H_GPU: f32 = 26.;
pub const H_NET: f32 = 26.;

/// Spectrum bar colors: mockup accents on dark, status tokens on light.
pub fn color_cpu(theme: &Theme) -> Hsla {
    if theme.is_light {
        theme.status.info
    } else {
        rgb(0x89_dc_eb).into()
    }
}
pub fn color_ram(theme: &Theme) -> Hsla {
    if theme.is_light {
        theme.status.info
    } else {
        rgb(0x89_b4_fa).into()
    }
}
pub fn color_gpu(theme: &Theme) -> Hsla {
    if theme.is_light {
        theme.status.warning
    } else {
        rgb(0xf9_e2_af).into()
    }
}
pub fn color_net(theme: &Theme) -> Hsla {
    theme.text.muted
}
pub fn color_label(theme: &Theme) -> Hsla {
    theme.text.secondary
}
pub fn color_value_default(theme: &Theme) -> Hsla {
    theme.text.primary
}

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

/// Label above, bars below (mockup structure). `value_color` tints the
/// right-hand value (CPU/RAM/GPU match bar color; net uses text primary).
pub fn render_spectrum_row(
    label: &str,
    history: &SpectrumHistory,
    value_text: &str,
    bar_color: Hsla,
    value_color: Hsla,
    bar_height: f32,
    theme: &Theme,
) -> impl IntoElement {
    let max = history.samples.iter().cloned().fold(1.0_f32, f32::max);
    // Always paint HISTORY_LEN columns (pad leading zeros) — mockup is 24-wide.
    let pad = HISTORY_LEN.saturating_sub(history.samples.len());
    let mut values: Vec<f32> = std::iter::repeat_n(0.0, pad)
        .chain(history.samples.iter().copied())
        .collect();
    if values.len() > HISTORY_LEN {
        values.drain(0..values.len() - HISTORY_LEN);
    }
    let bars: Vec<_> = values
        .iter()
        .map(|&v| {
            let height_pct = (v / max).clamp(0.0, 1.0);
            let h = if history.samples.is_empty() {
                2.0
            } else {
                (bar_height * height_pct).max(2.0)
            };
            div().flex_1().h(px(h)).rounded(px(1.)).bg(bar_color)
        })
        .collect();

    div()
        .flex()
        .flex_col()
        .gap(px(6.))
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .child(
                    div()
                        .text_size(px(11.))
                        .text_color(color_label(theme))
                        .child(label.to_string()),
                )
                .child(
                    div()
                        .font_family("JetBrains Mono")
                        .text_size(px(11.))
                        .text_color(value_color)
                        .child(value_text.to_string()),
                ),
        )
        .child(
            div()
                .flex()
                .items_end()
                .gap(px(2.))
                .h(px(bar_height))
                .children(bars),
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ring_buffer_holds_at_most_24_samples_oldest_dropped_first() {
        let mut history = SpectrumHistory::default();
        for i in 0..30 {
            push_sample(&mut history, i as f32);
        }
        assert_eq!(history.samples.len(), 24);
        assert_eq!(history.samples.front().copied(), Some(6.0));
        assert_eq!(history.samples.back().copied(), Some(29.0));
    }

    #[test]
    fn empty_history_has_no_samples() {
        let history = SpectrumHistory::default();
        assert!(history.samples.is_empty());
    }

    #[test]
    fn history_len_matches_mockup() {
        assert_eq!(HISTORY_LEN, 24);
    }
}
