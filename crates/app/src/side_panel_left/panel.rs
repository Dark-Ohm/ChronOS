use gpui::{Context, IntoElement, Window, div, img, prelude::*, px, rgb};
use gpui_rsx::rsx;

use super::SidePanelLeft;
use super::sessions_list::{SIDEBAR_COLLAPSED_WIDTH, SIDEBAR_EXPANDED_WIDTH};
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
    cx: &mut Context<SidePanelLeft>,
) -> impl IntoElement {
    let dot_color = status_color(panel.state.agent_status);
    let collapsed = panel.state.sessions_collapsed;
    let sidebar_width = if collapsed {
        SIDEBAR_COLLAPSED_WIDTH
    } else {
        SIDEBAR_EXPANDED_WIDTH
    };
    let body_width = 352. - sidebar_width;
    let sessions = &panel.sessions;

    let body = div()
        .id("side-panel-body-wrap")
        .flex_1()
        .min_h(px(0.))
        .flex()
        .flex_row()
        .overflow_hidden();

    let body = if collapsed {
        body.child(
            div()
                .id("sessions-sidebar-collapsed")
                .w(px(SIDEBAR_COLLAPSED_WIDTH))
                .h_full()
                .flex()
                .flex_col()
                .items_center()
                .bg(rgb(0x18_18_25))
                .border_r_1()
                .border_color(rgb(0x23_23_36))
                .gap(px(4.))
                .p(px(8.))
                .child(
                    div()
                        .id("sessions-expand")
                        .w(px(32.))
                        .h(px(32.))
                        .rounded(px(6.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_size(px(14.))
                        .text_color(rgb(0x6c_70_86))
                        .cursor_pointer()
                        .hover(|s| s.bg(rgb(0x23_23_36)).text_color(rgb(0xcd_d6_f4)))
                        .on_click(cx.listener(|this, _, _, cx| this.toggle_collapse(cx)))
                        .child(">"),
                )
                .child(
                    div()
                        .id("sessions-new-icon")
                        .w(px(32.))
                        .h(px(32.))
                        .rounded(px(6.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_size(px(14.))
                        .text_color(rgb(0x6c_70_86))
                        .cursor_pointer()
                        .hover(|s| s.bg(rgb(0x23_23_36)).text_color(rgb(0xcd_d6_f4)))
                        .on_click(cx.listener(|this, _, _, cx| this.create_new_session(cx)))
                        .child("+"),
                )
                .children(sessions.iter().map(|s| {
                    let is_active = s.active;
                    let sid = s.id.clone();
                    div()
                        .id(format!("session-dot-{sid}"))
                        .w(px(32.))
                        .h(px(32.))
                        .rounded(px(6.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded_full()
                        .when(is_active, |el| el.bg(rgb(0x45_47_5a)))
                        .when(!is_active, |el| el.cursor_pointer())
                        .child(
                            div()
                                .w(px(6.))
                                .h(px(6.))
                                .rounded_full()
                                .bg(if is_active {
                                    rgb(0xa6_e3_a1)
                                } else {
                                    rgb(0x58_5b_70)
                                }),
                        )
                })),
        )
    } else {
        body.child(
            div()
                .id("sessions-sidebar-expanded")
                .w(px(SIDEBAR_EXPANDED_WIDTH))
                .h_full()
                .flex()
                .flex_col()
                .bg(rgb(0x18_18_25))
                .border_r_1()
                .border_color(rgb(0x23_23_36))
                .child(
                    div()
                        .id("sessions-header")
                        .flex()
                        .items_center()
                        .justify_between()
                        .flex_none()
                        .px(px(10.))
                        .py(px(8.))
                        .border_b_1()
                        .border_color(rgb(0x23_23_36))
                        .child(
                            div()
                                .text_size(px(11.5))
                                .font_weight(gpui::FontWeight::SEMIBOLD)
                                .text_color(rgb(0xa6_ad_c8))
                                .child("Sessions"),
                        )
                        .child(
                            div()
                                .id("sessions-collapse")
                                .w(px(20.))
                                .h(px(20.))
                                .rounded(px(4.))
                                .flex()
                                .items_center()
                                .justify_center()
                                .text_size(px(11.))
                                .text_color(rgb(0x6c_70_86))
                                .cursor_pointer()
                                .hover(|s| s.bg(rgb(0x23_23_36)).text_color(rgb(0xcd_d6_f4)))
                                .on_click(cx.listener(|this, _, _, cx| this.toggle_collapse(cx)))
                                .child("<"),
                        ),
                )
                .child(
                    div()
                        .id("sessions-new")
                        .flex_none()
                        .mx(px(8.))
                        .mt(px(8.))
                        .mb(px(4.))
                        .px(px(10.))
                        .py(px(6.))
                        .rounded(px(6.))
                        .border_1()
                        .border_color(rgb(0x31_32_44))
                        .text_size(px(11.5))
                        .text_color(rgb(0xa6_ad_c8))
                        .cursor_pointer()
                        .hover(|s| s.bg(rgb(0x23_23_36)).border_color(rgb(0x45_47_5a)))
                        .on_click(cx.listener(|this, _, _, cx| this.create_new_session(cx)))
                        .child("+ New session"),
                )
                .child(
                    div()
                        .id("sessions-list-scroll")
                        .flex_1()
                        .min_h(px(0.))
                        .overflow_y_scroll()
                        .flex()
                        .flex_col()
                        .gap(px(2.))
                        .p(px(8.))
                        .children(sessions.iter().map(|s| {
                            let is_active = s.active;
                            let title = s.title.clone();
                            let sid = s.id.clone();
                            div()
                                .id(format!("session-item-{sid}"))
                                .w_full()
                                .h(px(32.))
                                .px(px(10.))
                                .rounded(px(6.))
                                .flex()
                                .items_center()
                                .gap(px(8.))
                                .cursor_pointer()
                                .when(is_active, |el| el.bg(rgb(0x31_32_44)))
                                .when(!is_active, |el| el.hover(|s| s.bg(rgb(0x23_23_36))))
                                .child(
                                    div()
                                        .w(px(6.))
                                        .h(px(6.))
                                        .rounded_full()
                                        .bg(if is_active {
                                            rgb(0xa6_e3_a1)
                                        } else {
                                            rgb(0x58_5b_70)
                                        }),
                                )
                                .child(
                                    div()
                                        .text_size(px(11.5))
                                        .text_color(if is_active {
                                            rgb(0xcd_d6_f4)
                                        } else {
                                            rgb(0xa6_ad_c8)
                                        })
                                        .child(title),
                                )
                        })),
                ),
        )
    };

    let body = body.child(
        div()
            .id("chat-area")
            .w(px(body_width))
            .h_full()
            .p(px(14.))
            .text_color(rgb(0xa6_ad_c8))
            .child("Chat goes here"),
    );

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
                    <div
                        w={px(8.)}
                        h={px(8.)}
                        rounded_full
                        bg={dot_color}
                    />
                    <div
                        text_size={px(11.5)}
                        font_weight={gpui::FontWeight::SEMIBOLD}
                        text_color={rgb(0xa6_ad_c8)}
                    >
                        {"Agent"}
                    </div>
                </div>
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
            {body}
        </div>
    }
}
