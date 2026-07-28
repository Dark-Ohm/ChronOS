use agent_client_protocol::schema::v1::{
    ClientCapabilities, FileSystemCapabilities, Implementation, InitializeRequest,
    PermissionOptionKind, ReadTextFileRequest, ReadTextFileResponse,
    RequestPermissionOutcome, RequestPermissionRequest, RequestPermissionResponse,
    SelectedPermissionOutcome, WriteTextFileRequest, WriteTextFileResponse,
};
use agent_client_protocol::schema::ProtocolVersion;
use agent_client_protocol::{AcpAgent, Agent, Client, ConnectionTo, Error as AcpError, LineDirection};
use anyhow::{Context, Result};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::oneshot;
use tracing::{debug, error, info, warn};

/// Log target for ACP permission auto-approvals.
const TARGET_PERM: &str = "chronos::acp::permission";
/// Log target for Hermes agent stderr (forwarded line-by-line in real time).
const TARGET_STDERR: &str = "hermes.stderr";

use super::client::{Command, SharedModels, SharedSession};
use std::sync::Mutex;

/// Configuration for spawning the Hermes agent process.
#[derive(Debug, Clone)]
pub struct HermesConfig {
    /// Command to spawn the Hermes agent (default: "hermes").
    pub command: String,
    /// Arguments to pass to the agent command.
    pub args: Vec<String>,
}

impl Default for HermesConfig {
    fn default() -> Self {
        Self {
            command: "hermes".to_string(),
            // --accept-hooks: shell has no TTY for hook prompts; without it
            // hermes may stall or exit when hooks fire mid-session.
            args: vec!["acp".to_string(), "--accept-hooks".to_string()],
        }
    }
}

/// Transport layer wrapping the ACP agent subprocess connection.
///
/// Manages the Hermes agent process lifecycle via the ACP SDK's
/// `AcpAgent` (subprocess spawn + stdio) and the `Client` builder.
/// The connection runs in a background tokio task; callers interact
/// through the channel-based `HermesTransport` handle.
pub struct HermesTransport {
    /// Handle to the background connection task. Dropping this aborts the task.
    _handle: tokio::task::JoinHandle<()>,
}

/// Held ACP session, shared across command handlers.
///
/// `D3`: commands used to be processed sequentially with a `&mut` borrow in
/// the connection loop, so a single stalled `SendPrompt` blocked
/// `CreateSession`/`SetModel`/subsequent prompts. We now spawn each command
/// as its own task and guard the session behind an `Arc<Mutex>` so they run
/// concurrently without data races.
/// (Type alias lives in `client.rs` as `SharedSession`.)

