use agent_client_protocol::schema::{InitializeRequest, ProtocolVersion};
use agent_client_protocol::{Agent, Client, ConnectionTo, Error as AcpError};
use agent_client_protocol_tokio::AcpAgent;
use anyhow::{Context, Result};
use std::str::FromStr;
use tokio::sync::oneshot;
use tracing::{debug, error, info};

use super::client::Command;

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
            args: vec!["acp".to_string()],
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

impl HermesTransport {
    /// Spawn the Hermes agent process, establish an ACP connection,
    /// and return a handle for sending commands.
    ///
    /// The connection is initialized (ACP protocol handshake) before returning.
    /// Commands are processed sequentially through the returned channel.
    pub(crate) async fn spawn(config: HermesConfig) -> Result<(Self, tokio::sync::mpsc::UnboundedSender<Command>)> {
        let command_str = format!("{} {}", config.command, config.args.join(" "));
        debug!("Spawning Hermes agent: {command_str}");

        let agent =
            AcpAgent::from_str(&command_str).context("failed to parse ACP agent command")?;

        let (cmd_tx, mut cmd_rx) = tokio::sync::mpsc::unbounded_channel::<Command>();
        let (conn_tx, conn_rx) = oneshot::channel::<ConnectionTo<Agent>>();

        let handle = tokio::spawn(async move {
            let result = Client
                .builder()
                .name("chronos-shell")
                .connect_with(agent, async |cx| {
                    // ACP protocol handshake: announce our supported version.
                    cx.send_request(InitializeRequest::new(ProtocolVersion::V1))
                        .block_task()
                        .await
                        .map_err(|e| AcpError::internal_error().data(e.to_string()))?;

                    info!("ACP connection initialized with Hermes agent");

                    // Send the connection handle back to the caller.
                    let _ = conn_tx.send(cx.clone());

                    // Process commands from the client.
                    while let Some(cmd) = cmd_rx.recv().await {
                        let _ = super::client::execute_command(cmd, &cx).await;
                    }

                    info!("Hermes ACP command channel closed");
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
