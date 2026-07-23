use gpui::{IntoElement, SharedString, Window, div, prelude::*, px, rgb};

use super::SidePanelLeft;
use super::chat_view::{ChatMessage, MessageRole};
use super::state::AgentStatus;

pub fn render_composer(
    panel: &SidePanelLeft,
    _window: &mut Window,
    cx: &mut Context<SidePanelLeft>,
) -> impl IntoElement {
    let text = &panel.composer_text;
    let selected_model = panel.composer_selected_model.clone();
    let selected_mode = panel.composer_selected_mode.clone();
    let selected_model_display = if selected_model.is_empty() {
        "Model".to_string()
    } else {
        selected_model.clone()
    };
    let selected_mode_display = if selected_mode.is_empty() {
        "Mode".to_string()
    } else {
        selected_mode.clone()
    };
    let model_open = panel.composer_model_dropdown_open;
    let mode_open = panel.composer_mode_dropdown_open;
    let has_text = !text.is_empty();
    let enabled = panel.state.agent_status != AgentStatus::Disconnected;
    let focused = panel.composer_focused;

    let input_display: SharedString = if text.is_empty() {
        "Type a message...".into()
    } else {
        text.clone().into()
    };
    let input_text_color = if text.is_empty() {
        rgb(0x58_5b_70)
    } else {
        rgb(0xcd_d6_f4)
    };

    let focus = panel.composer_focus.clone();

    // ── Model picker ──────────────────────────────────────────────
    let model_items: Vec<_> = panel
        .available_models
        .iter()
        .enumerate()
        .map(|(i, m)| {
            let m_id = m.id.clone();
            let m_name = if m.name.is_empty() { m.id.clone() } else { m.name.clone() };
            let is_active = m.id == selected_model;
            div()
                .id(format!("model-item-{i}"))
                .w_full()
                .px(px(10.))
                .py(px(5.))
                .rounded(px(4.))
                .text_size(px(11.))
                .text_color(if is_active { rgb(0xcd_d6_f4) } else { rgb(0xa6_ad_c8) })
                .when(is_active, |el| el.bg(rgb(0x31_32_44)))
                .when(!is_active, |el| el.hover(|s| s.bg(rgb(0x23_23_36))))
                .cursor_pointer()
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.composer_selected_model = m_id.clone();
                    this.composer_model_dropdown_open = false;
                    cx.notify();
                }))
                .child(m_name)
        })
        .collect();

    let model_picker = div()
        .relative()
        .child(
            div()
                .id("composer-model-picker")
                .h(px(28.))
                .px(px(8.))
                .rounded(px(6.))
                .flex()
                .items_center()
                .border_1()
                .border_color(rgb(0x31_32_44))
                .text_size(px(10.))
                .text_color(rgb(0xa6_ad_c8))
                .cursor_pointer()
                .hover(|s| s.border_color(rgb(0x45_47_5a)).bg(rgb(0x1e_1e_30)))
                .on_click(cx.listener(|this, _, _, cx| {
                    this.composer_model_dropdown_open = !this.composer_model_dropdown_open;
                    this.composer_mode_dropdown_open = false;
                    cx.notify();
                }))
                .child(selected_model_display),
        )
        .when(model_open, |el| {
            el.child(
                div()
                    .id("composer-model-dropdown")
                    .absolute()
                    .bottom(px(32.))
                    .left(px(0.))
                    .min_w(px(200.))
                    .bg(rgb(0x1e_1e_30))
                    .border_1()
                    .border_color(rgb(0x31_32_44))
                    .rounded(px(6.))
                    .p(px(4.))
                    .flex()
                    .flex_col()
                    .gap(px(2.))
                    .children(model_items),
            )
        });

    // ── Mode picker ───────────────────────────────────────────────
    let mode_items: Vec<_> = panel
        .available_modes
        .iter()
        .enumerate()
        .map(|(i, m)| {
            let m_id = m.id.clone();
            let m_name = if m.name.is_empty() { m.id.clone() } else { m.name.clone() };
            let is_active = m.id == selected_mode;
            div()
                .id(format!("mode-item-{i}"))
                .w_full()
                .px(px(10.))
                .py(px(5.))
                .rounded(px(4.))
                .text_size(px(11.))
                .text_color(if is_active { rgb(0xcd_d6_f4) } else { rgb(0xa6_ad_c8) })
                .when(is_active, |el| el.bg(rgb(0x31_32_44)))
                .when(!is_active, |el| el.hover(|s| s.bg(rgb(0x23_23_36))))
                .cursor_pointer()
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.composer_selected_mode = m_id.clone();
                    this.composer_mode_dropdown_open = false;
                    cx.notify();
                }))
                .child(m_name)
        })
        .collect();

    let mode_picker = div()
        .relative()
        .child(
            div()
                .id("composer-mode-picker")
                .h(px(28.))
                .px(px(8.))
                .rounded(px(6.))
                .flex()
                .items_center()
                .border_1()
                .border_color(rgb(0x31_32_44))
                .text_size(px(10.))
                .text_color(rgb(0xa6_ad_c8))
                .cursor_pointer()
                .hover(|s| s.border_color(rgb(0x45_47_5a)).bg(rgb(0x1e_1e_30)))
                .on_click(cx.listener(|this, _, _, cx| {
                    this.composer_mode_dropdown_open = !this.composer_mode_dropdown_open;
                    this.composer_model_dropdown_open = false;
                    cx.notify();
                }))
                .child(selected_mode_display),
        )
        .when(mode_open, |el| {
            el.child(
                div()
                    .id("composer-mode-dropdown")
                    .absolute()
                    .bottom(px(32.))
                    .left(px(0.))
                    .min_w(px(80.))
                    .bg(rgb(0x1e_1e_30))
                    .border_1()
                    .border_color(rgb(0x31_32_44))
                    .rounded(px(6.))
                    .p(px(4.))
                    .flex()
                    .flex_col()
                    .gap(px(2.))
                    .children(mode_items),
            )
        });

    // ── Text input ────────────────────────────────────────────────
    let text_input = div()
        .id("composer-input")
        .flex_1()
        .min_h(px(28.))
        .max_h(px(120.))
        .px(px(10.))
        .py(px(6.))
        .rounded(px(6.))
        .bg(rgb(0x1e_1e_30))
        .border_1()
        .border_color(if focused { rgb(0x89_b4_fa) } else { rgb(0x31_32_44) })
        .text_size(px(12.))
        .text_color(input_text_color)
        .track_focus(&focus)
        .on_click(cx.listener(|this, _, window, cx| {
            this.composer_focused = true;
            this.composer_model_dropdown_open = false;
            this.composer_mode_dropdown_open = false;
            window.focus(&this.composer_focus, cx);
            cx.notify();
        }))
        .on_key_down(cx.listener(|this, event, window, cx| {
            this.handle_composer_key(event, window, cx);
        }))
        .child(input_display);

    // ── Send button ───────────────────────────────────────────────
    let send_color = if has_text { rgb(0x89_b4_fa) } else { rgb(0x45_47_5a) };

    let send_button = div()
        .id("composer-send")
        .w(px(28.))
        .h(px(28.))
        .rounded(px(6.))
        .flex()
        .items_center()
        .justify_center()
        .text_size(px(14.))
        .text_color(send_color)
        .when(has_text, |el| {
            el.cursor_pointer()
                .hover(|s| s.bg(rgb(0x31_32_44)))
                .on_click(cx.listener(|this, _, window, cx| {
                    this.send_composer(window, cx);
                }))
        })
        .child("\u{27A4}");

    // ── Attach button ─────────────────────────────────────────────
    let attach_button = div()
        .id("composer-attach")
        .w(px(28.))
        .h(px(28.))
        .rounded(px(6.))
        .flex()
        .items_center()
        .justify_center()
        .text_size(px(14.))
        .text_color(rgb(0x58_5b_70))
        .cursor_pointer()
        .hover(|s| s.bg(rgb(0x31_32_44)).text_color(rgb(0xa6_ad_c8)))
        .child("+");

    // ── Compose ───────────────────────────────────────────────────
    div()
        .id("composer-wrap")
        .flex_none()
        .px(px(10.))
        .py(px(8.))
        .border_t_1()
        .border_color(rgb(0x23_23_36))
        .flex()
        .flex_col()
        .gap(px(6.))
        .when(!enabled, |el| el.opacity(0.5))
        .child(
            div().flex().flex_row().items_end().gap(px(6.))
                .child(attach_button)
                .child(model_picker)
                .child(text_input)
                .child(mode_picker)
                .child(send_button),
        )
}

