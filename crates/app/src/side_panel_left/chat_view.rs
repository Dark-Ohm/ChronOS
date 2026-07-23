use gpui::{Context, IntoElement, ScrollHandle, Window, div, point, prelude::*, px, rgb};

use super::SidePanelLeft;
use super::tool_card::ToolCard;

#[derive(Clone, Debug, PartialEq)]
pub enum MessageRole {
    User,
    Agent,
}

#[derive(Clone, Debug)]
pub struct ToolCallPreview {
    pub name: String,
    pub status: String,
    pub args: Option<String>,
    pub result: Option<String>,
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
    pub expanded_tool_calls: std::collections::HashSet<(usize, usize)>,
}

impl ChatView {
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
            scroll: ScrollHandle::new(),
            expanded_tool_calls: std::collections::HashSet::new(),
        }
    }

    pub fn push_message(&mut self, msg: ChatMessage) {
        self.messages.push(msg);
    }

    pub fn scroll_to_bottom(&self) {
        self.scroll.set_offset(point(px(f32::MAX), px(f32::MAX)));
    }

    pub fn render(
        &self,
        panel: &SidePanelLeft,
        _window: &mut Window,
        cx: &mut Context<SidePanelLeft>,
    ) -> impl IntoElement {
        let has_messages = !self.messages.is_empty();
        let expanded = &panel.chat.expanded_tool_calls;

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
                let mut el = el;
                for (msg_idx, msg) in self.messages.iter().enumerate() {
                    el = el.child(render_message(msg, msg_idx, expanded, cx));
                }
                el
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

fn render_tool_cards(
    tool_calls: &[ToolCallPreview],
    msg_idx: usize,
    expanded: &std::collections::HashSet<(usize, usize)>,
    cx: &mut Context<SidePanelLeft>,
) -> Option<impl IntoElement> {
    if tool_calls.is_empty() {
        return None;
    }

    let cards: Vec<_> = tool_calls
        .iter()
        .enumerate()
        .map(|(tc_idx, tc)| {
            let is_expanded = expanded.contains(&(msg_idx, tc_idx));
            div()
                .id(format!("tool-card-{msg_idx}-{tc_idx}"))
                .child(
                    ToolCard {
                        name: &tc.name,
                        status: &tc.status,
                        args: tc.args.as_deref(),
                        result: tc.result.as_deref(),
                        expanded: is_expanded,
                    }
                    .render(Some(cx.listener(move |this, _, _, cx| {
                        let key = (msg_idx, tc_idx);
                        if this.chat.expanded_tool_calls.contains(&key) {
                            this.chat.expanded_tool_calls.remove(&key);
                        } else {
                            this.chat.expanded_tool_calls.insert(key);
                        }
                        cx.notify();
                    }))),
                )
        })
        .collect();

    Some(
        div()
            .mt(px(6.))
            .flex()
            .flex_col()
            .gap(px(4.))
            .children(cards),
    )
}

fn render_message(
    msg: &ChatMessage,
    msg_idx: usize,
    expanded: &std::collections::HashSet<(usize, usize)>,
    cx: &mut Context<SidePanelLeft>,
) -> impl IntoElement {
    let (bg, label, text_color) = match msg.role {
        MessageRole::User => (rgb(0x31_32_44), "You", rgb(0xcd_d6_f4)),
        MessageRole::Agent => (rgb(0x1e_1e_30), "Agent", rgb(0xa6_ad_c8)),
    };

    let is_user = msg.role == MessageRole::User;

    let role_label = div()
        .text_size(px(10.))
        .font_weight(gpui::FontWeight::SEMIBOLD)
        .mb(px(4.))
        .text_color(if is_user {
            rgb(0x89_b4_fa)
        } else {
            rgb(0xa6_e3_a1)
        })
        .child(label.to_string());

    let content_block = div()
        .text_size(px(12.))
        .text_color(text_color)
        .child(msg.content.clone());

    let tool_cards_section = render_tool_cards(&msg.tool_calls, msg_idx, expanded, cx);

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
        .children(tool_cards_section);

    div()
        .px(px(12.))
        .py(px(2.))
        .w_full()
        .flex()
        .when(is_user, |el| el.justify_end())
        .when(!is_user, |el| el.justify_start())
        .child(bubble)
}
