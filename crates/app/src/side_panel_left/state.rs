#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PanelState {
    Peek,
    Pinned,
    Resizing,
}

#[derive(Clone, Copy, PartialEq)]
pub enum AgentStatus {
    Connected,
    Disconnected,
    Thinking,
}

/// Streaming state for an in-progress ACP prompt turn.
pub struct StreamingState {
    /// Whether a streaming turn is in progress.
    pub active: bool,
    /// Join handle for the event receiver task (aborted on drop/cancel).
    pub receiver_task: Option<gpui::Task<()>>,
    /// Join handle for the ACP prompt task (aborted on drop/cancel).
    pub acp_task: Option<gpui::Task<()>>,
}

impl StreamingState {
    pub fn new() -> Self {
        Self {
            active: false,
            receiver_task: None,
            acp_task: None,
        }
    }

    pub fn reset(&mut self) {
        self.active = false;
        // `gpui::Task` has no abort(); dropping the handle cancels the task.
        drop(self.receiver_task.take());
        drop(self.acp_task.take());
    }
}

pub struct SidePanelLeftState {
    pub state: PanelState,
    pub width: f32,
    pub height: f32,
    pub min_width: f32,
    pub max_width: f32,
    pub session_id: Option<String>,
    pub agent_status: AgentStatus,
    pub sessions_collapsed: bool,
    pub active_session_id: Option<String>,
    /// When true, chat tiles alongside sidebar (exclusive = full width).
    /// When false (default), chat overlays — exclusive stays at sidebar width.
    pub dock_chat: bool,
    /// Last exclusive_zone value sent to compositor (avoids redundant Wayland round-trips).
    pub last_exclusive_zone: Option<f32>,
}

impl SidePanelLeftState {
    pub fn new() -> Self {
        // Open with chat column visible (T137) — rail-only Super+A hid composer.
        let mut s = Self {
            state: PanelState::Peek,
            width: Self::DEFAULT_CHAT_WIDTH,
            height: 1080.0,
            min_width: super::sessions_list::SIDEBAR_MIN_WIDTH,
            max_width: 960.0,
            session_id: None,
            agent_status: AgentStatus::Disconnected,
            sessions_collapsed: true,
            active_session_id: None,
            dock_chat: false,
            last_exclusive_zone: None,
        };
        s.ensure_chat_width();
        s
    }

    pub fn sidebar_width(&self) -> f32 {
        if self.sessions_collapsed {
            super::sessions_list::SIDEBAR_COLLAPSED_WIDTH
        } else {
            super::sessions_list::SIDEBAR_EXPANDED_WIDTH
        }
    }

    /// Exclusive zone px: full panel when docked; bar strip (sidebar + handle)
    /// when overlay. Handle is on the inner edge — without it in the zone the
    /// grab strip sits on top of tiled windows (live 2026-07-25: reserved 36
    /// vs window 46). Mirrors right panel `RAIL_ONLY_WIDTH = rail + handle`.
    pub fn exclusive_px(&self) -> f32 {
        if self.dock_chat {
            self.width
        } else {
            self.sidebar_width() + super::sessions_list::SIDEBAR_HANDLE_WIDTH
        }
    }

    /// Recalculate min_width after collapse state changes.
    /// Expanded sessions need room for the 200px column + handle.
    pub fn recalc_min_width(&mut self) {
        self.min_width = self.sidebar_width() + super::sessions_list::SIDEBAR_HANDLE_WIDTH;
        if self.width < self.min_width {
            self.width = self.min_width;
        }
    }

    pub fn resize(&mut self, new_width: f32) {
        self.width = new_width.clamp(self.min_width, self.max_width);
    }

    /// Default width when opening chat / turning dock on from sidebar-only.
    pub const DEFAULT_CHAT_WIDTH: f32 = 352.;

    pub fn ensure_chat_width(&mut self) {
        let need = self.sidebar_width() + super::sessions_list::SIDEBAR_HANDLE_WIDTH + 120.0; // min thread column so chat is usable
        if self.width < need {
            self.width = Self::DEFAULT_CHAT_WIDTH.max(need).min(self.max_width);
        }
    }
}
