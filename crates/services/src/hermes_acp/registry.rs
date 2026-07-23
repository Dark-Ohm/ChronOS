use super::transport::HermesConfig;

/// Descriptor for an ACP-compatible agent backend.
#[derive(Debug, Clone)]
pub struct AgentDescriptor {
    /// Stable identifier (e.g. "hermes", "cline").
    pub id: &'static str,
    /// Display name shown in the UI (e.g. "Hermes", "Cline").
    pub display_name: &'static str,
    /// Command + args to spawn this backend via ACP stdio.
    pub config: HermesConfig,
}

/// Returns the built-in list of known ACP-compatible agent backends.
///
/// Only backends that have been verified to implement the ACP stdio
/// protocol (correct InitializeRequest handshake, session creation,
/// prompt/response round-trip) are included. "Declared but not
/// tested" backends are NOT listed here — add them only after a live
/// ACP handshake succeeds.
pub fn known_agents() -> Vec<AgentDescriptor> {
    vec![
        AgentDescriptor {
            id: "hermes",
            display_name: "Hermes",
            config: HermesConfig::default(),
        },
    ]
}
