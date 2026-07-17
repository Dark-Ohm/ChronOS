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
    #[zbus(property)]
    fn active_connections(&self) -> zbus::Result<Vec<zbus::zvariant::OwnedObjectPath>>;
}

#[proxy(
    interface = "org.freedesktop.NetworkManager.Connection.Active",
    default_service = "org.freedesktop.NetworkManager"
)]
trait ActiveConnection {
    #[zbus(property)]
    fn type_(&self) -> zbus::Result<String>;
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

/// Honest wired-ethernet detection.
///
/// An active NetworkManager connection whose `Type` equals `"802-3-ethernet"`
/// means a real wired link is up. This replaces the bar widget's guess
/// (`Full && ssid.is_none()`), which falsely fired whenever Wi-Fi was absent
/// but connectivity was full (e.g. a disabled/never-configured radio).
async fn detect_wired(conn: &Connection) -> bool {
    let Ok(mgr) = NetworkManagerProxy::new(conn).await else {
        return false;
    };
    let Ok(active_paths) = mgr.active_connections().await else {
        return false;
    };
    for path in active_paths {
        let Some(builder) = ActiveConnectionProxy::builder(conn)
            .destination("org.freedesktop.NetworkManager")
            .ok()
            .and_then(|b| b.path(path).ok())
        else {
            continue;
        };
        let Ok(proxy) = builder.build().await else {
            continue;
        };
        if let Ok(t) = proxy.type_().await {
            if t == "802-3-ethernet" {
                return true;
            }
        }
    }
    false
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
    /// Quiet-network heartbeat: a still-alive bus that emits no connectivity
    /// changes is NORMAL and must not trigger a reconnect. We only treat the
    /// connection as dead if an explicit ping fails.
    const HEARTBEAT: Duration = Duration::from_secs(30);
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
                let wired = detect_wired(&conn).await;
                data.set(NetworkData {
                    connectivity,
                    wired,
                    ..NetworkData::default()
                });
                status.set(ServiceStatus::Available);
                conn_slot.set(Some(conn.clone()));
                info!("NetworkSubscriber connected");

                // Subscribe to connectivity property changes. A quiet network
                // emits no changes — that is NORMAL, so there is NO idle
                // timeout here (the previous ~60s timeout made the subscriber
                // flap: "signal timeout; retrying" every minute on a silent
                // bus). The heartbeat below pings the bus periodically; only a
                // failed ping counts as a real disconnect.
                let stream = mgr.receive_connectivity_changed().await;
                let mut stream = std::pin::pin!(stream);
                loop {
                    tokio::select! {
                        changed = stream.next() => {
                            match changed {
                                Some(_) => {
                                    if let Ok(c) = mgr.connectivity().await {
                                        let connectivity = map_connectivity(c);
                                        let wired = detect_wired(&conn).await;
                                        let mut d = data.get_cloned();
                                        d.connectivity = connectivity;
                                        d.wired = wired;
                                        data.set(d);
                                    }
                                }
                                None => break, // stream ended cleanly
                            }
                        }
                        _ = tokio::time::sleep(HEARTBEAT) => {
                            if mgr.connectivity().await.is_err() {
                                warn!("NetworkSubscriber heartbeat failed; reconnecting");
                                break;
                            }
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
