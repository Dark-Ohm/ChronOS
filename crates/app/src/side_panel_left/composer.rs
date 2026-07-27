use chronos_services::hermes_acp::StreamingEvent;
use chronos_ui::{Theme, on_fill};
use gpui::{IntoElement, SharedString, Window, div, prelude::*, px};

use super::SidePanelLeft;
use super::chat_view::{ChatMessage, MessageRole};
use super::state::AgentStatus;

impl SidePanelLeft {
    /// Scan available_modes for a mode whose `id` contains "bypass", "dont",
    /// or "yolo" (case-insensitive). Cache result in `composer_yolo_bypass_id`.
    pub(crate) fn detect_yolo_bypass_mode(&mut self) -> Option<String> {
        let found = self.available_modes.iter().find(|m| {
            let lower = m.id.to_lowercase();
            lower.contains("bypass") || lower.contains("dont") || lower.contains("yolo")
        });
        let id = found.map(|m| m.id.clone());
        self.composer_yolo_bypass_id = id.clone();
        id
    }

    /// Toggle YOLO mode: if currently on the bypass mode, restore previous;
    /// otherwise switch to bypass mode (saving current as previous).
    pub(crate) fn toggle_yolo(&mut self, cx: &mut gpui::Context<Self>) {
        let Some(ref yolo_id) = self.composer_yolo_bypass_id else {
            return;
        };

        if self.composer_selected_mode == *yolo_id {
            // Toggle off — restore previous mode
            if !self.composer_previous_mode.is_empty() {
                self.composer_selected_mode = std::mem::take(&mut self.composer_previous_mode);
            }
        } else {
            // Toggle on — save current, switch to yolo
            self.composer_previous_mode = self.composer_selected_mode.clone();
            self.composer_selected_mode = yolo_id.clone();
        }
        cx.notify();
    }
}

pub fn render_composer(
    panel: &SidePanelLeft,
    _window: &mut Window,
    cx: &mut Context<SidePanelLeft>,
) -> impl IntoElement {
    let theme = *Theme::global(cx);
    let text = &panel.composer_text;
    let has_text = !text.is_empty();

    let send_active = has_text && panel.state.agent_status != AgentStatus::Thinking;

    // ── YOLO state ──────────────────────────────────────────────────
    let yolo_mode_id = panel.composer_yolo_bypass_id.as_deref();
    let has_modes = !panel.available_modes.is_empty();
    let is_yolo_active = yolo_mode_id
        .map(|yid| panel.composer_selected_mode == *yid)
        .unwrap_or(false);

    // ── Text display (placeholder or content) ───────────────────────
    let agent_display_name = panel
        .agents
        .iter()
        .find(|a| a.id == panel.active_agent_id)
        .map(|a| a.display_name.as_str())
        .unwrap_or("Agent");

    // ── Pickers row (above textarea) ────────────────────────────────
    let pickers_row = div()
        .id("composer-pickers-row")
        .flex_none()
        .flex()
        .items_center()
        .gap(px(6.))
        .children(model_picker(panel, cx))
        .children(mode_picker(panel, cx))
        .children(yolo_button(panel, is_yolo_active, has_modes, cx));

    // ── Textarea ────────────────────────────────────────────────────
    let enabled = panel.state.agent_status != AgentStatus::Disconnected;
    let focus = panel.composer_focus.clone();

    let input_display: SharedString = if text.is_empty() {
        format!("Message {agent_display_name} — @ to include context, / for commands").into()
    } else {
        text.clone().into()
    };

    let input_text_color = if text.is_empty() {
        theme.text.muted
    } else {
        theme.text.primary
    };

    let panel_content_width = panel.state.width - 24.0;
    let glyph_approx_px = 7.0;
    let max_chars_per_line = (panel_content_width / glyph_approx_px).max(10.0) as usize;

    let lines: usize = text
        .lines()
        .map(|l| {
            let raw = l.len();
            if raw == 0 {
                1
            } else {
                (raw + max_chars_per_line - 1) / max_chars_per_line
            }
        })
        .sum();
    let visible_lines = lines.max(3).min(100);
    let line_height_px = 18.0;
    let input_height = px((visible_lines as f32 * line_height_px).min(panel.state.height * 0.45));

    let text_input = div()
        .id("composer-input-canvas")
        .flex_1()
        .min_h(px(48.))
        .max_h(px(panel.state.height * 0.45))
        .h(input_height)
        .px(px(6.))
        .py(px(2.))
        .overflow_y_scroll()
        .text_size(px(12.5))
        .line_height(px(18.))
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

    // ── Input container (bordered box: attach + textarea + send) ─────
    let input_container = div()
        .id("composer-input-container")
        .flex_none()
        .flex()
        .items_end()
        .gap(px(6.))
        .bg(theme.bg.primary)
        .border_1()
        .border_color(theme.border.subtle)
        .rounded(px(8.))
        .px(px(7.))
        .py(px(6.))
        .child(attach_button(panel, cx))
        .child(text_input)
        .child(send_button(panel, send_active, cx));

    // ── Compose container ───────────────────────────────────────────
    div()
        .id("composer-wrap")
        .flex_none()
        .bg(theme.bg.primary)
        .border_t_1()
        .border_color(theme.border.subtle)
        .px(px(9.))
        .py(px(9.))
        .flex()
        .flex_col()
        .gap(px(7.))
        .when(!enabled, |el| el.opacity(0.5))
        .child(pickers_row)
        .child(input_container)
}

