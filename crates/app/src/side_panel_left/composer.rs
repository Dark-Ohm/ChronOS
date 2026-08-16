use chronos_services::hermes_acp::StreamingEvent;
use chronos_ui::{Theme, on_fill};
use gpui::{
    Context, ExternalPaths, IntoElement, SharedString, Window, div, img, prelude::*, px,
};
use gpui_component::input::Input;
use gpui_component::searchable_list::{SearchableListItem, SearchableListDelegate, SearchableVec};
use gpui_component::select::{Select, SelectState};
use gpui_component::{Sizable, Size as KitSize};

use crate::side_panel_left::ChatTab;
use super::chat_view::{ChatMessage, MessageRole, Segment};
use super::state::AgentStatus;

// ── Select item types (T287-A) ──────────────────────────────────────────
// The kit `Select` delegates over these. Value is the model/mode id
// (Clone + PartialEq); `title()` mirrors the old manual row: name, falling
// back to id when the ACP agent omits a display name.
#[derive(Clone)]
pub(crate) struct ModelSelectItem {
    id: SharedString,
    name: SharedString,
}

impl SearchableListItem for ModelSelectItem {
    type Value = SharedString;

    fn title(&self) -> SharedString {
        if self.name.is_empty() {
            self.id.clone()
        } else {
            self.name.clone()
        }
    }

    fn value(&self) -> &Self::Value {
        &self.id
    }
}

/// Delegate for the model picker. `SearchableVec` handles the kit search
/// (case-insensitive substring on `title()`) and the single-select strategy.
pub(crate) type ModelSelectDelegate = SearchableVec<ModelSelectItem>;

/// Build the model delegate from the ACP session's advertised models.
pub(crate) fn model_delegate_from(models: &[chronos_services::ModelInfo]) -> ModelSelectDelegate {
    SearchableVec::from(
        models
            .iter()
            .map(|m| ModelSelectItem {
                id: m.id.clone().into(),
                name: m.name.clone().into(),
            })
            .collect::<Vec<_>>(),
    )
}

pub(crate) fn model_delegate_empty() -> ModelSelectDelegate {
    SearchableVec::from(Vec::new())
}

#[derive(Clone)]
pub(crate) struct ModeSelectItem {
    id: SharedString,
    name: SharedString,
}

impl SearchableListItem for ModeSelectItem {
    type Value = SharedString;

    fn title(&self) -> SharedString {
        if self.name.is_empty() {
            self.id.clone()
        } else {
            self.name.clone()
        }
    }

    fn value(&self) -> &Self::Value {
        &self.id
    }
}

pub(crate) type ModeSelectDelegate = SearchableVec<ModeSelectItem>;

pub(crate) fn mode_delegate_from(modes: &[chronos_services::SessionMode]) -> ModeSelectDelegate {
    SearchableVec::from(
        modes
            .iter()
            .map(|m| ModeSelectItem {
                id: m.id.clone().into(),
                name: m.name.clone().into(),
            })
            .collect::<Vec<_>>(),
    )
}

pub(crate) fn mode_delegate_empty() -> ModeSelectDelegate {
    SearchableVec::from(Vec::new())
}

// ── Select delegate sync ─────────────────────────────────────────────────
// The kit `Select` state is owned by `ChatTab`; these helpers keep its
// delegate and selection in sync with `available_models`/`composer_selected_model`
// on every render. `set_items` runs every render — it is cheap, and no
// `ChatTab` notify path fires while a dropdown is open (typing/search is
// local to the kit's own list entity; hover only touches a global timer),
// so the kit's filtered view is never reset mid-open.
fn sync_model_items(panel: &ChatTab, window: &mut Window, cx: &mut Context<ChatTab>) {
    panel.composer_model_select.update(cx, |s, cx| {
        s.set_items(model_delegate_from(&panel.available_models), window, cx);
        let selected = panel.composer_selected_model.clone().into();
        sync_selection(s, &selected, window, cx);
    });
}

fn sync_mode_items(panel: &ChatTab, window: &mut Window, cx: &mut Context<ChatTab>) {
    panel.composer_mode_select.update(cx, |s, cx| {
        s.set_items(mode_delegate_from(&panel.available_modes), window, cx);
        let selected = panel.composer_selected_mode.clone().into();
        sync_selection(s, &selected, window, cx);
    });
}

