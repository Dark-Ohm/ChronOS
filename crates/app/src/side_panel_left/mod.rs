mod chat_view;
mod composer;
mod hover_strip;
mod panel;
pub mod sessions_list;
mod state;
pub mod text_input;
mod tool_card;

/// Detects RTL base direction by the first strong (directional) character.
pub fn is_rtl_text(text: &str) -> bool {
    for ch in text.chars() {
        match ch {
            '\u{05D0}'..='\u{05EA}' => return true,
            '\u{0600}'..='\u{06FF}' | '\u{0750}'..='\u{077F}' | '\u{08A0}'..='\u{08FF}' => return true,
            'A'..='Z' | 'a'..='z' => return false,
            _ => {}
        }
    }
    false
}

pub use state::{PanelState, SidePanelLeftState};

use chronos_luau::bar::BAR_HEIGHT;
use chronos_services::hermes_acp::{
    AgentDescriptor, HermesClient, StreamingEvent, known_agents, load_shared_env,
};
use chronos_services::threads::{ThreadRecord, ThreadStore};
use chronos_services::{ModelInfo, SessionMode};
use gpui::{
    App, Bounds, DisplayId, Focusable, Global, Size, UTF16Selection, Window,
    WindowBackgroundAppearance, WindowBounds, WindowHandle, WindowKind, WindowOptions,
    layer_shell::*, point, prelude::*, px,
};
use std::collections::HashMap;
use std::ops::Range;

pub struct LeftPanelResize;

const PANEL_EDGE_GAP: f32 = BAR_HEIGHT;

#[derive(Default)]
pub struct SidePanelLeftState_ {
    handle: Option<WindowHandle<SidePanelLeft>>,
    pinned: bool,
    peek_generation: u64,
}

impl Global for SidePanelLeftState_ {}

fn display_height(display_id: Option<DisplayId>, cx: &App) -> f32 {
    display_id
        .and_then(|id| cx.find_display(id))
        .or_else(|| cx.primary_display())
        .map(|d| f32::from(d.bounds().size.height))
        .unwrap_or(1080.)
}

fn window_options(display_id: Option<DisplayId>, cx: &App) -> WindowOptions {
    let display_h = display_height(display_id, cx);
    let panel_h = (display_h - PANEL_EDGE_GAP).max(100.);
    // Super+A opens wide enough for chat column (not rail-only strip).
    let open_w = state::SidePanelLeftState::DEFAULT_CHAT_WIDTH;
    WindowOptions {
        display_id,
        titlebar: None,
        window_bounds: Some(WindowBounds::Windowed(Bounds {
            origin: point(px(0.), px(0.)),
            size: Size::new(px(open_w), px(panel_h)),
        })),
        app_id: Some("chronos-side-panel-left".to_string()),
        window_background: WindowBackgroundAppearance::Transparent,
        kind: WindowKind::LayerShell(LayerShellOptions {
            namespace: "side_panel_left".to_string(),
            layer: Layer::Overlay,
            anchor: Anchor::LEFT | Anchor::TOP,
            // Bar-only exclusive = sidebar + handle (full open strip). Chat
            // overlay still uses exclusive_px() which is the same strip until
            // dock. exclusive_edge LEFT required on LEFT|TOP corner anchor.
            exclusive_zone: Some(px(sessions_list::SIDEBAR_MIN_WIDTH)),
            exclusive_edge: Some(Anchor::LEFT),
            margin: None,
            keyboard_interactivity: KeyboardInteractivity::OnDemand,
            ..Default::default()
        }),
        ..Default::default()
    }
}

pub struct SidePanelLeft {
    state: state::SidePanelLeftState,
    /// Available agent backends from the registry.
    agents: Vec<AgentDescriptor>,
    /// Shared env vars from ~/.config/chronos/.env (passed to agent spawns).
    shared_env: HashMap<String, String>,
    /// Lazy-spawned clients keyed by agent id.
    clients: HashMap<String, HermesClient>,
    /// Which agent backend is currently active.
    active_agent_id: String,
    /// Whether the agent switcher dropdown is open.
    agent_menu_open: bool,
    sessions: Vec<sessions_list::ThreadListItem>,
    /// SQLite-backed thread store (T150). Loaded on startup, mutated by actions.
    thread_store: Option<ThreadStore>,
    /// Search query for filtering the thread list.
    thread_search: String,
    /// Show archived threads (hidden by default).
    show_archived: bool,
    /// True while replaying a session via load_session.
    thread_loading: bool,
    /// ID of the thread whose context menu is open.
    thread_menu_open: Option<String>,
    /// Whether the sidebar search field is focused (routes keyboard input).
    search_focused: bool,
    /// When set, shows an inline rename input for this thread.
    rename_thread_id: Option<String>,
    /// Current text in the inline rename input.
    rename_input: String,
    /// Available modes from the active ACP session.
    available_modes: Vec<chronos_services::SessionMode>,
    /// Available models from the active ACP session.
    available_models: Vec<chronos_services::ModelInfo>,
    pub(crate) chat: chat_view::ChatView,
    pub(crate) composer_focus: gpui::FocusHandle,
    pub(crate) composer_input: text_input::TextInputState,
    /// Shaped line from the last TextInputElement prepaint; needed by
    /// EntityInputHandler::bounds_for_range / character_index_for_point
    /// for IME candidate window positioning.
    pub(crate) composer_last_layout: Option<gpui::ShapedLine>,
    pub(crate) composer_last_bounds: Option<gpui::Bounds<gpui::Pixels>>,
    pub(crate) composer_selected_model: String,
    pub(crate) composer_selected_mode: String,
    /// The mode ID that was active before YOLO toggle, to restore on toggle-off.
    pub(crate) composer_previous_mode: String,
    /// Cached ID of the bypass/YOLO mode found in available_modes, if any.
    pub(crate) composer_yolo_bypass_id: Option<String>,
    pub(crate) composer_model_dropdown_open: bool,
    pub(crate) composer_mode_dropdown_open: bool,
    pub(crate) composer_model_search: String,
    pub(crate) composer_focused: bool,
    pub(crate) composer_last_click: Option<(std::time::Instant, gpui::Point<gpui::Pixels>)>,
    pub(crate) composer_blink_task: Option<gpui::Task<()>>,
    /// Streaming state for the current ACP prompt turn.
    pub(crate) streaming: state::StreamingState,
    resize_start_x: Option<f32>,
    resize_start_width: Option<f32>,
    /// Width the platform window was last physically resized to. `render`
    /// only issues `window.resize()` when `state.width` has drifted from
    /// this, so a fast drag (many `DragMoveEvent`s between paints) collapses
    /// to at most one Wayland `set_size` protocol round-trip per frame
    /// instead of one per raw pointer-motion event.
    last_resized_width: Option<f32>,
}

