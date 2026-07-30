use chronos_services::hermes_acp::StreamingEvent;
use chronos_ui::{Theme, on_fill};
use gpui::{
    Context, CursorStyle, ExternalPaths, IntoElement, MouseButton, MouseDownEvent,
    MouseMoveEvent, MouseUpEvent, Point, SharedString, Window, div, prelude::*, px,
};

use super::SidePanelLeft;
use super::chat_view::{ChatMessage, MessageRole, Segment};
use super::is_rtl_text;
use super::text_input::{TextInputElement, next_word_boundary, prev_word_boundary};
use super::state::AgentStatus;

/// Compute cursor byte-offset from a mouse position, using last prepaint geometry.
fn mouse_offset(
    line: &Option<gpui::ShapedLine>,
    bounds: &Option<gpui::Bounds<gpui::Pixels>>,
    position: &Point<gpui::Pixels>,
    actual_content: &str,
) -> usize {
    if actual_content.is_empty() { return 0; }
    let Some(line) = line else { return 0 };
    let Some(bounds) = bounds else { return 0 };
    if position.y < bounds.top() { return 0; }
    if position.y > bounds.bottom() { return actual_content.len(); }
    line.closest_index_for_x(position.x - bounds.left()).min(actual_content.len())
}

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
    let text = &panel.composer_input.content;
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

    let placeholder: SharedString = format!("Message {agent_display_name} — @ to include context, / for commands").into();

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
        .text_color(theme.text.primary)
        .when(is_rtl_text(text), |el| el.text_right())
        .track_focus(&focus)
        .on_click(cx.listener(|this, _, window, cx| {
            this.composer_focused = true;
            this.composer_model_dropdown_open = false;
            this.composer_mode_dropdown_open = false;
            this.composer_model_search.clear();
            window.focus(&this.composer_focus, cx);
            this.start_blink(cx);
            cx.notify();
        }))
        .cursor(CursorStyle::IBeam)
        .on_key_down(cx.listener(|this, event, window, cx| {
            this.handle_composer_key(event, window, cx);
        }))
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(|this, event: &MouseDownEvent, _window, cx| {
                let content = this.composer_input.content.clone();
                let offset = mouse_offset(&this.composer_last_layout, &this.composer_last_bounds, &event.position, &content);
                let shift = event.modifiers.shift;
                this.composer_input.on_mouse_down(offset, shift);
                // Double-click detection: select word (only when actual content exists)
                let now = std::time::Instant::now();
                let last = this.composer_last_click;
                this.composer_last_click = Some((now, event.position));
                let double = last.map_or(false, |(t, p)| {
                    now.duration_since(t).as_millis() < 500
                        && (p.x - event.position.x).abs() < px(5.)
                        && (p.y - event.position.y).abs() < px(5.)
                });
                if double && !content.is_empty() {
                    this.composer_input.move_to(prev_word_boundary(&content, offset));
                    this.composer_input.select_to(next_word_boundary(&content, offset));
                }
                cx.notify();
            }),
        )
        .on_mouse_up(
            MouseButton::Left,
            cx.listener(|this, _: &MouseUpEvent, _, cx| {
                this.composer_input.on_mouse_up();
                cx.notify();
            }),
        )
        .on_mouse_up_out(
            MouseButton::Left,
            cx.listener(|this, _: &MouseUpEvent, _, cx| {
                this.composer_input.on_mouse_up();
                cx.notify();
            }),
        )
        .on_mouse_move(cx.listener(|this, event: &MouseMoveEvent, _, cx| {
            let content = this.composer_input.content.clone();
            let offset = mouse_offset(&this.composer_last_layout, &this.composer_last_bounds, &event.position, &content);
            this.composer_input.on_mouse_move(offset);
            cx.notify();
        }))
        .on_drop(cx.listener(|this, paths: &ExternalPaths, _, cx| {
            let text = paths.paths().iter()
                .map(|p| p.display().to_string())
                .collect::<Vec<_>>()
                .join(" ");
            this.composer_input.insert_char(&text);
            this.composer_input.has_drop_hover = false;
            cx.notify();
        }))
        .on_drag_move::<ExternalPaths>(cx.listener(|this, _event: &gpui::DragMoveEvent<ExternalPaths>, _, cx| {
            this.composer_input.has_drop_hover = true;
            cx.notify();
        }))
        .when(panel.composer_input.has_drop_hover, |el| {
            el.bg(theme.accent.primary.opacity(0.08))
        })
        .child(
            TextInputElement {
                content: if text.is_empty() { placeholder.clone() } else { text.clone() },
                selected_range: panel.composer_input.selected_range.clone(),
                selection_reversed: panel.composer_input.selection_reversed,
                cursor_visible: panel.composer_input.cursor_visible,
                is_focused: panel.composer_focused,
                entity: cx.weak_entity(),
            },
        );

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

    let search_q = panel.composer_model_search.to_lowercase();

    let filtered: Vec<_> = if search_q.is_empty() {
        panel.available_models.iter().collect()
    } else {
        panel
            .available_models
            .iter()
            .filter(|m| {
                m.id.to_lowercase().contains(&search_q)
                    || m.name.to_lowercase().contains(&search_q)
            })
            .collect()
    };

    let search_active = !panel.composer_model_search.is_empty();
    let search_display: gpui::SharedString = if panel.composer_model_search.is_empty() {
        "Search models…".into()
    } else {
        panel.composer_model_search.clone().into()
    };
    let total = panel.available_models.len();
    let counter_text = if search_active {
        format!("{} of {}", filtered.len(), total)
    } else {
        format!("{}", total)
    };

    let model_items: Vec<_> = filtered
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
                    this.composer_model_search.clear();
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

    let list_content: Option<gpui::AnyElement> = if model_items.is_empty() {
        Some(
            div()
                .text_size(px(10.))
                .text_color(theme.text.disabled)
                .px(px(8.))
                .py(px(6.))
                .child("Nothing found")
                .into_any(),
        )
    } else {
        Some(
            div()
                .flex()
                .flex_col()
                .gap(px(2.))
                .children(model_items)
                .into_any(),
        )
    };

    let dropdown = if model_open {
        Some(
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
                .child(
                    div()
                        .flex_none()
                        .px(px(6.))
                        .py(px(4.))
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(6.))
                                .w_full()
                                .child(
                                    div()
                                        .flex_none()
                                        .text_size(px(10.))
                                        .text_color(theme.text.disabled)
                                        .child("🔍"),
                                )
                                .child(
                                    div()
                                        .flex_1()
                                        .text_size(px(10.5))
                                        .text_color(theme.text.primary)
                                        .child(search_display),
                                )
                                .child(
                                    div()
                                        .flex_none()
                                        .text_size(px(9.))
                                        .text_color(theme.text.muted)
                                        .child(counter_text),
                                ),
                        ),
                )
                .child(
                    div()
                        .flex_none()
                        .w_full()
                        .h(px(1.))
                        .bg(theme.border.subtle),
                )
                .child(
                    div()
                        .id("composer-model-dropdown-list")
                        .flex_1()
                        .max_h(px(250.))
                        .overflow_y_scroll()
                        .children(list_content),
                ),
        )
    } else {
        None
    };

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
                                if !this.composer_model_dropdown_open {
                                    this.composer_model_search.clear();
                                }
                                this.composer_mode_dropdown_open = false;
                                cx.notify();
                            }))
                    })
                    .child(format!("{} ⌄", selected_model_display)),
            )
            .children(dropdown),
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
                                this.composer_model_search.clear();
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
                        .max_h(px(300.))
                        .overflow_y_scroll()
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
    pub(crate) fn start_blink(&mut self, cx: &mut gpui::Context<Self>) {
        self.composer_input.cursor_visible = true;
        self.composer_blink_task.take();
        let handle = cx.weak_entity();
        let interval = super::text_input::CURSOR_BLINK_INTERVAL;
        let task = cx.spawn(async move |_, cx| {
            loop {
                cx.background_executor().timer(interval).await;
                let Ok(()) = handle.update(cx, |this, cx| {
                    this.composer_input.cursor_visible = !this.composer_input.cursor_visible;
                    cx.notify();
                }) else { break };
            }
        });
        self.composer_blink_task = Some(task);
    }

    pub(crate) fn stop_blink(&mut self) {
        self.composer_input.cursor_visible = false;
        self.composer_blink_task.take();
    }

    pub(crate) fn handle_composer_key(
        &mut self,
        event: &gpui::KeyDownEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // ── Sidebar search / rename input ────────────────────────────────
        if self.search_focused {
            let key = event.keystroke.key.as_str();
            let modifiers = &event.keystroke.modifiers;
            match key {
                "escape" => {
                    if self.rename_thread_id.is_some() {
                        self.cancel_rename(cx);
                    } else {
                        self.search_focused = false;
                        self.thread_search.clear();
                        self.search_threads("", cx);
                    }
                    return;
                }
                "return" | "enter" => {
                    if self.rename_thread_id.is_some() {
                        self.commit_rename(cx);
                    } else {
                        self.search_focused = false;
                        let q = self.thread_search.clone();
                        self.search_threads(&q, cx);
                    }
                    return;
                }
                "backspace" => {
                    if self.rename_thread_id.is_some() {
                        self.rename_input.pop();
                    } else {
                        self.thread_search.pop();
                        let q = self.thread_search.clone();
                        self.search_threads(&q, cx);
                    }
                    cx.notify();
                    return;
                }
                _ => {
                    if let Some(ch) = event.keystroke.key_char.as_ref() {
                        if !modifiers.alt && !modifiers.platform && !modifiers.control {
                            if self.rename_thread_id.is_some() {
                                self.rename_input.push_str(ch);
                            } else {
                                self.thread_search.push_str(ch);
                                let q = self.thread_search.clone();
                                self.search_threads(&q, cx);
                            }
                        }
                    }
                    cx.notify();
                    return;
                }
            }
        }

        if event.keystroke.key == "escape" {
            if self.composer_model_dropdown_open || self.composer_mode_dropdown_open {
                self.composer_model_dropdown_open = false;
                self.composer_mode_dropdown_open = false;
                self.composer_model_search.clear();
                cx.notify();
                return;
            }
            self.composer_focused = false;
            cx.notify();
            return;
        }

        // ── Model picker search input ──────────────────────────────────
        if self.composer_model_dropdown_open {
            let key = event.keystroke.key.as_str();
            let modifiers = &event.keystroke.modifiers;
            match key {
                "escape" => {
                    self.composer_model_dropdown_open = false;
                    self.composer_model_search.clear();
                }
                "return" | "enter" => {
                    let q = self.composer_model_search.to_lowercase();
                    if let Some(first) = self.available_models.iter().find(|m| {
                        let id = m.id.to_lowercase();
                        let name = m.name.to_lowercase();
                        q.is_empty() || id.contains(&q) || name.contains(&q)
                    }) {
                        let m_id = first.id.clone();
                        self.composer_selected_model = m_id.clone();
                        self.composer_model_dropdown_open = false;
                        self.composer_model_search.clear();
                        if let Some(client) = self.clients.get(&self.active_agent_id).cloned() {
                            cx.spawn(async move |this, cx| {
                                if let Err(e) = client.set_model(&m_id).await {
                                    tracing::warn!("set_model failed: {e}");
                                }
                                let _ = this.update(cx, |_this, cx| {
                                    cx.notify();
                                });
                            })
                            .detach();
                        }
                    }
                }
                "backspace" => {
                    self.composer_model_search.pop();
                }
                _ => {
                    if let Some(ch) = event.keystroke.key_char.as_ref() {
                        if !modifiers.alt && !modifiers.platform && !modifiers.control {
                            self.composer_model_search.push_str(ch);
                        }
                    }
                }
            }
            cx.notify();
            return;
        }

        if self.composer_mode_dropdown_open {
            self.composer_model_dropdown_open = false;
            self.composer_mode_dropdown_open = false;
            cx.notify();
            return;
        }

        let key = event.keystroke.key.as_str();
        let modifiers = &event.keystroke.modifiers;

        use super::text_input::TextInputState;

        match key {
            "backspace" if modifiers.platform => self.composer_input.delete_word_backward(),
            "backspace" => self.composer_input.backspace(),
            "delete" => self.composer_input.delete_forward(),
            "left" if modifiers.control => self.composer_input.cursor_left_word(),
            "left" if modifiers.shift => self.composer_input.select_left(),
            "left" => self.composer_input.cursor_left(),
            "right" if modifiers.control => self.composer_input.cursor_right_word(),
            "right" if modifiers.shift => self.composer_input.select_right(),
            "right" => self.composer_input.cursor_right(),
            "home" if modifiers.shift => self.composer_input.select_home(),
            "home" => self.composer_input.home(),
            "end" if modifiers.shift => self.composer_input.select_end(),
            "end" => self.composer_input.end(),
            "escape" => {
                self.composer_focused = false;
                self.stop_blink();
            }
            "enter" if modifiers.shift => self.composer_input.insert_char("\n"),
            "enter" => {
                self.send_composer(_window, cx);
                return;
            }
            "up" | "down" => {}
            _ => {
                // Printable character insertion is owned by the IME path
                // (`replace_text_in_range` via the `EntityInputHandler` in
                // text_input.rs) — do NOT insert `key_char` here too, or every
                // keystroke lands twice. Only clipboard/select shortcuts, which
                // the IME path never commits, are handled here.
                //
                // Clipboard uses `control || platform`: on Linux the clipboard
                // is Ctrl (`control`); `platform` (Super/Cmd) is kept so the
                // same binding works if ever run on macOS. Matches select-all.
                if (modifiers.control || modifiers.platform) && key == "a" {
                    self.composer_input.select_all();
                } else if (modifiers.control || modifiers.platform) && key == "c" {
                    self.composer_input.copy_selection(cx);
                } else if (modifiers.control || modifiers.platform) && key == "x" {
                    self.composer_input.cut_selection(cx);
                } else if (modifiers.control || modifiers.platform) && key == "v" {
                    self.composer_input.paste(cx);
                }
            }
        }
        cx.notify();
    }

    pub(crate) fn send_composer(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        // Don't send if agent is thinking
        if self.state.agent_status == AgentStatus::Thinking {
            return;
        }

        let text = self.composer_input.content.trim().to_string();
        if text.is_empty() {
            return;
        }

        tracing::info!(
            "composer: send model={} mode={} text={:?}",
            self.composer_selected_model,
            self.composer_selected_mode,
            text
        );

        self.composer_input.clear();

        // Set auto-title from the first user message (T151).
        let is_first_user_message = !self.chat.messages.iter().any(|m| m.role == MessageRole::User);
        if is_first_user_message {
            let thread_id = self.state.active_session_id.clone();
            if let Some(thread_id) = thread_id {
                self.set_auto_title(&thread_id, &text, cx);
            }
        }

        self.chat.push_message(ChatMessage {
            role: MessageRole::User,
            segments: vec![Segment::Response { content: text.clone() }],
        });
        self.chat.scroll_to_bottom();

        if let Some(client) = self.clients.get(&self.active_agent_id).cloned() {
            self.state.agent_status = AgentStatus::Thinking;
            tracing::info!("composer: turn START (model={} mode={} text_len={})",
                self.composer_selected_model, self.composer_selected_mode, text.len());

            // Create a streaming channel for real-time events.
            let (event_tx, event_rx) = tokio::sync::mpsc::unbounded_channel();

            // Push a placeholder agent message (segments filled by streaming).
            self.chat.push_message(ChatMessage {
                role: MessageRole::Agent,
                segments: Vec::new(),
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
                            if let Some(last_msg) = this.chat.messages.last_mut() {
                                if last_msg.role == MessageRole::Agent {
                                    // H1: guard against lost streaming events.
                                    let response_len: usize = last_msg.segments.iter()
                                        .filter_map(|s| match s {
                                            Segment::Response { content } => Some(content.len()),
                                            _ => None,
                                        })
                                        .sum();
                                    let expected_len = prompt_response.text.len();
                                    if response_len != expected_len {
                                        tracing::warn!(
                                            "composer: response segment length {} != prompt_response.text.len() {}",
                                            response_len, expected_len,
                                        );
                                    }
                                    let has_response = last_msg.segments.iter().any(|s| matches!(s, Segment::Response { .. }));
                                    if !has_response && !prompt_response.text.is_empty() {
                                        tracing::warn!(
                                            "composer: no Response segments from streaming, inserting from prompt_response ({} chars)",
                                            expected_len,
                                        );
                                        last_msg.segments.push(Segment::Response {
                                            content: prompt_response.text,
                                        });
                                    }
                                    let has_thinking = last_msg.segments.iter().any(|s| matches!(s, Segment::Thinking { .. }));
                                    if !has_thinking && !prompt_response.thought.is_empty() {
                                        last_msg.segments.push(Segment::Thinking {
                                            content: prompt_response.thought,
                                        });
                                    }
                                    // Sync tool call statuses from final response.
                                    for final_tool in &prompt_response.tools {
                                        if let Some(tool) = last_msg.segments.iter_mut().find_map(|s| {
                                            if let Segment::ToolCall { tool } = s {
                                                if tool.id == final_tool.id { Some(tool) } else { None }
                                            } else { None }
                                        }) {
                                            tool.status = final_tool.status.clone();
                                            tool.args.clone_from(&final_tool.args);
                                            tool.result.clone_from(&final_tool.result);
                                        }
                                    }
                                    // D1: honestly close any tool still pending.
                                    this.mark_pending_tools_stale();
                                }
                            }
                            // Collapse all Thinking segments in the last message.
                            let last_msg_idx = this.chat.messages.len().wrapping_sub(1);
                            if let Some(last_msg) = this.chat.messages.last() {
                                for (seg_idx, seg) in last_msg.segments.iter().enumerate() {
                                    if matches!(seg, Segment::Thinking { .. }) {
                                        this.chat.collapsed_reasoning.insert((last_msg_idx, seg_idx));
                                    }
                                }
                            }
                            this.chat.scroll_to_bottom();
                            this.state.agent_status = AgentStatus::Connected;
                            // Cache the updated transcript to the store (T151).
                            this.cache_transcript(cx);
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
                            if let Some(last_msg) = this.chat.messages.last_mut() {
                                if last_msg.role == MessageRole::Agent {
                                    let has_empty_response = last_msg.segments.last().map_or(false, |s| {
                                        matches!(s, Segment::Response { content } if content.is_empty())
                                    });
                                    if has_empty_response {
                                        if let Some(Segment::Response { content }) = last_msg.segments.last_mut() {
                                            *content = format!("Error: {e}");
                                        }
                                    } else {
                                        last_msg.segments.push(Segment::Response {
                                            content: format!("Error: {e}"),
                                        });
                                    }
                                }
                            }
                            // Collapse all Thinking segments in the last message.
                            let last_msg_idx = this.chat.messages.len().wrapping_sub(1);
                            if let Some(last_msg) = this.chat.messages.last() {
                                for (seg_idx, seg) in last_msg.segments.iter().enumerate() {
                                    if matches!(seg, Segment::Thinking { .. }) {
                                        this.chat.collapsed_reasoning.insert((last_msg_idx, seg_idx));
                                    }
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
                                                    let append = last_msg.segments.last_mut().and_then(|s| {
                                                        if let Segment::Response { content } = s {
                                                            Some(content)
                                                        } else {
                                                            None
                                                        }
                                                    });
                                                    if let Some(content) = append {
                                                        content.push_str(&delta);
                                                    } else {
                                                        last_msg.segments.push(Segment::Response { content: delta });
                                                    }
                                                }
                                            }
                                            this.chat.scroll_to_bottom();
                                        }
                                        StreamingEvent::ThoughtChunk(delta) => {
                                            if let Some(last_msg) = this.chat.messages.last_mut() {
                                                if last_msg.role == MessageRole::Agent {
                                                    let append = last_msg.segments.last_mut().and_then(|s| {
                                                        if let Segment::Thinking { content } = s {
                                                            Some(content)
                                                        } else {
                                                            None
                                                        }
                                                    });
                                                    if let Some(content) = append {
                                                        content.push_str(&delta);
                                                    } else {
                                                        let seg_idx = last_msg.segments.len();
                                                        last_msg.segments.push(Segment::Thinking { content: delta });
                                                        let msg_idx = this.chat.messages.len().wrapping_sub(1);
                                                        this.chat.collapsed_reasoning.remove(&(msg_idx, seg_idx));
                                                    }
                                                }
                                            }
                                            this.chat.scroll_to_bottom();
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
                                                    let found = last_msg.segments.iter_mut().rev().find_map(|s| {
                                                        if let Segment::ToolCall { tool } = s {
                                                            if tool.id == id { Some(tool) } else { None }
                                                        } else { None }
                                                    });
                                                    if let Some(tool) = found {
                                                        tool.status = status;
                                                        tool.args = args;
                                                        tool.result = result;
                                                    } else {
                                                        last_msg.segments.push(Segment::ToolCall {
                                                            tool: super::chat_view::ToolCallPreview {
                                                                id,
                                                                name,
                                                                status,
                                                                args,
                                                                result,
                                                            },
                                                        });
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
                                        if last_msg.role == MessageRole::Agent {
                                            let has_nonempty_response = last_msg.segments.iter().any(|s| {
                                                matches!(s, Segment::Response { content } if !content.is_empty())
                                            });
                                            if !has_nonempty_response {
                                                last_msg.segments.push(Segment::Response {
                                                    content: if events_received == 0 {
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
                                                    },
                                                });
                                            }
                                        }
                                    }
                                    // Collapse all Thinking segments in the last message.
                                    let last_msg_idx = this.chat.messages.len().wrapping_sub(1);
                                    if let Some(last_msg) = this.chat.messages.last() {
                                        for (seg_idx, seg) in last_msg.segments.iter().enumerate() {
                                            if matches!(seg, Segment::Thinking { .. }) {
                                                this.chat.collapsed_reasoning.insert((last_msg_idx, seg_idx));
                                            }
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
                segments: vec![Segment::Response {
                    content: "ACP client not connected. Please wait for initialization.".to_string(),
                }],
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
            for seg in msg.segments.iter_mut() {
                if let Segment::ToolCall { tool } = seg {
                    let s = tool.status.trim().to_ascii_lowercase();
                    if !TERMINAL.contains(&s.as_str()) {
                        tool.status = "stale".to_string();
                    }
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
                let found = last_msg.segments.iter_mut().rev().find_map(|s| {
                    if let Segment::Response { content } = s { Some(content) } else { None }
                });
                if let Some(content) = found {
                    if content.is_empty() {
                        *content = "⏹ Turn cancelled by user.".to_string();
                    } else {
                        content.push_str("\n\n⏹ Turn cancelled by user.");
                    }
                } else {
                    last_msg.segments.push(Segment::Response {
                        content: "⏹ Turn cancelled by user.".to_string(),
                    });
                }
            }
        }
        // Collapse all Thinking segments in the last message.
        let last_msg_idx = self.chat.messages.len().wrapping_sub(1);
        if let Some(last_msg) = self.chat.messages.last() {
            for (seg_idx, seg) in last_msg.segments.iter().enumerate() {
                if matches!(seg, Segment::Thinking { .. }) {
                    self.chat.collapsed_reasoning.insert((last_msg_idx, seg_idx));
                }
            }
        }
        self.mark_pending_tools_stale();
        self.chat.scroll_to_bottom();
        self.state.agent_status = AgentStatus::Connected;
        cx.notify();
    }
}
