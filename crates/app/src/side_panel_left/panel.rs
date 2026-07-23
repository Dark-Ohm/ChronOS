use gpui::{Context, IntoElement, Window, div, img, prelude::*, px, rgb};

use super::SidePanelLeft;
use super::sessions_list::{SIDEBAR_COLLAPSED_WIDTH, SIDEBAR_EXPANDED_WIDTH};
use super::state::AgentStatus;

const HANDLE_WIDTH: f32 = 4.;

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
    let agent_menu_open = panel.agent_menu_open;

    let agent_name = panel
        .agents
        .iter()
        .find(|a| a.id == panel.active_agent_id)
        .map(|a| a.display_name)
        .unwrap_or("Agent");

    // Build sidebar (no listeners)
    let sidebar = build_sessions_sidebar(panel, collapsed);

    // Resize handlers (borrows cx) — must be built before `composer`/`chat`
    // below: their RPIT return captures `cx`'s lifetime for as long as the
    // resulting element is alive (Rust 2024 impl Trait capture rules), so
    // any `cx.listener(...)` call in between would conflict (E0502).
    let resize_drag_handler = cx.listener(
        |this, ev: &gpui::DragMoveEvent<super::LeftPanelResize>, window, cx| {
            let current_x = f32::from(ev.event.position.x);
            this.update_resize(current_x, window, cx);
        },
    );

    let resize_mouse_handler = cx.listener(
        |this, ev: &gpui::MouseDownEvent, _window, _cx| {
            this.start_resize(f32::from(ev.position.x));
        },
    );

    // Build chat (borrows cx)
    let chat = div()
        .id("chat-area")
        .flex_1()
        .min_h(px(0.))
        .flex()
        .flex_col()
        .overflow_hidden()
        .child(panel.chat.render(panel, _window, cx));

    // Build composer (borrows cx)
    let composer = super::composer::render_composer(panel, _window, cx);

    // Build agent dropdown (no listeners)
    let active_id = panel.active_agent_id.clone();
    let dropdown = if agent_menu_open {
        let agents_snapshot: Vec<(&'static str, &'static str)> = panel
            .agents
            .iter()
            .map(|a| (a.id, a.display_name))
            .collect();
        Some(
            div()
                .id("agent-dropdown")
                .w(px(172.))
                .bg(rgb(0x1e_1e_2e))
                .border_1()
                .border_color(rgb(0x23_23_36))
                .rounded(px(8.))
                .p(px(4.))
                .mx(px(8.))
                .mt(px(4.))
                .flex()
                .flex_col()
                .children(agents_snapshot.into_iter().map(|(id, name)| {
                    let is_selected = id == active_id.as_str();
                    div()
                        .id(format!("agent-option-{id}"))
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap(px(8.))
                        .px(px(8.))
                        .py(px(6.))
                        .rounded(px(6.))
                        .cursor_pointer()
                        .text_size(px(11.5))
                        .text_color(rgb(0xa6_ad_c8))
                        .hover(|s| s.bg(rgb(0x23_23_36)))
                        .child(
                            div().text_color(if is_selected {
                                rgb(0xcd_d6_f4)
                            } else {
                                rgb(0xa6_ad_c8)
                            }).child(name),
                        )
                        .when(is_selected, |el| {
                            el.child(
                                div()
                                    .text_size(px(10.))
                                    .text_color(rgb(0x00_7a_cc))
                                    .child("✓"),
                            )
                        })
                })),
        )
    } else {
        None
    };

    // Clipped content
    let clipped_content = div()
        .id("clipped-content")
        .flex_1()
        .min_h(px(0.))
        .flex()
        .flex_col()
        .overflow_hidden()
        .child(sidebar)
        .child(chat)
        .child(composer);

    // Header with listeners
    let header = div()
        .id("side-panel-header")
        .flex()
        .items_center()
        .justify_between()
        .flex_none()
        .px(px(14.))
        .py(px(10.))
        .border_b_1()
        .border_color(rgb(0x23_23_36))
        .child(
            div()
                .id("agent-cluster")
                .flex()
                .items_center()
                .gap(px(7.))
                .cursor_pointer()
                .rounded(px(6.))
                .px(px(6.))
                .py(px(3.))
                .mx(px(-6.))
                .my(px(-3.))
                .hover(|s| s.bg(rgb(0x23_23_36)))
                .on_click(cx.listener(|this, _, _, cx| {
                    this.agent_menu_open = !this.agent_menu_open;
                    cx.notify();
                }))
                .child(div().w(px(7.)).h(px(7.)).rounded_full().bg(dot_color))
                .child(
                    div()
                        .text_size(px(12.5))
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(rgb(0xa6_ad_c8))
                        .child(agent_name),
                )
                .child(
                    div()
                        .text_size(px(9.))
                        .text_color(rgb(0x6c_70_86))
                        .child("⌄"),
                ),
        )
        .child(
            div()
                .id("side-panel-left-close")
                .w(px(20.))
                .h(px(20.))
                .rounded(px(6.))
                .flex()
                .items_center()
                .justify_center()
                .text_color(rgb(0x6c_70_86))
                .cursor_pointer()
                .hover(|s| s.bg(rgb(0x23_23_36)).text_color(rgb(0xcd_d6_f4)))
                .on_click(|_ev, window, cx| {
                    crate::side_panel_left::close_this(window, cx);
                })
                .child(img("icons/x.svg").w(px(12.)).h(px(12.))),
        );

    let dropdown = dropdown.map(|d| {
        d.children(panel.agents.iter().map(|agent| {
            let agent_id = agent.id.to_string();
            div()
                .id(format!("agent-click-{}", agent.id))
                .absolute()
                .size_full()
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.switch_agent(&agent_id, cx);
                }))
        }))
    });

    div()
        .id("side-panel-left-root")
        .w(px(panel.state.width))
        .h_full()
        .flex()
        .flex_row()
        .on_hover(|hovered, _window, cx| {
            if *hovered {
                super::hold_peek(cx);
            } else {
                super::schedule_release_peek(cx);
            }
        })
        .child(
            div()
                .id("main-content")
                .flex_1()
                .h_full()
                .flex()
                .flex_col()
                .bg(rgb(0x1e_1e_2e))
                .child(header)
                .children(dropdown)
                .child(clipped_content),
        )
        .child(
            div()
                .id("resize-handle")
                .w(px(HANDLE_WIDTH))
                .h_full()
                .cursor_col_resize()
                .flex()
                .items_center()
                .justify_center()
                .bg(rgb(0x18_18_25))
                .border_l_1()
                .border_color(rgb(0x23_23_36))
                .on_mouse_down(gpui::MouseButton::Left, resize_mouse_handler)
                .on_drag(super::LeftPanelResize, |_, _, _, cx| {
                    cx.new(|_| gpui::EmptyView)
                })
                .on_drag_move(resize_drag_handler)
                .child(
                    div()
                        .w(px(1.))
                        .h_full()
                        .bg(rgb(0x45_47_5a)),
                ),
        )
}

/// Build sessions sidebar without cx.listeners.
fn build_sessions_sidebar(
    panel: &SidePanelLeft,
    collapsed: bool,
) -> impl IntoElement {
    let sessions = &panel.sessions;

    if collapsed {
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
            }))
            .into_any()
    } else {
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
            )
            .into_any()
    }
}