// ── Attach button ──────────────────────────────────────────────────────
fn attach_button(_panel: &SidePanelLeft, _cx: &mut Context<SidePanelLeft>) -> impl IntoElement {
    let theme = *Theme::global(_cx);
    div()
        .id("composer-attach")
        .w(px(18.))
        .h(px(18.))
        .rounded(px(5.))
        .flex()
        .items_center()
        .justify_center()
        .text_size(px(11.))
        .text_color(theme.text.muted)
        .cursor_pointer()
        .hover(|s| s.bg(theme.border.subtle).text_color(theme.text.secondary))
        .child("+")
}

// ── YOLO button ────────────────────────────────────────────────────────
fn yolo_button(
    panel: &SidePanelLeft,
    is_yolo_active: bool,
    has_modes: bool,
    cx: &mut Context<SidePanelLeft>,
) -> Option<impl IntoElement> {
    let theme = *Theme::global(cx);
    // YOLO only renders if there are modes at all
    if !has_modes {
        return None;
    }

    let has_bypass = panel.composer_yolo_bypass_id.is_some();

    // Build the base element with id FIRST (needed by hover/on_click).
    let base = div()
        .id("composer-yolo")
        .flex_none()
        .px(px(6.))
        .py(px(2.))
        .rounded(px(4.))
        .text_size(px(10.))
        .font_weight(gpui::FontWeight::SEMIBOLD);

    Some(
        if is_yolo_active {
            // Active: text #f38ba8, bg rgba(0xf38ba8, 0.12)
            base.text_color(theme.status.error)
                .bg(theme.status.error.opacity(0.12))
        } else if has_bypass {
            // Inactive but available
            base.text_color(theme.text.muted)
                .cursor_pointer()
                .hover(|s| s.bg(theme.border.subtle))
                .on_click(cx.listener(|this, _, _, cx| {
                    this.detect_yolo_bypass_mode();
                    this.toggle_yolo(cx);
                }))
        } else {
            // Disabled (no bypass mode found) — muted, no hover/cursor
            base.text_color(theme.text.disabled)
        }
        .child("YOLO"),
    )
}

