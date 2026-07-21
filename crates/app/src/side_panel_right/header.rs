//! Panel header — active window title (static for now) + close button.
//! Styles from `design/System Sidebar.dc.html` (header block).

use gpui::{IntoElement, div, img, prelude::*, px, rgb};
use gpui_rsx::rsx;

/// Static title until active-window wiring lands.
const WINDOW_TITLE: &str = "kitty";

pub fn render_header() -> impl IntoElement {
    rsx! {
        <div
            class="flex items-center justify-between"
            flex_none
            px={px(14.)}
            py={px(10.)}
            border_b_1
            border_color={rgb(0x23_23_36)}
        >
            <div
                text_size={px(11.5)}
                font_weight={gpui::FontWeight::SEMIBOLD}
                text_color={rgb(0xa6_ad_c8)}
            >
                {WINDOW_TITLE}
            </div>
            <div
                id="side-panel-close"
                w={px(20.)}
                h={px(20.)}
                rounded={px(6.)}
                class="flex items-center justify-center"
                text_color={rgb(0x6c_70_86)}
                cursor_pointer
                hover={|s| s.bg(rgb(0x23_23_36)).text_color(rgb(0xcd_d6_f4))}
                onClick={|_ev, window, cx| {
                    crate::side_panel_right::close_this(window, cx);
                }}
            >
                {img("icons/x.svg").w(px(12.)).h(px(12.))}
            </div>
        </div>
    }
}
