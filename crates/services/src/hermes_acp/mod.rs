pub mod client;
#[cfg(test)]
mod client_smoke;
pub mod registry;
pub mod session;
pub mod transport;

pub use client::{HermesClient, PromptResponse, StreamingEvent};
pub use registry::{AgentDescriptor, known_agents, load_shared_env};
pub use session::{AcpSession, ModelInfo, SessionMode, SessionModels, SessionModes};
pub use transport::{HermesConfig, HermesTransport};