// ── Model picker ───────────────────────────────────────────────────────
fn model_picker(
    panel: &SidePanelLeft,
    cx: &mut Context<SidePanelLeft>,
) -> Option<impl IntoElement> {
    let theme = *Theme::global(cx);
    // Show a muted, inert "Model" placeholder pill instead of hiding when
    // there's no data yet — our Hermes ACP agent only advertises available
    // models on a session response, and (live smoke, 2026-07-23) that can
    // still be empty right after connect. Hiding the indicator entirely
    // reads as "no such feature", not "loading" — keep the affordance
    // visible, just disabled.
    let has_data = !panel.available_models.is_empty();
    let selected_model = panel.composer_selected_model.clone();
    let selected_model_display = if selected_model.is_empty() {
        "Model".to_string()
    } else {
        selected_model.clone()
    };
    let model_open = has_data && panel.composer_model_dropdown_open;

    let model_items: Vec<_> = panel
        .available_models
        .iter()
        .enumerate()
        .map(|(i, m)| {
            let m_id = m.id.clone();
            let m_name = if m.name.is_empty() {
                m.id.clone()
            } else {
                m.name.clone()
            };
            let is_active = m.id == selected_model;
            div()
                .id(format!("model-item-{i}"))
                .w_full()
                .px(px(10.))
                .py(px(5.))
                .rounded(px(4.))
                .text_size(px(11.))
                .text_color(if is_active {
                    theme.text.primary
                } else {
                    theme.text.secondary
                })
                .when(is_active, |el| el.bg(theme.border.default))
                .when(!is_active, |el| el.hover(|s| s.bg(theme.border.subtle)))
                .cursor_pointer()
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.composer_selected_model = m_id.clone();
                    this.composer_model_dropdown_open = false;
                    // Notify the agent to switch model.
                    if let Some(client) = this.clients.get(&this.active_agent_id).cloned() {
                        let model = m_id.clone();
                        cx.spawn(async move |this, cx| {
                            if let Err(e) = client.set_model(&model).await {
                                tracing::warn!("set_model failed: {e}");
                            }
                            let _ = this.update(cx, |_this, cx| {
                                cx.notify();
                            });
                        })
                        .detach();
                    }
                    cx.notify();
                }))
                .child(m_name)
        })
        .collect();

    Some(
        div()
            .id("composer-model-picker-wrap")
            .relative()
            .child(
                div()
                    .id("composer-model-picker")
                    .h(px(22.))
                    .px(px(8.))
                    .rounded(px(6.))
                    .border_1()
                    .border_color(theme.border.subtle)
                    .flex()
                    .items_center()
                    .text_size(px(10.5))
                    .text_color(if has_data {
                        theme.text.secondary
                    } else {
                        theme.text.disabled
                    })
                    .when(has_data, |el| {
                        el.cursor_pointer()
                            .hover(|s| s.bg(theme.border.subtle))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.composer_model_dropdown_open =
                                    !this.composer_model_dropdown_open;
                                this.composer_mode_dropdown_open = false;
                                cx.notify();
                            }))
                    })
                    .child(format!("{} ⌄", selected_model_display)),
            )
            .when(model_open, |el| {
                el.child(
                    div()
                        .id("composer-model-dropdown")
                        .absolute()
                        .bottom(px(26.))
                        .right(px(0.))
                        .min_w(px(200.))
                        .bg(theme.bg.primary)
                        .border_1()
                        .border_color(theme.border.default)
                        .rounded(px(6.))
                        .p(px(4.))
                        .flex()
                        .flex_col()
                        .gap(px(2.))
                        .children(model_items),
                )
            }),
    )
}

