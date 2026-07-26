use agent_client_protocol::{
    Agent, ActiveSession, ConnectionTo, SessionMessage,
    schema::{ContentBlock, SessionNotification, SessionUpdate, ToolCallContent, ToolCallStatus},
    util::MatchDispatch,
};
use anyhow::{Context, Result};
use std::collections::HashMap;
use tracing::{debug, info, warn};

use super::session::{AcpSession, ModelInfo, SessionMode, SessionModes, SessionModels};
use super::transport::{HermesConfig, HermesTransport};

/// Tool call info extracted from an ACP turn.
#[derive(Clone, Debug)]
pub struct ToolCallInfo {
    pub id: String,
    pub name: String,
    pub status: String,
    pub args: Option<String>,
    pub result: Option<String>,
}

/// Response from send_prompt including session metadata.
pub struct PromptResponse {
    pub text: String,
    pub thought: String,
    pub tools: Vec<ToolCallInfo>,
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
    /// Switch the active model on the held session.
    SetModel {
        model_id: String,
        reply: tokio::sync::oneshot::Sender<Result<()>>,
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
        Command::SetModel { model_id, reply } => {
            let result = set_model_on_active(cx, active, &model_id).await;
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
    let models = response.models.as_ref();
    if let Some(m) = models {
        debug!(
            current_model = %m.current_model_id,
            count = m.available_models.len(),
            "Session models available"
        );
        Some(SessionModels {
            current_id: m.current_model_id.to_string(),
            available: m
                .available_models
                .iter()
                .map(|model| {
                    debug!(
                        model_id = %model.model_id,
                        name = %model.name,
                        "  model entry"
                    );
                    ModelInfo {
                        id: model.model_id.to_string(),
                        name: model.name.clone(),
                        description: model.description.clone(),
                    }
                })
                .collect(),
        })
    } else {
        // Log config_options to see if Hermes puts model info there.
        if let Some(opts) = &response.config_options {
            debug!(
                config_option_count = opts.len(),
                "No session.models; config_options present"
            );
            for opt in opts {
                debug!(
                    opt_id = %opt.id,
                    opt_name = %opt.name,
                    "  config_option"
                );
            }
        } else {
            debug!("No session.models and no config_options");
        }
        None
    }
}

fn status_string(status: &ToolCallStatus) -> String {
    match status {
        ToolCallStatus::Pending => "pending".to_string(),
        ToolCallStatus::InProgress => "running".to_string(),
        ToolCallStatus::Completed => "done".to_string(),
        ToolCallStatus::Failed => "error".to_string(),
        _ => "unknown".to_string(),
    }
}

/// Read a full ACP turn, collecting text, thought chunks, and tool calls.
async fn read_turn(
    session: &mut ActiveSession<'static, Agent>,
) -> Result<(String, String, Vec<ToolCallInfo>)> {
    let mut text = String::new();
    let mut thought = String::new();
    let mut tools: HashMap<String, ToolCallInfo> = HashMap::new();

    loop {
        let update = session.read_update().await?;
        match update {
            SessionMessage::SessionMessage(dispatch) => MatchDispatch::new(dispatch)
                .if_notification(async |notif: SessionNotification| {
                    match notif.update {
                        SessionUpdate::AgentMessageChunk(chunk) => {
                            if let ContentBlock::Text(t) = chunk.content {
                                text.push_str(&t.text);
                            }
                        }
                        SessionUpdate::AgentThoughtChunk(chunk) => {
                            if let ContentBlock::Text(t) = chunk.content {
                                thought.push_str(&t.text);
                            }
                        }
                        SessionUpdate::ToolCall(tc) => {
                            let id = tc.tool_call_id.0.to_string();
                            tools.entry(id.clone()).or_insert_with(|| ToolCallInfo {
                                id,
                                name: tc.title,
                                status: status_string(&tc.status),
                                args: tc.raw_input.as_ref().map(|v| {
                                    serde_json::to_string_pretty(v)
                                        .unwrap_or_else(|_| v.to_string())
                                }),
                                result: None,
                            });
                        }
                        SessionUpdate::ToolCallUpdate(update) => {
                            let id = update.tool_call_id.0.to_string();
                            let entry = tools.entry(id).or_insert_with(|| ToolCallInfo {
                                id: update.tool_call_id.0.to_string(),
                                name: String::new(),
                                status: "pending".to_string(),
                                args: None,
                                result: None,
                            });
                            if let Some(title) = update.fields.title {
                                entry.name = title;
                            }
                            if let Some(status) = update.fields.status {
                                entry.status = status_string(&status);
                            }
                            if let Some(raw_input) = update.fields.raw_input {
                                entry.args = Some(
                                    serde_json::to_string_pretty(&raw_input)
                                        .unwrap_or_else(|_| raw_input.to_string()),
                                );
                            }
                            if let Some(raw_output) = update.fields.raw_output {
                                entry.result = Some(
                                    serde_json::to_string_pretty(&raw_output)
                                        .unwrap_or_else(|_| raw_output.to_string()),
                                );
                            }
                            // Extract text result from content if no raw_output.
                            if entry.result.is_none() {
                                if let Some(content) = update.fields.content {
                                    for item in content {
                                        if let ToolCallContent::Content(c) = item {
                                            if let ContentBlock::Text(t) = c.content {
                                                entry.result = Some(t.text);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                    Ok(())
                })
                .await
                .otherwise_ignore()?,
            SessionMessage::StopReason(_) => break,
            _ => {}
        }
    }

    // Sort tools by insertion order (HashMap doesn't preserve order, but
    // we don't have a sequence id — just return in arbitrary stable order).
    let tools_vec: Vec<ToolCallInfo> = tools.into_values().collect();
    Ok((text, thought, tools_vec))
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

/// Send session/set_model on the active session.
async fn set_model_on_active(
    _cx: &ConnectionTo<Agent>,
    active: &mut Option<ActiveSession<'static, Agent>>,
    model_id: &str,
) -> Result<()> {
    let session = active
        .as_mut()
        .context("no active session — create one first")?;

    let session_id = session.session_id().to_string();
    let model_id_owned = model_id.to_string();
    let conn = session.connection();
    let request = agent_client_protocol::schema::SetSessionModelRequest::new(
        session_id.clone(),
        model_id_owned.clone(),
    );

    info!(%session_id, %model_id_owned, "Sending set_model");

    let (tx, rx) = tokio::sync::oneshot::channel();
    conn.send_request_to(Agent, request)
        .on_receiving_result(async move |result| {
            let outcome = match result {
                Ok(_response) => {
                    info!(%session_id, %model_id_owned, "set_model OK");
                    Ok(())
                }
                Err(e) => {
                    warn!(%session_id, %model_id_owned, "set_model failed: {e}");
                    Err(anyhow::anyhow!("set_model error: {e}"))
                }
            };
            let _ = tx.send(outcome);
            Ok(())
        })
        .context("failed to send set_model request")?;

    rx.await.context("set_model response channel closed")?
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

    let (text, thought, tools) = match read_turn(session).await {
        Ok(result) => result,
        Err(e) => {
            warn!(%session_id, "read response failed: {e}; dropping session");
            *active = None;
            return Err(anyhow::anyhow!("failed to read response: {e}"));
        }
    };

    debug!(
        %session_id,
        "Response received ({} chars, {} tools)",
        text.len(),
        tools.len()
    );

    Ok(PromptResponse {
        text,
        thought,
        tools,
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
    pub async fn new(
        config: HermesConfig,
        shared_env: std::collections::HashMap<String, String>,
    ) -> Result<Self, anyhow::Error> {
        let (transport, cmd_tx) = HermesTransport::spawn(config, shared_env).await?;
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

    /// Switch the active model on the held ACP session.
    pub async fn set_model(&self, model_id: &str) -> Result<()> {
        let (reply, rx) = tokio::sync::oneshot::channel();
        self.cmd_tx
            .send(Command::SetModel {
                model_id: model_id.to_string(),
                reply,
            })
            .context("command channel closed")?;
        rx.await.context("reply channel closed")?
    }
}
