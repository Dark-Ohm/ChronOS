use gpui::{Context, IntoElement, ScrollHandle, Window, div, point, prelude::*, px, rgb};

use super::SidePanelLeft;

#[derive(Clone, Debug, PartialEq)]
pub enum MessageRole {
    User,
    Agent,
}

#[derive(Clone, Debug)]
pub struct ToolCallPreview {
    pub name: String,
    pub status: String,
}

#[derive(Clone, Debug)]
pub struct ChatMessage {
    pub role: MessageRole,
    pub content: String,
    pub tool_calls: Vec<ToolCallPreview>,
}

pub struct ChatView {
    messages: Vec<ChatMessage>,
    scroll: ScrollHandle,
}

impl ChatView {
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
            scroll: ScrollHandle::new(),
        }
    }

    pub fn push_message(&mut self, msg: ChatMessage) {
        self.messages.push(msg);
    }

    pub fn scroll_to_bottom(&self) {
        // ScrollHandle doesn't expose a direct "scroll to bottom" in gpui-ce,
        // but setting the offset to f32::MAX effectively does it on next layout.
        self.scroll.set_offset(point(px(f32::MAX), px(f32::MAX)));
    }

    pub fn render(
        &self,
        panel: &SidePanelLeft,
        _window: &mut Window,
        _cx: &mut Context<SidePanelLeft>,
    ) -> impl IntoElement {
        let has_messages = !self.messages.is_empty();

        let messages_el = div()
            .id("chat-messages-scroll")
            .flex_1()
            .min_h(px(0.))
            .overflow_y_scroll()
            .track_scroll(&self.scroll)
            .flex()
            .flex_col()
            .gap(px(2.))
            .when(has_messages, |el| {
                el.children(self.messages.iter().map(|msg| render_message(msg)))
            })
            .when(!has_messages, |el| {
                el.child(
                    div()
                        .flex_1()
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_size(px(12.))
                        .text_color(rgb(0x58_5b_70))
                        .child("No messages yet"),
                )
            });

        messages_el
    }
}

fn render_message(msg: &ChatMessage) -> impl IntoElement {
    let (bg, align, label, text_color) = match msg.role {
        MessageRole::User => (
            rgb(0x31_32_44),
            "flex-end",
            "You",
            rgb(0xcd_d6_f4),
        ),
        MessageRole::Agent => (
            rgb(0x1e_1e_30),
            "flex-start",
            "Agent",
            rgb(0xa6_ad_c8),
        ),
    };

    let is_user = msg.role == MessageRole::User;

    let role_label = div()
        .text_size(px(10.))
        .font_weight(gpui::FontWeight::SEMIBOLD)
        .mb(px(4.))
        .text_color(if is_user { rgb(0x89_b4_fa) } else { rgb(0xa6_e3_a1) })
        .child(label.to_string());

    let content_block = div()
        .text_size(px(12.))
        .text_color(text_color)
        .child(msg.content.clone());

    let tool_calls_section = if msg.tool_calls.is_empty() {
        None
    } else {
        Some(
            div()
                .mt(px(6.))
                .flex()
                .flex_col()
                .gap(px(4.))
                .children(msg.tool_calls.iter().map(|tc| {
                    let status_color = match tc.status.as_str() {
                        "running" => rgb(0xf9_e2_af),
                        "done" => rgb(0xa6_e3_a1),
                        "error" => rgb(0xf3_8b_a8),
                        _ => rgb(0x58_5b_70),
                    };
                    div()
                        .flex()
                        .items_center()
                        .gap(px(6.))
                        .text_size(px(10.))
                        .child(
                            div()
                                .w(px(5.))
                                .h(px(5.))
                                .rounded_full()
                                .bg(status_color),
                        )
                        .child(
                            div()
                                .text_color(rgb(0x6c_70_86))
                                .child(format!("{}: {}", tc.name, tc.status)),
                        )
                })),
        )
    };

    let bubble = div()
        .max_w(px(290.))
        .px(px(12.))
        .py(px(8.))
        .rounded(px(8.))
        .bg(bg)
        .flex()
        .flex_col()
        .child(role_label)
        .child(content_block)
        .children(tool_calls_section);

    div()
        .px(px(12.))
        .py(px(2.))
        .w_full()
        .flex()
        .when(is_user, |el| el.justify_end())
        .when(!is_user, |el| el.justify_start())
        .child(bubble)
}
