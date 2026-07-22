use agent_client_protocol::{Agent, ConnectionTo};
use tracing::debug;

use super::session::AcpSession;
use super::transport::HermesTransport;

/// Commands sent from the client to the background connection task.
pub(crate) enum Command {
    CreateSession {
        reply: tokio::sync::oneshot::Sender<Result<AcpSession, String>>,
    },
    SendPrompt {
        prompt: String,
        reply: tokio::sync::oneshot::Sender<Result<String, String>>,
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

async fn create_session(cx: &ConnectionTo<Agent>) -> Result<AcpSession, String> {
    debug!("Creating ACP session");
    let active_session = cx
        .build_session_cwd()
        .map_err(|e| format!("failed to build session: {e}"))?
        .block_task()
        .start_session()
        .await
        .map_err(|e| format!("failed to start session: {e}"))?;

    let session_id = active_session.session_id().clone();
    debug!("ACP session created: {session_id}");

    // Spawn a background task to keep the session alive.
    cx.spawn(async move {
        let _active = active_session;
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(3600)).await;
        }
    })
    .map_err(|e| format!("failed to spawn session task: {e}"))?;

    Ok(AcpSession::new(session_id))
}

async fn send_prompt(cx: &ConnectionTo<Agent>, prompt: &str) -> Result<String, String> {
    debug!("Creating session for prompt");

    let mut active_session = cx
        .build_session_cwd()
        .map_err(|e| format!("failed to build session: {e}"))?
        .block_task()
        .start_session()
        .await
        .map_err(|e| format!("failed to start session: {e}"))?;

    debug!("Sending prompt: {}", &prompt[..prompt.len().min(80)]);
    active_session
        .send_prompt(prompt)
        .map_err(|e| format!("failed to send prompt: {e}"))?;

    debug!("Reading response");
    let response = active_session
        .read_to_string()
        .await
        .map_err(|e| format!("failed to read response: {e}"))?;

    debug!("Response received ({} chars)", response.len());
    Ok(response)
}

/// Client for communicating with the Hermes agent via ACP.
pub struct HermesClient {
    _transport: HermesTransport,
    cmd_tx: tokio::sync::mpsc::UnboundedSender<Command>,
}

impl HermesClient {
    /// Create a new client, spawning the Hermes agent process.
    pub async fn new() -> Result<Self, anyhow::Error> {
        let (transport, cmd_tx) = HermesTransport::spawn(Default::default()).await?;
        Ok(Self {
            _transport: transport,
            cmd_tx,
        })
    }

    /// Create a new ACP session with the agent.
    pub async fn create_session(&self) -> Result<AcpSession, String> {
        let (reply, rx) = tokio::sync::oneshot::channel();
        self.cmd_tx
            .send(Command::CreateSession { reply })
            .map_err(|_| "command channel closed".to_string())?;
        rx.await.map_err(|_| "reply channel closed".to_string())?
    }

    /// Send a prompt to the agent and return the response text.
    ///
    /// Creates a new session for each prompt (stateless).
    pub async fn send_prompt(&self, prompt: &str) -> Result<String, String> {
        let (reply, rx) = tokio::sync::oneshot::channel();
        self.cmd_tx
            .send(Command::SendPrompt {
                prompt: prompt.to_string(),
                reply,
            })
            .map_err(|_| "command channel closed".to_string())?;
        rx.await.map_err(|_| "reply channel closed".to_string())?
    }
}
