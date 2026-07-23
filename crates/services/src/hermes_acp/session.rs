use agent_client_protocol::schema::SessionId;
use std::fmt;

/// A single session mode (e.g. "ask", "accept_edits", "dont_ask").
#[derive(Debug, Clone)]
pub struct SessionMode {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
}

/// Available modes for a session.
#[derive(Debug, Clone)]
pub struct SessionModes {
    pub current_id: String,
    pub available: Vec<SessionMode>,
}

/// A single model entry (e.g. "openrouter:anthropic/claude-sonnet-4").
#[derive(Debug, Clone)]
pub struct ModelInfo {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
}

/// Available models for a session.
#[derive(Debug, Clone)]
pub struct SessionModels {
    pub current_id: String,
    pub available: Vec<ModelInfo>,
}

#[derive(Debug, Clone)]
pub struct AcpSession {
    pub id: SessionId,
    pub modes: Option<SessionModes>,
    pub models: Option<SessionModels>,
}

impl AcpSession {
    pub fn new(id: SessionId) -> Self {
        Self {
            id,
            modes: None,
            models: None,
        }
    }

    pub fn with_modes(mut self, modes: Option<SessionModes>) -> Self {
        self.modes = modes;
        self
    }

    pub fn with_models(mut self, models: Option<SessionModels>) -> Self {
        self.models = models;
        self
    }
}

impl fmt::Display for AcpSession {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "AcpSession({})", self.id)
    }
}
