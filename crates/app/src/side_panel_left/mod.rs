mod state;
mod panel;
pub mod sessions_list;
mod chat_view;
mod composer;
mod tool_card;
mod hover_strip;

pub use state::{PanelState, SidePanelLeftState};

use chronos_luau::bar::BAR_HEIGHT;
use chronos_services::hermes_acp::{AgentDescriptor, HermesClient, known_agents};
use chronos_services::{ModelInfo, SessionMode};
use gpui::{
    App, Bounds, DisplayId, Focusable, Global, Size, Window, WindowBackgroundAppearance,
    WindowBounds, WindowHandle, WindowKind, WindowOptions, layer_shell::*, point, prelude::*, px,
};
use std::collections::HashMap;

pub struct LeftPanelResize;

const PANEL_WIDTH: f32 = 352.;
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
    WindowOptions {
        display_id,
        titlebar: None,
        window_bounds: Some(WindowBounds::Windowed(Bounds {
            origin: point(px(0.), px(0.)),
            size: Size::new(px(PANEL_WIDTH), px(panel_h)),
        })),
        app_id: Some("chronos-side-panel-left".to_string()),
        window_background: WindowBackgroundAppearance::Transparent,
        kind: WindowKind::LayerShell(LayerShellOptions {
            namespace: "side_panel_left".to_string(),
            layer: Layer::Overlay,
            anchor: Anchor::LEFT | Anchor::TOP,
            exclusive_zone: None,
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
    /// Lazy-spawned clients keyed by agent id.
    clients: HashMap<String, HermesClient>,
    /// Which agent backend is currently active.
    active_agent_id: String,
    /// Whether the agent switcher dropdown is open.
    agent_menu_open: bool,
    sessions: Vec<sessions_list::SessionItem>,
    /// Available modes from the active ACP session.
    available_modes: Vec<chronos_services::SessionMode>,
    /// Available models from the active ACP session.
    available_models: Vec<chronos_services::ModelInfo>,
    pub(crate) chat: chat_view::ChatView,
    pub(crate) composer_focus: gpui::FocusHandle,
    pub(crate) composer_text: String,
    pub(crate) composer_cursor: usize,
    pub(crate) composer_selected_model: String,
    pub(crate) composer_selected_mode: String,
    pub(crate) composer_model_dropdown_open: bool,
    pub(crate) composer_mode_dropdown_open: bool,
    pub(crate) composer_focused: bool,
    resize_start_x: Option<f32>,
    resize_start_width: Option<f32>,
}

impl Render for SidePanelLeft {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        panel::render_panel(self, _window, cx)
    }
}

impl Focusable for SidePanelLeft {
    fn focus_handle(&self, _cx: &gpui::App) -> gpui::FocusHandle {
        self.composer_focus.clone()
    }
}

impl SidePanelLeft {
    fn new(cx: &mut Context<Self>) -> Self {
        let agents = known_agents();
        let active_agent_id = agents
            .first()
            .map(|a| a.id.to_string())
            .unwrap_or_default();

        // Lazy-spawn the default agent (first in registry).
        let default_config = agents
            .first()
            .map(|a| a.config.clone())
            .unwrap_or_default();
        let agent_id = active_agent_id.clone();
        cx.spawn(async move |this, cx| {
            match HermesClient::new(default_config).await {
                Ok(client) => {
                    let _ = this.update(cx, |this, _cx| {
                        this.clients.insert(agent_id, client);
                        this.state.agent_status = state::AgentStatus::Connected;
                        tracing::info!("side_panel_left: ACP client connected");
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
            state: state::SidePanelLeftState::new(),
            agents,
            clients: HashMap::new(),
            active_agent_id,
            agent_menu_open: false,
            sessions: Vec::new(),
            available_modes: Vec::new(),
            available_models: Vec::new(),
            chat: chat_view::ChatView::new(),
            composer_focus: cx.focus_handle(),
            composer_text: String::new(),
            composer_cursor: 0,
            composer_selected_model: String::new(),
            composer_selected_mode: String::new(),
            composer_model_dropdown_open: false,
            composer_mode_dropdown_open: false,
            composer_focused: false,
            resize_start_x: None,
            resize_start_width: None,
        }
    }

    fn toggle_collapse(&mut self, cx: &mut Context<Self>) {
        self.state.sessions_collapsed = !self.state.sessions_collapsed;
        cx.notify();
    }

    fn create_new_session(&mut self, cx: &mut Context<Self>) {
        let id = uuid::Uuid::new_v4().to_string();
        let title = format!("Session {}", self.sessions.len() + 1);
        self.sessions.push(sessions_list::SessionItem {
            id: id.clone(),
            title,
            active: true,
        });
        // Deactivate previous active sessions
        for s in self.sessions.iter_mut().rev().skip(1) {
            s.active = false;
        }
        self.state.active_session_id = Some(id);
        cx.notify();
    }

    fn select_session(&mut self, session_id: &str, cx: &mut Context<Self>) {
        for s in &mut self.sessions {
            s.active = s.id == session_id;
        }
        self.state.active_session_id = Some(session_id.to_string());
        cx.notify();
    }

    fn start_resize(&mut self, start_x: f32) {
        self.resize_start_x = Some(start_x);
        self.resize_start_width = Some(self.state.width);
    }

    fn update_resize(&mut self, current_x: f32, window: &mut Window, cx: &mut Context<Self>) {
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
        // Must resolve the same display the panel was actually opened on
        // (`crate::monitor::pult_display`), not `None`/primary — on a
        // multi-monitor setup the OS-primary display can be a different,
        // shorter monitor than the one showing this panel, which silently
        // shrinks the window height on every resize tick.
        let display_id = crate::monitor::pult_display(cx);
        let display_h = display_height(display_id, cx);
        let panel_h = (display_h - PANEL_EDGE_GAP).max(100.);
        window.resize(Size::new(px(self.state.width), px(panel_h)));
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
        cx.spawn(async move |this, cx| {
            match HermesClient::new(desc.config).await {
                Ok(client) => {
                    let _ = this.update(cx, |this, _cx| {
                        this.clients.insert(agent_id, client);
                        this.state.agent_status = state::AgentStatus::Connected;
                        tracing::info!("side_panel_left: switched to agent {}", this.active_agent_id);
                    });
                }
                Err(e) => {
                    tracing::warn!("side_panel_left: agent spawn failed: {e}");
                    let _ = this.update(cx, |this, _cx| {
                        this.state.agent_status = state::AgentStatus::Disconnected;
                    });
                }
            }
        })
        .detach();
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
        match handle.update(cx, |_, window: &mut Window, _| window.remove_window()) {
            Ok(()) => {
                tracing::info!("side_panel_left: closed");
            }
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
            if app_cx
                .global::<SidePanelLeftState_>()
                .peek_generation
                != generation
            {
                return;
            }
            close_peek_if_not_pinned(app_cx);
        });
    })
    .detach();
}

pub fn toggle(_window: &mut Window, cx: &mut App) {
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
            hover_strip::init_hover_strip(cx);
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
    fn state_default_width() {
        let state = state::SidePanelLeftState::new();
        assert_eq!(state.width, 352.0);
    }
}
