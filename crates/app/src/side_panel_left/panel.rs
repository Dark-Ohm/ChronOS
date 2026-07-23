use gpui::{Context, IntoElement, Window, div, img, prelude::*, px, rgb};
use gpui_rsx::rsx;

use super::SidePanelLeft;
use super::state::AgentStatus;

fn status_color(status: AgentStatus) -> gpui::Rgba {
    match status {
        AgentStatus::Connected => rgb(0xa6_e3_a1),
        AgentStatus::Disconnected => rgb(0xf3_8b_a8),
        AgentStatus::Thinking => rgb(0xf9_e2_af),
    }
}

pub fn render_panel(
    panel: &SidePanelLeft,
    _window: &mut Window,
    _cx: &mut Context<SidePanelLeft>,
) -> impl IntoElement {
    let dot_color = status_color(panel.state.agent_status);

    rsx! {
        <div
            w={px(352.)}
            h_full
            flex
            flex_col
        >
            // Header
            <div
                class="flex items-center justify-between"
                flex_none
                px={px(14.)}
                py={px(10.)}
                border_b_1
                border_color={rgb(0x23_23_36)}
            >
                <div
                    class="flex items-center"
                    gap={px(8.)}
                >
                    // Status indicator dot
                    <div
                        w={px(8.)}
                        h={px(8.)}
                        rounded_full
                        bg={dot_color}
                    />
                    // Label
                    <div
                        text_size={px(11.5)}
                        font_weight={gpui::FontWeight::SEMIBOLD}
                        text_color={rgb(0xa6_ad_c8)}
                    >
                        {"Agent"}
                    </div>
                </div>
                // Close button
                <div
                    id="side-panel-left-close"
                    w={px(20.)}
                    h={px(20.)}
                    rounded={px(6.)}
                    class="flex items-center justify-center"
                    text_color={rgb(0x6c_70_86)}
                    cursor_pointer
                    hover={|s| s.bg(rgb(0x23_23_36)).text_color(rgb(0xcd_d6_f4))}
                    onClick={|_ev, window, cx| {
                        crate::side_panel_left::close_this(window, cx);
                    }}
                >
                    {img("icons/x.svg").w(px(12.)).h(px(12.))}
                </div>
            </div>
            // Body placeholder
            <div
                flex_1
                p={px(14.)}
                text_color={rgb(0xa6_ad_c8)}
            >
                {"Chat goes here"}
            </div>
        </div>
    }
}