impl HermesTransport {
    /// Spawn the Hermes agent process, establish an ACP connection,
    /// and return a handle for sending commands.
    ///
    /// The connection is initialized (ACP protocol handshake) before returning.
    /// Commands are processed concurrently through the returned channel.
    pub(crate) async fn spawn(
        config: HermesConfig,
        shared_env: std::collections::HashMap<String, String>,
    ) -> Result<(Self, tokio::sync::mpsc::UnboundedSender<Command>)> {
        debug!("Spawning agent: {} {:?}", config.command, config.args);

        // Prepend shared env vars from ~/.config/chronos/.env.
        // AcpAgent::from_args treats leading KEY=value as env vars.
        let mut agent_args: Vec<String> = shared_env
            .iter()
            .map(|(k, v)| format!("{k}={v}"))
            .collect();
        agent_args.push(config.command.clone());
        agent_args.extend(config.args.iter().cloned());

        // D4: forward Hermes stderr line-by-line to tracing in real time so a
        // hung-but-alive agent is diagnosable instead of blind. The `debug`
        // callback fires for every stdio line (stdin/stdout/stderr).
        // `escalate_traceback` latches on when a Python `Traceback` starts and
        // stays set until the non-indented exception summary line — see E1
        // inside the Stderr arm below. Held in Arc so the `Fn` callback can
        // share it across invocations.
        let escalate_traceback = Arc::new(AtomicBool::new(false));
        // T144: shared models store, written by with_debug, read by commands.
        // DELETE with workaround when upstream issue #301 is fixed.
        let intercepted_models: SharedModels = Arc::new(Mutex::new(None));
        let debug_models = intercepted_models.clone();
        let agent = AcpAgent::from_args(agent_args)
            .map_err(|e| anyhow::anyhow!("failed to create ACP agent from args: {e}"))?
            .with_debug(move |line: &str, direction: LineDirection| {
                match direction {
                    LineDirection::Stdout => {
                        // stdout of the agent = protocol traffic; debug only.
                        tracing::debug!(target: TARGET_STDERR, "{line}");
                        // T144: intercept session/new models from raw JSON-RPC.
                        // DELETE when upstream issue #301 is fixed.
                        super::client::intercept_session_models(line, &debug_models);
                    }
                    LineDirection::Stdin => {
                        // our requests to the agent; debug only.
                        tracing::debug!(target: TARGET_STDERR, "→ {line}");
                    }
                    LineDirection::Stderr => {
                        // D4 (errata E1): at RUST_LOG=info a Traceback's first
                        // line is visible (warn) but the rest of the Python stack
                        // is logged at `debug` and lost. Once we see a `Traceback`
                        // line, escalate THAT line and every trailing stack line
                        // to `warn` until the block ends. A Python traceback ends
                        // at the first line with no leading whitespace
                        // (`ExceptionType: message`) — that line is also warned,
                        // and on it we drop the escalation flag.
                        //
                        // `with_debug` requires `Fn + Send + Sync + 'static`, so
                        // the latch lives in an `Arc<AtomicBool>` captured by value.
                        let latch = escalate_traceback.clone();
                        let lowered = line.to_ascii_lowercase();
                        if latch.load(Ordering::Relaxed) {
                            warn!(target: TARGET_STDERR, "{line}");
                            // A non-indented line ends the block (the exception
                            // summary, e.g. `ValueError: ...`). Drop the latch.
                            if !line.starts_with(' ') && !line.starts_with('\t') {
                                latch.store(false, Ordering::Relaxed);
                            }
                        } else if lowered.contains("error") || lowered.contains("traceback") {
                            latch.store(true, Ordering::Relaxed);
                            warn!(target: TARGET_STDERR, "{line}");
                        } else {
                            debug!(target: TARGET_STDERR, "{line}");
                        }
                    }
                }
            });
        let (cmd_tx, mut cmd_rx) = tokio::sync::mpsc::unbounded_channel::<Command>();
        let (conn_tx, conn_rx) = oneshot::channel::<ConnectionTo<Agent>>();

        // D3: shared session handle for concurrent command handlers.
        let active_session: SharedSession = Arc::new(tokio::sync::Mutex::new(None));

        let handle = tokio::spawn(async move {
            let result = Client
                .builder()
                .name("chronos-shell")
                .on_receive_request(
                    async move |request: RequestPermissionRequest, responder, _cx: ConnectionTo<Agent>| {
                        // Auto-approve: prefer AllowAlways > AllowOnce > first option.
                        let chosen = request
                            .options
                            .iter()
                            .find(|o| o.kind == PermissionOptionKind::AllowAlways)
                            .or_else(|| {
                                request
                                    .options
                                    .iter()
                                    .find(|o| o.kind == PermissionOptionKind::AllowOnce)
                            })
                            .or_else(|| request.options.first());

                        match chosen {
                            Some(opt) => {
                                info!(
                                    target: TARGET_PERM,
                                    tool = request.tool_call.fields.title.as_deref().unwrap_or("<unknown>"),
                                    option = %opt.name,
                                    "ACP permission auto-approved"
                                );
                                if let Err(e) = responder
                                    .respond(RequestPermissionResponse::new(
                                        RequestPermissionOutcome::Selected(
                                            SelectedPermissionOutcome::new(opt.option_id.clone()),
                                        ),
                                    ))
                                {
                                    warn!(target: TARGET_PERM, "respond failed: {e}");
                                }
                            }
                            None => {
                                warn!(
                                    target: TARGET_PERM,
                                    tool = request.tool_call.fields.title.as_deref().unwrap_or("<unknown>"),
                                    "ACP permission request has no options — cancelling"
                                );
                                if let Err(e) = responder
                                    .respond(RequestPermissionResponse::new(
                                        RequestPermissionOutcome::Cancelled,
                                    ))
                                {
                                    warn!(target: TARGET_PERM, "respond failed: {e}");
                                }
                            }
                        }

                        Ok::<(), AcpError>(())
                    },
                    agent_client_protocol::on_receive_request!(),
                )
                // D0: we advertise fs read/write so Hermes can delegate file
                // edits to us. We implement the handlers below. We intentionally
                // do NOT advertise `terminal` — we don't implement `terminal/*`.
                .on_receive_request(
                    async move |req: ReadTextFileRequest, responder, _cx: ConnectionTo<Agent>| {
                        handle_read_text_file(req, responder).await
                    },
                    agent_client_protocol::on_receive_request!(),
                )
                .on_receive_request(
                    async move |req: WriteTextFileRequest, responder, _cx: ConnectionTo<Agent>| {
                        handle_write_text_file(req, responder).await
                    },
                    agent_client_protocol::on_receive_request!(),
                )
                .connect_with(agent, async |cx| {
                    // D0: honest handshake. Announce what we actually serve.
                    let caps = ClientCapabilities::new()
                        .fs(
                            FileSystemCapabilities::new()
                                .read_text_file(true)
                                .write_text_file(true),
                        )
                        // We intentionally do NOT advertise `terminal` — we
                        // don't implement `terminal/*`. Advertised but
                        // unimplemented capabilities make the agent send
                        // requests we can only fail, killing the turn.
                        .terminal(false);
                    let client_info = Implementation::new("chronos-shell", env!("CARGO_PKG_VERSION"));

                    cx.send_request(
                        InitializeRequest::new(ProtocolVersion::V1)
                            .client_capabilities(caps)
                            .client_info(client_info),
                    )
                    .block_task()
                    .await
                    .map_err(|e| AcpError::internal_error().data(e.to_string()))?;

                    info!("ACP connection initialized with Hermes agent");

                    // Send the connection handle back to the caller.
                    let _ = conn_tx.send(cx.clone());

                    // D3: process commands concurrently.
                    while let Some(cmd) = cmd_rx.recv().await {
                        let cx = cx.clone();
                        let session = active_session.clone();
                        let im = intercepted_models.clone();
                        tokio::spawn(async move {
                            super::client::execute_command(cmd, &cx, &session, &im).await;
                        });
                    }

                    info!("Hermes ACP command channel closed (client dropped)");
                    Ok::<(), AcpError>(())
                })
                .await;

            if let Err(e) = result {
                error!("Hermes ACP connection terminated: {e:?}");
            }
        });

        // Wait for the connection to be established.
        conn_rx
            .await
            .context("failed to receive ACP connection handle")?;

        info!("Hermes ACP transport spawned");
        Ok((Self { _handle: handle }, cmd_tx))
    }

