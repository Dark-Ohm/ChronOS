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
    pub session_id: Option<String>,
    pub agent_status: AgentStatus,
}

impl SidePanelLeftState {
    pub fn new() -> Self {
        Self {
            state: PanelState::Peek,
            width: 352.0,
            session_id: None,
            agent_status: AgentStatus::Connected,
        }
    }
}
