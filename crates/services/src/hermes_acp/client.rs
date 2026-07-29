use agent_client_protocol::{
    ActiveSession, Agent, ConnectionTo, SessionMessage, UntypedMessage,
    schema::v1::{
        ContentBlock, SessionNotification, SessionUpdate, ToolCallContent, ToolCallStatus,
    },
    util::MatchDispatch,
};
use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::{Arc, Mutex as StdMutex};
use tokio::sync::Mutex;
use tracing::{debug, info, warn};

/// Held ACP session, shared across command handlers (see transport D3).
pub(crate) type SharedSession = Arc<Mutex<Option<ActiveSession<'static, Agent>>>>;

/// T144 workaround: models intercepted from raw `session/new` response JSON.
///
/// Written by `with_debug` callback (sync Fn), read by `models_from_session`.
/// Separate from `SharedSession` because `with_debug` is `Fn + Send + Sync + 'static`
/// and cannot hold a `tokio::sync::Mutex` guard.
///
/// DELETE when upstream issue #301 is fixed and `ActiveSession.response()`
/// carries `config_options`.
pub(crate) type SharedModels = Arc<StdMutex<Option<SessionModels>>>;

/// Streaming event emitted during a prompt turn.
#[derive(Clone, Debug)]
pub enum StreamingEvent {
    /// Incremental text chunk from the agent.
    TextChunk(String),
    /// Incremental reasoning/thought chunk.
    ThoughtChunk(String),
    /// A tool call appeared or was updated.
    ToolCall {
        id: String,
        name: String,
        status: String,
        args: Option<String>,
        result: Option<String>,
    },
    /// Turn completed successfully.
    Done,
    /// Turn failed.
    Error(String),
}

/// ACP session info returned by session/list.
#[derive(Debug, Clone, Deserialize)]
pub struct SessionInfo {
    #[serde(rename = "sessionId")]
    pub session_id: String,
    pub cwd: Option<String>,
    pub title: Option<String>,
    #[serde(rename = "updatedAt")]
    pub updated_at: Option<String>,
}

use super::session::{AcpSession, ModelInfo, SessionMode, SessionModels, SessionModes};
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
        /// Optional streaming callback. When provided, events are emitted
        /// as the turn progresses (text chunks, tool calls, etc.).
        on_event: Option<tokio::sync::mpsc::UnboundedSender<StreamingEvent>>,
    },
    /// Switch the active model on the held session.
    SetModel {
        model_id: String,
        reply: tokio::sync::oneshot::Sender<Result<()>>,
    },
    /// List sessions known to the agent (session/list).
    ListSessions {
        cwd: Option<String>,
        reply: tokio::sync::oneshot::Sender<Result<Vec<SessionInfo>>>,
    },
    /// Load a session by ACP session id, replaying history as streaming events.
    LoadSession {
        acp_session_id: String,
        reply: tokio::sync::oneshot::Sender<Result<()>>,
        on_event: tokio::sync::mpsc::UnboundedSender<StreamingEvent>,
    },
}

