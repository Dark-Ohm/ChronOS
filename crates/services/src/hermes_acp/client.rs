use agent_client_protocol::{Agent, ConnectionTo};
use anyhow::{Context, Result};
use tracing::debug;

use super::session::AcpSession;
use super::transport::{HermesConfig, HermesTransport};

/// Commands sent from the client to the background connection task.
pub(crate) enum Command {
    CreateSession {
        reply: tokio::sync::oneshot::Sender<Result<AcpSession>>,
    },
    SendPrompt {
        prompt: String,
        reply: tokio::sync::oneshot::Sender<Result<String>>,
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

    // Spawn a background task to keep the session alive.
    cx.spawn(async move {
        let _active = active_session;
        // Keep the task alive forever (pending never resolves).
        Ok::<(), agent_client_protocol::Error>(std::future::pending::<()>().await)
    })
    .context("failed to spawn session task")?;

    Ok(AcpSession::new(session_id))
}

async fn send_prompt(cx: &ConnectionTo<Agent>, prompt: &str) -> Result<String> {
    debug!("Creating session for prompt");

    let mut active_session = cx
        .build_session_cwd()
        .context("failed to build session")?
        .block_task()
        .start_session()
        .await
        .context("failed to start session")?;

    debug!("Sending prompt: {}", &prompt[..prompt.len().min(80)]);
    active_session
        .send_prompt(prompt)
        .context("failed to send prompt")?;

    debug!("Reading response");
    let response = active_session
        .read_to_string()
        .await
        .context("failed to read response")?;

    debug!("Response received ({} chars)", response.len());
    Ok(response)
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

    /// Send a prompt to the agent and return the response text.
    ///
    /// Creates a new session for each prompt (stateless).
    pub async fn send_prompt(&self, prompt: &str) -> Result<String> {
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