// ── Mode picker ────────────────────────────────────────────────────────
fn mode_picker(panel: &SidePanelLeft, cx: &mut Context<SidePanelLeft>) -> Option<impl IntoElement> {
    let theme = *Theme::global(cx);
    // Same "muted placeholder, not hidden" reasoning as model_picker above.
    let has_data = !panel.available_modes.is_empty();
    let selected_mode = panel.composer_selected_mode.clone();
    let selected_mode_display = if selected_mode.is_empty() {
        "Mode".to_string()
    } else {
        selected_mode.clone()
    };
    let mode_open = has_data && panel.composer_mode_dropdown_open;

    let mode_items: Vec<_> = panel
        .available_modes
        .iter()
        .enumerate()
        .map(|(i, m)| {
            let m_id = m.id.clone();
            let m_name = if m.name.is_empty() {
                m.id.clone()
            } else {
                m.name.clone()
            };
            let is_active = m.id == selected_mode;
            div()
                .id(format!("mode-item-{i}"))
                .w_full()
                .px(px(10.))
                .py(px(5.))
                .rounded(px(4.))
                .text_size(px(11.))
                .text_color(if is_active {
                    theme.text.primary
                } else {
                    theme.text.secondary
                })
                .when(is_active, |el| el.bg(theme.border.default))
                .when(!is_active, |el| el.hover(|s| s.bg(theme.border.subtle)))
                .cursor_pointer()
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.composer_selected_mode = m_id.clone();
                    this.composer_mode_dropdown_open = false;
                    cx.notify();
                }))
                .child(m_name)
        })
        .collect();

    Some(
        div()
            .id("composer-mode-picker-wrap")
            .relative()
            .child(
                div()
                    .id("composer-mode-picker")
                    .h(px(22.))
                    .px(px(8.))
                    .rounded(px(6.))
                    .border_1()
                    .border_color(theme.border.subtle)
                    .flex()
                    .items_center()
                    .text_size(px(10.5))
                    .text_color(if has_data {
                        theme.text.secondary
                    } else {
                        theme.text.disabled
                    })
                    .when(has_data, |el| {
                        el.cursor_pointer()
                            .hover(|s| s.bg(theme.border.subtle))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.composer_mode_dropdown_open =
                                    !this.composer_mode_dropdown_open;
                                this.composer_model_dropdown_open = false;
                                cx.notify();
                            }))
                    })
                    .child(format!("{} ⌄", selected_mode_display)),
            )
            .when(mode_open, |el| {
                el.child(
                    div()
                        .id("composer-mode-dropdown")
                        .absolute()
                        .bottom(px(26.))
                        .right(px(0.))
                        .min_w(px(80.))
                        .bg(theme.bg.primary)
                        .border_1()
                        .border_color(theme.border.default)
                        .rounded(px(6.))
                        .p(px(4.))
                        .flex()
                        .flex_col()
                        .gap(px(2.))
                        .children(mode_items),
                )
            }),
    )
}

// ── Send / Stop button (dark style) ────────────────────────────────────
fn send_button(
    panel: &SidePanelLeft,
    active: bool,
    cx: &mut Context<SidePanelLeft>,
) -> impl IntoElement {
    let theme = *Theme::global(cx);
    let is_connected = panel.state.agent_status != AgentStatus::Disconnected;
    let is_streaming = panel.streaming.active;
    let fill = on_fill(theme.accent.primary);

    // D2: while a turn is streaming, the button becomes Stop (■) instead of
    // Send (▶), giving the user a way to cancel a hung/dead turn.
    if is_streaming {
        return div()
            .id("composer-stop")
            .w(px(22.))
            .h(px(22.))
            .rounded(px(6.))
            .flex()
            .items_center()
            .justify_center()
            .bg(theme.status.error)
            .text_color(theme.text.primary)
            .cursor_pointer()
            .hover(|s| s.bg(theme.status.error))
            .on_click(cx.listener(|this, _, _window, cx| {
                this.cancel_streaming(cx);
            }))
            .child("■");
    }

    div()
        .id("composer-send")
        .w(px(22.))
        .h(px(22.))
        .rounded(px(6.))
        .flex()
        .items_center()
        .justify_center()
        .text_size(px(11.))
        .when(active && is_connected, |el| {
            el.bg(theme.accent.primary)
                .text_color(fill)
                .cursor_pointer()
                .hover(|s| s.bg(theme.accent.hover))
                .on_click(cx.listener(|this, _, window, cx| {
                    this.send_composer(window, cx);
                }))
        })
        .when(!active || !is_connected, |el| {
            el.bg(theme.bg.tertiary)
                .border_1()
                .border_color(theme.border.default)
                .text_color(theme.text.disabled)
        })
        .child("▶")
}

