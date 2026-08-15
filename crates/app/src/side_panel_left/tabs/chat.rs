//! T279 / Slice A2 — Chat tab, extracted from the legacy `ChatTab`
//! god-object (T278 carve). Owns ACP, transcript, composer, focus, and
//! streaming state. Owns no window handle, panel width, dock flag,
//! active tab, or window lifecycle — the workspace coordinator
//! (`WorkspaceView`) forwards visible-width and project/thread commands.
//!
//! Responsive layout consumes the visible content width mirrored by the
//! coordinator (`WorkspaceView::render` mirrors `visible_w` into
//! `state.width`, and `render_panel` branches on that mirror) — never
//! `window.bounds().size.width == 920`. T279 round 2: the standalone
//! `ChatLayout`/`chat_layout_for_visible_width` breakpoint helper was
//! deleted — prod never called it (the render path branches on the
//! mirrored width directly), so it was green-test theatre.

use chronos_services::hermes_acp::{
    AgentDescriptor, HermesClient, StreamingEvent, known_agents, load_shared_env,
};
use chronos_services::threads::{ThreadRecord, ThreadStore};
use chronos_services::{ModelInfo, SessionMode};
use gpui::{
    App, Bounds, Context, Focusable, Global, Task, UTF16Selection, Window,
    FocusHandle, Pixels, ShapedLine, Hsla,
    point, prelude::*, px,
};
use gpui::{AnimationExt, AnyElement, IntoElement, div, img, svg};
use chronos_ui::{Theme, WindowRootExt, elevation_glow_bar};
use std::collections::HashMap;
use std::ops::Range;
use std::path::{Path, PathBuf};

use crate::motion;
use crate::side_panel_left::sessions_list;
use crate::side_panel_left::sessions_list::{
    SIDEBAR_COLLAPSED_WIDTH, SIDEBAR_EXPANDED_WIDTH, SIDEBAR_HANDLE_WIDTH,
};
use crate::side_panel_left::state;
use crate::side_panel_left::state::AgentStatus;
use crate::side_panel_left::chat_view;

/// T285 — pure decision: given a restored thread's `acp_session_id` and its
/// `cwd`, should the spawn connect via `load_session` (resume the Hermes
/// session) or `create_session` (mint a new one)?
///
/// `HermesClient::load_session` bails on an empty `cwd`, and there is nothing
/// to resume without an `acp_session_id` — both cases fall back to Create.
#[derive(Debug, PartialEq, Eq)]
pub enum ConnectSessionAction {
    Load { acp_id: String, cwd: String },
    Create,
}

pub fn connect_session_action(restored_acp_id: Option<&str>, cwd: &str) -> ConnectSessionAction {
    match (restored_acp_id.filter(|s| !s.is_empty()), cwd) {
        (Some(acp_id), cwd) if !cwd.is_empty() => {
            ConnectSessionAction::Load { acp_id: acp_id.to_string(), cwd: cwd.to_string() }
        }
        _ => ConnectSessionAction::Create,
    }
}

pub struct ChatTab {
    pub(crate) state: state::SidePanelLeftState,
    /// Available agent backends from the registry.
    pub(crate) agents: Vec<AgentDescriptor>,
    /// Shared env vars from ~/.config/chronos/.env (passed to agent spawns).
    shared_env: HashMap<String, String>,
    /// Lazy-spawned clients keyed by agent id.
    pub(crate) clients: HashMap<String, HermesClient>,
    /// Which agent backend is currently active.
    pub(crate) active_agent_id: String,
    /// Whether the agent switcher dropdown is open.
    agent_menu_open: bool,
    sessions: Vec<sessions_list::ThreadListItem>,
    /// SQLite-backed thread store (T150). Loaded on startup, mutated by actions.
    thread_store: Option<ThreadStore>,
    /// Search query for filtering the thread list.
    pub(crate) thread_search: String,
    /// Show archived threads (hidden by default).
    show_archived: bool,
    /// True while replaying a session via load_session.
    thread_loading: bool,
    /// ID of the thread whose context menu is open.
    thread_menu_open: Option<String>,
    /// Whether the sidebar search field is focused (routes keyboard input).
    pub(crate) search_focused: bool,
    /// T195: Follow mode — when ON, agent tool calls push activity to the
    /// right panel's activity strip and auto-open files in Editor.
    pub(crate) follow_enabled: bool,
    /// When set, shows an inline rename input for this thread.
    pub(crate) rename_thread_id: Option<String>,
    /// Current text in the inline rename input.
    pub(crate) rename_input: String,
    /// Available modes from the active ACP session.
    pub(crate) available_modes: Vec<chronos_services::SessionMode>,
    /// Available models from the active ACP session.
    pub(crate) available_models: Vec<chronos_services::ModelInfo>,
    pub(crate) chat: chat_view::ChatView,
    pub(crate) composer_focus: gpui::FocusHandle,
    pub(crate) composer_input: crate::side_panel_left::text_input::TextInputState,
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
    /// T247: a message that arrived while the ACP client was still connecting
    /// (agent_status Thinking). The user message is pushed immediately; the
    /// ACP turn fires once the client connects (ChatTab::new spawn) or
    /// is dropped honestly if connect fails / the agent is switched.
    pub(crate) pending_send: Option<String>,
}

impl Render for ChatTab {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // T278: `ChatTab` no longer owns window lifecycle, exclusive
        // zone, width, dock, or resize. Those responsibilities moved to
        // `WorkspaceView` (content canvas) and `RailView` (rail surface).
        // All `set_exclusive_zone()` / `set_exclusive_
        // edge()` calls are gone — the surface bounds and zone are set
        // once at open time (`content_window_options` /
        // `rail_window_options`) and live-mutated only by their owning
        // views. This entity is purely the legacy product-state child,
        // rendered as a sub-element of `WorkspaceView`.
        //
        // The renderer still reads `state.width`/`state.dock_chat` for
        // layout decisions (chat_open threshold, dock-mode chrome). They
        // are mirrored from `SidePanelLeftState_` by `WorkspaceView::render`
        // before this render fires, so changes propagate through the SoT.
        render_panel(self, window, cx)
    }
}

impl Focusable for ChatTab {
    fn focus_handle(&self, _cx: &gpui::App) -> gpui::FocusHandle {
        self.composer_focus.clone()
    }
}

