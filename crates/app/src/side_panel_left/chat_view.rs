use gpui::{Context, IntoElement, ScrollHandle, Window, div, point, prelude::*, px};
use super::is_rtl_text;
use chronos_ui::Theme;
use serde::{Deserialize, Serialize};

use super::SidePanelLeft;
use super::tool_card::ToolCard;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum MessageRole {
    User,
    Agent,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolCallPreview {
    pub id: String,
    pub name: String,
    pub status: String,
    pub args: Option<String>,
    pub result: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Segment {
    Thinking { content: String },
    ToolCall { tool: ToolCallPreview },
    Response { content: String },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: MessageRole,
    pub segments: Vec<Segment>,
}

pub struct ChatView {
    pub(crate) messages: Vec<ChatMessage>,
    scroll: ScrollHandle,
    pub expanded_tool_calls: std::collections::HashSet<(usize, usize)>,
    pub collapsed_reasoning: std::collections::HashSet<(usize, usize)>,
}

impl ChatView {
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
            scroll: ScrollHandle::new(),
            expanded_tool_calls: std::collections::HashSet::new(),
            collapsed_reasoning: std::collections::HashSet::new(),
        }
    }

    pub fn push_message(&mut self, msg: ChatMessage) {
        self.messages.push(msg);
    }

    pub fn toggle_reasoning(&mut self, msg_idx: usize, seg_idx: usize) {
        let key = (msg_idx, seg_idx);
        if self.collapsed_reasoning.contains(&key) {
            self.collapsed_reasoning.remove(&key);
        } else {
            self.collapsed_reasoning.insert(key);
        }
    }

    pub fn scroll_to_bottom(&self) {
        self.scroll.scroll_to_bottom();
    }

    pub fn render(
        &self,
        panel: &SidePanelLeft,
        _window: &mut Window,
        cx: &mut Context<SidePanelLeft>,
    ) -> impl IntoElement {
        let theme = *Theme::global(cx);
        let has_messages = !self.messages.is_empty();

        let messages_el = div()
            .id("chat-messages-scroll")
            .flex_1()
            .min_h(px(0.))
            .overflow_y_scroll()
            .track_scroll(&self.scroll)
            .flex()
            .flex_col()
            .gap(px(9.))
            .px(px(14.))
            .py(px(14.))
            .when(has_messages, |el| {
                let mut el = el;
                let last_idx = self.messages.len().saturating_sub(1);
                for (msg_idx, msg) in self.messages.iter().enumerate() {
                    let is_last = msg_idx == last_idx;
                    el = el.child(render_message(
                        msg,
                        msg_idx,
                        &self.expanded_tool_calls,
                        &self.collapsed_reasoning,
                        panel.streaming.active,
                        is_last,
                        &theme,
                        cx,
                    ));
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
                        .text_color(theme.interactive.active)
                        .child("No messages yet"),
                )
            });

        messages_el
    }
}

fn render_segment_content(
    content: &str,
    theme: &Theme,
) -> impl IntoElement + use<> {
    div()
        .text_size(px(12.))
        .line_height(px(18.))
        .text_color(theme.text.primary)
        .when(is_rtl_text(content), |el| el.text_right())
        .child(content.to_string())
}