    /// Shut down the transport by aborting the background connection task.
    pub fn shutdown(&self) {
        self._handle.abort();
        info!("Hermes ACP transport shut down");
    }
}

impl Drop for HermesTransport {
    fn drop(&mut self) {
        self._handle.abort();
    }
}

/// D0: handle `fs/read_text_file` — read a UTF-8 text file and return its contents.
async fn handle_read_text_file(
    req: ReadTextFileRequest,
    responder: agent_client_protocol::Responder<ReadTextFileResponse>,
) -> Result<(), AcpError> {
    let path = req.path.clone();
    debug!(target: "chronos::acp::fs", path = %path.display(), "fs/read_text_file");
    // errata: run the blocking read on a blocking thread — std::fs in an async
    // handler pins the executor and can deadlock the ACP connection.
    let read = tokio::task::spawn_blocking({
        let path = path.clone();
        move || std::fs::read_to_string(&path)
    })
    .await;
    match read {
        Ok(Ok(content)) => {
            if let Err(e) = responder.respond(ReadTextFileResponse::new(content)) {
                warn!(target: "chronos::acp::fs", "respond failed: {e}");
            }
            Ok(())
        }
        Ok(Err(e)) => {
            warn!(target: "chronos::acp::fs", path = %path.display(), "fs/read_text_file failed: {e}");
            Err(AcpError::invalid_params().data(format!("failed to read {path:?}: {e}")))
        }
        Err(join_err) => {
            warn!(target: "chronos::acp::fs", "fs read task panicked: {join_err}");
            Err(AcpError::internal_error().data("fs read task failed"))
        }
    }
}

/// D0: handle `fs/write_text_file` — write UTF-8 content to a file, creating/
/// truncating it. Parent directories are created if missing.
async fn handle_write_text_file(
    req: WriteTextFileRequest,
    responder: agent_client_protocol::Responder<WriteTextFileResponse>,
) -> Result<(), AcpError> {
    let path = req.path.clone();
    debug!(target: "chronos::acp::fs", path = %path.display(), "fs/write_text_file");
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                warn!(target: "chronos::acp::fs", path = %path.display(), "fs/write_text_file mkdir failed: {e}");
                return Err(AcpError::invalid_params()
                    .data(format!("failed to create parent dir for {path:?}: {e}")));
            }
        }
    }
    // errata: do the blocking write on a blocking thread.
    let write = tokio::task::spawn_blocking({
        let path = path.clone();
        let content = req.content.clone();
        move || std::fs::write(&path, &content)
    })
    .await;
    match write {
        Ok(Ok(())) => {
            if let Err(e) = responder.respond(WriteTextFileResponse::new()) {
                warn!(target: "chronos::acp::fs", "respond failed: {e}");
            }
            Ok(())
        }
        Ok(Err(e)) => {
            warn!(target: "chronos::acp::fs", path = %path.display(), "fs/write_text_file failed: {e}");
            Err(AcpError::invalid_params().data(format!("failed to write {path:?}: {e}")))
        }
        Err(join_err) => {
            warn!(target: "chronos::acp::fs", "fs write task panicked: {join_err}");
            Err(AcpError::internal_error().data("fs write task failed"))
        }
    }
}
