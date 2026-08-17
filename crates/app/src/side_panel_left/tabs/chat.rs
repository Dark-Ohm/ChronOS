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

use chronos_services::hermes_acp::{AgentDescriptor, HermesClient, StreamingEvent, known_agents};
use chronos_services::threads::{ThreadRecord, ThreadStore};
use crate::side_panel_left::composer::{ModeSelectDelegate, ModelSelectDelegate};
use gpui::{
    Context, Focusable, Window,
    prelude::*, px,
};
use gpui::{AnimationExt, IntoElement, div, svg};
use chronos_ui::{Theme, WindowRootExt, elevation_glow_bar};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::motion;
use crate::side_panel_left::sessions_list;
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
    /// T286: the composer field is a `gpui-component` `Input` bound to this
    /// `InputState` — wrap, caret, selection, IME and blink come from the kit.
    pub(crate) composer_input: gpui::Entity<gpui_component::input::InputState>,
    /// Subscription to composer `InputState` events (send on PressEnter,
    /// repaint on Change so the send button tracks the text).
    _composer_events: gpui::Subscription,
    pub(crate) composer_selected_model: String,
    pub(crate) composer_selected_mode: String,
    /// The mode ID that was active before YOLO toggle, to restore on toggle-off.
    pub(crate) composer_previous_mode: String,
    /// Cached ID of the bypass/YOLO mode found in available_modes, if any.
    pub(crate) composer_yolo_bypass_id: Option<String>,
    /// T287-A: model picker is a kit `Select` (its own keyboard nav + search).
    pub(crate) composer_model_select:
        gpui::Entity<gpui_component::select::SelectState<ModelSelectDelegate>>,
    /// T287-A: mode picker — same kit Select, non-searchable.
    pub(crate) composer_mode_select:
        gpui::Entity<gpui_component::select::SelectState<ModeSelectDelegate>>,
    _composer_model_select_events: gpui::Subscription,
    _composer_mode_select_events: gpui::Subscription,
    /// File-drag hover highlight on the composer (T286, was on TextInputState).
    pub(crate) composer_drop_hover: bool,
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
        // layout decisions (dock-mode chrome, canvas sizing). They
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

impl ChatTab {
    pub(crate) fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let agents = known_agents();
        let shared_env = chronos_services::hermes_acp::load_shared_env();
        let active_agent_id = agents.first().map(|a| a.id.to_string()).unwrap_or_default();

        // T286: the composer field is a `gpui-component` `Input`. The state is
        // created here (the panel opens inside `open_window`, so a window is
        // available) — auto_grow gives min 3 rows growing to a cap, soft_wrap
        // wraps at the column width, submit_on_enter makes plain Enter a send
        // (PressEnter) while Shift+Enter inserts a newline.
        let composer_input = cx.new(|cx| {
            gpui_component::input::InputState::new(window, cx)
                .auto_grow(3, 30)
                .soft_wrap(true)
                .submit_on_enter(true)
        });
        let agent_display_name = agents
            .first()
            .map(|a| a.display_name.as_str())
            .unwrap_or("Agent");
        composer_input.update(cx, |s, cx| {
            s.set_placeholder(
                format!("Message {agent_display_name} — @ to include context, / for commands"),
                window,
                cx,
            );
        });
        // Send on the kit's PressEnter (primary Enter only — Shift+Enter
        // already inserted a newline inside the Input) and repaint on Change
        // so the send button's active state tracks the text.
        let composer_events = cx.subscribe_in(&composer_input, window, |this, _, event, window, cx| {
            match event {
                gpui_component::input::InputEvent::PressEnter {
                    secondary: false,
                    shift: false,
                } => this.send_composer(window, cx),
                gpui_component::input::InputEvent::Change => cx.notify(),
                _ => {}
            }
        });

