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
    /// Remembered expanded-chat width (N), restored on the next expand so a
    /// summon→expand→close→summon cycle returns the same N instead of the
    /// 352px default. Mirrors right panel `tab_resize_memory`. Survives panel
    /// close by being mirrored into the global `SidePanelLeftState_`.
    pub remembered_chat_width: Option<f32>,
}

impl SidePanelLeftState {
    pub fn new() -> Self {
        // T220: summon rail-only — only the 36px strip + 4px handle show; the
        // chat column does NOT auto-open on Super+A/peek (that hid the composer
        // in the old T137 behaviour). Chat is revealed by the dock toggle or a
        // resize drag, which expand to `remembered_chat_width` (or default).
        let s = Self {
            state: PanelState::Peek,
            width: super::sessions_list::SIDEBAR_MIN_WIDTH,
            height: 1080.0,
            min_width: super::sessions_list::SIDEBAR_MIN_WIDTH,
            max_width: 960.0,
            session_id: None,
            agent_status: AgentStatus::Disconnected,
            sessions_collapsed: true,
            active_session_id: None,
            dock_chat: false,
            last_exclusive_zone: None,
            remembered_chat_width: None,
        };
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
    /// vs window 40 = 36 rail + 4 handle). Mirrors right panel
    /// `RAIL_ONLY_WIDTH = rail + handle`.
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
        // A manual resize (drag / dock toggle) sets the remembered chat width
        // so a later summon→expand returns here, not the 352px default.
        let rail = self.sidebar_width() + super::sessions_list::SIDEBAR_HANDLE_WIDTH;
        if self.width - rail > 1.0 {
            self.remembered_chat_width = Some(self.width);
        }
    }

    /// Default width when opening chat / turning dock on from sidebar-only.
    pub const DEFAULT_CHAT_WIDTH: f32 = 352.;

    /// Rail-only summon width: collapsed sidebar strip + resize handle.
    /// Equal to the right panel's `RAIL_ONLY_WIDTH` (asserted by
    /// `rails_and_handles_match_right_panel`). T220: this is the width a
    /// summon opens at — only the rail shows; chat reveals separately.
    pub fn rail_only_width() -> f32 {
        super::sessions_list::SIDEBAR_MIN_WIDTH
    }

    /// Expand the chat column from rail-only to a usable width.
    /// Prefers the remembered width from a previous expand/resize; falls back
    /// to `DEFAULT_CHAT_WIDTH`. Never narrower than the sidebar strip + 120px
    /// thread column so the chat stays usable.
    pub fn ensure_chat_width(&mut self) {
        let need = self.sidebar_width() + super::sessions_list::SIDEBAR_HANDLE_WIDTH + 120.0; // min thread column so chat is usable
        let target = self
            .remembered_chat_width
            .unwrap_or(Self::DEFAULT_CHAT_WIDTH)
            .max(need)
            .min(self.max_width);
        if self.width < target {
            self.width = target;
        }
        self.remembered_chat_width = Some(self.width);
    }
}