impl SidePanelLeft {
    pub(crate) fn handle_composer_key(
        &mut self,
        event: &gpui::KeyDownEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if event.keystroke.key == "escape" {
            if self.composer_model_dropdown_open || self.composer_mode_dropdown_open {
                self.composer_model_dropdown_open = false;
                self.composer_mode_dropdown_open = false;
                cx.notify();
                return;
            }
        }

        if self.composer_model_dropdown_open || self.composer_mode_dropdown_open {
            self.composer_model_dropdown_open = false;
            self.composer_mode_dropdown_open = false;
        }

        let key = event.keystroke.key.as_str();
        let modifiers = &event.keystroke.modifiers;

        match key {
            "backspace" => {
                self.composer_text.pop();
                self.composer_cursor = self.composer_text.len();
                cx.notify();
            }
            "left" => {
                if self.composer_cursor > 0 {
                    self.composer_cursor -= 1;
                }
                cx.notify();
            }
            "right" => {
                if self.composer_cursor < self.composer_text.len() {
                    self.composer_cursor += 1;
                }
                cx.notify();
            }
            "home" => {
                self.composer_cursor = 0;
                cx.notify();
            }
            "end" => {
                self.composer_cursor = self.composer_text.len();
                cx.notify();
            }
            "enter" => {
                if modifiers.shift {
                    self.composer_text.push('\n');
                    self.composer_cursor = self.composer_text.len();
                    cx.notify();
                } else {
                    self.send_composer(_window, cx);
                }
            }
            "up" | "down" => {}
            _ => {
                if let Some(ch) = event.keystroke.key_char.as_ref() {
                    if !modifiers.alt && !modifiers.platform && !modifiers.control {
                        if self.composer_cursor >= self.composer_text.len() {
                            self.composer_text.push_str(ch);
                        } else {
                            self.composer_cursor = self.composer_cursor.min(self.composer_text.len());
                            self.composer_text.insert_str(self.composer_cursor, ch);
                        }
                        self.composer_cursor += ch.len();
                        cx.notify();
                    }
                }
            }
        }
    }

