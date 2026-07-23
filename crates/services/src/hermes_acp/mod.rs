pub mod client;
pub mod registry;
pub mod session;
pub mod transport;

pub use client::{HermesClient, PromptResponse};
pub use registry::{AgentDescriptor, known_agents};
pub use session::{AcpSession, ModelInfo, SessionMode, SessionModes, SessionModels};
pub use transport::{HermesConfig, HermesTransport};
