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
}

impl SidePanelLeftState {
    pub fn new() -> Self {
        Self {
            state: PanelState::Peek,
            width: 352.0,
            height: 1080.0,
            min_width: super::PANEL_RAIL_TOTAL_WIDTH,
            max_width: 960.0,
            session_id: None,
            agent_status: AgentStatus::Connected,
            sessions_collapsed: false,
            active_session_id: None,
        }
    }

    pub fn resize(&mut self, new_width: f32) {
        self.width = new_width.clamp(self.min_width, self.max_width);
    }
}