/// Keep the kit trigger's title in sync with `composer_selected_*` without
/// disturbing the user's dropdown cursor (only re-points selection when the
/// committed value actually changed).
fn sync_selection<D>(
    s: &mut SelectState<D>,
    selected: &<D::Item as SearchableListItem>::Value,
    window: &mut Window,
    cx: &mut Context<SelectState<D>>,
) where
    D: SearchableListDelegate,
    <D::Item as SearchableListItem>::Value: PartialEq + Clone,
{
    if s.selected_value().map(|v| v != selected).unwrap_or(true) {
        s.set_selected_value(selected, window, cx);
    }
}

impl ChatTab {
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

    /// T287-A: model `Confirm` handler. Side effect is 1:1 with the old
    /// `model_picker` `on_click` — only the event source changed (kit
    /// `SelectEvent::Confirm` instead of a hand-rolled click).
    pub(crate) fn apply_model_select(
        &mut self,
        model_id: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.composer_selected_model = model_id.to_string();
        self.composer_input.update(cx, |s, cx| s.focus(window, cx));
        if let Some(client) = self.clients.get(&self.active_agent_id).cloned() {
            let model_id = model_id.to_string();
            cx.spawn(async move |this, cx| {
                if let Err(e) = client.set_model(&model_id).await {
                    tracing::warn!("set_model failed: {e}");
                }
                let _ = this.update(cx, |_this, cx| {
                    cx.notify();
                });
            })
            .detach();
        }
        cx.notify();
    }

    /// T287-A: mode `Confirm` — same 1:1 as the old `mode_picker` `on_click`
    /// (set selected mode, refocus composer, repaint). No ACP call — the old
    /// mode click never made one either.
    pub(crate) fn apply_mode_select(
        &mut self,
        mode_id: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.composer_selected_mode = mode_id.to_string();
        self.composer_input.update(cx, |s, cx| s.focus(window, cx));
        cx.notify();
    }
}

