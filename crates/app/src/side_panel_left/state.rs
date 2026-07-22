#[derive(Clone, Copy, PartialEq)]
pub enum PanelState {
    Peek,
    Pinned,
    Resizing,
}

pub struct SidePanelLeftState {
    pub state: PanelState,
    pub width: f32,
    pub session_id: Option<String>,
}

impl SidePanelLeftState {
    pub fn new() -> Self {
        Self {
            state: PanelState::Peek,
            width: 352.0,
            session_id: None,
        }
    }
}
