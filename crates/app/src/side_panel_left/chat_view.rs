use gpui::{Context, IntoElement, ScrollHandle, Window, div, point, prelude::*, px};
use chronos_ui::Theme;

use super::SidePanelLeft;
use super::tool_card::ToolCard;

#[derive(Clone, Debug, PartialEq)]
pub enum MessageRole {
    User,
    Agent,
}

#[derive(Clone, Debug)]
pub struct ToolCallPreview {
    /// Stable ACP tool-call id (used for merging updates, D1).
    pub id: String,
    pub name: String,
    pub status: String,
    pub args: Option<String>,
    pub result: Option<String>,
}

#[derive(Clone, Debug)]
pub struct ChatMessage {
    pub role: MessageRole,
    pub content: String,
    pub thought: Option<String>,
    pub tool_calls: Vec<ToolCallPreview>,
}

pub struct ChatView {
    pub(crate) messages: Vec<ChatMessage>,
    scroll: ScrollHandle,
    pub expanded_tool_calls: std::collections::HashSet<(usize, usize)>,
    pub collapsed_reasoning: std::collections::HashSet<usize>,
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

    pub fn toggle_reasoning(&mut self, msg_idx: usize) {
        if self.collapsed_reasoning.contains(&msg_idx) {
            self.collapsed_reasoning.remove(&msg_idx);
        } else {
            self.collapsed_reasoning.insert(msg_idx);
        }
    }

    pub fn scroll_to_bottom(&self) {
        // Use the fork's flag-based API (`div.rs:4063`), consumed at layout.
        // Writing `f32::MAX` into the offset by hand poisons the layout
        // arithmetic (`child_bounds.top() + offset.y`) once the content
        // actually becomes scrollable.
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
        let expanded = &panel.chat.expanded_tool_calls;

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
                        expanded,
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

fn render_tool_cards(
    tool_calls: &[ToolCallPreview],
    msg_idx: usize,
    expanded: &std::collections::HashSet<(usize, usize)>,
    theme: &Theme,
    cx: &mut Context<SidePanelLeft>,
) -> Option<impl IntoElement + use<>> {
    if tool_calls.is_empty() {
        return None;
    }

    let cards: Vec<_> = tool_calls
        .iter()
        .enumerate()
        .map(|(tc_idx, tc)| {
            let is_expanded = expanded.contains(&(msg_idx, tc_idx));
            div().id(format!("tool-card-{msg_idx}-{tc_idx}")).child(
                ToolCard {
                    name: &tc.name,
                    status: &tc.status,
                    args: tc.args.as_deref(),
                    result: tc.result.as_deref(),
                    expanded: is_expanded,
                    theme,
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
    collapsed_reasoning: &std::collections::HashSet<usize>,
    streaming_active: bool,
    is_last: bool,
    theme: &Theme,
    cx: &mut Context<SidePanelLeft>,
) -> impl IntoElement + use<> {
    let is_user = msg.role == MessageRole::User;

    let content = div()
        .text_size(px(12.))
        .line_height(px(18.))
        .text_color(if is_user {
            theme.text.primary
        } else {
            theme.text.primary
        })
        .child(msg.content.clone());

    let reasoning_collapsed = {
        let user_collapsed = collapsed_reasoning.contains(&msg_idx);
        if is_last && streaming_active {
            false
        } else {
            user_collapsed
        }
    };

    let reasoning_toggle = cx.listener(move |this, _, _, cx| {
        this.chat.toggle_reasoning(msg_idx);
        cx.notify();
    });

    let tool_cards_section = render_tool_cards(&msg.tool_calls, msg_idx, expanded, theme, cx);

    let reasoning_section = msg.thought.as_ref().filter(|t| !t.is_empty()).map(|thought| {
        let header = div()
            .id(format!("reasoning-header-{msg_idx}"))
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
                    .id(format!("reasoning-body-{msg_idx}"))
                    .text_size(px(11.))
                    .line_height(px(16.))
                    .text_color(theme.text.muted)
                    .overflow_y_scroll()
                    .max_h(px(300.))
                    .child(thought.clone()),
            )
        };

        div()
            .id(format!("reasoning-{msg_idx}"))
            .mt(px(4.))
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
    });

    if is_user {
        // User message: right-aligned bubble
        div().w_full().flex().justify_end().child(
            div()
                .bg(theme.bg.elevated)
                .rounded(px(9.))
                .px(px(10.))
                .py(px(7.))
                .flex()
                .flex_col()
                .child(content)
                .children(tool_cards_section),
        )
    } else {
        // Agent message: left-aligned bubble
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
                    .children(tool_cards_section)
                    .children(reasoning_section)
                    .child(content),
            )
    }
}