impl Render for SidePanelLeft {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Exclusive zone: sidebar-only when dock off, full width when dock on.
        // Must call set_exclusive_edge(LEFT) or Hyprland silently ignores the
        // zone on our LEFT|TOP corner anchor (DECISIONS 2026-07-23).
        let new_zone = self.state.exclusive_px();
        if self.state.last_exclusive_zone != Some(new_zone) {
            window.set_exclusive_edge(gpui::layer_shell::Anchor::LEFT);
            window.set_exclusive_zone(px(new_zone));
            self.state.last_exclusive_zone = Some(new_zone);
        }

        if self.last_resized_width != Some(self.state.width) {
            let display_id = crate::monitor::pult_display(cx);
            let display_h = display_height(display_id, cx);
            let panel_h = (display_h - PANEL_EDGE_GAP).max(100.);
            self.state.height = panel_h;
            window.resize(Size::new(px(self.state.width), px(panel_h)));
            self.last_resized_width = Some(self.state.width);
        }
        panel::render_panel(self, window, cx)
    }
}

impl Focusable for SidePanelLeft {
    fn focus_handle(&self, _cx: &gpui::App) -> gpui::FocusHandle {
        self.composer_focus.clone()
    }
}

impl gpui::EntityInputHandler for SidePanelLeft {
    fn text_for_range(
        &mut self,
        range_utf16: Range<usize>,
        actual_range: &mut Option<Range<usize>>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<String> {
        let range = self.composer_input.range_from_utf16(&range_utf16);
        actual_range.replace(self.composer_input.range_to_utf16(&range));
        Some(self.composer_input.content[range].to_string())
    }

    fn selected_text_range(
        &mut self,
        _ignore_disabled_input: bool,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        Some(UTF16Selection {
            range: self.composer_input.range_to_utf16(&self.composer_input.selected_range),
            reversed: self.composer_input.selection_reversed,
        })
    }

    fn marked_text_range(
        &self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Range<usize>> {
        self.composer_input
            .marked_range
            .as_ref()
            .map(|range| self.composer_input.range_to_utf16(range))
    }

    fn unmark_text(&mut self, _window: &mut Window, _cx: &mut Context<Self>) {
        self.composer_input.marked_range = None;
    }

    fn replace_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        new_text: &str,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let range = range_utf16
            .as_ref()
            .map(|r| self.composer_input.range_from_utf16(r))
            .or(self.composer_input.marked_range.clone())
            .unwrap_or(self.composer_input.selected_range.clone());
        self.composer_input.replace_range(range, new_text);
        cx.notify();
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        new_text: &str,
        new_selected_range_utf16: Option<Range<usize>>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let range = range_utf16
            .as_ref()
            .map(|r| self.composer_input.range_from_utf16(r))
            .or(self.composer_input.marked_range.clone())
            .unwrap_or(self.composer_input.selected_range.clone());
        self.composer_input.content =
            (self.composer_input.content[..range.start].to_owned()
                + new_text
                + &self.composer_input.content[range.end..])
                .into();
        if !new_text.is_empty() {
            self.composer_input.marked_range = Some(range.start..range.start + new_text.len());
        } else {
            self.composer_input.marked_range = None;
        }
        let new_range = new_selected_range_utf16
            .as_ref()
            .map(|r| self.composer_input.range_from_utf16(r))
            .map(|r| r.start + range.start..r.end + range.end)
            .unwrap_or(range.start + new_text.len()..range.start + new_text.len());
        self.composer_input.selected_range = new_range;
        cx.notify();
    }

    fn bounds_for_range(
        &mut self,
        range_utf16: Range<usize>,
        bounds: Bounds<gpui::Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Bounds<gpui::Pixels>> {
        let layout = self.composer_last_layout.as_ref()?;
        let range = self.composer_input.range_from_utf16(&range_utf16);
        Some(Bounds::from_corners(
            gpui::point(
                bounds.left() + layout.x_for_index(range.start),
                bounds.top(),
            ),
            gpui::point(
                bounds.left() + layout.x_for_index(range.end),
                bounds.bottom(),
            ),
        ))
    }

    fn character_index_for_point(
        &mut self,
        point: gpui::Point<gpui::Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<usize> {
        let bounds = self.composer_last_bounds.as_ref()?;
        let line_point = bounds.localize(&point)?;
        let layout = self.composer_last_layout.as_ref()?;
        let utf8_index = layout.index_for_x(point.x - line_point.x)?;
        Some(self.composer_input.offset_to_utf16(utf8_index))
    }
}

impl SidePanelLeft {
    fn new(cx: &mut Context<Self>) -> Self {
        let agents = known_agents();
        let shared_env = chronos_services::hermes_acp::load_shared_env();
        let active_agent_id = agents.first().map(|a| a.id.to_string()).unwrap_or_default();

        // Open the thread store (T150). Falls back to None if the DB can't
        // be opened — the panel still works with in-memory sessions.
        let thread_store = ThreadStore::open_default().ok();
        let mut threads = thread_store
            .as_ref()
            .map(|store| {
                store
                    .list(None, false, false)
                    .unwrap_or_default()
                    .into_iter()
                    .map(|record| sessions_list::ThreadListItem {
                        record,
                        active: false,
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        threads.sort_by(|a, b| {
            match (a.record.pinned, b.record.pinned) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => b.record.updated_at.cmp(&a.record.updated_at),
            }
        });

        // Lazy-spawn the default agent (first in registry).
        let default_config = agents.first().map(|a| a.config.clone()).unwrap_or_default();
        let agent_id = active_agent_id.clone();
        let env_for_spawn = shared_env.clone();
        let mut state = state::SidePanelLeftState::new();
        // Connecting until HermesClient::new + create_session complete.
        state.agent_status = state::AgentStatus::Thinking;

        cx.spawn(async move |this, cx| {
            match HermesClient::new(default_config, env_for_spawn).await {
                Ok(client) => {
                    // Fetch modes/models at connect time, not just after the
                    // first prompt — otherwise the model/mode pickers stay
                    // hidden for the entire life of a thread nobody has
                    // messaged yet (live smoke, 2026-07-23: composer showed
                    // only attach/send, no indicators, on a fresh thread).
                    let session = client.create_session().await;
                    let _ = this.update(cx, |this, _cx| {
                        this.clients.insert(agent_id, client);
                        this.state.agent_status = state::AgentStatus::Connected;
                        tracing::info!("side_panel_left: ACP client connected");
                        if let Ok(session) = session {
                            this.state.session_id = Some(session.id.to_string());
                            if let Some(modes) = session.modes {
                                this.composer_selected_mode = modes.current_id;
                                this.available_modes = modes.available;
                                this.detect_yolo_bypass_mode();
                            }
                            if let Some(models) = session.models {
                                this.composer_selected_model = models.current_id;
                                this.available_models = models.available;
                            }
                        } else if let Err(e) = session {
                            tracing::warn!(
                                "side_panel_left: create_session after connect failed: {e}"
                            );
                        }
                    });
                }
                Err(e) => {
                    tracing::warn!("side_panel_left: ACP client init failed: {e}");
                    let _ = this.update(cx, |this, _cx| {
                        this.state.agent_status = state::AgentStatus::Disconnected;
                    });
                }
            }
        })
        .detach();

        Self {
            state,
            agents,
            shared_env,
            clients: HashMap::new(),
            active_agent_id,
            agent_menu_open: false,
            sessions: threads,
            thread_store,
            thread_search: String::new(),
            show_archived: false,
            thread_loading: false,
            thread_menu_open: None,
            search_focused: false,
            rename_thread_id: None,
            rename_input: String::new(),
            available_modes: Vec::new(),
            available_models: Vec::new(),
            chat: chat_view::ChatView::new(),
            composer_focus: cx.focus_handle(),
            composer_last_click: None,
            composer_blink_task: None,
            composer_input: text_input::TextInputState::new(),
            composer_last_layout: None,
            composer_last_bounds: None,
            composer_selected_model: String::new(),
            composer_selected_mode: String::new(),
            composer_previous_mode: String::new(),
            composer_yolo_bypass_id: None,
            composer_model_dropdown_open: false,
            composer_mode_dropdown_open: false,
            composer_model_search: String::new(),
            composer_focused: false,
            streaming: state::StreamingState::new(),
            resize_start_x: None,
            resize_start_width: None,
            last_resized_width: None,
        }
    }

    fn toggle_collapse(&mut self, cx: &mut Context<Self>) {
        self.state.sessions_collapsed = !self.state.sessions_collapsed;
        self.state.recalc_min_width();
        cx.notify();
    }

    /// Sort sessions: pinned first, then by `updated_at` descending.
    /// The store's `list()` orders by `updated_at DESC` only; pinned-first
    /// ordering is applied here in the frontend (service layer is T150).
    fn sort_sessions(&mut self) {
        self.sessions.sort_by(|a, b| {
            match (a.record.pinned, b.record.pinned) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => b.record.updated_at.cmp(&a.record.updated_at),
            }
        });
    }

    fn create_new_session(&mut self, cx: &mut Context<Self>) {
        let id = uuid::Uuid::new_v4().to_string();
        let cwd = std::env::current_dir().unwrap_or_default().to_string_lossy().to_string();

        // Insert into the thread store (T150). If the store is unavailable,
        // fall back to an in-memory-only session.
        if let Some(store) = &self.thread_store {
            if let Ok(record) = store.insert(&id, &self.active_agent_id, &cwd) {
                self.sessions.push(sessions_list::ThreadListItem {
                    record,
                    active: true,
                });
            } else {
                tracing::warn!("create_new_session: store insert failed for {id}");
            }
        } else {
            self.sessions.push(sessions_list::ThreadListItem {
                record: ThreadRecord {
                    id: id.clone(),
                    agent_id: self.active_agent_id.clone(),
                    acp_session_id: None,
                    title: String::new(),
                    title_override: None,
                    cwd,
                    last_model: None,
                    pinned: false,
                    archived: false,
                    created_at: chrono::Utc::now().to_rfc3339(),
                    updated_at: chrono::Utc::now().to_rfc3339(),
                    transcript_json: None,
                },
                active: true,
            });
        }
        // Deactivate previous active sessions
        for s in self.sessions.iter_mut().rev().skip(1) {
            s.active = false;
        }
        self.sort_sessions();
        self.state.active_session_id = Some(id);
        // Clear local transcript; mint a fresh ACP session on the agent.
        self.chat = chat_view::ChatView::new();
        self.streaming.reset();
        self.state.session_id = None;
        if let Some(client) = self.clients.get(&self.active_agent_id).cloned() {
            self.state.agent_status = state::AgentStatus::Thinking;
            cx.spawn(async move |this, cx| match client.create_session().await {
                Ok(session) => {
                    let _ = this.update(cx, |this, cx| {
                        this.state.session_id = Some(session.id.to_string());
                        this.state.agent_status = state::AgentStatus::Connected;
                        // Update the thread's acp_session_id in the store.
                        if let Some(store) = &this.thread_store {
                            let _ = store.update(&this.state.active_session_id.clone().unwrap_or_default(), Some(&session.id.to_string()), None, None, None);
                        }
                        // Update the in-memory thread record.
                        for t in &mut this.sessions {
                            if t.active {
                                t.record.acp_session_id = Some(session.id.to_string());
                            }
                        }
                        if let Some(modes) = session.modes {
                            this.composer_selected_mode = modes.current_id;
                            this.available_modes = modes.available;
                            this.detect_yolo_bypass_mode();
                        }
                        if let Some(models) = session.models {
                            this.composer_selected_model = models.current_id;
                            this.available_models = models.available;
                        }
                        cx.notify();
                    });
                }
                Err(e) => {
                    tracing::warn!("side_panel_left: new ACP session failed: {e}");
                    let _ = this.update(cx, |this, cx| {
                        this.state.agent_status = state::AgentStatus::Disconnected;
                        this.chat.push_message(chat_view::ChatMessage {
                            role: chat_view::MessageRole::Agent,
                            segments: vec![chat_view::Segment::Response {
                                content: format!("Error: failed to create session: {e}"),
                            }],
                        });
                        cx.notify();
                    });
                }
            })
            .detach();
        }
        cx.notify();
    }

    fn select_session(&mut self, session_id: &str, cx: &mut Context<Self>) {
        // Cache current transcript before switching away.
        self.cache_transcript(cx);

        // Find the thread and its acp_session_id + cwd.
        let thread = self.sessions.iter().find(|s| s.record.id == session_id);
        let acp_session_id = thread.and_then(|t| t.record.acp_session_id.clone());
        let cwd = thread.map(|t| t.record.cwd.clone()).unwrap_or_default();

        // Mark active.
        for s in &mut self.sessions {
            s.active = s.record.id == session_id;
        }
        self.state.active_session_id = Some(session_id.to_string());
        self.thread_menu_open = None;

        // Clear chat and show loading state.
        self.chat = chat_view::ChatView::new();
        self.streaming.reset();
        self.state.session_id = None;
        self.thread_loading = true;

        // Show cached transcript immediately if available.
        if let Some(store) = &self.thread_store {
            if let Ok(Some(json)) = store.transcript(session_id) {
                if let Ok(messages) = serde_json::from_str::<Vec<chat_view::ChatMessage>>(&json) {
                    self.chat.messages = messages;
                    self.chat.scroll_to_bottom();
                }
            }
        }

        if let Some(client) = self.clients.get(&self.active_agent_id).cloned() {
            if let (Some(acp_id), true) = (&acp_session_id, !cwd.is_empty()) {
                let cwd_path = std::path::PathBuf::from(&cwd);
                self.state.agent_status = state::AgentStatus::Thinking;
                cx.notify();

                // Create streaming channel for replay events.
                let (event_tx, event_rx) = tokio::sync::mpsc::unbounded_channel();

                // Push a placeholder agent message (filled by replay events).
                self.chat.push_message(chat_view::ChatMessage {
                    role: chat_view::MessageRole::Agent,
                    segments: Vec::new(),
                });
                self.chat.scroll_to_bottom();

                // Spawn ACP load_session task.
                let acp_id_owned = acp_id.clone();
                let load_task = cx.spawn(async move |this, cx| {
                    match client.load_session(&acp_id_owned, &cwd_path, event_tx).await {
                        Ok(()) => {
                            tracing::info!("select_session: load_session replay complete for {acp_id_owned}");
                        }
                        Err(e) => {
                            tracing::warn!("select_session: load_session failed: {e}");
                            let _ = this.update(cx, |this, cx| {
                                if let Some(last_msg) = this.chat.messages.last_mut() {
                                    if last_msg.role == chat_view::MessageRole::Agent {
                                        last_msg.segments.push(chat_view::Segment::Response {
                                            content: format!("Error loading session: {e}"),
                                        });
                                    }
                                }
                                cx.notify();
                            });
                        }
                    }
                });

                // Spawn GPUI task to consume replay events (same handler as
                // composer streaming — see composer.rs for the full pattern).
                let streaming_task = cx.spawn(async move |this, cx| {
                    use std::time::Duration;
                    let mut rx = event_rx;
                    const TURN_TIMEOUT: Duration = Duration::from_secs(180);
                    let mut last_event = cx.background_executor().now();
                    let mut timer = cx.background_executor().timer(TURN_TIMEOUT);

                    loop {
                        tokio::select! {
                            event = rx.recv() => match event {
                                Some(event) => {
                                    last_event = cx.background_executor().now();
                                    let upd = this.update(cx, |this, cx| {
                                        match event {
                                            StreamingEvent::TextChunk(delta) => {
                                                if let Some(last_msg) = this.chat.messages.last_mut() {
                                                    if last_msg.role == chat_view::MessageRole::Agent {
                                                        let append = last_msg.segments.last_mut().and_then(|s| {
                                                            if let chat_view::Segment::Response { content } = s { Some(content) } else { None }
                                                        });
                                                        if let Some(content) = append {
                                                            content.push_str(&delta);
                                                        } else {
                                                            last_msg.segments.push(chat_view::Segment::Response { content: delta });
                                                        }
                                                    }
                                                }
                                                this.chat.scroll_to_bottom();
                                            }
                                            StreamingEvent::ThoughtChunk(delta) => {
                                                if let Some(last_msg) = this.chat.messages.last_mut() {
                                                    if last_msg.role == chat_view::MessageRole::Agent {
                                                        let append = last_msg.segments.last_mut().and_then(|s| {
                                                            if let chat_view::Segment::Thinking { content } = s { Some(content) } else { None }
                                                        });
                                                        if let Some(content) = append {
                                                            content.push_str(&delta);
                                                        } else {
                                                            let seg_idx = last_msg.segments.len();
                                                            last_msg.segments.push(chat_view::Segment::Thinking { content: delta });
                                                            let msg_idx = this.chat.messages.len().wrapping_sub(1);
                                                            this.chat.collapsed_reasoning.remove(&(msg_idx, seg_idx));
                                                        }
                                                    }
                                                }
                                                this.chat.scroll_to_bottom();
                                            }
                                            StreamingEvent::ToolCall { id, name, status, args, result } => {
                                                if let Some(last_msg) = this.chat.messages.last_mut() {
                                                    if last_msg.role == chat_view::MessageRole::Agent {
                                                        let found = last_msg.segments.iter_mut().rev().find_map(|s| {
                                                            if let chat_view::Segment::ToolCall { tool } = s {
                                                                if tool.id == id { Some(tool) } else { None }
                                                            } else { None }
                                                        });
                                                        if let Some(tool) = found {
                                                            tool.status = status;
                                                            tool.args = args;
                                                            tool.result = result;
                                                        } else {
                                                            last_msg.segments.push(chat_view::Segment::ToolCall {
                                                                tool: chat_view::ToolCallPreview { id, name, status, args, result },
                                                            });
                                                        }
                                                    }
                                                }
                                            }
                                            StreamingEvent::Done => {}
                                            StreamingEvent::Error(_) => {}
                                        }
                                        cx.notify();
                                    });
                                    if upd.is_err() { return; }
                                }
                                None => break,
                            },
                            _ = &mut timer => {
                                let silent = last_event.elapsed();
                                if silent >= TURN_TIMEOUT {
                                    break;
                                } else {
                                    timer = cx.background_executor().timer(TURN_TIMEOUT - silent);
                                    continue;
                                }
                            }
                        }
                    }
                });

                self.streaming.active = true;
                self.streaming.acp_task = Some(load_task);
                self.streaming.receiver_task = Some(streaming_task);
            } else if acp_session_id.is_none() {
                // New thread with no ACP session yet — show empty state.
                self.chat.push_message(chat_view::ChatMessage {
                    role: chat_view::MessageRole::Agent,
                    segments: vec![chat_view::Segment::Response {
                        content: "Новый тред. Напиши сообщение, чтобы начать.".to_string(),
                    }],
                });
                self.chat.scroll_to_bottom();
                self.thread_loading = false;
            } else {
                self.thread_loading = false;
            }
        } else {
            self.thread_loading = false;
        }
        cx.notify();
    }

    /// Rename a thread — writes `title_override` to the store.
    fn rename_thread(&mut self, thread_id: &str, new_title: &str, cx: &mut Context<Self>) {
        if let Some(store) = &self.thread_store {
            if let Err(e) = store.update(thread_id, None, Some(new_title), None, None) {
                tracing::warn!("rename_thread: store update failed: {e}");
            }
        }
        for t in &mut self.sessions {
            if t.record.id == thread_id {
                t.record.title_override = Some(new_title.to_string());
                t.record.title = new_title.to_string();
            }
        }
        cx.notify();
    }

    /// Toggle pinned state.
    fn toggle_pin(&mut self, thread_id: &str, cx: &mut Context<Self>) {
        if let Some(store) = &self.thread_store {
            let pinned = self
                .sessions
                .iter()
                .find(|t| t.record.id == thread_id)
                .map(|t| !t.record.pinned)
                .unwrap_or(false);
            if let Err(e) = store.set_pinned(thread_id, pinned) {
                tracing::warn!("toggle_pin: store update failed: {e}");
            }
            for t in &mut self.sessions {
                if t.record.id == thread_id {
                    t.record.pinned = pinned;
                }
            }
        }
        self.sort_sessions();
        cx.notify();
    }

    /// Toggle archived state.
    fn toggle_archive(&mut self, thread_id: &str, cx: &mut Context<Self>) {
        if let Some(store) = &self.thread_store {
            let archived = self
                .sessions
                .iter()
                .find(|t| t.record.id == thread_id)
                .map(|t| !t.record.archived)
                .unwrap_or(false);
            if let Err(e) = store.set_archived(thread_id, archived) {
                tracing::warn!("toggle_archive: store update failed: {e}");
            }
            for t in &mut self.sessions {
                if t.record.id == thread_id {
                    t.record.archived = archived;
                }
            }
            // If we just archived the active thread, close it.
            if archived && self.state.active_session_id.as_deref() == Some(thread_id) {
                self.chat = chat_view::ChatView::new();
                self.state.active_session_id = None;
                self.state.session_id = None;
            }
        }
        self.sort_sessions();
        cx.notify();
    }

    /// Delete a thread from the store and the list.
    fn delete_thread(&mut self, thread_id: &str, cx: &mut Context<Self>) {
        if let Some(store) = &self.thread_store {
            if let Err(e) = store.delete(thread_id) {
                tracing::warn!("delete_thread: store delete failed: {e}");
            }
        }
        self.sessions.retain(|t| t.record.id != thread_id);
        if self.state.active_session_id.as_deref() == Some(thread_id) {
            self.chat = chat_view::ChatView::new();
            self.state.active_session_id = None;
            self.state.session_id = None;
        }
        cx.notify();
    }

    /// Search threads by title and content.
    fn search_threads(&mut self, query: &str, cx: &mut Context<Self>) {
        self.thread_search = query.to_string();
        if query.is_empty() {
            // Restore full list from store.
            if let Some(store) = &self.thread_store {
                if let Ok(records) = store.list(None, false, self.show_archived) {
                    self.sessions = records
                        .into_iter()
                        .map(|record| {
                            let active = self
                                .state
                                .active_session_id
                                .as_deref()
                                == Some(record.id.as_str());
                            sessions_list::ThreadListItem { record, active }
                        })
                        .collect();
                }
            }
        } else if let Some(store) = &self.thread_store {
            if let Ok(records) = store.search(query) {
                self.sessions = records
                    .into_iter()
                    .map(|record| {
                        let active = self
                            .state
                            .active_session_id
                            .as_deref()
                            == Some(record.id.as_str());
                        sessions_list::ThreadListItem { record, active }
                    })
                    .collect();
            }
        }
        self.sort_sessions();
        cx.notify();
    }

    /// Toggle showing archived threads.
    fn toggle_archived(&mut self, cx: &mut Context<Self>) {
        self.show_archived = !self.show_archived;
        // Re-fetch from store with new filter.
        if let Some(store) = &self.thread_store {
            if let Ok(records) = store.list(None, false, self.show_archived) {
                self.sessions = records
                    .into_iter()
                    .map(|record| {
                        let active = self
                            .state
                            .active_session_id
                            .as_deref()
                            == Some(record.id.as_str());
                        sessions_list::ThreadListItem { record, active }
                    })
                    .collect();
            }
        }
        self.sort_sessions();
        cx.notify();
    }

    /// Open the context menu for a thread.
    fn open_thread_menu(&mut self, thread_id: &str, cx: &mut Context<Self>) {
        self.thread_menu_open = Some(thread_id.to_string());
        cx.notify();
    }

    /// Close the context menu.
    fn close_thread_menu(&mut self, cx: &mut Context<Self>) {
        self.thread_menu_open = None;
        cx.notify();
    }

    /// Begin inline rename of a thread.
    fn begin_rename(&mut self, thread_id: &str, current_title: &str, cx: &mut Context<Self>) {
        self.rename_thread_id = Some(thread_id.to_string());
        self.rename_input = current_title.to_string();
        self.search_focused = true;
        self.thread_menu_open = None;
        cx.notify();
    }

    /// Commit the inline rename.
    fn commit_rename(&mut self, cx: &mut Context<Self>) {
        if let Some(thread_id) = self.rename_thread_id.take() {
            let new_title = self.rename_input.trim().to_string();
            if !new_title.is_empty() {
                self.rename_thread(&thread_id, &new_title, cx);
            }
        }
        self.rename_input.clear();
        self.search_focused = false;
        cx.notify();
    }

    /// Cancel the inline rename.
    fn cancel_rename(&mut self, cx: &mut Context<Self>) {
        self.rename_thread_id = None;
        self.rename_input.clear();
        self.search_focused = false;
        cx.notify();
    }

    /// Focus the sidebar search field.
    fn start_search(&mut self, cx: &mut Context<Self>) {
        self.search_focused = true;
        self.thread_menu_open = None;
        self.rename_thread_id = None;
        cx.notify();
    }

    /// Cache the current chat transcript to the store.
    fn cache_transcript(&mut self, cx: &mut Context<Self>) {
        if let Some(thread_id) = &self.state.active_session_id {
            if let Some(store) = &self.thread_store {
                if let Ok(json) = serde_json::to_string(&self.chat.messages) {
                    if let Err(e) = store.cache_transcript(thread_id, &json) {
                        tracing::warn!("cache_transcript: store update failed: {e}");
                    }
                }
            }
        }
        cx.notify();
    }

    /// Set auto-title from the first user message.
    fn set_auto_title(&mut self, thread_id: &str, first_prompt: &str, cx: &mut Context<Self>) {
        let title = sessions_list::auto_title_from_text(first_prompt);
        if let Some(store) = &self.thread_store {
            if let Err(e) = store.update(thread_id, None, Some(&title), None, None) {
                tracing::warn!("set_auto_title: store update failed: {e}");
            }
        }
        for t in &mut self.sessions {
            if t.record.id == thread_id {
                t.record.title = title.clone();
            }
        }
        cx.notify();
    }

    fn start_resize(&mut self, start_x: f32) {
        self.resize_start_x = Some(start_x);
        self.resize_start_width = Some(self.state.width);
    }

    fn update_resize(&mut self, current_x: f32, _window: &mut Window, cx: &mut Context<Self>) {
        let (start_x, start_width) = match (self.resize_start_x, self.resize_start_width) {
            (Some(x), Some(w)) => (x, w),
            _ => return, // Resize not armed — ignore stray drag events.
        };
        // The window shrinks/grows under the cursor mid-drag, which can
        // transiently put the pointer outside the window's current bounds
        // and fire a hover-leave — that would schedule a peek-close while
        // still dragging (ghost-window: the handle keeps receiving
        // DragMoveEvent for a window that's gone). Re-arm the peek hold on
        // every tick so a resize drag can never trigger the leave-debounce.
        hold_peek(cx);
        let delta = current_x - start_x;
        self.state.resize(start_width + delta);
        // The actual `window.resize()` Wayland round-trip happens in
        // `render()`, coalesced to once per paint via `last_resized_width`.
        // Doing it here fires once per raw DragMoveEvent (per pointer
        // motion), flooding `zwlr_layer_surface_v1.set_size`; the throttle
        // is a set_size-rate optimization only. (The "resize dies at rest
        // width" bug was unrelated — a flex min-width issue in `panel.rs`.)
        cx.notify();
    }

    /// Switch the active agent backend. Closes the dropdown, lazily spawns
    /// the client if it hasn't been created yet, and updates the status.
    fn switch_agent(&mut self, agent_id: &str, cx: &mut Context<Self>) {
        if agent_id == self.active_agent_id {
            self.agent_menu_open = false;
            return;
        }

        self.active_agent_id = agent_id.to_string();
        self.agent_menu_open = false;
        self.sessions.clear();
        self.state.active_session_id = None;
        self.streaming.reset();
        self.thread_menu_open = None;

        // If client already exists, just mark connected.
        if self.clients.contains_key(agent_id) {
            self.state.agent_status = state::AgentStatus::Connected;
            cx.notify();
            return;
        }

        // Lazy-spawn: find the descriptor, spawn the client in background.
        let descriptor = self.agents.iter().find(|a| a.id == agent_id).cloned();
        let Some(desc) = descriptor else {
            self.state.agent_status = state::AgentStatus::Disconnected;
            cx.notify();
            return;
        };

        self.state.agent_status = state::AgentStatus::Thinking;
        cx.notify();

        let agent_id = agent_id.to_string();
        let env_for_spawn = self.shared_env.clone();
        cx.spawn(
            async move |this, cx| match HermesClient::new(desc.config, env_for_spawn).await {
                Ok(client) => {
                    let session = client.create_session().await;
                    let _ = this.update(cx, |this, _cx| {
                        this.clients.insert(agent_id, client);
                        this.state.agent_status = state::AgentStatus::Connected;
                        tracing::info!(
                            "side_panel_left: switched to agent {}",
                            this.active_agent_id
                        );
                        if let Ok(session) = session {
                            this.state.session_id = Some(session.id.to_string());
                            if let Some(modes) = session.modes {
                                this.composer_selected_mode = modes.current_id;
                                this.available_modes = modes.available;
                                this.detect_yolo_bypass_mode();
                            }
                            if let Some(models) = session.models {
                                this.composer_selected_model = models.current_id;
                                this.available_models = models.available;
                            }
                        }
                    });
                }
                Err(e) => {
                    tracing::warn!("side_panel_left: agent spawn failed: {e}");
                    let _ = this.update(cx, |this, _cx| {
                        this.state.agent_status = state::AgentStatus::Disconnected;
                    });
                }
            },
        )
        .detach();
    }
}

impl Drop for SidePanelLeft {
    fn drop(&mut self) {
        // Dropping a `gpui::Task` handle cancels it — there is no abort().
        drop(self.streaming.receiver_task.take());
        drop(self.streaming.acp_task.take());
        tracing::info!("SidePanelLeft dropped — streaming tasks aborted");
    }
}

fn open_window(cx: &mut App, pinned: bool) {
    if cx.global::<SidePanelLeftState_>().handle.is_some() {
        if pinned {
            cx.global_mut::<SidePanelLeftState_>().pinned = true;
            tracing::info!("side_panel_left: upgraded peek → pinned");
        }
        return;
    }
    let display_id = crate::monitor::pult_display(cx);
    match cx.open_window(window_options(display_id, cx), |_, view_cx| {
        view_cx.new(|cx| SidePanelLeft::new(cx))
    }) {
        Ok(handle) => {
            let state = cx.global_mut::<SidePanelLeftState_>();
            state.handle = Some(handle);
            state.pinned = pinned;
            tracing::info!(
                "side_panel_left: opened ({})",
                if pinned { "pinned" } else { "peek" }
            );
        }
        Err(err) => tracing::warn!(
            "side_panel_left: failed to open ({}): {err}",
            if pinned { "pinned" } else { "peek" }
        ),
    }
}

pub fn open_pinned(cx: &mut App) {
    open_window(cx, true);
}

pub fn open_peek(cx: &mut App) {
    open_window(cx, false);
}

pub fn close(cx: &mut App) {
    if let Some(handle) = cx.global_mut::<SidePanelLeftState_>().handle.take() {
        cx.global_mut::<SidePanelLeftState_>().pinned = false;
        // Clear exclusive zone before destroying the surface so the
        // compositor reclaims reserved space even if it doesn't auto-clean.
        match handle.update(cx, |_, window: &mut Window, _| {
            window.set_exclusive_zone(px(0.));
            window.remove_window()
        }) {
            Ok(()) => tracing::info!("side_panel_left: closed"),
            Err(e) => tracing::warn!(
                "side_panel_left: close() could not reach the window ({e}) — possible ghost"
            ),
        }
    } else {
        cx.global_mut::<SidePanelLeftState_>().pinned = false;
    }
}

pub(crate) fn close_this(window: &mut Window, cx: &mut App) {
    let this = window.window_handle();
    let tracked = cx
        .global::<SidePanelLeftState_>()
        .handle
        .as_ref()
        .map(|h| **h == this)
        .unwrap_or(false);
    if tracked {
        let state = cx.global_mut::<SidePanelLeftState_>();
        state.handle.take();
        state.pinned = false;
    }
    window.set_exclusive_zone(px(0.));
    window.remove_window();
    tracing::info!("side_panel_left: close_this");
}

/// Pure decision: should a peek-leave request close the panel?
fn should_close_on_peek_leave(state: &SidePanelLeftState_) -> bool {
    !state.pinned
}

/// Cursor entered strip or panel — cancel any pending peek-close.
pub(crate) fn hold_peek(cx: &mut App) {
    let state = cx.global_mut::<SidePanelLeftState_>();
    state.peek_generation = state.peek_generation.wrapping_add(1);
}

/// Cursor left strip or panel — close after debounce if still unpinned
/// and no later enter bumped the generation.
pub(crate) fn schedule_release_peek(cx: &mut App) {
    let generation = cx.global::<SidePanelLeftState_>().peek_generation;
    schedule_release_from_app(cx, generation);
}

/// Mouse left the strip and the panel. Closes only if not pinned.
pub fn close_peek_if_not_pinned(cx: &mut App) {
    if !should_close_on_peek_leave(cx.global::<SidePanelLeftState_>()) {
        return;
    }
    close(cx);
}

const PEEK_LEAVE_DEBOUNCE: std::time::Duration = std::time::Duration::from_millis(280);

pub(crate) fn schedule_release_from_app(cx: &mut gpui::App, generation: u64) {
    cx.spawn(async move |app_cx: &mut gpui::AsyncApp| {
        app_cx
            .background_executor()
            .timer(PEEK_LEAVE_DEBOUNCE)
            .await;
        app_cx.update(|app_cx| {
            if app_cx.global::<SidePanelLeftState_>().peek_generation != generation {
                return;
            }
            close_peek_if_not_pinned(app_cx);
        });
    })
    .detach();
}

/// Toggle the pinned panel open/closed. Called from the IPC handler (no
/// `Window` in scope there — matches `launcher::toggle(cx)`'s shape).
pub fn toggle(cx: &mut App) {
    if cx.global::<SidePanelLeftState_>().handle.is_some() {
        close(cx);
    } else {
        open_pinned(cx);
    }
}

pub fn init(cx: &mut App) {
    cx.set_global(SidePanelLeftState_::default());
    // Defer the strip one tick so `cx.displays()` / pult uuid match what
    // `bar::init` sees a moment later. Opening the strip synchronously in
    // `main` before the bar historically landed it on the wrong output
    // (HDMI-A-1) while the panel+bar bound to DP-1 (pult).
    cx.spawn(async move |cx| {
        cx.background_executor()
            .timer(std::time::Duration::from_millis(50))
            .await;
        cx.update(|cx| {
            // Hover-peek disabled by design decision (2026-07-23): the
            // panel is now a keybind-toggled, pinned-only dock — auto-open
            // on hover fought with "stays put until I close it" (the user
            // kept nudging the edge and getting an unwanted peek). The
            // strip + `hold_peek`/`schedule_release_peek`/`close_peek_if_
            // not_pinned` debounce machinery in this module is kept
            // working and unit-tested — flip this back on if hover-peek is
            // ever wanted again as an *additional* quick-look mode
            // alongside the keybind toggle, not a replacement for it.
            // hover_strip::init_hover_strip(cx);
            // Optional smoke: pin-open for grim without hover/ydotool.
            // Not product wiring — only when env is set.
            if std::env::var_os("CHRONOS_SMOKE_SIDE_PANEL_LEFT").is_some() {
                open_pinned(cx);
            }
        });
    })
    .detach();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_starts_as_peek() {
        let state = state::SidePanelLeftState::new();
        assert_eq!(state.state, PanelState::Peek);
    }

    #[test]
    fn state_default_width_opens_chat_column() {
        // T137: Super+A must show composer, not rail-only strip.
        let state = state::SidePanelLeftState::new();
        assert_eq!(state.width, state::SidePanelLeftState::DEFAULT_CHAT_WIDTH);
        assert!(state.width > sessions_list::SIDEBAR_MIN_WIDTH);
    }

    #[test]
    fn state_min_width_is_sidebar_plus_handle() {
        let state = state::SidePanelLeftState::new();
        assert_eq!(state.min_width, sessions_list::SIDEBAR_MIN_WIDTH);
    }

    #[test]
    fn toggle_collapse_recalculates_min_width() {
        let mut state = state::SidePanelLeftState::new();
        assert!(state.sessions_collapsed);
        assert_eq!(state.width, state::SidePanelLeftState::DEFAULT_CHAT_WIDTH);
        // Expand sessions: min must fit 200 + handle
        state.sessions_collapsed = false;
        state.recalc_min_width();
        assert_eq!(
            state.min_width,
            sessions_list::SIDEBAR_EXPANDED_WIDTH + sessions_list::SIDEBAR_HANDLE_WIDTH
        );
        assert!(state.width >= state.min_width);
    }

    #[test]
    fn clamp_width_below_min_after_recalc() {
        let mut state = state::SidePanelLeftState::new();
        state.resize(10.0);
        assert_eq!(state.width, sessions_list::SIDEBAR_MIN_WIDTH);
    }

    #[test]
    fn exclusive_px_dock_vs_overlay() {
        let mut state = state::SidePanelLeftState::new();
        assert!(!state.dock_chat);
        // Bar strip includes handle so tiles don't sit under the grab edge.
        assert_eq!(state.exclusive_px(), sessions_list::SIDEBAR_MIN_WIDTH);
        state.sessions_collapsed = false;
        assert_eq!(
            state.exclusive_px(),
            sessions_list::SIDEBAR_EXPANDED_WIDTH + sessions_list::SIDEBAR_HANDLE_WIDTH
        );
        state.width = 400.0;
        state.dock_chat = true;
        assert_eq!(state.exclusive_px(), 400.0);
    }

    #[test]
    fn ensure_chat_width_expands_from_sidebar_only() {
        let mut state = state::SidePanelLeftState::new();
        state.width = sessions_list::SIDEBAR_MIN_WIDTH;
        state.ensure_chat_width();
        assert!(state.width > sessions_list::SIDEBAR_MIN_WIDTH);
        assert_eq!(state.width, state::SidePanelLeftState::DEFAULT_CHAT_WIDTH);
    }
}
