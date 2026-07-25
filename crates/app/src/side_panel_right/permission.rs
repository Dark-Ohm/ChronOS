//! Static permission card (Claude Code mock) — no backend wiring.
//! Styles from `design/System Sidebar.dc.html`.

use gpui::{App, IntoElement, div, prelude::*, px};
use chronos_ui::Theme;
use gpui_rsx::rsx;

pub fn render_permission_card(cx: &App) -> impl IntoElement {
    let theme = *Theme::global(cx);
    rsx! {
        <div
            flex_none
            px={px(14.)}
            py={px(12.)}
            border_b_1
            border_color={theme.border.subtle}
            bg={theme.bg.primary}
        >
            <div
                text_size={px(13.)}
                font_weight={gpui::FontWeight::SEMIBOLD}
                text_color={theme.text.primary}
                mb={px(2.)}
            >
                {"Claude Code"}
            </div>
            <div
                text_size={px(11.)}
                text_color={theme.text.secondary}
                mb={px(9.)}
            >
                {"Claude needs your permission to run a command"}
            </div>
            <div class="flex" gap={px(7.)}>
                <div
                    id="perm-allow"
                    class="flex-1 items-center justify-center"
                    py={px(6.)}
                    rounded={px(6.)}
                    border_1
                    border_color={theme.accent.primary}
                    text_color={theme.accent.primary}
                    text_size={px(11.5)}
                    font_weight={gpui::FontWeight::SEMIBOLD}
                    cursor_pointer
                    hover={|s| s.border_color(theme.accent.hover).text_color(theme.accent.hover)}
                >
                    {"Allow"}
                </div>
                <div
                    id="perm-deny"
                    class="flex-1 items-center justify-center"
                    py={px(6.)}
                    rounded={px(6.)}
                    border_1
                    border_color={theme.text.disabled}
                    text_color={theme.text.secondary}
                    text_size={px(11.5)}
                    font_weight={gpui::FontWeight::SEMIBOLD}
                    cursor_pointer
                    hover={|s| s.bg(theme.border.subtle)}
                >
                    {"Deny"}
                </div>
            </div>
        </div>
    }
}
