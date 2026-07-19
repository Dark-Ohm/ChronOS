//! OSD volume strip view — progress bar + icon/label from `OsdPopupState`.

use gpui::{App, Context, FontWeight, Render, Window, div, prelude::*, px};

use chronos_ui::Theme;

use crate::osd::OsdPopupState;

/// Empty view; all content is read from the `OsdPopupState` global.
pub struct OsdView {}

impl OsdView {
    pub fn new(_cx: &mut App) -> Self {
        Self {}
    }
}

impl Render for OsdView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = Theme::global(cx);
        let Some(display) = cx.global::<OsdPopupState>().display().cloned() else {
            return div().into_any_element();
        };

        let fraction = display.bar_fraction();
        let percent = display.percent_label();
        let muted = display.muted;
        let is_source = display.is_source;
        let name = display.name;

        let icon = if is_source {
            if muted { "🎤̸" } else { "🎤" }
        } else if muted {
            "🔇"
        } else if fraction < 0.01 {
            "🔈"
        } else if fraction < 0.5 {
            "🔉"
        } else {
            "🔊"
        };

        let kind_label = if is_source {
            "Микрофон"
        } else {
            "Громкость"
        };

        let bg = theme.bg.elevated;
        let bar_track = theme.bg.secondary;
        let bar_fill = if muted {
            theme.text.muted
        } else {
            theme.accent.primary
        };
        let text_primary = if muted {
            theme.text.muted
        } else {
            theme.text.primary
        };
        let text_secondary = theme.text.secondary;
        let radius = theme.radius_lg;

        // Outer: full layer-shell width (BOTTOM|LEFT|RIGHT), centre the card.
        let card = div()
            .flex()
            .items_center()
            .gap(px(12.))
            .w(px(320.))
            .h(px(80.))
            .px(px(16.))
            .py(px(12.))
            .rounded(radius)
            .bg(bg)
            .child(
                div()
                    .text_color(text_primary)
                    .text_lg()
                    .child(icon.to_string()),
            )
            .child(
                div()
                    .flex_1()
                    .flex_col()
                    .gap(px(6.))
                    .child(
                        div()
                            .flex()
                            .justify_between()
                            .items_center()
                            .child(
                                div()
                                    .text_color(text_primary)
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child(kind_label.to_string()),
                            )
                            .child(div().text_color(text_secondary).child(if muted {
                                "mute".to_string()
                            } else {
                                format!("{percent}%")
                            })),
                    )
                    // Track + fill.
                    .child(
                        div()
                            .w_full()
                            .h(px(8.))
                            .rounded(theme.radius)
                            .bg(bar_track)
                            .overflow_hidden()
                            .child(
                                div()
                                    .h_full()
                                    .w(px((320.0 - 16.0 * 2.0 - 28.0) * fraction))
                                    .rounded(theme.radius)
                                    .bg(bar_fill),
                            ),
                    )
                    .child(
                        div()
                            .text_color(text_secondary)
                            .text_xs()
                            .child(if name.is_empty() { String::new() } else { name }),
                    ),
            );

        div()
            .size_full()
            .flex()
            .justify_center()
            .items_end()
            .child(card)
            .into_any_element()
    }
}