fn render_thinking_block(
    content: &str,
    msg_idx: usize,
    seg_idx: usize,
    collapsed_reasoning: &std::collections::HashSet<(usize, usize)>,
    streaming_active: bool,
    is_last_msg_and_seg: bool,
    theme: &Theme,
    cx: &mut Context<SidePanelLeft>,
) -> impl IntoElement + use<> {
    let reasoning_collapsed = {
        let user_collapsed = collapsed_reasoning.contains(&(msg_idx, seg_idx));
        if is_last_msg_and_seg && streaming_active {
            false
        } else {
            user_collapsed
        }
    };

    let reasoning_toggle = cx.listener(move |this, _, _, cx| {
        this.chat.toggle_reasoning(msg_idx, seg_idx);
        cx.notify();
    });

    if content.is_empty() {
        return div().into_any_element();
    }

    let header = div()
        .id(format!("reasoning-header-{msg_idx}-{seg_idx}"))
        .cursor_pointer()
        .text_size(px(10.))
        .font_weight(gpui::FontWeight::MEDIUM)
        .text_color(theme.text.muted)
        .on_click(reasoning_toggle)
        .child(if reasoning_collapsed {
            "Reasoning  ⌄"
        } else {
            "Reasoning  ⌃"
        });

    let body = if reasoning_collapsed {
        None
    } else {
        Some(
            div()
                .id(format!("reasoning-body-{msg_idx}-{seg_idx}"))
                .text_size(px(11.))
                .line_height(px(16.))
                .text_color(theme.text.muted)
                .overflow_y_scroll()
                .max_h(px(300.))
                .child(content.to_string()),
        )
    };

    div()
        .id(format!("reasoning-{msg_idx}-{seg_idx}"))
        .rounded(px(6.))
        .bg(theme.bg.tertiary)
        .border_1()
        .border_color(theme.border.subtle)
        .px(px(8.))
        .py(px(6.))
        .flex()
        .flex_col()
        .gap(px(4.))
        .child(header)
        .children(body)
        .into_any_element()
}

fn render_tool_card_segment(
    tool: &ToolCallPreview,
    msg_idx: usize,
    seg_idx: usize,
    expanded: &std::collections::HashSet<(usize, usize)>,
    theme: &Theme,
    cx: &mut Context<SidePanelLeft>,
) -> impl IntoElement + use<> {
    let is_expanded = expanded.contains(&(msg_idx, seg_idx));
    ToolCard {
        name: &tool.name,
        status: &tool.status,
        args: tool.args.as_deref(),
        result: tool.result.as_deref(),
        expanded: is_expanded,
        theme,
    }
    .render(Some(cx.listener(move |this, _, _, cx| {
        let key = (msg_idx, seg_idx);
        if this.chat.expanded_tool_calls.contains(&key) {
            this.chat.expanded_tool_calls.remove(&key);
        } else {
            this.chat.expanded_tool_calls.insert(key);
        }
        cx.notify();
    })))
    .into_any_element()
}

fn render_message(
    msg: &ChatMessage,
    msg_idx: usize,
    expanded: &std::collections::HashSet<(usize, usize)>,
    collapsed_reasoning: &std::collections::HashSet<(usize, usize)>,
    streaming_active: bool,
    is_last: bool,
    theme: &Theme,
    cx: &mut Context<SidePanelLeft>,
) -> impl IntoElement + use<> {
    let is_user = msg.role == MessageRole::User;

    let last_seg_idx = msg.segments.len().saturating_sub(1);
    let mut seg_elements: Vec<gpui::AnyElement> = Vec::new();

    for (seg_idx, seg) in msg.segments.iter().enumerate() {
        let is_last_seg = seg_idx == last_seg_idx;
        let is_last_msg_and_seg = is_last && is_last_seg;

        match seg {
            Segment::Thinking { content } => {
                seg_elements.push(render_thinking_block(
                    content,
                    msg_idx,
                    seg_idx,
                    collapsed_reasoning,
                    streaming_active,
                    is_last_msg_and_seg,
                    theme,
                    cx,
                ).into_any_element());
            }
            Segment::ToolCall { tool } => {
                seg_elements.push(render_tool_card_segment(
                    tool,
                    msg_idx,
                    seg_idx,
                    expanded,
                    theme,
                    cx,
                ).into_any_element());
            }
            Segment::Response { content } => {
                seg_elements.push(render_segment_content(content, theme).into_any_element());
            }
        }
    }

    if is_user {
        div().w_full().flex().justify_end().child(
            div()
                .bg(theme.bg.elevated)
                .rounded(px(9.))
                .px(px(10.))
                .py(px(7.))
                .flex()
                .flex_col()
                .children(seg_elements),
        )
    } else {
        div()
            .w_full()
            .flex()
            .flex_col()
            .child(
                div()
                    .bg(theme.bg.secondary)
                    .border_1()
                    .border_color(theme.border.subtle)
                    .rounded(px(9.))
                    .px(px(10.))
                    .py(px(7.))
                    .flex()
                    .flex_col()
                    .gap(px(6.))
                    .children(seg_elements),
            )
    }
}