pub fn render_composer(
    panel: &ChatTab,
    window: &mut Window,
    cx: &mut Context<ChatTab>,
) -> impl IntoElement {
    let theme = *Theme::global(cx);
    let text: SharedString = panel.composer_input.read(cx).value();
    let has_text = !text.is_empty();

    let send_active = has_text && panel.state.agent_status != AgentStatus::Thinking;

    // ── YOLO state ──────────────────────────────────────────────────
    let yolo_mode_id = panel.composer_yolo_bypass_id.as_deref();
    let has_modes = !panel.available_modes.is_empty();
    let is_yolo_active = yolo_mode_id
        .map(|yid| panel.composer_selected_mode == *yid)
        .unwrap_or(false);

    let pickers_row = div()
        .id("composer-pickers-row")
        .flex_none()
        .flex()
        .items_center()
        .gap(px(6.))
        .child(model_picker(panel, window, cx))
        .child(mode_picker(panel, window, cx))
        .children(yolo_button(panel, is_yolo_active, has_modes, cx))
        .child(follow_button(panel, cx));

    // ── Textarea ────────────────────────────────────────────────────
    let enabled = panel.state.agent_status != AgentStatus::Disconnected;
    let focus = panel.composer_focus.clone();

    // T286: the composer field is the kit `Input` — wrap, caret, selection,
    // IME and text editing come from the component (auto_grow + soft_wrap
    // set on the InputState in `ChatTab::new`). The wrapper keeps the panel
    // keyboard hub (`composer_focus` + `handle_composer_key`): opening a
    // picker or starting a rename focuses the hub, closing refocuses the
    // Input, so the model-search / sidebar-search keys keep flowing.
    let text_input = div()
        .id("composer-input-canvas")
        .flex_1()
        .min_w(px(0.))
        .min_h(px(48.))
        .max_h(px(panel.state.height * 0.45))
        .text_size(px(12.5))
        .line_height(px(18.))
        .track_focus(&focus)
        .on_key_down(cx.listener(|this, event, window, cx| {
            this.handle_composer_key(event, window, cx);
        }))
        .on_drop(cx.listener(|this, paths: &ExternalPaths, window, cx| {
            let text = paths.paths().iter()
                .map(|p| p.display().to_string())
                .collect::<Vec<_>>()
                .join(" ");
            this.composer_input.update(cx, |s, cx| s.insert(text, window, cx));
            this.composer_drop_hover = false;
            cx.notify();
        }))
        .on_drag_move::<ExternalPaths>(cx.listener(|this, _event: &gpui::DragMoveEvent<ExternalPaths>, _, cx| {
            this.composer_drop_hover = true;
            cx.notify();
        }))
        .when(panel.composer_drop_hover, |el| {
            el.bg(theme.accent.primary.opacity(0.08))
        })
        .child(
            Input::new(&panel.composer_input)
                .appearance(false)
                .text_color(theme.text.primary)
                .text_size(px(12.5)),
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
fn attach_button(_panel: &ChatTab, _cx: &mut Context<ChatTab>) -> impl IntoElement {
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
    panel: &ChatTab,
    is_yolo_active: bool,
    has_modes: bool,
    cx: &mut Context<ChatTab>,
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

// ── Follow button ─────────────────────────────────────────────────────
// T287-C: Follow moved off the Zed-style thread header into the pickers
// row (T195 logic untouched — same `follow_enabled` / `AgentFollowState`).
fn follow_button(
    panel: &ChatTab,
    cx: &mut Context<ChatTab>,
) -> impl IntoElement {
    let theme = *Theme::global(cx);
    div()
        .id("composer-follow")
        .flex_none()
        .w(px(26.))
        .h(px(20.))
        .rounded(px(5.))
        .flex()
        .items_center()
        .justify_center()
        .when(panel.follow_enabled, |el| {
            el.bg(theme.accent.primary.opacity(0.16)).text_color(theme.accent.primary)
        })
        .when(!panel.follow_enabled, |el| {
            el.text_color(theme.text.muted)
        })
        .cursor_pointer()
        .hover(|s| s.bg(theme.border.subtle).text_color(theme.text.primary))
        .on_click(cx.listener(|this, _, _, cx| {
            this.follow_enabled = !this.follow_enabled;
            cx.update_global::<crate::agent_follow::AgentFollowState, _>(|state, _| {
                state.enabled = this.follow_enabled;
                if !this.follow_enabled {
                    state.last_tool = None;
                }
            });
            cx.notify();
        }))
        .child(img("icons/rail-preview.svg").w(px(16.)).h(px(16.)))
}

// ── Model picker ───────────────────────────────────────────────────────
// ── Model picker ───────────────────────────────────────────────────────
// T287-A: kit `Select`. The trigger is a 150 px pill; the dropdown is the
// kit's anchored search list (searchable). When `available_models` is empty
// the trigger is disabled and shows the "Model" placeholder — the ACP
// agent doesn't advertise models until a session response, so this is an
// intentional "loading" affordance, not a missing feature.
fn model_picker(
    panel: &ChatTab,
    window: &mut Window,
    cx: &mut Context<ChatTab>,
) -> impl IntoElement {
    sync_model_items(panel, window, cx);
    let has_data = !panel.available_models.is_empty();
    div()
        .id("composer-model-picker-wrap")
        .flex_none()
        .w(px(150.))
        .child(
            Select::new(&panel.composer_model_select)
                .placeholder("Model")
                .disabled(!has_data)
                .search_placeholder("Search models…")
                .with_size(KitSize::XSmall),
        )
}

// ── Mode picker ────────────────────────────────────────────────────────
// Same kit Select, non-searchable (the kit's `searchable(false)` is the
// cheap path — no need to hand-roll a second picker). Same "Model"
// placeholder-when-empty rule.
fn mode_picker(
    panel: &ChatTab,
    window: &mut Window,
    cx: &mut Context<ChatTab>,
) -> impl IntoElement {
    sync_mode_items(panel, window, cx);
    let has_data = !panel.available_modes.is_empty();
    div()
        .id("composer-mode-picker-wrap")
        .flex_none()
        .w(px(90.))
        .child(
            Select::new(&panel.composer_mode_select)
                .placeholder("Mode")
                .disabled(!has_data)
                .with_size(KitSize::XSmall),
        )
}

// ── Send / Stop button (dark style) ────────────────────────────────────
fn send_button(
    panel: &ChatTab,
    active: bool,
    cx: &mut Context<ChatTab>,
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
impl ChatTab {
    pub(crate) fn handle_composer_key(
        &mut self,
        event: &gpui::KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // T286: the composer text editing itself is owned by the kit `Input`
        // (its own focus handle). This handler is the PANEL keyboard hub
        // (`composer_focus`): it only routes keys for the sidebar search /
        // rename field, which borrow the hub while active. Closing any of
        // those returns focus to the Input.
        // T287-A: the model/mode picker keyboard nav and search moved into
        // the kit `Select` (its own key_context + list focus handle) — this
        // hub no longer has a branch for them.
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
                    self.composer_input.update(cx, |s, cx| s.focus(window, cx));
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
                    self.composer_input.update(cx, |s, cx| s.focus(window, cx));
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
    }

    pub(crate) fn send_composer(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let text = self.composer_input.read(cx).value().trim().to_string();
        if text.is_empty() {
            return;
        }
        self.composer_input.update(cx, |s, cx| s.set_value("", window, cx));

        // The client is only ever absent during the initial connect (first
        // HermesClient::new + create_session can take seconds). `agent_status`
        // is Thinking then AND mid-turn, so gate on the client map, not the
        // status: a message sent while the previous turn is still streaming
        // must NOT be queued — `pending_send` drains only on connect, so it
        // would sit forever with a false "отправлено автоматически" promise.
        let connecting = !self.clients.contains_key(&self.active_agent_id)
            && self.state.agent_status == AgentStatus::Thinking;
        let mid_turn = self.state.agent_status == AgentStatus::Thinking && !connecting;

        if connecting {
            // T247: while the client connects, don't drop the message silently
            // (the old `return` left the thread at "No messages yet" with the
            // text stranded in the composer — the exact audit symptom: the IPC
            // landed 4s before the client connected). Push the user message
            // now, show an honest note, and defer the ACP turn until the
            // connect handler drains `pending_send`.
            self.push_user_message(&text, cx);
            self.chat.push_message(ChatMessage {
                role: MessageRole::Agent,
                segments: vec![Segment::Response {
                    content: "Агент ещё подключается — сообщение будет отправлено автоматически."
                        .to_string(),
                }],
            });
            self.chat.scroll_to_bottom();
            self.pending_send = Some(text);
            cx.notify();
            return;
        }

        if mid_turn {
            // Agent is busy with an earlier prompt. Honest note — the message
            // is shown so nothing is silently lost, but it is not auto-queued
            // (a second ACP turn mid-stream would corrupt the active turn).
            self.push_user_message(&text, cx);
            self.chat.push_message(ChatMessage {
                role: MessageRole::Agent,
                segments: vec![Segment::Response {
                    content: "Агент ещё отвечает — дождись окончания хода и отправь сообщение снова."
                        .to_string(),
                }],
            });
            self.chat.scroll_to_bottom();
            cx.notify();
            return;
        }

        self.push_user_message(&text, cx);
        self.start_acp_turn(text, cx);
        cx.notify();
    }

    /// Push a user message to the thread + auto-title on the first one (T151).
    /// Shared by direct sends and the deferred-until-connect path so a queued
    /// message is never double-pushed when the client finally connects.
    fn push_user_message(&mut self, text: &str, cx: &mut Context<Self>) {
        let is_first_user_message = !self.chat.messages.iter().any(|m| m.role == MessageRole::User);
        if is_first_user_message {
            if let Some(thread_id) = self.state.active_session_id.clone() {
                self.set_auto_title(&thread_id, text, cx);
            }
        }
        self.chat.push_message(ChatMessage {
            role: MessageRole::User,
            segments: vec![Segment::Response { content: text.to_string() }],
        });
        self.chat.scroll_to_bottom();
    }

    /// Run the ACP turn for `text` (the user message is already pushed by
    /// the caller via [`Self::push_user_message`]). Called directly from
    /// `send_composer`, or — after the client connects — for a message that
    /// was queued while it was still connecting (T247). `pub(crate)` for the
    /// ChatTab::new connect handler in `mod.rs`.
    pub(crate) fn start_acp_turn(&mut self, text: String, cx: &mut Context<Self>) {
        tracing::info!(
            "composer: send model={} mode={} text={:?}",
            self.composer_selected_model,
            self.composer_selected_mode,
            text
        );

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
                                            // T195: push tool call to Follow state
                                            if this.follow_enabled {
                                                let ft = crate::agent_follow::ToolCallPreview {
                                                    id: id.clone(),
                                                    name: name.clone(),
                                                    status: status.clone(),
                                                    args: args.clone(),
                                                    result: result.clone(),
                                                };
                                                cx.update_global::<crate::agent_follow::AgentFollowState, _>(|state, _| {
                                                    state.push_tool(ft.clone());
                                                });
                                                if let Some(path_str) = crate::agent_follow::AgentFollowState::extract_file_path(&ft) {
                                                    cx.set_global(crate::side_panel_right::preview_target::PreviewTarget {
                                                        path: Some(std::path::PathBuf::from(path_str)),
                                                        generation: 1,
                                                        intent: crate::side_panel_right::preview_target::PreviewIntent::View,
                                                    });
                                                }
                                            }
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
