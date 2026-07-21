//! Static permission card (Claude Code mock) — no backend wiring.
//! Styles from `design/System Sidebar.dc.html`.

use gpui::{IntoElement, div, prelude::*, px, rgb};
use gpui_rsx::rsx;

pub fn render_permission_card() -> impl IntoElement {
    rsx! {
        <div
            flex_none
            px={px(14.)}
            py={px(12.)}
            border_b_1
            border_color={rgb(0x23_23_36)}
            bg={rgb(0x1e_1e_30)}
        >
            <div
                text_size={px(13.)}
                font_weight={gpui::FontWeight::SEMIBOLD}
                text_color={rgb(0xcd_d6_f4)}
                mb={px(2.)}
            >
                {"Claude Code"}
            </div>
            <div
                text_size={px(11.)}
                text_color={rgb(0xa6_ad_c8)}
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
                    border_color={rgb(0x00_7a_cc)}
                    text_color={rgb(0x00_7a_cc)}
                    text_size={px(11.5)}
                    font_weight={gpui::FontWeight::SEMIBOLD}
                    cursor_pointer
                    hover={|s| s.border_color(rgb(0xcb_a6_f7)).text_color(rgb(0xcb_a6_f7))}
                >
                    {"Allow"}
                </div>
                <div
                    id="perm-deny"
                    class="flex-1 items-center justify-center"
                    py={px(6.)}
                    rounded={px(6.)}
                    border_1
                    border_color={rgb(0x45_47_5a)}
                    text_color={rgb(0xa6_ad_c8)}
                    text_size={px(11.5)}
                    font_weight={gpui::FontWeight::SEMIBOLD}
                    cursor_pointer
                    hover={|s| s.bg(rgb(0x23_23_36))}
                >
                    {"Deny"}
                </div>
            </div>
        </div>
    }
}