/// Execute a command against the ACP connection, reusing `active` across turns.
///
/// Called from the transport's background task where `cx` is alive.
/// `session` is an `Arc<Mutex<...>>` shared across concurrently-spawned
/// command handlers (D3), so each branch locks it for the duration of the
/// call.
pub(crate) async fn execute_command(
    cmd: Command,
    cx: &ConnectionTo<Agent>,
    session: &SharedSession,
    intercepted_models: &SharedModels,
) {
    match cmd {
        Command::CreateSession { reply } => {
            let result = ensure_fresh_session(cx, session, intercepted_models).await;
            let _ = reply.send(result);
        }
        Command::SendPrompt {
            prompt,
            reply,
            on_event,
        } => {
            let result = if let Some(event_tx) = on_event {
                send_prompt_streaming(cx, session, &prompt, &event_tx, intercepted_models).await
            } else {
                send_prompt_on_active(cx, session, &prompt, intercepted_models).await
            };
            let _ = reply.send(result);
        }
        Command::SetModel { model_id, reply } => {
            let result = set_model_on_active(cx, session, &model_id).await;
            let _ = reply.send(result);
        }
        Command::ListSessions { cwd, reply } => {
            let result = list_sessions_command(cx, cwd.as_deref()).await;
            let _ = reply.send(result);
        }
        Command::LoadSession {
            acp_session_id,
            reply,
            on_event,
        } => {
            let result = load_session_command(cx, session, &acp_session_id, &on_event, intercepted_models).await;
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

fn models_from_session(
    session: &ActiveSession<'static, Agent>,
    intercepted: &SharedModels,
) -> Option<SessionModels> {
    // T144: upstream issue #301 — ActiveSession.response() discards
    // config_options. Until fixed we use the with_debug workaround.
    // When upstream fixes it, the workaround is removed and this function
    // reads config_options from session.response().
    {
        let resp = session.response();
        if let Some(opts) = &resp.config_options {
            debug!(count = opts.len(), "session config_options present (upstream fix)");
            // TODO T144: parse config_options into SessionModels
        }
    }
    if let Ok(guard) = intercepted.lock() {
        if let Some(models) = guard.as_ref() {
            debug!(
                current = %models.current_id,
                count = models.available.len(),
                "models_from_session: intercepted (workaround)"
            );
            return Some(models.clone());
        }
    }
    None
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

/// Whether a tool-call status means the tool has finished (so the D6 watchdog
/// may treat silence as suspicious again). Mirrors `mark_pending_tools_stale`
/// in the UI: done / error / stale / canceled / denied / expired.
fn is_terminal_status(status: &ToolCallStatus) -> bool {
    matches!(
        status,
        ToolCallStatus::Completed | ToolCallStatus::Failed
    )
}

/// T144 workaround: parse models from raw `session/new` JSON-RPC response.
///
/// Called from `transport.rs`'s `with_debug` Stdout handler on every line.
/// Returns quickly when the line doesn't look like a session/new result.
/// Stores parsed models in `dest` for `models_from_session`.
///
/// DELETE this function when upstream issue #301 is fixed.
pub(crate) fn intercept_session_models(line: &str, dest: &SharedModels) {
    if !line.contains("\"models\"") || !line.contains("\"availableModels\"") {
        return;
    }
    let Ok(val) = serde_json::from_str::<serde_json::Value>(line) else { return };
    let Some(result) = val.get("result") else { return };
    let Some(models_val) = result.get("models") else { return };
    let Some(available) = models_val.get("availableModels").and_then(|v| v.as_array()) else {
        return;
    };
    let current_id = models_val
        .get("currentModelId")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let effective_current = if current_id.is_empty() {
        available
            .first()
            .and_then(|m| m.get("modelId").and_then(|v| v.as_str()))
            .unwrap_or("")
    } else {
        current_id
    };
    let available_models: Vec<ModelInfo> = available
        .iter()
        .filter_map(|m| {
            let id = m.get("modelId")?.as_str()?;
            Some(ModelInfo {
                id: id.to_string(),
                name: m
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or(id)
                    .to_string(),
                description: m
                    .get("description")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
            })
        })
        .collect();
    if available_models.is_empty() {
        return;
    }
    if let Ok(mut guard) = dest.lock() {
        *guard = Some(SessionModels {
            current_id: effective_current.to_string(),
            available: available_models,
        });
        debug!(
            current = %effective_current,
            count = guard.as_ref().map(|m| m.available.len()).unwrap_or(0),
            "intercepted_session_models"
        );
    }
}

/// Read a full ACP turn, collecting text, thought chunks, and tool calls.
async fn read_turn(
    session: &mut ActiveSession<'static, Agent>,
) -> Result<(String, String, Vec<ToolCallInfo>)> {
    let mut text = String::new();
    let mut thought = String::new();
    let mut tools: HashMap<String, ToolCallInfo> = HashMap::new();

    // D6 (non-streaming path): same root cause as stream_read_turn — the
    // library never delivers StopReason via update_rx, so this loop would
    // hang forever after the response is physically complete. Apply the same
    // watchdog so the turn closes honestly instead of wedging.
    const TURN_COMPLETE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(15);
    // T147 (errata): same absolute deadline as the streaming path.
    const TURN_ABSOLUTE_DEADLINE: std::time::Duration = std::time::Duration::from_secs(1800);
    let turn_start = std::time::Instant::now();
    let mut saw_output = false;

    loop {
        if turn_start.elapsed() >= TURN_ABSOLUTE_DEADLINE {
            warn!(
                elapsed_s = turn_start.elapsed().as_secs_f64(),
                text_len = text.len(),
                "read_turn: absolute deadline hit — closing turn"
            );
            break;
        }
        let read = tokio::time::timeout(TURN_COMPLETE_TIMEOUT, session.read_update()).await;
        let update = match read {
            Ok(Ok(u)) => u,
            Ok(Err(_e)) => break, // channel closed — turn over
            Err(_elapsed) => {
                if saw_output {
                    warn!("read_turn: no further ACP update for {}s after output — closing turn (D6)", TURN_COMPLETE_TIMEOUT.as_secs());
                    break;
                }
                continue;
            }
        };
        match update {
            SessionMessage::SessionMessage(dispatch) => {
                saw_output = true;
                MatchDispatch::new(dispatch)
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
                            tracing::info!(tool_id = %id, title = ?update.fields.title, status = ?update.fields.status, raw_input = update.fields.raw_input.is_some(), raw_output = update.fields.raw_output.is_some(), content = update.fields.content.as_ref().map(|c| c.len()), "ACP raw: ToolCallUpdate");
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
                        other @ _ => {
                            // E5: this used to be a silent `_ => {}`, hiding any
                            // SessionUpdate we didn't explicitly handle. A dropped
                            // message here is exactly how a parser "looks broken"
                            // while the agent is just sending something we ignore.
                            tracing::debug!("read_turn: unhandled SessionUpdate variant: {other:?}");
                        }
                    }
                    Ok(())
                })
                .await
                .otherwise(|msg| async move {
                    // E5: `otherwise_ignore` (util/typed.rs:407) silently drops
                    // any message no handler matched. Make the drop audible.
                    tracing::debug!("read_turn: dropped ACP message (no handler matched): {msg:?}");
                    Ok(())
                })
                .await?
            }
            SessionMessage::StopReason(_) => break,
            other @ _ => {
                // E5: same treatment for the outer SessionMessage match.
                tracing::debug!("read_turn: unhandled SessionMessage variant: {other:?}");
            }
        }
    }

    // Sort tools by insertion order (HashMap doesn't preserve order, but
    // we don't have a sequence id — just return in arbitrary stable order).
    let tools_vec: Vec<ToolCallInfo> = tools.into_values().collect();
    Ok((text, thought, tools_vec))
}

/// Read a full ACP turn, emitting streaming events via `on_event` as they arrive.
/// Returns the accumulated text, thought, and tools when the turn completes.
async fn stream_read_turn(
    session: &mut ActiveSession<'static, Agent>,
    on_event: &tokio::sync::mpsc::UnboundedSender<StreamingEvent>,
) -> Result<(String, String, Vec<ToolCallInfo>)> {
    let mut text = String::new();
    let mut thought = String::new();
    let mut tools: HashMap<String, ToolCallInfo> = HashMap::new();

    // D6: the agent's stop reason (`stopReason: end_turn`) arrives on the
    // wire, but `agent-client-protocol` 0.11.1 does NOT surface it through
    // `read_update()`'s `update_rx` — it is returned from `send_prompt` /
    // `ProxySessionMessages` and consumed there, so `SessionMessage::StopReason`
    // never reaches this loop. The loop therefore hangs forever at
    // `read_update().await` even though the turn is physically over (observed
    // live: response present, panel never completes). We race each read
    // against a watchdog timeout measured from the LAST received update, so a
    // turn that has gone quiet (and already produced output) is closed
    // honestly instead of wedging the panel. A terminating `StopReason` still
    // wins via the timeout firing on silence.
    const TURN_COMPLETE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(120);
    // T147: absolute turn deadline, checked EVERY iteration (not just on
    // silence). A turn that streams updates continuously never enters the
    // 120s silence window, so the old check (inside Err(_elapsed) under
    // open_tools > 0) was a no-op for live turns. This is the real guard:
    // regardless of in-flight tools or streaming pace, the turn is closed
    // once it has run this long. 30 min allows long-running cargo builds
    // and similar agent actions while still providing an upper bound.
    const TURN_ABSOLUTE_DEADLINE: std::time::Duration = std::time::Duration::from_secs(1800);
    let turn_start = std::time::Instant::now();
    let mut saw_output = false;
    // Track in-flight tool calls. Silence while a tool is still running is NOT
    // suspicious — D6 must not close the turn under a live tool (bri f ЗАХОД 3).
    let mut open_tools: u32 = 0;
    // How many times the D6 window was extended because a tool looked live.
    let mut extensions: u32 = 0;

    loop {
        // T147: checked EVERY iteration, not just on silence.
        if turn_start.elapsed() >= TURN_ABSOLUTE_DEADLINE {
            warn!(
                elapsed_s = turn_start.elapsed().as_secs_f64(),
                open_tools,
                extensions,
                text_len = text.len(),
                "stream_read_turn: absolute deadline hit — closing turn"
            );
            break;
        }

        let read = tokio::time::timeout(TURN_COMPLETE_TIMEOUT, session.read_update()).await;
        let update = match read {
            Ok(Ok(u)) => u,
            Ok(Err(e)) => {
                warn!("stream_read_turn: read_update error ({e}) — ending turn");
                break;
            }
            Err(_elapsed) => {
                if open_tools > 0 {
                    extensions += 1;
                    tracing::debug!(
                        "stream_read_turn: {open_tools} tool(s) still in flight — \
                         extending D6 window ({extensions}), not closing turn"
                    );
                    continue;
                }
                if saw_output {
                    warn!(
                        "stream_read_turn: no further ACP update for {}s after output \
                         — closing turn (D6: StopReason not delivered via update_rx)",
                        TURN_COMPLETE_TIMEOUT.as_secs()
                    );
                    break;
                }
                continue;
            }
        };
        // D6 diagnostic: log that an update arrived (type logged via the
        // specific arms below) so a live smoke run can confirm the library
        // delivers chunks but never a StopReason through update_rx.
        tracing::debug!("stream_read_turn: update arrived");
        match update {
            SessionMessage::SessionMessage(dispatch) => {
                saw_output = true;
                MatchDispatch::new(dispatch)
                    .if_notification(async |notif: SessionNotification| {
                        match notif.update {
                            SessionUpdate::AgentMessageChunk(chunk) => {
                                if let ContentBlock::Text(t) = chunk.content {
                                    let delta = t.text.clone();
                                    text.push_str(&delta);
                                    if on_event.send(StreamingEvent::TextChunk(delta)).is_err() {
                                        return Ok(());
                                    }
                                }
                            }
                            SessionUpdate::AgentThoughtChunk(chunk) => {
                                if let ContentBlock::Text(t) = chunk.content {
                                    let delta = t.text.clone();
                                    thought.push_str(&delta);
                                    if on_event.send(StreamingEvent::ThoughtChunk(delta)).is_err() {
                                        return Ok(());
                                    }
                                }
                            }
                            SessionUpdate::ToolCall(tc) => {
                                let id = tc.tool_call_id.0.to_string();
                                tracing::info!(tool_id = %id, title = %tc.title, status = ?tc.status, raw_input = tc.raw_input.is_some(), "ACP raw: ToolCall");
                                // A tool just started — count it as in-flight so
                                // D6 does not close the turn while it runs.
                                open_tools += 1;
                                let entry = tools.entry(id.clone()).or_insert_with(|| ToolCallInfo {
                                    id,
                                    name: tc.title.clone(),
                                    status: status_string(&tc.status),
                                    args: tc.raw_input.as_ref().map(|v| {
                                        serde_json::to_string_pretty(v)
                                            .unwrap_or_else(|_| v.to_string())
                                    }),
                                    result: None,
                                });
                                if on_event
                                    .send(StreamingEvent::ToolCall {
                                        id: entry.id.clone(),
                                        name: entry.name.clone(),
                                        status: entry.status.clone(),
                                        args: entry.args.clone(),
                                        result: entry.result.clone(),
                                    })
                                    .is_err()
                                {
                                    return Ok(());
                                }
                            }
                            SessionUpdate::ToolCallUpdate(update) => {
                                let id = update.tool_call_id.0.to_string();
                                tracing::info!(tool_id = %id, title = ?update.fields.title, status = ?update.fields.status, raw_input = update.fields.raw_input.is_some(), raw_output = update.fields.raw_output.is_some(), content = update.fields.content.as_ref().map(|c| c.len()), "ACP raw: ToolCallUpdate");
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
                                    // Tool finished — drop it from the in-flight count.
                                    if is_terminal_status(&status) {
                                        open_tools = open_tools.saturating_sub(1);
                                    }
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
                                if on_event
                                    .send(StreamingEvent::ToolCall {
                                        id: entry.id.clone(),
                                        name: entry.name.clone(),
                                        status: entry.status.clone(),
                                        args: entry.args.clone(),
                                        result: entry.result.clone(),
                                    })
                                    .is_err()
                                {
                                    return Ok(());
                                }
                            }
                            other @ _ => {
                                // E5: was a silent `_ => {}`. Make unhandled
                                // SessionUpdate audible instead of looking broken.
                                tracing::debug!("stream_read_turn: unhandled SessionUpdate variant: {other:?}");
                            }
                        }
                        Ok(())
                    })
                    .await
                    .otherwise(|msg| async move {
                        // E5: `otherwise_ignore` (util/typed.rs:407) silently
                        // drops any message no handler matched. Make it audible.
                        tracing::debug!("stream_read_turn: dropped ACP message (no handler matched): {msg:?}");
                        Ok(())
                    })
                    .await?
        }
        SessionMessage::StopReason(_) => break,
        other @ _ => {
            // E5: same treatment for the outer SessionMessage match.
            tracing::debug!("stream_read_turn: unhandled SessionMessage variant: {other:?}");
        }
    }
    }

    let _ = on_event.send(StreamingEvent::Done);
    let tools_vec: Vec<ToolCallInfo> = tools.into_values().collect();
    Ok((text, thought, tools_vec))
}

/// Send prompt with streaming events emitted via `on_event`.
async fn send_prompt_streaming(
    cx: &ConnectionTo<Agent>,
    session: &SharedSession,
    prompt: &str,
    on_event: &tokio::sync::mpsc::UnboundedSender<StreamingEvent>,
    intercepted_models: &SharedModels,
) -> Result<PromptResponse> {
    // D3: lock the shared session ONLY to obtain/replace the handle.
    // The turn itself (stream_read_turn) runs OUTSIDE the lock so a long
    // turn cannot block CreateSession/SetModel. Previously the guard was
    // held for the whole turn (client.rs:424), which just moved the
    // bottleneck from the command channel into the mutex — observed
    // behavior unchanged. Also a likely contributor to D6: holding
    // &mut ActiveSession (and thus its update_rx channel) for the entire
    // turn interferes with the library's own prompt completion routing.
    let session_id;
    let modes;
    let models;
    let mut active = {
        let mut guard = session.lock().await;
        if guard.is_none() {
            debug!("No held session — creating before first prompt");
            let new_session = start_new_session(cx).await?;
            *guard = Some(new_session);
        }
        let session_ref = guard
            .as_mut()
            .context("internal: active session missing after ensure")?;
        session_id = session_ref.session_id().to_string();
        modes = modes_from_session(session_ref);
        models = models_from_session(session_ref, intercepted_models);
        // Take the session out of the shared slot and release the lock.
        guard.take().context("internal: active session missing on take")?
    };

    debug!(
        %session_id,
        "Sending prompt (streaming): {}",
        prompt.chars().take(80).collect::<String>()
    );

    if let Err(e) = active.send_prompt(prompt) {
        warn!(%session_id, "send_prompt failed: {e}; dropping session");
        return Err(anyhow::anyhow!("failed to send prompt: {e}"));
    }

    let (text, thought, tools) = match stream_read_turn(&mut active, on_event).await {
        Ok(result) => result,
        Err(e) => {
            warn!(%session_id, "read response failed: {e}");
            return Err(anyhow::anyhow!("failed to read response: {e}"));
        }
    };

    // Put the (still-alive) session back so it can be reused / modelled on.
    {
        let mut guard = session.lock().await;
        *guard = Some(active);
    }

    debug!(
        %session_id,
        "Streaming response complete ({} chars, {} tools)",
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

fn acp_session_meta(
    session: &ActiveSession<'static, Agent>,
    intercepted_models: &SharedModels,
) -> AcpSession {
    AcpSession::new(session.session_id().clone())
        .with_modes(modes_from_session(session))
        .with_models(models_from_session(session, intercepted_models))
}

async fn start_new_session(cx: &ConnectionTo<Agent>) -> Result<ActiveSession<'static, Agent>> {
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
    session: &SharedSession,
    intercepted_models: &SharedModels,
) -> Result<AcpSession> {
    let mut guard = session.lock().await;
    *guard = None;
    // T144: clear stale intercepted models — the new session/new response
    // will repopulate via with_debug. DELETE with workaround (issue #301).
    if let Ok(mut g) = intercepted_models.lock() {
        *g = None;
    }
    let new_session = start_new_session(cx).await?;
    let meta = acp_session_meta(&new_session, intercepted_models);
    *guard = Some(new_session);
    Ok(meta)
}

/// Send session/set_model on the active session.
///
/// Uses `UntypedMessage` because `SetSessionModelRequest` was removed from
/// ACP 2.0.0 schema (upstream dropped `session/set_model`). Hermes 0.18.2
/// still expects the old method name.
///
/// Send session/list to the agent via UntypedMessage (ACP 2.0.0 has no typed method).
async fn list_sessions_command(
    cx: &ConnectionTo<Agent>,
    cwd: Option<&str>,
) -> Result<Vec<SessionInfo>> {
    let mut params = serde_json::Map::new();
    if let Some(c) = cwd {
        params.insert("cwd".to_string(), serde_json::Value::String(c.to_string()));
    }
    let request = UntypedMessage::new("session/list", serde_json::Value::Object(params))?;

    let (tx, rx) = tokio::sync::oneshot::channel();
    cx.send_request_to(Agent, request)
        .on_receiving_result(async move |result| {
            let outcome = match result {
                Ok(value) => {
                    match serde_json::from_value::<Vec<SessionInfo>>(value) {
                        Ok(sessions) => {
                            debug!(count = sessions.len(), "session/list OK");
                            Ok(sessions)
                        }
                        Err(e) => {
                            warn!("session/list: failed to deserialize response: {e}");
                            Ok(Vec::new())
                        }
                    }
                }
                Err(e) => {
                    warn!("session/list failed: {e}");
                    Err(anyhow::anyhow!("list_sessions error: {e}"))
                }
            };
            let _ = tx.send(outcome);
            Ok(())
        })
        .context("failed to send session/list request")?;

    rx.await.context("list_sessions response channel closed")?
}

/// Send session/load to the agent. The agent replays history as session/update
/// events on the same connection — we handle them through the existing streaming
/// path (stream_read_turn). Because `load_session` doesn't create a new prompt
/// turn, we DON'T use stream_read_turn; we just hold the session so subsequent
/// prompts reuse it.
async fn load_session_command(
    cx: &ConnectionTo<Agent>,
    session: &SharedSession,
    acp_session_id: &str,
    on_event: &tokio::sync::mpsc::UnboundedSender<StreamingEvent>,
    intercepted_models: &SharedModels,
) -> Result<()> {
    let request = UntypedMessage::new(
        "session/load",
        serde_json::json!({"sessionId": acp_session_id}),
    )?;

    let session_id = acp_session_id.to_string();
    let (tx, rx) = tokio::sync::oneshot::channel();

    info!(%session_id, "Sending session/load");

    cx.send_request_to(Agent, request)
        .on_receiving_result(async move |result| {
            match result {
                Ok(_value) => {
                    info!(%session_id, "session/load OK");
                    let _ = tx.send(Ok(()));
                }
                Err(e) => {
                    warn!(%session_id, "session/load failed: {e}");
                    let _ = tx.send(Err(anyhow::anyhow!("load_session error: {e}")));
                }
            }
            Ok(())
        })
        .context("failed to send session/load request")?;

    rx.await.context("load_session response channel closed")??;

    // After loading, the agent replays the history as streaming events.
    // Read them via the same stream_read_turn mechanism used by send_prompt.
    let mut active = {
        let mut guard = session.lock().await;
        guard.take().context("no active session for load")?
    };

    let (_text, _thought, _tools) = stream_read_turn(&mut active, on_event).await?;

    {
        let mut guard = session.lock().await;
        *guard = Some(active);
    }

    Ok(())
}

/// DELETE when Hermes ships ACP 2.0.0-compatible model config options.
async fn set_model_on_active(
    _cx: &ConnectionTo<Agent>,
    session: &SharedSession,
    model_id: &str,
) -> Result<()> {
    let mut guard = session.lock().await;
    let session_ref = guard
        .as_mut()
        .context("no active session — create one first")?;

    let session_id = session_ref.session_id().to_string();
    let model_id_owned = model_id.to_string();
    let conn = session_ref.connection();

    let request = UntypedMessage::new(
        "session/set_model",
        serde_json::json!({
            "sessionId": session_id,
            "modelId": model_id_owned,
        }),
    )?;

    info!(%session_id, model_id = %model_id_owned, "Sending session/set_model");

    let (tx, rx) = tokio::sync::oneshot::channel();
    conn.send_request_to(Agent, request)
        .on_receiving_result(async move |result| {
            let outcome = match result {
                Ok(_value) => {
                    info!(%session_id, model_id = %model_id_owned, "session/set_model OK");
                    Ok(())
                }
                Err(e) => {
                    warn!(%session_id, model_id = %model_id_owned, "session/set_model failed: {e}");
                    Err(anyhow::anyhow!("set_model error: {e}"))
                }
            };
            let _ = tx.send(outcome);
            Ok(())
        })
        .context("failed to send session/set_model request")?;

    rx.await.context("set_model response channel closed")?
}

/// Ensure a session exists, send prompt, read full turn text.
async fn send_prompt_on_active(
    cx: &ConnectionTo<Agent>,
    session: &SharedSession,
    prompt: &str,
    intercepted_models: &SharedModels,
) -> Result<PromptResponse> {
    // D3: lock only to obtain/replace the handle; read the turn outside.
    let session_id;
    let modes;
    let models;
    let mut active = {
        let mut guard = session.lock().await;
        if guard.is_none() {
            debug!("No held session — creating before first prompt");
            let new_session = start_new_session(cx).await?;
            *guard = Some(new_session);
        }
        let session_ref = guard
            .as_mut()
            .context("internal: active session missing after ensure")?;
        session_id = session_ref.session_id().to_string();
        modes = modes_from_session(session_ref);
        models = models_from_session(session_ref, intercepted_models);
        guard.take().context("internal: active session missing on take")?
    };

    debug!(
        %session_id,
        "Sending prompt: {}",
        prompt.chars().take(80).collect::<String>()
    );

    if let Err(e) = active.send_prompt(prompt) {
        warn!(%session_id, "send_prompt failed: {e}; dropping session");
        return Err(anyhow::anyhow!("failed to send prompt: {e}"));
    }

    let (text, thought, tools) = match read_turn(&mut active).await {
        Ok(result) => result,
        Err(e) => {
            warn!(%session_id, "read response failed: {e}");
            return Err(anyhow::anyhow!("failed to read response: {e}"));
        }
    };

    {
        let mut guard = session.lock().await;
        *guard = Some(active);
    }

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
                on_event: None,
            })
            .context("command channel closed")?;
        rx.await.context("reply channel closed")?
    }

    /// Send a prompt with streaming events emitted via the channel.
    ///
    /// Returns the final `PromptResponse` when the turn completes.
    /// Events (text chunks, tool calls, etc.) are sent to `event_tx`
    /// as they arrive from the ACP agent.
    pub async fn send_prompt_streaming(
        &self,
        prompt: &str,
        event_tx: tokio::sync::mpsc::UnboundedSender<StreamingEvent>,
    ) -> Result<PromptResponse> {
        let (reply, rx) = tokio::sync::oneshot::channel();
        self.cmd_tx
            .send(Command::SendPrompt {
                prompt: prompt.to_string(),
                reply,
                on_event: Some(event_tx),
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

    /// List sessions known to the ACP agent, optionally filtered by cwd.
    pub async fn list_sessions(&self, cwd: Option<&str>) -> Result<Vec<SessionInfo>> {
        let (reply, rx) = tokio::sync::oneshot::channel();
        self.cmd_tx
            .send(Command::ListSessions {
                cwd: cwd.map(|s| s.to_string()),
                reply,
            })
            .context("command channel closed")?;
        rx.await.context("reply channel closed")?
    }

    /// Load an ACP session by id, replaying history as streaming events.
    /// Returns Ok(()) when the replay starts; events arrive on `event_tx`.
    pub async fn load_session(
        &self,
        acp_session_id: &str,
        event_tx: tokio::sync::mpsc::UnboundedSender<StreamingEvent>,
    ) -> Result<()> {
        let (reply, rx) = tokio::sync::oneshot::channel();
        self.cmd_tx
            .send(Command::LoadSession {
                acp_session_id: acp_session_id.to_string(),
                reply,
                on_event: event_tx,
            })
            .context("command channel closed")?;
        rx.await.context("reply channel closed")?
    }
}