    pub(crate) fn send_composer(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        let text = self.composer_text.trim().to_string();
        if text.is_empty() {
            return;
        }

        tracing::info!(
            "composer: send model={} mode={} text={:?}",
            self.composer_selected_model,
            self.composer_selected_mode,
            text
        );

        self.composer_text.clear();
        self.composer_cursor = 0;

        self.chat.push_message(ChatMessage {
            role: MessageRole::User,
            content: text.clone(),
            tool_calls: Vec::new(),
        });
        self.chat.scroll_to_bottom();

        if let Some(client) = self.clients.get(&self.active_agent_id).cloned() {
            self.state.agent_status = AgentStatus::Thinking;
            cx.notify();

            cx.spawn(async move |this, cx| {
                match client.send_prompt(&text).await {
                    Ok(prompt_response) => {
                        let _ = this.update(cx, |this, cx| {
                            this.chat.push_message(ChatMessage {
                                role: MessageRole::Agent,
                                content: prompt_response.text,
                                tool_calls: Vec::new(),
                            });
                            this.chat.scroll_to_bottom();
                            this.state.agent_status = AgentStatus::Connected;
                            // Update available modes/models from the session.
                            if let Some(modes) = prompt_response.modes {
                                this.available_modes = modes.available;
                                if this.composer_selected_mode.is_empty() {
                                    this.composer_selected_mode = modes.current_id;
                                }
                            }
                            if let Some(models) = prompt_response.models {
                                this.available_models = models.available;
                                if this.composer_selected_model.is_empty() {
                                    this.composer_selected_model = models.current_id;
                                }
                            }
                            cx.notify();
                        });
                    }
                    Err(e) => {
                        tracing::warn!("composer: ACP send failed: {e}");
                        let _ = this.update(cx, |this, cx| {
                            this.chat.push_message(ChatMessage {
                                role: MessageRole::Agent,
                                content: format!("Error: {e}"),
                                tool_calls: Vec::new(),
                            });
                            this.chat.scroll_to_bottom();
                            this.state.agent_status = AgentStatus::Connected;
                            cx.notify();
                        });
                    }
                }
            })
            .detach();
        } else {
            self.chat.push_message(ChatMessage {
                role: MessageRole::Agent,
                content: "ACP client not connected. Please wait for initialization.".to_string(),
                tool_calls: Vec::new(),
            });
            self.chat.scroll_to_bottom();
        }

        cx.notify();
    }
}