impl gpui::EntityInputHandler for ChatTab {
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
        // While a sub-input owns the keyboard — the sidebar thread search /
        // rename field (`search_focused`) or the model-picker search
        // (`composer_model_dropdown_open`) — its text lives in a separate
        // String driven by `on_key_down`. The composer's IME handler is still
        // bound to `composer_focus`, so without this guard those keystrokes
        // also land in `composer_input` and leak into the message box behind
        // the popup.
        if self.search_focused || self.composer_model_dropdown_open {
            return;
        }
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
        // Same guard as `replace_text_in_range`: don't let IME compose land in
        // `composer_input` while a sub-input (thread search/rename or model
        // search) owns the keyboard.
        if self.search_focused || self.composer_model_dropdown_open {
            return;
        }
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

impl ChatTab {
    pub(crate) fn new(cx: &mut Context<Self>) -> Self {
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

        // T288: resolve the ACP session cwd BEFORE cx.spawn. The panel scope
        // (`active_project_path`) is seeded by `restore_active_project_on_startup`
        // during `init` (side_panel_left/mod.rs:870), so the global already
        // holds the shell's active project at this point. Reading it here — and
        // not lazily inside the async body — means the session lands in the
        // project dir (`…/ChronOS`), not the `packaging/` cwd `chronos-start`
        // inherited (T288 symptom). Falls back to process cwd when unscoped.
        let session_cwd = {
            let active = cx
                .global::<crate::side_panel_left::SidePanelLeftState_>()
                .active_project_path
                .as_deref();
            let process = std::env::current_dir().unwrap_or_default();
            session_cwd(active, &process)
        };

        cx.spawn(async move |this, cx| {
            match HermesClient::new(default_config, env_for_spawn).await {
                Ok(client) => {
                    // T285: decide whether to resume the restored thread's ACP
                    // session or mint a fresh one. If `restore_project_thread`
                    // already painted a cached transcript (startup restore),
                    // we must bind the session WITHOUT re-replaying — otherwise
                    // the thread would be duplicated ("баннан" twice).
                    let session_cwd_for_fallback = session_cwd.clone();
                    let action = this
                        .update(cx, |this, _cx| {
                            let active = this
                                .sessions
                                .iter()
                                .find(|s| s.record.id == this.state.active_session_id.as_deref().unwrap_or(""));
                            let acp_id = active.and_then(|t| t.record.acp_session_id.clone());
                            let cwd = active
                                .map(|t| t.record.cwd.clone())
                                .unwrap_or_default();
                            connect_session_action(acp_id.as_deref(), &cwd)
                        })
                        .unwrap_or(ConnectSessionAction::Create);

                    match action {
                        ConnectSessionAction::Load { acp_id, cwd } => {
                            let _ = this.update(cx, |this, cx| {
                                this.clients.insert(agent_id, client);
                                this.state.agent_status = state::AgentStatus::Thinking;
                                tracing::info!(
                                    "side_panel_left: ACP client connected, resuming session {acp_id}"
                                );
                                // Cache already painted by restore_project_thread;
                                // bind only, do not re-replay into the transcript.
                                let replay = this.chat.messages.is_empty();
                                let acp_id_for_state = acp_id.clone();
                                if let Some(client) = this.clients.get(&this.active_agent_id).cloned() {
                                    this.run_load_session(
                                        client,
                                        acp_id,
                                        std::path::PathBuf::from(&cwd),
                                        replay,
                                        // Startup restore: if Hermes dropped the
                                        // session, fall back to a fresh one.
                                        Some(session_cwd_for_fallback),
                                        cx,
                                    );
                                }
                                this.state.session_id = Some(acp_id_for_state);
                                // T247: fire any queued message now that the
                                // session is bound.
                                if let Some(text) = this.pending_send.take() {
                                    this.start_acp_turn(text, cx);
                                }
                                cx.notify();
                            });
                        }
                        ConnectSessionAction::Create => {
                            // Fetch modes/models at connect time, not just
                            // after the first prompt — otherwise the
                            // model/mode pickers stay hidden for the entire
                            // life of a thread nobody has messaged yet
                            // (live smoke, 2026-07-23).
                            let session = client.create_session(&session_cwd).await;
                            let _ = this.update(cx, |this, cx| {
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
                                // T247: fire any message queued while the client
                                // was still connecting.
                                if let Some(text) = this.pending_send.take() {
                                    this.start_acp_turn(text, cx);
                                }
                                cx.notify();
                            });
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!("side_panel_left: ACP client init failed: {e}");
                    let _ = this.update(cx, |this, cx| {
                        this.state.agent_status = state::AgentStatus::Disconnected;
                        // T247: the queued message can never be delivered —
                        // close the thread honestly instead of leaving a
                        // dangling promise.
                        if this.pending_send.take().is_some() {
                            this.chat.push_message(chat_view::ChatMessage {
                                role: chat_view::MessageRole::Agent,
                                segments: vec![chat_view::Segment::Response {
                                    content: format!(
                                        "Не удалось подключиться к агенту ({e}) — сообщение не отправлено."
                                    ),
                                }],
                            });
                            this.chat.scroll_to_bottom();
                        }
                        cx.notify();
                    });
                }
            }
        })
        .detach();

        let mut this = Self {
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
            follow_enabled: false,
            rename_thread_id: None,
            rename_input: String::new(),
            available_modes: Vec::new(),
            available_models: Vec::new(),
            chat: chat_view::ChatView::new(),
            composer_focus: cx.focus_handle(),
            composer_last_click: None,
            composer_blink_task: None,
            composer_input: crate::side_panel_left::text_input::TextInputState::new(),
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
            pending_send: None,
        };

        // T281 gate 8 — on startup, restore the last valid session of the
        // persisted active project so a restart reopens where the user left
        // off. `restore_project_thread` only loads a thread the store
        // validates (id + project_path + archived=0); a stale / archived /
        // deleted / cross-project active id yields empty Chat. Mirrors the
        // project-switch path used by `switch_project`.
        if let Some(active) = crate::project_switcher::cached().active.clone() {
            let path = PathBuf::from(active);
            cx.global_mut::<crate::side_panel_left::SidePanelLeftState_>()
                .active_project_path = Some(path.clone());
            this.restore_project_thread(path.as_path(), cx);
        }
        this
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

    /// T280 / T288 — canonical project path to scope a thread to: the
    /// workspace's active project when set, else the process cwd.
    ///
    /// T288: resolution is delegated to the `session_cwd` helper so there is a
    /// single source of truth — `project_path`, `ChatTab::new`, and
    /// `switch_agent` all flow through here instead of each calling
    /// `std::env::current_dir()` independently (the two-source split that let
    /// `cwd` diverge onto `packaging/`).
    fn project_path(&self, cx: &mut Context<Self>) -> String {
        let active = cx
            .global::<crate::side_panel_left::SidePanelLeftState_>()
            .active_project_path
            .as_deref();
        let process = std::env::current_dir().unwrap_or_default();
        session_cwd(active, &process).to_string_lossy().to_string()
    }

    /// T280 — persist the active-thread selection for the current project.
    /// Best-effort: no store or no active project → no-op (never panics).
    fn persist_active_thread(&self, thread_id: Option<&str>, cx: &mut Context<Self>) {
        let project = cx
            .global::<crate::side_panel_left::SidePanelLeftState_>()
            .active_project_path
            .clone();
        let Some(project) = project else {
            return;
        };
        if let Some(store) = &self.thread_store {
            let p = project.to_string_lossy().to_string();
            if let Err(e) = store.set_active_thread(&p, thread_id) {
                tracing::warn!("persist_active_thread: {e}");
            }
        }
    }

    /// T279 / Task 3 — mint a fresh thread. `pub(crate)` so the workspace
    /// coordinator (`create_thread` free fn) reaches it through
    /// `SidePanelLeftState_.chat` — the Sessions-tab "+ New" path.
    pub(crate) fn create_new_session(&mut self, cx: &mut Context<Self>) {
        let id = uuid::Uuid::new_v4().to_string();
        // T288: single source of truth for the session cwd — `project_path`
        // (active project, else process cwd). `cwd` and `project` must agree
        // so the ACP session and the persisted `ThreadRecord` can't drift
        // apart (the pre-T288 split read `current_dir()` for `cwd` and
        // `project_path` separately, landing the session in `packaging/`).
        let cwd = self.project_path(cx);
        let project = cwd.clone();

        // Insert into the thread store (T150), scoped to the project. If the
        // store is unavailable, fall back to an in-memory-only session.
        if let Some(store) = &self.thread_store {
            if let Ok(record) = store.insert_for_project(&id, &self.active_agent_id, &cwd, &project) {
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
                    cwd: cwd.clone(),
                    project_path: project.clone(),
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
        self.state.active_session_id = Some(id.clone());
        // T280: persist the new thread as the project's active selection.
        self.persist_active_thread(Some(&id), cx);
        // Clear local transcript; mint a fresh ACP session on the agent.
        self.chat = chat_view::ChatView::new();
        self.streaming.reset();
        // T247: a queued message belongs to the old thread — drop it.
        self.pending_send = None;
        self.state.session_id = None;
        if let Some(client) = self.clients.get(&self.active_agent_id).cloned() {
            self.state.agent_status = state::AgentStatus::Thinking;
            cx.spawn(async move |this, cx| match client.create_session(Path::new(&cwd)).await {
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

    /// T279 / Task 3 — coordinator-facing thread load. Guarantees the
    /// record is in the local list (the workspace Sessions tab may hold a
    /// record this column has not seen yet), then delegates to the legacy
    /// `select_session` (outgoing-transcript cache, cached replay, ACP
    /// `load_session`).
    pub fn load_thread(&mut self, thread: ThreadRecord, cx: &mut Context<Self>) {
        let id = thread.id.clone();
        if !self.sessions.iter().any(|t| t.record.id == id) {
            self.sessions.push(sessions_list::ThreadListItem {
                record: thread,
                active: false,
            });
            self.sort_sessions();
        }
        self.select_session(&id, cx);
    }

    /// T279 / Task 3 — load a thread by id: record comes from the store
    /// (`ThreadStore::get`), falling back to the local list (covers a
    /// thread created this session that the store has not flushed yet).
    pub(crate) fn load_thread_by_id(&mut self, thread_id: &str, cx: &mut Context<Self>) {
        let record = self
            .thread_store
            .as_ref()
            .and_then(|s| s.get(thread_id).ok().flatten())
            .or_else(|| {
                self.sessions
                    .iter()
                    .find(|t| t.record.id == thread_id)
                    .map(|t| t.record.clone())
            });
        match record {
            Some(record) => self.load_thread(record, cx),
            None => {
                tracing::warn!("load_thread_by_id: unknown thread {thread_id}");
            }
        }
    }

    /// T279 / Task 4 — clear the chat column for a project switch or
    /// removal. Caches the outgoing transcript, then resets chat,
    /// streaming, `pending_send`, and both session ids so nothing from
    /// the old scope paints into the new one.
    pub fn clear_for_project(&mut self, _project_path: &std::path::Path, cx: &mut Context<Self>) {
        self.cache_transcript(cx);
        self.chat = chat_view::ChatView::new();
        self.streaming.reset();
        self.pending_send = None;
        self.state.active_session_id = None;
        self.state.session_id = None;
        for s in &mut self.sessions {
            s.active = false;
        }
        cx.notify();
    }

    /// T280 — restore the project's persisted active thread. Runs right after
    /// `clear_for_project` on a project switch. A valid persisted row is
    /// loaded into the chat column and mirrored on the SoT; missing / stale /
    /// archived / cross-project ids yield empty Chat (the store's
    /// `active_thread` already validates both id and project_path).
    pub fn restore_project_thread(&mut self, project_path: &std::path::Path, cx: &mut Context<Self>) {
        let Some(store) = &self.thread_store else {
            return;
        };
        let p = project_path.to_string_lossy().to_string();
        let Ok(Some(record)) = store.active_thread(&p) else {
            return;
        };
        cx.global_mut::<crate::side_panel_left::SidePanelLeftState_>()
            .active_session_id = Some(record.id.clone());
        self.load_thread(record, cx);
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
        // T280: persist this thread as the project's active selection.
        self.persist_active_thread(Some(session_id), cx);
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
                // T285: the cache is already painted above. Only replay the
                // ACP events into the transcript when the cache was empty —
                // otherwise we'd double-paint ("баннан" twice). A restored
                // thread with a cached transcript gets the session bound only.
                let replay_into_chat = self.chat.messages.is_empty();
                let cwd_path = std::path::PathBuf::from(&cwd);
                // Sessions-click path: no create_session fallback on failure.
                self.run_load_session(client, acp_id.clone(), cwd_path, replay_into_chat, None, cx);
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

    /// T285 — resume an existing Hermes ACP session via `load_session`, wiring
    /// the same replay consumer `select_session` used inline. When
    /// `replay_into_chat` is false the transcript is already painted (cache or
    /// a prior restore); we bind the session but do not mutate the visible
    /// chat, so `load_session`'s re-sent TextChunk/Thought events don't
    /// duplicate the thread.
    ///
    /// `fallback_cwd`, when `Some`, makes a dead `load_session` (Hermes no
    /// longer holds that session) fall back to `create_session` in that cwd —
    /// used by the startup restore path so a crashed Hermes still yields a
    /// working agent. The SQLite transcript is never wiped on failure.
    fn run_load_session(
        &mut self,
        client: HermesClient,
        acp_id: String,
        cwd_path: std::path::PathBuf,
        replay_into_chat: bool,
        fallback_cwd: Option<std::path::PathBuf>,
        cx: &mut Context<Self>,
    ) {
        self.state.agent_status = state::AgentStatus::Thinking;
        cx.notify();

        // Create streaming channel for replay events.
        let (event_tx, event_rx) = tokio::sync::mpsc::unbounded_channel();

        if replay_into_chat {
            // Push a placeholder agent message (filled by replay events).
            self.chat.push_message(chat_view::ChatMessage {
                role: chat_view::MessageRole::Agent,
                segments: Vec::new(),
            });
            self.chat.scroll_to_bottom();
        }

        // Spawn ACP load_session task.
        let acp_id_owned = acp_id.clone();
        let fallback_cwd_owned = fallback_cwd.clone();
        let agent_id_for_fallback = self.active_agent_id.clone();
        let load_task = cx.spawn(async move |this, cx| {
            match client.load_session(&acp_id_owned, &cwd_path, event_tx).await {
                Ok(()) => {
                    tracing::info!("side_panel_left: load_session replay complete for {acp_id_owned}");
                }
                Err(e) => {
                    tracing::warn!("side_panel_left: load_session failed: {e}");
                    if let Some(fb_cwd) = fallback_cwd_owned {
                        // Explicit fallback: the restored session died in
                        // Hermes. Mint a fresh one rather than leaving the
                        // agent dead. Log clearly — do not fail silently.
                        tracing::warn!("side_panel_left: load_session failed, new session");
                        let client_for_create = client;
                        match client_for_create.create_session(&fb_cwd).await {
                            Ok(session) => {
                                let _ = this.update(cx, |this, cx| {
                                    this.clients.insert(agent_id_for_fallback, client_for_create);
                                    this.state.session_id = Some(session.id.to_string());
                                    this.state.agent_status = state::AgentStatus::Connected;
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
                            Err(e2) => {
                                tracing::warn!("side_panel_left: create_session fallback failed: {e2}");
                                let _ = this.update(cx, |this, cx| {
                                    this.state.agent_status = state::AgentStatus::Disconnected;
                                    cx.notify();
                                });
                            }
                        }
                    } else if replay_into_chat {
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
                                // When binding only (replay_into_chat=false),
                                // the transcript is already painted — skip all
                                // message mutations so load_session's echoed
                                // events don't duplicate the thread.
                                if !replay_into_chat {
                                    cx.notify();
                                    return;
                                }
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
                                        // T195: push tool call to Follow state (right panel activity)
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
                // T280: deterministic clear of the persisted active id.
                self.persist_active_thread(None, cx);
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
            // T280: deterministic clear of the persisted active id.
            self.persist_active_thread(None, cx);
        }
        cx.notify();
    }

    /// Search threads by title and content.
    pub(crate) fn search_threads(&mut self, query: &str, cx: &mut Context<Self>) {
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
    pub(crate) fn commit_rename(&mut self, cx: &mut Context<Self>) {
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
    pub(crate) fn cancel_rename(&mut self, cx: &mut Context<Self>) {
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
    pub(crate) fn cache_transcript(&mut self, cx: &mut Context<Self>) {
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
    pub(crate) fn set_auto_title(&mut self, thread_id: &str, first_prompt: &str, cx: &mut Context<Self>) {
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
        // T247: a message queued for the previous agent is stale now.
        self.pending_send = None;
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
        // T288: capture the project cwd before spawn so the agent session
        // is created in the active project, not `packaging/` (process cwd).
        // `self` exists here, so route through `project_path` — the single
        // resolution source used by all three create_session call sites.
        let session_cwd = PathBuf::from(self.project_path(cx));
        cx.spawn(
            async move |this, cx| match HermesClient::new(desc.config, env_for_spawn).await {
                Ok(client) => {
                    let session = client.create_session(&session_cwd).await;
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

impl Drop for ChatTab {
    fn drop(&mut self) {
        // Dropping a `gpui::Task` handle cancels it — there is no abort().
        drop(self.streaming.receiver_task.take());
        drop(self.streaming.acp_task.take());
        tracing::info!("ChatTab dropped — streaming tasks aborted");
    }
}

fn status_color(status: AgentStatus, theme: &Theme) -> gpui::Hsla {
    match status {
        AgentStatus::Connected => theme.status.success,
        AgentStatus::Disconnected => theme.status.error,
        AgentStatus::Thinking => theme.status.warning,
    }
}

/// T288 — resolve the ACP session working directory.
///
/// Single source of truth for "where does a new ACP session start": the
/// workspace's active project when set (and non-empty), else the process
/// cwd. Every call site that needs a session cwd — `project_path`,
/// `ChatTab::new` (pre-spawn capture), `switch_agent`, and
/// `create_new_session` — delegates here instead of each independently
/// reading `std::env::current_dir()` (the two-source split that let `cwd`
/// drift onto `packaging/` while `project_path` pointed at `ChronOS`).
///
/// `active_project` is the shell's active project scope (`Option<&Path>`
/// from `SidePanelLeftState_`); `process_cwd` is the process working
/// directory captured at the call site so the helper stays pure and
/// testable without a GPUI context.
fn session_cwd(active_project: Option<&Path>, process_cwd: &Path) -> PathBuf {
    active_project
        .filter(|p| !p.as_os_str().is_empty())
        .map(Path::to_path_buf)
        .unwrap_or_else(|| process_cwd.to_path_buf())
}

pub fn render_panel(
    panel: &ChatTab,
    _window: &mut Window,
    cx: &mut Context<ChatTab>,
) -> impl IntoElement {
    let theme = *Theme::global(cx);
    let dot_color = status_color(panel.state.agent_status, &theme);
    // T230-errata: elevation glow lives on the thread column now (header
    // moved there — see below), needs to exist before thread_column builds.
    let elev = Theme::global(cx).elevation_popup();
    // T217 — top-corner radius where the panel meets the bar. Left-anchored:
    // screen x runs 0..width. Same per-corner rule as the right panel (both
    // resolve through `crate::state::panel_corner_radius`); a corner the bar
    // sits above stays square, a free one rhymes with the bar's pill radius.
    let corner_tl = crate::state::panel_corner_radius(0.0);
    let corner_tr = crate::state::panel_corner_radius(panel.state.width);
    let collapsed = panel.state.sessions_collapsed;
    let agent_menu_open = panel.agent_menu_open;
    // Chat visibility is width-driven (or forced by dock). Dock only changes
    // exclusive zone — it must NOT hide the thread (T126 accept errata).
    let sidebar_w = if collapsed {
        SIDEBAR_COLLAPSED_WIDTH
    } else {
        SIDEBAR_EXPANDED_WIDTH
    };
    let past_sidebar = panel.state.width > sidebar_w + SIDEBAR_HANDLE_WIDTH + 1.0;
    let chat_open = panel.state.dock_chat || past_sidebar;

    let agent_name = panel
        .agents
        .iter()
        .find(|a| a.id == panel.active_agent_id)
        .map(|a| a.display_name.clone())
        .unwrap_or_else(|| "Agent".to_string());

    // Thread title: active session's title, or a placeholder for a fresh
    // thread. Deliberately NOT `agent_name` — the outer header already
    // shows the agent (agent-cluster dot + name + switcher); repeating it
    // here reads as a duplicate label.
    let thread_title = panel
        .sessions
        .iter()
        .find(|s| s.active)
        .map(|s| s.display_title().to_string())
        .unwrap_or_else(|| "New Agent Thread".to_string());

    // T278: resize handles and the resize-handle drag element used to live
    // here. They moved to `WorkspaceView` (which owns the new transparent
    // 4 px grab on the visible slice's outer edge). The legacy panel
    // body now renders inside `WorkspaceView`'s 920 px canvas without
    // its own resize affordance — the drag is driven by the workspace's
    // input region, which already excludes the part of the canvas the
    // legacy sidebar would have rendered into.

    // Build sidebar (now borrows cx — click handlers on collapse/expand)
    let sidebar = build_sessions_sidebar(panel, collapsed, &theme, cx);

    // Thread header listener (built before any RPIT that captures cx)
    let thread_new_chat_handler = cx.listener(|this, _, _, cx| {
        this.create_new_session(cx);
    });
    // T195: Follow toggle — built before composer/chat to avoid RPIT capture.
    let thread_follow_handler = cx.listener(|this, _, _, cx| {
        this.follow_enabled = !this.follow_enabled;
        cx.update_global::<crate::agent_follow::AgentFollowState, _>(|state, _| {
            state.enabled = this.follow_enabled;
            if !this.follow_enabled {
                state.last_tool = None;
            }
        });
        cx.notify();
    });

    // Build agent dropdown with click handlers — must be built BEFORE
    // chat/composer because those call cx.listener() internally and
    // Rust 2024 RPIT capture rules make the returned elements hold a
    // mutable borrow on cx for their entire lifetime.
    let active_id = panel.active_agent_id.clone();
    let dropdown = if agent_menu_open {
        Some(
            div()
                .id("agent-dropdown")
                .w(px(172.))
                .bg(theme.bg.primary)
                .border_1()
                .border_color(theme.border.subtle)
                .rounded(px(8.))
                .p(px(4.))
                .mx(px(8.))
                .mt(px(4.))
                .flex()
                .flex_col()
                .children(panel.agents.iter().map(|agent| {
                    let is_selected = agent.id == active_id.as_str();
                    let agent_id = agent.id.to_string();
                    div()
                        .id(format!("agent-option-{}", agent.id))
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap(px(8.))
                        .px(px(8.))
                        .py(px(6.))
                        .rounded(px(6.))
                        .cursor_pointer()
                        .text_size(px(11.5))
                        .text_color(theme.text.secondary)
                        .hover(|s| s.bg(theme.border.subtle))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.switch_agent(&agent_id, cx);
                        }))
                        .child(
                            div()
                                .text_color(if is_selected {
                                    theme.text.primary
                                } else {
                                    theme.text.secondary
                                })
                                .child(agent.display_name.clone()),
                        )
                        .when(is_selected, |el| {
                            el.child(
                                div()
                                    .text_size(px(10.))
                                    .text_color(theme.accent.primary)
                                    .child("✓"),
                            )
                        })
                })),
        )
    } else {
        None
    };

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
    let composer = crate::side_panel_left::composer::render_composer(panel, _window, cx);

    // Thread header (block A) — static chrome
    // Use builder for the header because rsx! with cx.listener listeners
    // hits RPIT capture issues (see rakes §Rust 2024 RPIT capture).
    let thread_header = div()
        .id("thread-header")
        .flex_none()
        .h(px(38.))
        .px(px(12.))
        .border_b_1()
        .border_color(theme.border.subtle)
        .flex()
        .items_center()
        .justify_between()
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(6.))
                .child(
                    div()
                        .text_size(px(13.))
                        .text_color(theme.accent.primary)
                        .child("✦"),
                )
                .child(
                    div()
                        .text_size(px(12.5))
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(theme.text.primary)
                        .child(thread_title),
                ),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(4.))
                .child(
                    div()
                        .id("thread-new-chat")
                        .w(px(20.))
                        .h(px(20.))
                        .rounded(px(5.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_size(px(12.))
                        .text_color(theme.text.muted)
                        .cursor_pointer()
                        .hover(|s| s.bg(theme.border.subtle).text_color(theme.text.primary))
                        .on_click(thread_new_chat_handler)
                        .child("＋"),
                )
                .child(
                    div()
                        .id("thread-history")
                        .w(px(20.))
                        .h(px(20.))
                        .rounded(px(5.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_size(px(12.))
                        .text_color(theme.text.muted)
                        .cursor_pointer()
                        .hover(|s| s.bg(theme.border.subtle).text_color(theme.text.primary))
                        .child("☰"),
                )
                .child(
                    // T211: 👁 emoji was a color-bitmap → `text_color` couldn't
                    // tint it → ON/OFF were pixel-identical (0px diff). Swap to a
                    // `currentColor` SVG (follow.svg) + an accent bg when enabled,
                    // so the affordance actually flips visually.
                    div()
                        .id("thread-follow")
                        .w(px(26.))
                        .h(px(20.))
                        .rounded(px(5.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_size(px(10.))
                        .when(panel.follow_enabled, |el| {
                            el.bg(theme.accent.primary.opacity(0.16)).text_color(theme.accent.primary)
                        })
                        .when(!panel.follow_enabled, |el| {
                            el.text_color(theme.text.muted)
                        })
                        .cursor_pointer()
                        .hover(|s| s.bg(theme.border.subtle).text_color(theme.text.primary))
                        .on_click(thread_follow_handler)
                        .child(img("icons/follow.svg").w(px(16.)).h(px(16.))),
                )
                .child(
                    div()
                        .id("thread-more")
                        .w(px(20.))
                        .h(px(20.))
                        .rounded(px(5.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_size(px(12.))
                        .text_color(theme.text.muted)
                        .cursor_pointer()
                        .hover(|s| s.bg(theme.border.subtle).text_color(theme.text.primary))
                        .child("⋯"),
                ),
        );

    // Clipped content — sidebar beside the thread column, NOT stacked above
    // it. `flex_col` here (both were siblings of sidebar in one vertical
    // stack) made sidebar's `.h_full()` compete for vertical space against
    // thread_header/chat/composer instead of sitting in its own column —
    // sidebar came up short of the panel bottom and the thread column got
    // squeezed into a sliver near the bottom (live smoke, 2026-07-23).
    let thread_column = div()
        .id("thread-column")
        .flex_1()
        .min_w(px(0.))
        .h_full()
        .flex()
        .flex_col()
        .overflow_hidden()
        .child(thread_header)
        .child(chat)
        .child(composer);

    // Header with listeners. Built AFTER thread_column so its `cx.listener`
    // calls don't overlap `composer`'s RPIT-captured borrow of `cx` (Rust
    // 2024 impl Trait capture rules — composer's borrow lives as long as
    // the `composer` binding does, i.e. until thread_column moves it above;
    // a `cx.listener` call spliced in before that move would conflict,
    // E0502). Wrapped around `thread_column` below instead of sitting above
    // the whole sidebar+thread row (T230-errata) — it used to be a sibling
    // of `clipped_content` at the `main-content` level, so every resize
    // drag that crossed the `chat_open` width threshold popped the header
    // in/out and shoved the rail's `.h_full()` sidebar down with it (rail
    // visibly reflowed on resize — reported live 2026-08-04).
    let header = div()
        .id("side-panel-header")
        .flex()
        .items_center()
        .justify_between()
        .flex_none()
        .px(px(14.))
        .py(px(10.))
        .border_b_1()
        .border_color(theme.border.subtle)
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
                .hover(|s| s.bg(theme.border.subtle))
                .on_click(cx.listener(|this, _, _, cx| {
                    this.agent_menu_open = !this.agent_menu_open;
                    cx.notify();
                }))
                .child(
                    svg()
                        .path("icons/chronos-sigil.svg")
                        .size(px(15.))
                        .text_color(theme.accent.primary),
                )
                .child(
                    div()
                        .text_size(px(12.5))
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(theme.text.secondary)
                        .child(agent_name),
                )
                .child({
                    let status_text = match panel.state.agent_status {
                        crate::side_panel_left::state::AgentStatus::Connected => "Connected",
                        crate::side_panel_left::state::AgentStatus::Disconnected => "Disconnected",
                        crate::side_panel_left::state::AgentStatus::Thinking => "Thinking…",
                    };
                    div()
                        .text_size(px(10.5))
                        .text_color(theme.text.muted)
                        .child(status_text)
                })
                .child(
                    div()
                        .text_size(px(9.))
                        .text_color(theme.text.muted)
                        .child("⌄"),
                )
                .child(div().w(px(7.)).h(px(7.)).rounded_full().bg(dot_color)),
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
                .text_color(theme.text.muted)
                .cursor_pointer()
                .hover(|s| s.bg(theme.border.subtle).text_color(theme.text.primary))
                .on_click(|_ev, window, cx| {
                    crate::side_panel_left::close_this(window, cx);
                })
                .child(img("icons/x.svg").w(px(12.)).h(px(12.))),
        );

    // Thread column + its header, stacked in their own flex_col — a sibling
    // of `sidebar` inside `clipped_content`'s flex_row below. The rail
    // (`sidebar`) never sees this subtree, so header show/hide (tied to
    // `chat_open`, which is resize-driven) can never reflow it.
    let thread_column_with_header = div()
        .id("thread-column-wrap")
        .flex_1()
        .min_w(px(0.))
        .h_full()
        .flex()
        .flex_col()
        .overflow_hidden()
        .child(header)
        .children(dropdown)
        .children(elev.glow.map(elevation_glow_bar))
        .child(thread_column);

    let clipped_content = div()
        .id("clipped-content")
        .flex_1()
        .min_h(px(0.))
        .flex()
        .flex_row()
        .overflow_hidden()
        .child(sidebar)
        .when(chat_open, |el| el.child(thread_column_with_header));

    // Outer: sole window-level on_hover. Motion is native with_animation on the
    // shell row (T129) — not gpui_animation transition_when (silent no-op on
    // fresh layer-shell windows).
    //
    // T217: round the top corners + clip when either corner is free (bar does
    // not reach it). The window base is transparent, so rounded cutouts show
    // the desktop behind. When both corners are covered (full-width bar) no
    // rounding and no clip — keeps the elevation shadows intact.
    div()
        .id("side-panel-left-root")
        .window_font(&theme)
        .w(px(panel.state.width))
        .h_full()
        .flex()
        .flex_row()
        .when(corner_tl > 0.0 || corner_tr > 0.0, |d| {
            d.rounded_tl(px(corner_tl))
                .rounded_tr(px(corner_tr))
                .overflow_hidden()
        })
        .on_hover(|hovered, _window, cx| {
            if *hovered {
                crate::side_panel_left::hold_peek(cx);
            } else {
                crate::side_panel_left::schedule_release_peek(cx);
            }
        })
        .child(
            div()
                .id("side-panel-left-motion")
                .relative()
                .flex_1()
                .min_w(px(0.))
                .h_full()
                .flex()
                .flex_row()
                .child(
                    div()
                        .id("main-content")
                        .flex_1()
                        .min_w(px(0.))
                        .overflow_hidden()
                        .h_full()
                        .flex()
                        .flex_col()
                        .bg(theme.bg.primary)
                        .shadow(elev.shadows.to_vec())
                        .child(clipped_content),
                )
                // T278: the resize handle used to live here. It now lives in
                // `WorkspaceView` (transparent 4 px grab on the visible
                // slice's outer edge). The legacy body renders inside the
                // workspace's input region, which already enforces the
                // left-aligned visible-rect boundary.
                .with_animation(
                    "side-panel-left-enter",
                    motion::enter_animation(),
                    motion::apply_enter_from_left,
                ),
        )
}

fn build_sessions_sidebar(
    panel: &ChatTab,
    collapsed: bool,
    theme: &Theme,
    cx: &mut Context<ChatTab>,
) -> impl IntoElement + use<> {
    let sessions = &panel.sessions;

    if collapsed {
        div()
            .id("sessions-sidebar-collapsed")
            .w(px(SIDEBAR_COLLAPSED_WIDTH))
            .h_full()
            .flex()
            .flex_col()
            .items_center()
            .bg(theme.bg.tertiary)
            .border_r_1()
            .border_color(theme.border.subtle)
            .gap(px(4.))
            .p(px(4.))
            .child(
                div()
                    .id("sessions-expand")
                    .w(px(28.))
                    .h(px(28.))
                    .rounded(px(6.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_size(px(12.))
                    .text_color(theme.text.muted)
                    .cursor_pointer()
                    .hover(|s| s.bg(theme.border.subtle).text_color(theme.text.primary))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.toggle_collapse(cx);
                    }))
                    .child(">"),
            )
            .child(
                div()
                    .id("sessions-new-icon")
                    .w(px(28.))
                    .h(px(28.))
                    .rounded(px(6.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_size(px(12.))
                    .text_color(theme.text.muted)
                    .cursor_pointer()
                    .hover(|s| s.bg(theme.border.subtle).text_color(theme.text.primary))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.create_new_session(cx);
                    }))
                    .child("+"),
            )
            .children(sessions.iter().map(|s| {
                let is_active = s.active;
                let sid = s.record.id.clone();
                div()
                    .id(format!("session-dot-{sid}"))
                    .w(px(28.))
                    .h(px(28.))
                    .rounded(px(6.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded_full()
                    .when(is_active, |el| el.bg(theme.text.disabled))
                    .when(!is_active, |el| {
                        el.cursor_pointer().on_click(cx.listener({
                            let sid = sid.clone();
                            move |this, _, _, cx| this.select_session(&sid, cx)
                        }))
                    })
                    .child(div().w(px(6.)).h(px(6.)).rounded_full().bg(if is_active {
                        theme.status.success
                    } else {
                        theme.interactive.active
                    }))
            }))
            .child(div().flex_1()) // spacer
            .child({
                let docked = panel.state.dock_chat;
                div()
                    .id("dock-toggle")
                    .w(px(28.))
                    .h(px(28.))
                    .rounded(px(6.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_size(px(11.))
                    .text_color(if docked {
                        theme.accent.primary
                    } else {
                        theme.text.muted
                    })
                    .cursor_pointer()
                    .hover(|s| s.bg(theme.border.subtle))
                    .on_click(cx.listener(|this, _, _, cx| {
                        // T220: when collapsing the chat (dock on → off), remember
                        // the current expanded width so a later summon→expand
                        // returns it, not the 352px default. When expanding
                        // (off → on), grow to the remembered/default width.
                        if this.state.dock_chat {
                            this.state.remembered_chat_width = Some(this.state.width);
                        } else {
                            this.state.ensure_chat_width();
                        }
                        this.state.dock_chat = !this.state.dock_chat;
                        // Force exclusive recompute next paint.
                        this.state.last_exclusive_zone = None;
                        cx.notify();
                    }))
                    .child(if docked { "⊞" } else { "⊟" })
            })
            .into_any()
    } else {
        // ── Search / rename input ────────────────────────────────────────
        // When `rename_thread_id` is set, shows an inline rename input;
        // otherwise shows the search bar (or nothing if not focused and
        // query is empty).
        let search_or_rename: Option<AnyElement> = if panel.rename_thread_id.is_some() {
            // Inline rename input
            Some(
                div()
                    .id("thread-rename-input")
                    .flex_none()
                    .mx(px(8.))
                    .mt(px(4.))
                    .px(px(8.))
                    .py(px(5.))
                    .rounded(px(6.))
                    .border_1()
                    .border_color(theme.accent.primary)
                    .text_size(px(11.5))
                    .text_color(theme.text.primary)
                    .child(panel.rename_input.clone())
                    .into_any_element(),
            )
        } else if panel.search_focused || !panel.thread_search.is_empty() {
            // Search input
            Some(
                div()
                    .id("thread-search-input")
                    .flex_none()
                    .mx(px(8.))
                    .mt(px(4.))
                    .mb(px(2.))
                    .px(px(8.))
                    .py(px(5.))
                    .rounded(px(6.))
                    .border_1()
                    .border_color(if panel.search_focused {
                        theme.accent.primary
                    } else {
                        theme.border.default
                    })
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .text_size(px(11.5))
                    .text_color(theme.text.muted)
                    .child("🔍")
                    .child({
                        if panel.thread_search.is_empty() {
                            div().text_color(theme.text.muted).child("Search threads…")
                        } else {
                            div()
                                .text_color(theme.text.primary)
                                .child(panel.thread_search.clone())
                        }
                    })
                    .into_any_element(),
            )
        } else {
            None
        };

        // ── Context menu (floating) ──────────────────────────────────────
        let context_menu: Option<AnyElement> = panel.thread_menu_open.as_ref().map(|menu_id| {
            let mid = menu_id.clone();
            let is_pinned = panel
                .sessions
                .iter()
                .find(|t| t.record.id == *menu_id)
                .map(|t| t.record.pinned)
                .unwrap_or(false);
            let is_archived = panel
                .sessions
                .iter()
                .find(|t| t.record.id == *menu_id)
                .map(|t| t.record.archived)
                .unwrap_or(false);

            // Build each menu item with its own cx.listener (on_click expects
            // a closure, not a ClickEvent — see E0277 fix).
            let mid_rename = mid.clone();
            let rename_handler = cx.listener(move |this, _, _, cx| {
                let tid = mid_rename.clone();
                let title = this
                    .sessions
                    .iter()
                    .find(|t| t.record.id == tid)
                    .map(|t| t.display_title().to_string())
                    .unwrap_or_default();
                this.begin_rename(&tid, &title, cx);
            });
            let mid_pin = mid.clone();
            let pin_handler = cx.listener(move |this, _, _, cx| {
                this.toggle_pin(&mid_pin, cx);
            });
            let mid_archive = mid.clone();
            let archive_handler = cx.listener(move |this, _, _, cx| {
                this.toggle_archive(&mid_archive, cx);
            });
            let mid_delete = mid.clone();
            let delete_handler = cx.listener(move |this, _, _, cx| {
                this.delete_thread(&mid_delete, cx);
            });
            let pin_label = if is_pinned { "Unpin" } else { "Pin" };
            let archive_label = if is_archived { "Unarchive" } else { "Archive" };

            div()
                .id("thread-context-menu")
                .absolute()
                .right(px(8.))
                .top(px(40.))
                .w(px(130.))
                .rounded(px(8.))
                .bg(theme.bg.primary)
                .border_1()
                .border_color(theme.border.subtle)
                .shadow(vec![gpui::BoxShadow::new(
                    px(0.),
                    px(4.),
                    gpui::hsla(0., 0., 0., 0.3),
                )
                .blur_radius(px(12.))])
                .p(px(4.))
                .flex()
                .flex_col()
                .child(
                    div()
                        .id("ctx-rename")
                        .w_full()
                        .px(px(10.))
                        .py(px(5.))
                        .rounded(px(4.))
                        .text_size(px(11.5))
                        .text_color(theme.text.primary)
                        .cursor_pointer()
                        .hover(|s| s.bg(theme.border.subtle))
                        .on_click(rename_handler)
                        .child("Rename"),
                )
                .child(
                    div()
                        .id("ctx-pin")
                        .w_full()
                        .px(px(10.))
                        .py(px(5.))
                        .rounded(px(4.))
                        .text_size(px(11.5))
                        .text_color(theme.text.primary)
                        .cursor_pointer()
                        .hover(|s| s.bg(theme.border.subtle))
                        .on_click(pin_handler)
                        .child(pin_label),
                )
                .child(
                    div()
                        .id("ctx-archive")
                        .w_full()
                        .px(px(10.))
                        .py(px(5.))
                        .rounded(px(4.))
                        .text_size(px(11.5))
                        .text_color(theme.text.primary)
                        .cursor_pointer()
                        .hover(|s| s.bg(theme.border.subtle))
                        .on_click(archive_handler)
                        .child(archive_label),
                )
                .child(
                    div()
                        .id("ctx-divider")
                        .h(px(1.))
                        .my(px(2.))
                        .mx(px(4.))
                        .bg(theme.border.subtle),
                )
                .child(
                    div()
                        .id("ctx-delete")
                        .w_full()
                        .px(px(10.))
                        .py(px(5.))
                        .rounded(px(4.))
                        .text_size(px(11.5))
                        .text_color(gpui::hsla(0.0, 0.65, 0.65, 1.0))
                        .cursor_pointer()
                        .hover(|s| s.bg(theme.border.subtle))
                        .on_click(delete_handler)
                        .child("Delete"),
                )

                .into_any_element()
        });

        div()
            .id("sessions-sidebar-expanded")
            .relative()
            .w(px(SIDEBAR_EXPANDED_WIDTH))
            .h_full()
            .flex()
            .flex_col()
            .bg(theme.bg.tertiary)
            .border_r_1()
            .border_color(theme.border.subtle)
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
                    .border_color(theme.border.subtle)
                    .child(
                        div()
                            .text_size(px(11.5))
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(theme.text.secondary)
                            .child("Sessions"),
                    )
                    .child({
                        let docked = panel.state.dock_chat;
                        div()
                            .id("sessions-header-buttons")
                            .flex()
                            .items_center()
                            .gap(px(2.))
                            .child(
                                div()
                                    .id("dock-toggle-expanded")
                                    .w(px(20.))
                                    .h(px(20.))
                                    .rounded(px(4.))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .text_size(px(10.))
                                    .text_color(if docked {
                                        theme.accent.primary
                                    } else {
                                        theme.text.muted
                                    })
                                    .cursor_pointer()
                                    .hover(|s| s.bg(theme.border.subtle))
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        // T220: collapse remembers width; expand grows to it.
                                        if this.state.dock_chat {
                                            this.state.remembered_chat_width = Some(this.state.width);
                                        } else {
                                            this.state.ensure_chat_width();
                                        }
                                        this.state.dock_chat = !this.state.dock_chat;
                                        this.state.last_exclusive_zone = None;
                                        cx.notify();
                                    }))
                                    .child(if docked { "⊞" } else { "⊟" }),
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
                                    .text_color(theme.text.muted)
                                    .cursor_pointer()
                                    .hover(|s| {
                                        s.bg(theme.border.subtle).text_color(theme.text.primary)
                                    })
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.toggle_collapse(cx);
                                    }))
                                    .child("<"),
                            )
                    }),
            )
            .children(search_or_rename)
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
                    .border_color(theme.border.default)
                    .text_size(px(11.5))
                    .text_color(theme.text.secondary)
                    .cursor_pointer()
                    .hover(|s| s.bg(theme.border.subtle).border_color(theme.text.disabled))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.create_new_session(cx);
                    }))
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
                        let title = s.short_title();
                        let sid = s.record.id.clone();
                        let is_pinned = s.record.pinned;
                        let sid_click = sid.clone();
                        let sid_right = sid.clone();
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
                            .when(is_active, |el| el.bg(theme.border.default))
                            .when(!is_active, |el| el.hover(|s| s.bg(theme.border.subtle)))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.select_session(&sid_click, cx);
                            }))
                            .on_mouse_down(gpui::MouseButton::Right, cx.listener(
                                move |this, _ev, _window, cx| {
                                    this.open_thread_menu(&sid_right, cx);
                                },
                            ))
                            .child(div().w(px(6.)).h(px(6.)).rounded_full().bg(if is_active {
                                theme.status.success
                            } else {
                                theme.interactive.active
                            }))
                            .child(
                                div()
                                    .flex_1()
                                    .min_w(px(0.))
                                    .text_size(px(11.5))
                                    .text_color(if is_active {
                                        theme.text.primary
                                    } else {
                                        theme.text.secondary
                                    })
                                    .child(title),
                            )
                            .when(is_pinned, |el| {
                                el.child(
                                    div()
                                        .text_size(px(9.))
                                        .text_color(theme.text.muted)
                                        .child("📌"),
                                )
                            })
                    })),
            )
            .child(
                div()
                    .id("sessions-footer")
                    .flex_none()
                    .px(px(8.))
                    .py(px(6.))
                    .border_t_1()
                    .border_color(theme.border.subtle)
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .id("archived-toggle")
                            .text_size(px(10.5))
                            .text_color(if panel.show_archived {
                                theme.accent.primary
                            } else {
                                theme.text.muted
                            })
                            .cursor_pointer()
                            .hover(|s| s.text_color(theme.text.primary))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.toggle_archived(cx);
                            }))
                            .child(if panel.show_archived {
                                "Hide archived"
                            } else {
                                "Show archived"
                            }),
                    ),
            )
            .children(context_menu)
            .into_any()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Source contract (plan Task 3 gate): `tabs/chat.rs` must not
    /// reference the window-lifecycle surface — the chat tab is
    /// window-independent. Mirrors the T278
    /// `window_options_have_no_resize_calls` scan pattern.
    #[test]
    fn chat_tab_source_has_no_window_lifecycle() {
        let src = include_str!("chat.rs");
        let mut hits = Vec::new();
        for (i, line) in src.lines().enumerate() {
            let trimmed = line.trim_start();
            if trimmed.starts_with("//") || trimmed.starts_with("/*")
                || trimmed.starts_with('*') || trimmed.starts_with("//!")
            {
                continue;
            }
            // Strip inline string literals so the scan list below does
            // not self-match.
            let mut nos = String::with_capacity(line.len());
            for (idx, part) in line.split('"').enumerate() {
                if idx % 2 == 0 {
                    nos.push_str(part);
                }
            }
            // Needles split across string-concat positions so the gate
            // source does not self-match the literal contract scan. The
            // scanner reconstructs each token at runtime.
            let needles: [String; 3] = [
                "Window".to_string() + "Handle",
                "open_".to_string() + "window",
                "window.".to_string() + "resize(",
            ];
            for needle in &needles {
                if nos.contains(needle.as_str()) {
                    hits.push(format!("line {l}: {needle}", l = i + 1, needle = needle));
                }
            }
        }
        assert!(hits.is_empty(), "chat.rs has window-lifecycle refs: {hits:?}");
    }

    // ── T288: session_cwd resolution ──

    /// Active project wins.
    #[test]
    fn session_cwd_project_some_returns_project() {
        let project = Path::new("/home/neo/projects/chronos-ecosystem/ChronOS");
        let process = Path::new("/home/neo/projects/chronos-ecosystem/ChronOS/packaging");
        assert_eq!(
            session_cwd(Some(project), process),
            PathBuf::from(project),
            "active project must override process cwd"
        );
    }

    /// No active project → process cwd.
    #[test]
    fn session_cwd_none_returns_process() {
        let process = Path::new("/home/neo/projects/chronos-ecosystem/ChronOS/packaging");
        assert_eq!(
            session_cwd(None, process),
            PathBuf::from(process),
            "missing project falls back to process cwd"
        );
    }

    /// An empty active-project path is treated as unset (same as None) —
    /// otherwise the shell could hand the agent `cwd = ""`.
    #[test]
    fn session_cwd_empty_project_returns_process() {
        let process = Path::new("/home/neo/projects/chronos-ecosystem/ChronOS/packaging");
        assert_eq!(
            session_cwd(Some(Path::new("")), process),
            PathBuf::from(process),
            "empty project path must not win over process cwd"
        );
    }

    // ── T285: connect_session_action decision ──

    /// Restored thread with both acp id and cwd → resume via load_session.
    #[test]
    fn connect_action_load_when_id_and_cwd_present() {
        assert_eq!(
            connect_session_action(Some("acp-123"), "/home/neo/proj"),
            ConnectSessionAction::Load {
                acp_id: "acp-123".to_string(),
                cwd: "/home/neo/proj".to_string(),
            }
        );
    }

    /// No acp id → there is nothing to resume → create_session.
    #[test]
    fn connect_action_create_when_no_id() {
        assert_eq!(
            connect_session_action(None, "/home/neo/proj"),
            ConnectSessionAction::Create
        );
    }

    /// acp id present but cwd empty → load_session would bail in the client,
    /// so fall back to create_session (matches client guard).
    #[test]
    fn connect_action_create_when_cwd_empty() {
        assert_eq!(
            connect_session_action(Some("acp-123"), ""),
            ConnectSessionAction::Create
        );
    }

    /// Both empty → create_session.
    #[test]
    fn connect_action_create_when_both_empty() {
        assert_eq!(
            connect_session_action(None, ""),
            ConnectSessionAction::Create
        );
    }
}