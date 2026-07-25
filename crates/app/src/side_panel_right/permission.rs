//! Static permission card (Claude Code mock) — no backend wiring.
//! Styles from `design/System Sidebar.dc.html`.

use gpui::{App, IntoElement, div, prelude::*, px};
use chronos_ui::Theme;

use crate::side_panel_right::surfaces;

pub fn render_permission_card(cx: &App) -> impl IntoElement {
    let theme = *Theme::global(cx);
    div()
        .flex_none()
        .px(px(14.))
        .py(px(12.))
        .border_b_1()
        .border_color(theme.border.subtle)
        .bg(surfaces::card(&theme))
        .child(
            div()
                .text_size(px(13.))
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .text_color(theme.text.primary)
                .mb(px(2.))
                .child("Claude Code"),
        )
        .child(
            div()
                .text_size(px(11.))
                .text_color(theme.text.secondary)
                .mb(px(9.))
                .child("Claude needs your permission to run a command"),
        )
        .child(
            div()
                .flex()
                .gap(px(7.))
                .child(
                    div()
                        .id("perm-allow")
                        .flex_1()
                        .flex()
                        .items_center()
                        .justify_center()
                        .py(px(6.))
                        .rounded(px(6.))
                        .border_1()
                        .border_color(theme.accent.primary)
                        .text_color(theme.accent.primary)
                        .text_size(px(11.5))
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .cursor_pointer()
                        .hover(|s| {
                            s.border_color(theme.accent.hover)
                                .text_color(theme.accent.hover)
                        })
                        .child("Allow"),
                )
                .child(
                    div()
                        .id("perm-deny")
                        .flex_1()
                        .flex()
                        .items_center()
                        .justify_center()
                        .py(px(6.))
                        .rounded(px(6.))
                        .border_1()
                        .border_color(theme.text.disabled)
                        .text_color(theme.text.secondary)
                        .text_size(px(11.5))
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .cursor_pointer()
                        .hover(|s| s.bg(theme.border.subtle))
                        .child("Deny"),
                ),
        )
}
