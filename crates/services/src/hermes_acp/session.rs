use agent_client_protocol::schema::SessionId;
use std::fmt;

#[derive(Debug, Clone)]
pub struct AcpSession {
    pub id: SessionId,
}

impl AcpSession {
    pub fn new(id: SessionId) -> Self {
        Self { id }
    }
}

impl fmt::Display for AcpSession {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "AcpSession({})", self.id)
    }
}