        // T287-A: model/mode pickers are kit `Select` — state lives here
        // (created with a window available, like the composer Input), and
        // `Confirm` commits the same path the old `on_click` did.
        let composer_model_select = cx.new(|cx| {
            gpui_component::select::SelectState::new(
                crate::side_panel_left::composer::model_delegate_empty(),
                None,
                window,
                cx,
            )
            .searchable(true)
        });
        let composer_mode_select = cx.new(|cx| {
            gpui_component::select::SelectState::new(
                crate::side_panel_left::composer::mode_delegate_empty(),
                None,
                window,
                cx,
            )
        });
        let _composer_model_select_events = cx.subscribe_in(
            &composer_model_select,
            window,
            |this, _, event, window, cx| {
                if let gpui_component::select::SelectEvent::Confirm(Some(value)) = event {
                    this.apply_model_select(value.as_str(), window, cx);
                }
            },
        );
        let _composer_mode_select_events = cx.subscribe_in(
            &composer_mode_select,
            window,
            |this, _, event, window, cx| {
                if let gpui_component::select::SelectEvent::Confirm(Some(value)) = event {
                    this.apply_mode_select(value.as_str(), window, cx);
                }
            },
        );

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
            search_focused: false,
            follow_enabled: false,
            rename_thread_id: None,
            rename_input: String::new(),
            available_modes: Vec::new(),
            available_models: Vec::new(),
            chat: chat_view::ChatView::new(),
            composer_focus: cx.focus_handle(),
            composer_input,
            _composer_events: composer_events,
            composer_selected_model: String::new(),
            composer_selected_mode: String::new(),
            composer_previous_mode: String::new(),
            composer_yolo_bypass_id: None,
            composer_model_select,
            composer_mode_select,
            _composer_model_select_events,
            _composer_mode_select_events,
            composer_drop_hover: false,
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

    /// Cache the current chat transcript to the store.
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
    let agent_menu_open = panel.agent_menu_open;

    let agent_name = panel
        .agents
        .iter()
        .find(|a| a.id == panel.active_agent_id)
        .map(|a| a.display_name.clone())
        .unwrap_or_else(|| "Agent".to_string());

    // T278: resize handles and the resize-handle drag element used to live
    // here. They moved to `WorkspaceView` (which owns the new transparent
    // 4 px grab on the visible slice's outer edge). The legacy panel
    // body now renders inside `WorkspaceView`'s 920 px canvas without
    // its own resize affordance — the drag is driven by the workspace's
    // input region, which already excludes the part of the canvas the
    // legacy sidebar would have rendered into.

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

    // T287-C: the thread column is header + chat + composer at full canvas
    // width. The old Zed-style `thread-header` strip (✦ / ＋ ☰ 👁 ⋯) and the
    // inline sessions sidebar are gone — the sessions list lives on the
    // Sessions tab of the workspace rail.
    let thread_column = div()
        .id("thread-column")
        .flex_1()
        .min_w(px(0.))
        .h_full()
        .flex()
        .flex_col()
        .overflow_hidden()
        .child(chat)
        .child(composer);

    // Header with listeners. Built AFTER thread_column so its `cx.listener`
    // calls don't overlap `composer`'s RPIT-captured borrow of `cx` (Rust
    // 2024 impl Trait capture rules — composer's borrow lives as long as
    // the `composer` binding does, i.e. until thread_column moves it above;
    // a `cx.listener` call spliced in before that move would conflict,
    // E0502). Wrapped around `thread_column` so the cluster + dropdown sit
    // above the chat/composer stack (T230-errata — it used to be a sibling
    // of `clipped_content` at the `main-content` level).
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
        );

    // T287-C: the top strip is only the agent cluster — the fake window
    // chrome X (`side-panel-left-close`) is gone; the rail / Super+A / IPC
    // still close the panel (`close_this` lives in `side_panel_left::mod`).

    // Thread column + its header, stacked in their own flex_col.
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
        .child(thread_column_with_header);

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
                        // T266: the chat tab's plate follows surface alpha.
                        .bg(theme.surface_color(theme.bg.primary))
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