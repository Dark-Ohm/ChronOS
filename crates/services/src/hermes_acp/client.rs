use agent_client_protocol::{Agent, ConnectionTo};
use anyhow::{Context, Result};
use tracing::debug;

use super::session::{AcpSession, ModelInfo, SessionMode, SessionModes, SessionModels};
use super::transport::{HermesConfig, HermesTransport};

/// Response from send_prompt including session metadata.
pub struct PromptResponse {
    pub text: String,
    pub modes: Option<SessionModes>,
    pub models: Option<SessionModels>,
}

/// Commands sent from the client to the background connection task.
pub(crate) enum Command {
    CreateSession {
        reply: tokio::sync::oneshot::Sender<Result<AcpSession>>,
    },
    SendPrompt {
        prompt: String,
        reply: tokio::sync::oneshot::Sender<Result<PromptResponse>>,
    },
}

/// Execute a command against the ACP connection context.
///
/// Called from the transport's background task where `cx` is alive.
pub(crate) async fn execute_command(cmd: Command, cx: &ConnectionTo<Agent>) {
    match cmd {
        Command::CreateSession { reply } => {
            let result = create_session(cx).await;
            let _ = reply.send(result);
        }
        Command::SendPrompt { prompt, reply } => {
            let result = send_prompt(cx, &prompt).await;
            let _ = reply.send(result);
        }
    }
}

async fn create_session(cx: &ConnectionTo<Agent>) -> Result<AcpSession> {
    debug!("Creating ACP session");
    let active_session = cx
        .build_session_cwd()
        .context("failed to build session")?
        .block_task()
        .start_session()
        .await
        .context("failed to start session")?;

    let session_id = active_session.session_id().clone();
    debug!("ACP session created: {session_id}");

    // Extract modes and models from the session response.
    let response = active_session.response();
    let modes = response.modes.as_ref().map(|m| SessionModes {
        current_id: m.current_mode_id.to_string(),
        available: m
            .available_modes
            .iter()
            .map(|mode| SessionMode {
                id: mode.id.to_string(),
                name: mode.name.clone(),
                description: mode.description.clone(),
            })
            .collect(),
    });
    debug!("Session modes: {:?}", modes.as_ref().map(|m| m.available.len()));

    let models = {
        #[cfg(feature = "unstable_session_model")]
        {
            response.models.as_ref().map(|m| SessionModels {
                current_id: m.current_model_id.to_string(),
                available: m
                    .available_models
                    .iter()
                    .map(|model| ModelInfo {
                        id: model.model_id.to_string(),
                        name: model.name.clone(),
                        description: model.description.clone(),
                    })
                    .collect(),
            })
        }
        #[cfg(not(feature = "unstable_session_model"))]
        {
            None
        }
    };
    debug!("Session models: {:?}", models.as_ref().map(|m: &SessionModels| m.available.len()));

    // Spawn a background task to keep the session alive.
    cx.spawn(async move {
        let _active = active_session;
        // Keep the task alive forever (pending never resolves).
        Ok::<(), agent_client_protocol::Error>(std::future::pending::<()>().await)
    })
    .context("failed to spawn session task")?;

    Ok(AcpSession::new(session_id)
        .with_modes(modes)
        .with_models(models))
}

async fn send_prompt(cx: &ConnectionTo<Agent>, prompt: &str) -> Result<PromptResponse> {
    debug!("Creating session for prompt");

    let mut active_session = cx
        .build_session_cwd()
        .context("failed to build session")?
        .block_task()
        .start_session()
        .await
        .context("failed to start session")?;

    // Extract modes and models from the session response.
    let response = active_session.response();
    let modes = response.modes.as_ref().map(|m| SessionModes {
        current_id: m.current_mode_id.to_string(),
        available: m
            .available_modes
            .iter()
            .map(|mode| SessionMode {
                id: mode.id.to_string(),
                name: mode.name.clone(),
                description: mode.description.clone(),
            })
            .collect(),
    });

    let models = {
        #[cfg(feature = "unstable_session_model")]
        {
            response.models.as_ref().map(|m| SessionModels {
                current_id: m.current_model_id.to_string(),
                available: m
                    .available_models
                    .iter()
                    .map(|model| ModelInfo {
                        id: model.model_id.to_string(),
                        name: model.name.clone(),
                        description: model.description.clone(),
                    })
                    .collect(),
            })
        }
        #[cfg(not(feature = "unstable_session_model"))]
        {
            None
        }
    };

    debug!("Sending prompt: {}", &prompt[..prompt.len().min(80)]);
    active_session
        .send_prompt(prompt)
        .context("failed to send prompt")?;

    debug!("Reading response");
    let text = active_session
        .read_to_string()
        .await
        .context("failed to read response")?;

    debug!("Response received ({} chars)", text.len());
    Ok(PromptResponse { text, modes, models })
}

/// Client for communicating with the Hermes agent via ACP.
#[derive(Clone)]
pub struct HermesClient {
    _transport: std::sync::Arc<HermesTransport>,
    cmd_tx: tokio::sync::mpsc::UnboundedSender<Command>,
}

impl HermesClient {
    /// Create a new client, spawning the agent process with the given config.
    pub async fn new(config: HermesConfig) -> Result<Self, anyhow::Error> {
        let (transport, cmd_tx) = HermesTransport::spawn(config).await?;
        Ok(Self {
            _transport: std::sync::Arc::new(transport),
            cmd_tx,
        })
    }

    /// Create a new ACP session with the agent.
    pub async fn create_session(&self) -> Result<AcpSession> {
        let (reply, rx) = tokio::sync::oneshot::channel();
        self.cmd_tx
            .send(Command::CreateSession { reply })
            .context("command channel closed")?;
        rx.await.context("reply channel closed")?
    }

    /// Send a prompt to the agent and return the response with session metadata.
    ///
    /// Creates a new session for each prompt (stateless).
    pub async fn send_prompt(&self, prompt: &str) -> Result<PromptResponse> {
        let (reply, rx) = tokio::sync::oneshot::channel();
        self.cmd_tx
            .send(Command::SendPrompt {
                prompt: prompt.to_string(),
                reply,
            })
            .context("command channel closed")?;
        rx.await.context("reply channel closed")?
    }
}