// ── Existing helper methods (unchanged) ─────────────────────────────────
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
                            self.composer_cursor =
                                self.composer_cursor.min(self.composer_text.len());
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
        // Don't send if agent is thinking
        if self.state.agent_status == AgentStatus::Thinking {
            return;
        }

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
            thought: None,
            tool_calls: Vec::new(),
        });
        self.chat.scroll_to_bottom();

        if let Some(client) = self.clients.get(&self.active_agent_id).cloned() {
            self.state.agent_status = AgentStatus::Thinking;
            tracing::info!("composer: turn START (model={} mode={} text_len={})",
                self.composer_selected_model, self.composer_selected_mode, text.len());

            // Create a streaming channel for real-time events.
            let (event_tx, event_rx) = tokio::sync::mpsc::unbounded_channel();

            // Push a placeholder agent message that will be updated in-place.
            self.chat.push_message(ChatMessage {
                role: MessageRole::Agent,
                content: String::new(),
                thought: None,
                tool_calls: Vec::new(),
            });
            self.chat.scroll_to_bottom();

            // Spawn the ACP prompt task with streaming.
            let acp_task = cx.spawn(async move |this, cx| {
                match client.send_prompt_streaming(&text, event_tx).await {
                    Ok(prompt_response) => {
                        tracing::info!(
                            session_id = %prompt_response.session_id,
                            chars = prompt_response.text.len(),
                            tools = prompt_response.tools.len(),
                            "composer: ACP streaming reply complete"
                        );
                        let upd = this.update(cx, |this, cx| {
                            tracing::info!("composer: turn END (reason=ok, session={}, chars={})",
                                prompt_response.session_id, prompt_response.text.len());
                            tracing::debug!("composer: finalize update entered");
                            this.streaming.reset();
                            this.state.session_id = Some(prompt_response.session_id);
                            // Finalize the last agent message with complete data.
                            if let Some(last_msg) = this.chat.messages.last_mut() {
                                if last_msg.role == MessageRole::Agent {
                                    last_msg.content = prompt_response.text;
                                    last_msg.thought = if prompt_response.thought.is_empty() {
                                        None
                                    } else {
                                        Some(prompt_response.thought)
                                    };
                                    last_msg.tool_calls = prompt_response
                                        .tools
                                        .into_iter()
                                        .map(|t| super::chat_view::ToolCallPreview {
                                            id: t.id,
                                            name: t.name,
                                            status: t.status,
                                            args: t.args,
                                            result: t.result,
                                        })
                                        .collect();
                                    // D1: honestly close any tool still pending.
                                    this.mark_pending_tools_stale();
                                }
                            }
                            this.chat.scroll_to_bottom();
                            this.state.agent_status = AgentStatus::Connected;
                            // Update available modes/models from the session.
                            if let Some(modes) = prompt_response.modes {
                                this.available_modes = modes.available;
                                if this.composer_selected_mode.is_empty() {
                                    this.composer_selected_mode = modes.current_id;
                                }
                                this.detect_yolo_bypass_mode();
                            }
                            if let Some(models) = prompt_response.models {
                                this.available_models = models.available;
                                if this.composer_selected_model.is_empty() {
                                    this.composer_selected_model = models.current_id;
                                }
                            }
                            cx.notify();
                        });
                        if let Err(e) = upd {
                            tracing::error!("composer: finalize update FAILED: {e}");
                        }
                    }
                    Err(e) => {
                        tracing::warn!("composer: ACP send failed: {e}");
                        tracing::info!("composer: turn END (reason=error)");
                        let disconnected = e.to_string().contains("command channel closed")
                            || e.to_string().contains("reply channel closed");
                        let _ = this.update(cx, |this, cx| {
                            this.streaming.reset();
                            // Replace the placeholder with an error message.
                            if let Some(last_msg) = this.chat.messages.last_mut() {
                                if last_msg.role == MessageRole::Agent {
                                    last_msg.content = format!("Error: {e}");
                                }
                            }
                            // D1: honestly close any tool still pending on failure.
                            this.mark_pending_tools_stale();
                            this.chat.scroll_to_bottom();
                            this.state.agent_status = if disconnected {
                                AgentStatus::Disconnected
                            } else {
                                AgentStatus::Connected
                            };
                            cx.notify();
                        });
                    }
                }
            });

            // Store ACP task for cancellation on panel drop / new session.
            self.streaming.acp_task = Some(acp_task);

            // Spawn a GPUI task to consume streaming events and update the
            // placeholder agent message in real-time.
            let streaming_task = cx.spawn(async move |this, cx| {
                use std::time::Duration;
                let mut rx = event_rx;
                let mut events_received: u64 = 0;
                const TURN_TIMEOUT: Duration = Duration::from_secs(180);

                // E3: the service-side watchdog (`stream_read_turn`,
                // `TURN_COMPLETE_TIMEOUT`) is 120s. The panel timeout MUST be
                // strictly LARGER so it only fires when the service has already
                // given up — otherwise the panel can announce "⏱ Turn timed out"
                // on a turn that the service would have closed honestly a moment
                // later. The panel is the outer safety contour, not the primary
                // closer. 180s = 120s service window + 60s slack for the ACP
                // stream to drain into the UI before we declare a stall.

                // D2/D4 live-lock root cause: `cx.background_executor().timer()`
                // (Source/gpui/src/executor.rs:162) is NOT a cheap future — it is
                // `self.spawn(self.inner.scheduler().timer(duration))`, i.e. it
                // SPAWNS a task. Creating it inside the loop (the previous broken
                // pattern) meant one spawn + one cancel (drop schedules a runnable
                // via `ping`) PER EVENT — 125 events => 125 spawns + 125 cancels,
                // each waking the main thread => live-lock (gdb: dispatch_idles
                // never drains, ping re-arms the loop). Fix: create the timer
                // ONCE; on each event only stamp `last_event`. The timer fires at
                // most once per TURN_TIMEOUT of silence, so there is no per-event
                // spawn/cancel storm.
                let mut last_event = cx.background_executor().now();
                let mut timer = cx.background_executor().timer(TURN_TIMEOUT);

                loop {
                    tokio::select! {
                        event = rx.recv() => match event {
                            Some(event) => {
                                events_received += 1;
                                last_event = cx.background_executor().now();
                                tracing::debug!("composer: streaming event received");
                                let upd = this.update(cx, |this, cx| {
                                    match event {
                                        StreamingEvent::TextChunk(delta) => {
                                            if let Some(last_msg) = this.chat.messages.last_mut() {
                                                if last_msg.role == MessageRole::Agent {
                                                    last_msg.content.push_str(&delta);
                                                }
                                            }
                                            this.chat.scroll_to_bottom();
                                        }
                                        StreamingEvent::ThoughtChunk(delta) => {
                                            if let Some(last_msg) = this.chat.messages.last_mut() {
                                                if last_msg.role == MessageRole::Agent {
                                                    match &mut last_msg.thought {
                                                        Some(thought) => thought.push_str(&delta),
                                                        None => {
                                                            last_msg.thought = Some(delta);
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        StreamingEvent::ToolCall {
                                            id,
                                            name,
                                            status,
                                            args,
                                            result,
                                        } => {
                                            if let Some(last_msg) = this.chat.messages.last_mut() {
                                                if last_msg.role == MessageRole::Agent {
                                                    // D1: merge by stable tool-call id, not by
                                                    // display name — two tools with the same
                                                    // title would otherwise collapse into one.
                                                    if let Some(tc) = last_msg
                                                        .tool_calls
                                                        .iter_mut()
                                                        .find(|t| t.id == id)
                                                    {
                                                        tc.status = status;
                                                        tc.args = args;
                                                        tc.result = result;
                                                    } else {
                                                        last_msg.tool_calls.push(
                                                            super::chat_view::ToolCallPreview {
                                                                id,
                                                                name,
                                                                status,
                                                                args,
                                                                result,
                                                            },
                                                        );
                                                    }
                                                }
                                            }
                                        }
                                        StreamingEvent::Done => {
                                            // Final update happens in the ACP task.
                                        }
                                        StreamingEvent::Error(_) => {
                                            // Error handling happens in the ACP task.
                                        }
                                    }
                                    cx.notify();
                                });
                                if let Err(e) = upd {
                                    tracing::error!("composer: streaming update FAILED: {e}");
                                    return;
                                }
                            }
                            None => {
                                // Channel closed — ACP task finished.
                                break;
                            }
                        },
                        _ = &mut timer => {
                            // D2: timer fired. Distinguish a *real* stall
                            // (silence >= TURN_TIMEOUT since the last event)
                            // from a long-but-alive turn (tools running /
                            // agent mid-stream) by comparing against the
                            // stamped `last_event`, NOT by recreating the
                            // future each iteration.
                            let silent = last_event.elapsed();
                            if silent >= TURN_TIMEOUT {
                                // Diagnostic: zero events received is the
                                // signature of a BROKEN channel (sender/receiver
                                // mismatch), distinct from a merely slow agent.
                                if events_received == 0 {
                                    tracing::error!(
                                        "composer: turn timed out after {}s with ZERO streaming \
                                         events received — streaming channel is likely broken \
                                         (check D3 on_event sender / receiver wiring)",
                                        TURN_TIMEOUT.as_secs()
                                    );
                                } else {
                                    tracing::warn!(
                                        "composer: turn timed out after {}s of agent silence \
                                         ({} events delivered before stall)",
                                        TURN_TIMEOUT.as_secs(),
                                        events_received
                                    );
                                }
                                let _ = this.update(cx, |this, cx| {
                                    this.streaming.reset();
                                    if let Some(last_msg) = this.chat.messages.last_mut() {
                                        if last_msg.role == MessageRole::Agent
                                            && last_msg.content.is_empty()
                                        {
                                            last_msg.content = if events_received == 0 {
                                                format!(
                                                    "⏱ Turn timed out after {}s — no streaming \
                                                     events reached the UI (channel broken).",
                                                    TURN_TIMEOUT.as_secs()
                                                )
                                            } else {
                                                format!(
                                                    "⏱ Turn timed out after {}s of agent silence.",
                                                    TURN_TIMEOUT.as_secs()
                                                )
                                            };
                                        }
                                    }
                                    this.mark_pending_tools_stale();
                                    this.chat.scroll_to_bottom();
                                    this.state.agent_status = AgentStatus::Connected;
                                    cx.notify();
                                });
                                break;
                            } else {
                                // Agent still active (event arrived < TURN_TIMEOUT
                                // ago). Re-arm the timer for the remaining window
                                // and keep waiting. This re-arm is rare (<= once
                                // per TURN_TIMEOUT of silence), so no live-lock.
                                tracing::debug!(
                                    "composer: D2 timer fired but agent active ({}s since last \
                                     event < {}s) — extending window",
                                    silent.as_secs(),
                                    TURN_TIMEOUT.as_secs()
                                );
                                timer = cx.background_executor().timer(TURN_TIMEOUT - silent);
                                continue;
                            }
                        }
                    }
                }
            });

            // Store the streaming receiver task so it's aborted if the panel
            // is dropped or a new session is created.
            self.streaming.active = true;
            self.streaming.receiver_task = Some(streaming_task);
        } else {
            self.chat.push_message(ChatMessage {
                role: MessageRole::Agent,
                content: "ACP client not connected. Please wait for initialization.".to_string(),
                thought: None,
                tool_calls: Vec::new(),
            });
            self.chat.scroll_to_bottom();
        }

        cx.notify();
    }

    /// D1: any tool call still marked non-terminal (pending/running/unknown)
    /// when a turn ends — via Done, Error, timeout, or cancel — is honestly
    /// closed as `stale` instead of left spinning forever. The agent (Hermes)
    /// frequently does not emit a terminal `ToolCallUpdate` for `write`-class
    /// tools, so without this the spinner never resolves.
    pub fn mark_pending_tools_stale(&mut self) {
        const TERMINAL: &[&str] = &["done", "error", "stale", "canceled", "denied", "expired"];
        for msg in self.chat.messages.iter_mut() {
            if msg.role != MessageRole::Agent {
                continue;
            }
            for tc in msg.tool_calls.iter_mut() {
                let s = tc.status.trim().to_ascii_lowercase();
                if !TERMINAL.contains(&s.as_str()) {
                    tc.status = "stale".to_string();
                }
            }
        }
    }

    /// D2: local cancel of an in-progress turn. Aborts the streaming/ACP tasks,
    /// honestly closes any pending tool cards, and leaves a note in the chat so
    /// the user sees the turn was cancelled rather than silently dropping.
    pub fn cancel_streaming(&mut self, cx: &mut Context<Self>) {
        if !self.streaming.active {
            return;
        }
        tracing::info!("composer: turn END (reason=cancel)");
        self.streaming.reset();
        if let Some(last_msg) = self.chat.messages.last_mut() {
            if last_msg.role == MessageRole::Agent {
                if last_msg.content.is_empty() {
                    last_msg.content = "⏹ Turn cancelled by user.".to_string();
                } else {
                    // Don't discard a half-received answer — append the marker.
                    last_msg
                        .content
                        .push_str("\n\n⏹ Turn cancelled by user.");
                }
            }
        }
        self.mark_pending_tools_stale();
        self.chat.scroll_to_bottom();
        self.state.agent_status = AgentStatus::Connected;
        cx.notify();
    }
}
