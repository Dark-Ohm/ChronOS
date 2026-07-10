//! Network service via NetworkManager (D-Bus, system bus).
//!
//! ASYNC TEMPLATE (spec §5.1): `new()` captures `Handle::current()` and
//! `tokio::spawn`s a connect+retry loop. Requires `init_all()` to be called
//! inside the runtime (rt.block_on) — see spec §7.

use std::time::Duration;

use futures_signals::signal::{Mutable, Signal};
use futures_util::stream::StreamExt;
use tokio::runtime::Handle;
use tracing::{info, warn};
use zbus::{Connection, proxy};

use crate::Service;
use crate::ServiceStatus;
pub use types::{ConnectivityState, NetworkData};

pub mod types;

#[proxy(
    interface = "org.freedesktop.NetworkManager",
    default_service = "org.freedesktop.NetworkManager",
    default_path = "/org/freedesktop/NetworkManager"
)]
trait NetworkManager {
    #[zbus(property)]
    fn connectivity(&self) -> zbus::Result<u32>;
}

fn map_connectivity(c: u32) -> ConnectivityState {
    match c {
        4 => ConnectivityState::Full,
        3 => ConnectivityState::Limited,
        2 => ConnectivityState::Portal,
        1 => ConnectivityState::None,
        _ => ConnectivityState::Unknown,
    }
}

#[derive(Clone)]
pub struct NetworkSubscriber {
    data: Mutable<NetworkData>,
    status: Mutable<ServiceStatus>,
    /// Stored so future command methods (`connect`/`disconnect`, when wired in
    /// a follow-up spec) can reuse the live D-Bus connection instead of
    /// re-connecting. Currently unused — retained per plan.
    #[allow(dead_code)]
    conn: Mutable<Option<Connection>>,
}

impl NetworkSubscriber {
    /// Non-failing, synchronous constructor (spec §5.1). Starts in
    /// `Initializing`, spawns the async connect+retry loop, returns `Self`.
    ///
    /// # Panics
    ///
    /// Panics if called outside a tokio runtime — `Handle::current()` requires
    /// one. `init_all()` (spec §7) calls this inside `rt.block_on`, so the
    /// runtime is present there.
    pub fn new() -> Self {
        let data = Mutable::new(NetworkData::default());
        let status = Mutable::new(ServiceStatus::Initializing);
        let conn = Mutable::new(None);

        // `Handle::current()` panics if there is no tokio runtime. `init_all()`
        // (spec §7) calls this inside `rt.block_on`, so the runtime is present.
        // We don't use the handle directly (the spawned future already runs in
        // the runtime), but the call is the documented guard against calling
        // `new()` outside a runtime.
        let _handle = Handle::current();
        tokio::spawn(run(data.clone(), status.clone(), conn.clone()));

        Self { data, status, conn }
    }

    pub async fn connect(&self, _ssid: &str, _password: &str) -> anyhow::Result<()> {
        // Deferred: real NetworkManager AddAndActivateConnection wiring lands in a
        // separate spec. Stubbed so the method exists and compiles.
        anyhow::bail!("NetworkSubscriber::connect deferred to a follow-up spec")
    }

    pub async fn disconnect(&self) -> anyhow::Result<()> {
        anyhow::bail!("NetworkSubscriber::disconnect deferred to a follow-up spec")
    }
}

impl Service for NetworkSubscriber {
    type Data = NetworkData;
    type Error = anyhow::Error;
    fn subscribe(&self) -> impl Signal<Item = NetworkData> + Unpin + 'static {
        self.data.signal_cloned()
    }
    fn get(&self) -> NetworkData {
        self.data.get_cloned()
    }
    fn status(&self) -> ServiceStatus {
        self.status.get_cloned()
    }
}

/// Async connect + retry loop (spec §5.1). Exponential backoff 1s→2s→…→60s,
/// infinite retries. Internal `connect_timeout` (~5s) so a hung bus never
/// blocks the loop. A live `Disconnected` flips to `Unavailable` and retries.
///
/// Runs inside a `tokio::spawn` (see `new`), so the tokio runtime context is
/// already present — no `Handle::enter` needed for `tokio::time::sleep`.
async fn run(
    data: Mutable<NetworkData>,
    status: Mutable<ServiceStatus>,
    conn_slot: Mutable<Option<Connection>>,
) {
    const MAX_BACKOFF: Duration = Duration::from_secs(60);
    const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
    let mut backoff = Duration::from_secs(1);

    loop {
        let connect = async {
            let conn = Connection::system().await?;
            let mgr = NetworkManagerProxy::new(&conn).await?;
            let connectivity = mgr
                .connectivity()
                .await
                .map(map_connectivity)
                .unwrap_or(ConnectivityState::Unknown);
            Ok::<_, anyhow::Error>((conn, mgr, connectivity))
        };

        match tokio::time::timeout(CONNECT_TIMEOUT, connect).await {
            Ok(Ok((conn, mgr, connectivity))) => {
                data.set(NetworkData {
                    connectivity,
                    ..NetworkData::default()
                });
                status.set(ServiceStatus::Available);
                conn_slot.set(Some(conn));
                info!("NetworkSubscriber connected");

                // Subscribe to connectivity property changes; on stream error, break to retry.
                let stream = mgr.receive_connectivity_changed().await;
                let stream = std::pin::pin!(stream);
                let mut stream = std::pin::pin!(stream);
                loop {
                    match tokio::time::timeout(CONNECT_TIMEOUT, stream.next()).await {
                        Ok(Some(_)) => {
                            if let Ok(c) = mgr.connectivity().await {
                                let connectivity = map_connectivity(c);
                                data.set(NetworkData {
                                    connectivity,
                                    ..data.get_cloned()
                                });
                            }
                        }
                        Ok(None) => break, // stream ended cleanly
                        Err(_) => {
                            warn!("NetworkSubscriber signal timeout; retrying");
                            break;
                        }
                    }
                }
                status.set(ServiceStatus::Unavailable);
            }
            Ok(Err(e)) => {
                warn!("NetworkSubscriber connect failed, retrying: {e:?}");
                status.set(ServiceStatus::Unavailable);
            }
            Err(_) => {
                warn!("NetworkSubscriber connect timed out, retrying");
                status.set(ServiceStatus::Unavailable);
            }
        }

        // Ensure we are in the runtime for the sleep (handle is captured above).
        tokio::time::sleep(backoff).await;
        backoff = (backoff * 2).min(MAX_BACKOFF);
    }
}
