use agent_client_protocol::{Agent, ActiveSession, ConnectionTo};
use anyhow::{Context, Result};
use tracing::{debug, info, warn};

use super::session::{AcpSession, ModelInfo, SessionMode, SessionModes, SessionModels};
use super::transport::{HermesConfig, HermesTransport};

/// Response from send_prompt including session metadata.
pub struct PromptResponse {
    pub text: String,
    pub modes: Option<SessionModes>,
    pub models: Option<SessionModels>,
    /// ACP session id used for this turn (stable across prompts in one thread).
    pub session_id: String,
}

/// Commands sent from the client to the background connection task.
pub(crate) enum Command {
    /// Create or replace the held ACP session (UI "New session").
    CreateSession {
        reply: tokio::sync::oneshot::Sender<Result<AcpSession>>,
    },
    /// Prompt on the held session; creates one if missing.
    SendPrompt {
        prompt: String,
        reply: tokio::sync::oneshot::Sender<Result<PromptResponse>>,
    },
}

/// Execute a command against the ACP connection, reusing `active` across turns.
///
/// Called from the transport's background task where `cx` is alive.
pub(crate) async fn execute_command(
    cmd: Command,
    cx: &ConnectionTo<Agent>,
    active: &mut Option<ActiveSession<'static, Agent>>,
) {
    match cmd {
        Command::CreateSession { reply } => {
            let result = ensure_fresh_session(cx, active).await;
            let _ = reply.send(result);
        }
        Command::SendPrompt { prompt, reply } => {
            let result = send_prompt_on_active(cx, active, &prompt).await;
            let _ = reply.send(result);
        }
    }
}

fn modes_from_session(session: &ActiveSession<'static, Agent>) -> Option<SessionModes> {
    session.modes().as_ref().map(|m| SessionModes {
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
    })
}

fn models_from_session(session: &ActiveSession<'static, Agent>) -> Option<SessionModels> {
    // `models` on NewSessionResponse requires agent-client-protocol feature
    // `unstable_session_model` (enabled in crates/services Cargo.toml).
    let response = session.response();
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

fn acp_session_meta(session: &ActiveSession<'static, Agent>) -> AcpSession {
    AcpSession::new(session.session_id().clone())
        .with_modes(modes_from_session(session))
        .with_models(models_from_session(session))
}

async fn start_new_session(
    cx: &ConnectionTo<Agent>,
) -> Result<ActiveSession<'static, Agent>> {
    debug!("Starting new ACP session");
    let session = cx
        .build_session_cwd()
        .context("failed to build session")?
        .block_task()
        .start_session()
        .await
        .context("failed to start session")?;
    info!(session_id = %session.session_id(), "ACP session started");
    Ok(session)
}

/// Drop any held session and create a fresh one.
async fn ensure_fresh_session(
    cx: &ConnectionTo<Agent>,
    active: &mut Option<ActiveSession<'static, Agent>>,
) -> Result<AcpSession> {
    *active = None;
    let session = start_new_session(cx).await?;
    let meta = acp_session_meta(&session);
    *active = Some(session);
    Ok(meta)
}

/// Ensure a session exists, send prompt, read full turn text.
async fn send_prompt_on_active(
    cx: &ConnectionTo<Agent>,
    active: &mut Option<ActiveSession<'static, Agent>>,
    prompt: &str,
) -> Result<PromptResponse> {
    if active.is_none() {
        debug!("No held session — creating before first prompt");
        let session = start_new_session(cx).await?;
        *active = Some(session);
    }

    let session = active
        .as_mut()
        .context("internal: active session missing after ensure")?;

    let session_id = session.session_id().to_string();
    let modes = modes_from_session(session);
    let models = models_from_session(session);

    debug!(
        %session_id,
        "Sending prompt: {}",
        &prompt[..prompt.len().min(80)]
    );

    if let Err(e) = session.send_prompt(prompt) {
        warn!(%session_id, "send_prompt failed: {e}; dropping session");
        *active = None;
        return Err(anyhow::anyhow!("failed to send prompt: {e}"));
    }

    let text = match session.read_to_string().await {
        Ok(t) => t,
        Err(e) => {
            warn!(%session_id, "read response failed: {e}; dropping session");
            *active = None;
            return Err(anyhow::anyhow!("failed to read response: {e}"));
        }
    };

    debug!(
        %session_id,
        "Response received ({} chars)",
        text.len()
    );

    Ok(PromptResponse {
        text,
        modes,
        models,
        session_id,
    })
}

/// Client for communicating with an ACP agent (Hermes by default).
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

    /// Create a new ACP session with the agent (replaces any held session).
    pub async fn create_session(&self) -> Result<AcpSession> {
        let (reply, rx) = tokio::sync::oneshot::channel();
        self.cmd_tx
            .send(Command::CreateSession { reply })
            .context("command channel closed")?;
        rx.await.context("reply channel closed")?
    }

    /// Send a prompt on the held ACP session (creates one if none).
    ///
    /// Multi-turn: consecutive calls reuse the same session until
    /// [`create_session`] is called again or the agent drops the link.
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
