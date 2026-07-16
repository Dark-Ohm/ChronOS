//! org.freedesktop.Notifications — D-Bus **server** (daemon) for ChronOS.
//!
//! This is the FIRST server-side zbus use in ChronOS (all other services are
//! clients). It is written from scratch against the FDO notification spec and
//! our own `Service` trait. The donor (`reference/gpui-shell-main`) is
//! **unlicensed** (all-rights-reserved), so this is a rewrite-by-pattern:
//! the FDO method/signal *signatures* are the spec (not copyrightable), the
//! *logic* is ours. Zero lines copied.
//!
//! Architecture mirrors `upower`/`network`: `new()` is a non-failing sync
//! constructor that captures `Handle::current()` (panicking if called outside
//! a tokio runtime — same guard) and spawns the connect/serve loop. `init_all()`
//! (crates/services/src/lib.rs) calls this inside `rt.block_on`.

use std::sync::Arc;
use std::sync::Mutex as StdMutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use enumflags2::BitFlags;
use futures_signals::signal::{Mutable, Signal};
use tokio::runtime::Handle;
use tokio::sync::Notify;
use tracing::{info, warn};
use zbus::fdo::RequestNameFlags;
use zbus::object_server::SignalEmitter;
use zbus::{Connection, interface};

use crate::Service;
use crate::ServiceStatus;
pub use types::{
    CloseReason, Notification, NotificationState, Urgency,
};
pub mod types;

/// Well-known name we claim on the session bus.
const WELL_KNOWN_NAME: &str = "org.freedesktop.Notifications";
/// Object path we register the interface at.
const OBJECT_PATH: &str = "/org/freedesktop/Notifications";

/// Default lifetime for notifications when the caller passes `expire_timeout <= 0`.
const DEFAULT_EXPIRE_MS: u64 = 5_000;
/// Sentinel: caller wants the daemon default (FDO `-1` convention).
const EXPIRE_USE_DEFAULT: i32 = -1;
/// Sentinel: caller wants a sticky (never-expiring) notification.
const EXPIRE_NEVER: i32 = 0;

/// Capabilities we advertise via `GetCapabilities`.
const CAPABILITIES: &[&str] = &[
    "actions",
    "body",
    "body-markup",
    "persistence",
];

/// Server identity returned by `GetServerInformation`.
const SERVER_NAME: &str = "ChronOS Notifications";
const SERVER_VENDOR: &str = "ChronOS";
const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");
const SERVER_SPEC_VERSION: &str = "1.2";

/// Imperative commands the ChronOS-side API can issue (never part of the
/// `Service` trait — concrete methods on the subscriber, per our convention).
#[derive(Debug, Clone)]
pub enum NotificationCommand {
    /// Close one notification (emits `NotificationClosed` w/ reason DismissedByCall).
    Close(u32),
    /// Activate a notification action (emits `ActionInvoked`).
    InvokeAction(u32, String),
    /// Close every notification.
    DismissAll,
}

/// The subscriber: owns reactive state + the live D-Bus connection.
#[derive(Clone)]
pub struct NotificationSubscriber {
    data: Mutable<NotificationState>,
    status: Mutable<ServiceStatus>,
    /// Live connection, set once the serve loop registers the object. Held here
    /// so it is never dropped while the daemon is alive (and so `dispatch()`
    /// can emit signals directly without a self-RPC).
    conn: Mutable<Option<Connection>>,
    /// Next id allocator (std Mutex — no reactor needed; handlers run on the
    /// zbus executor thread, not a tokio context).
    inner: Arc<StdMutex<Inner>>,
    /// The tokio runtime handle captured in `new()` (called inside
    /// `rt.block_on`). Used to spawn expiry timers on the *real* runtime —
    /// never from the zbus executor thread (which has no reactor).
    runtime: Handle,
    /// Signalled when the bus connection drops, so the serve task can return
    /// and the outer retry loop reconnects.
    disconnected: Arc<Notify>,
}

#[derive(Default)]
struct Inner {
    next_id: u32,
}

impl NotificationSubscriber {
    /// Non-failing, synchronous constructor (spec §5.1) — same shape as
    /// `UPowerSubscriber::new()` / `NetworkSubscriber::new()`.
    ///
    /// # Panics
    ///
    /// Panics if called outside a tokio runtime — `Handle::current()` requires
    /// one. `init_all()` (spec §7) calls this inside `rt.block_on`.
    pub fn new() -> Self {
        let data = Mutable::new(NotificationState::default());
        let status = Mutable::new(ServiceStatus::Initializing);
        let conn = Mutable::new(None);

        // Guard: must run inside `rt.block_on` (spec §5.1 + §7).
        let _handle = Handle::current();

        let svc = Self {
            data: data.clone(),
            status: status.clone(),
            conn: conn.clone(),
            inner: Arc::new(StdMutex::new(Inner::default())),
            runtime: Handle::current(),
            disconnected: Arc::new(Notify::new()),
        };

        tokio::spawn(run(svc.clone(), data, status, conn, svc.disconnected.clone()));
        svc
    }

    /// Issue an imperative command (Close / InvokeAction / DismissAll).
    ///
    /// Unlike a naive self-RPC through a proxy, this mutates state and emits
    /// signals **directly** on the stored connection — no round-trip.
    pub async fn dispatch(&self, cmd: NotificationCommand) -> anyhow::Result<()> {
        match cmd {
            NotificationCommand::Close(id) => {
                self.close_internal(id, CloseReason::DismissedByCall).await;
            }
            NotificationCommand::InvokeAction(id, action_key) => {
                self.invoke_action_internal(id, &action_key).await;
            }
            NotificationCommand::DismissAll => {
                let ids: Vec<u32> = self.data.get_cloned().notifications.iter().map(|n| n.id).collect();
                for id in ids {
                    self.close_internal(id, CloseReason::DismissedByDaemon).await;
                }
            }
        }
        Ok(())
    }

    /// Remove a notification + emit `NotificationClosed`. Direct (no proxy).
    async fn close_internal(&self, id: u32, reason: CloseReason) {
        let removed = { self.data.lock_mut().remove(id).is_some() };
        if !removed {
            return;
        }
        self.data.lock_mut().recompute_flags();
        self.emit_closed(id, reason as u32).await;
    }

    /// Emit `ActionInvoked` for a given action key (if the notification has it),
    /// then close the notification (FDO: activating an action closes it).
    async fn invoke_action_internal(&self, id: u32, action_key: &str) {
        let has = self
            .data
            .get_cloned()
            .by_id(id)
            .map(|n| n.actions.iter().any(|(k, _)| k == action_key))
            .unwrap_or(false);
        if has {
            self.emit_action_invoked(id, action_key).await;
            self.close_internal(id, CloseReason::DismissedByCall).await;
        }
    }

    /// Emit `NotificationClosed` on the live connection if present.
    async fn emit_closed(&self, id: u32, reason: u32) {
        if let Some(conn) = self.conn.get_cloned() {
            if let Ok(emitter) = SignalEmitter::new(&conn, OBJECT_PATH).map(|e| e.into_owned()) {
                let _ = NotificationSubscriber::notification_closed(&emitter, id, reason).await;
            }
        }
    }

    /// Emit `ActionInvoked` on the live connection if present.
    async fn emit_action_invoked(&self, id: u32, action_key: &str) {
        if let Some(conn) = self.conn.get_cloned() {
            if let Ok(emitter) = SignalEmitter::new(&conn, OBJECT_PATH).map(|e| e.into_owned()) {
                let _ = NotificationSubscriber::action_invoked(&emitter, id, action_key.to_string()).await;
            }
        }
    }

    /// Arm a timer that closes an expiring notification (NO std::thread,
    /// NO nested runtime — runs on the service's tokio runtime via the
    /// `Handle` captured in `new()`).
    fn arm_expiry(&self, id: u32, in_ms: u64) {
        if in_ms == 0 {
            return;
        }
        let svc = self.clone();
        self.runtime.spawn(async move {
            tokio::time::sleep(Duration::from_millis(in_ms)).await;
            svc.close_internal(id, CloseReason::Expired).await;
        });
    }
}

impl Default for NotificationSubscriber {
    fn default() -> Self {
        Self::new()
    }
}

impl Service for NotificationSubscriber {
    type Data = NotificationState;
    type Error = anyhow::Error;

    fn subscribe(&self) -> impl Signal<Item = NotificationState> + Unpin + 'static {
        self.data.signal_cloned()
    }
    fn get(&self) -> NotificationState {
        self.data.get_cloned()
    }
    fn status(&self) -> ServiceStatus {
        self.status.get_cloned()
    }
}

/// The zbus interface implementation (server side, zbus 5.17).
#[interface(name = "org.freedesktop.Notifications")]
impl NotificationSubscriber {
    /// FDO `Notify`. Returns the notification id assigned by the daemon.
    async fn notify(
        &self,
        app_name: String,
        replaces_id: u32,
        app_icon: String,
        summary: String,
        body: String,
        actions: Vec<String>,
        hints: std::collections::HashMap<String, zbus::zvariant::OwnedValue>,
        expire_timeout: i32,
    ) -> u32 {
        // Parse urgency hint (FDO: `u8`).
        let urgency = hints
            .get("urgency")
            .and_then(|v| u8::try_from(v).ok())
            .map(Urgency::from_u8)
            .unwrap_or_default();

        // Parse actions: FDO passes them as a flat [key, label, key, label, ...].
        let actions: Vec<(String, String)> = actions
            .chunks_exact(2)
            .map(|c| (c[0].clone(), c[1].clone()))
            .collect();

        // Allocate / reuse id (first id is 1, monotonic).
        let id = if replaces_id != 0 && self.data.get_cloned().by_id(replaces_id).is_some() {
            replaces_id
        } else {
            let mut inner = self.inner.lock().unwrap();
            inner.next_id = inner.next_id.max(0).saturating_add(1);
            inner.next_id
        };

        // Resolve expiry.
        let expire_at = match expire_timeout {
            EXPIRE_NEVER => None,
            EXPIRE_USE_DEFAULT => Some(now_ms() + DEFAULT_EXPIRE_MS),
            ms if ms > 0 => Some(now_ms() + ms as u64),
            _ => Some(now_ms() + DEFAULT_EXPIRE_MS),
        };

        let note = Notification {
            id,
            app_name,
            app_icon,
            summary,
            body,
            urgency,
            actions,
            expire_at,
        };

        self.data.lock_mut().notifications.retain(|n| n.id != id);
        self.data.lock_mut().notifications.push(note);
        self.data.lock_mut().recompute_flags();

        // Arm expiry timer on the service runtime (NOT std::thread).
        if let Some(t) = expire_at {
            let in_ms = t.saturating_sub(now_ms());
            self.arm_expiry(id, in_ms);
        }

        id
    }

    /// FDO `CloseNotification`. Emits `NotificationClosed` (reason 2).
    async fn close_notification(&self, id: u32) {
        self.close_internal(id, CloseReason::DismissedByCall).await;
    }

    /// FDO `GetCapabilities`.
    async fn get_capabilities(&self) -> Vec<String> {
        CAPABILITIES.iter().map(|s| s.to_string()).collect()
    }

    /// FDO `GetServerInformation`.
    async fn get_server_information(
        &self,
    ) -> (String, String, String, String) {
        (
            SERVER_NAME.to_string(),
            SERVER_VENDOR.to_string(),
            SERVER_VERSION.to_string(),
            SERVER_SPEC_VERSION.to_string(),
        )
    }

    /// FDO `NotificationClosed` signal.
    #[zbus(signal)]
    async fn notification_closed(emitter: &SignalEmitter<'_>, id: u32, reason: u32) -> zbus::Result<()>;

    /// FDO `ActionInvoked` signal.
    #[zbus(signal)]
    async fn action_invoked(emitter: &SignalEmitter<'_>, id: u32, action_key: String) -> zbus::Result<()>;
}

/// Connect to the session bus, claim the well-known name, register the
/// interface, then serve until the bus connection drops. On disconnect, back
/// off and retry (spec §5.1 shape).
async fn run(
    svc: NotificationSubscriber,
    data: Mutable<NotificationState>,
    status: Mutable<ServiceStatus>,
    conn_slot: Mutable<Option<Connection>>,
    disconnected: Arc<Notify>,
) {
    const MAX_BACKOFF: Duration = Duration::from_secs(30);
    let mut backoff = Duration::from_secs(1);

    loop {
        match serve(&svc, &data, &status, &conn_slot, &disconnected).await {
            Ok(()) => {
                // Loop re-connects after disconnect.
                warn!("Notifications daemon disconnected; retrying");
            }
            Err(e) => {
                warn!("Notifications daemon connect failed, retrying: {e:?}");
            }
        }
        status.set(ServiceStatus::Unavailable);
        tokio::time::sleep(backoff).await;
        backoff = (backoff * 2).min(MAX_BACKOFF);
    }
}

/// One connection's lifetime: claim name, register interface, serve until the
/// connection closes (signalled via `disconnected`).
async fn serve(
    svc: &NotificationSubscriber,
    _data: &Mutable<NotificationState>,
    status: &Mutable<ServiceStatus>,
    conn_slot: &Mutable<Option<Connection>>,
    _disconnected: &Arc<Notify>,
) -> anyhow::Result<()> {
    let conn = Connection::session().await?;

    // Register our interface at the object path FIRST, then claim the
    // well-known name — this avoids the zbus warning about "Requesting name
    // before setting up the object server" (method calls can't arrive on an
    // unregistered interface).
    conn.object_server()
        .at(OBJECT_PATH, svc.clone())
        .await
        .map_err(|e| anyhow::anyhow!("object_server().at failed: {e}"))?;

    // Claim the well-known name. `ReplaceExisting | AllowReplacement` so a
    // stale previous instance yields gracefully. (If another daemon such as
    // mako/dunst/swaync already owns it, request_name_with_flags still binds
    // but we won't be primary owner; the smoke test warns to stop those first.)
    let flags = BitFlags::from(RequestNameFlags::ReplaceExisting | RequestNameFlags::AllowReplacement);
    conn.request_name_with_flags(WELL_KNOWN_NAME, flags).await?;

    // Signal readiness to the rest of ChronOS and keep the connection alive.
    conn_slot.set(Some(conn.clone()));
    status.set(ServiceStatus::Available);
    info!("org.freedesktop.Notifications server registered");

    // Keep the object server alive. The server runs on `conn`'s own internal
    // executor (spawned by `Connection::session()`); holding `conn` here
    // prevents the connection from being dropped, so it serves until the
    // process exits. We block on a never-resolving future; for a session daemon
    // the process lifetime is the daemon lifetime.
    std::future::pending::<()>().await;
    conn_slot.set(None);
    Ok(())
}

/// Epoch milliseconds (wall clock; sufficient for expiry math).
fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_signals::signal::SignalExt;
    use futures_util::stream::StreamExt;

    /// Edge case demanded by the Lead Architect: `NotificationSubscriber::new()`
    /// calls `Handle::current()`, which panics outside a tokio runtime. This
    /// pins the runtime guard so it is never silently removed (spec §5.1 + §7).
    #[test]
    fn new_panics_outside_runtime() {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _ = NotificationSubscriber::new();
        }));
        assert!(
            result.is_err(),
            "NotificationSubscriber::new() must panic outside a tokio runtime"
        );
    }

    /// Inside a runtime, `new()` must NOT panic and must start Initializing.
    #[tokio::test]
    async fn new_starts_initializing_in_runtime() {
        let svc = NotificationSubscriber::new();
        assert_eq!(svc.status(), ServiceStatus::Initializing);
    }

    /// `Notify` stores a notification in state; `GetCapabilities`/server info
    /// are spec-correct. Uses the subscriber directly (no bus needed).
    #[tokio::test]
    async fn notify_stores_and_caps_work() {
        let svc = NotificationSubscriber::new();

        let id = svc
            .notify(
                "TestApp".into(),
                0,
                "icon".into(),
                "Summary".into(),
                "Body".into(),
                vec!["default".into(), "Open".into()],
                std::collections::HashMap::new(),
                0, // sticky
            )
            .await;

        assert!(id >= 1, "id should be allocated");

        let state = svc.get();
        assert_eq!(state.notifications.len(), 1);
        let n = state.by_id(id).expect("stored");
        assert_eq!(n.summary, "Summary");
        assert_eq!(n.app_name, "TestApp");
        assert_eq!(n.actions, vec![("default".into(), "Open".into())]);
        assert_eq!(n.expire_at, None, "expire_timeout=0 => sticky");

        // Subscribe emits the new snapshot.
        let mut stream = svc.subscribe().to_stream();
        let _ = stream.next().await;

        // Capabilities + server identity (FDO contract).
        assert_eq!(
            svc.get_capabilities().await,
            vec![
                "actions".to_string(),
                "body".to_string(),
                "body-markup".to_string(),
                "persistence".to_string(),
            ]
        );
        let (name, vendor, ver, spec) = svc.get_server_information().await;
        assert_eq!(name, SERVER_NAME);
        assert_eq!(vendor, SERVER_VENDOR);
        assert_eq!(ver, env!("CARGO_PKG_VERSION"));
        assert_eq!(spec, SERVER_SPEC_VERSION);
    }

    /// First id is 1 (no off-by-one).
    #[tokio::test]
    async fn first_id_is_one() {
        let svc = NotificationSubscriber::new();
        let id = svc
            .notify("A".into(), 0, String::new(), "s".into(), String::new(),
                vec![], std::collections::HashMap::new(), 0)
            .await;
        assert_eq!(id, 1);
    }

    /// `CloseNotification` removes the notification and recomputes flags.
    #[tokio::test]
    async fn close_removes_notification() {
        let svc = NotificationSubscriber::new();
        let id = svc
            .notify("A".into(), 0, String::new(), "s".into(), String::new(),
                vec![], std::collections::HashMap::new(), 0)
            .await;
        assert_eq!(svc.get().notifications.len(), 1);

        svc.close_notification(id).await;
        assert_eq!(svc.get().notifications.len(), 0);
    }

    /// `DismissAll` via `dispatch` clears every notification directly.
    #[tokio::test]
    async fn dismiss_all_via_dispatch() {
        let svc = NotificationSubscriber::new();
        for i in 0..3 {
            let _ = svc
                .notify(format!("A{i}"), 0, String::new(), "s".into(), String::new(),
                    vec![], std::collections::HashMap::new(), 0)
                .await;
        }
        assert_eq!(svc.get().notifications.len(), 3);
        svc.dispatch(NotificationCommand::DismissAll).await.unwrap();
        assert_eq!(svc.get().notifications.len(), 0);
    }

    /// Expiring notification auto-closes via the tokio timer (no std::thread).
    #[tokio::test]
    async fn expiry_closes_after_timeout() {
        let svc = NotificationSubscriber::new();
        let id = svc
            .notify("A".into(), 0, String::new(), "s".into(), String::new(),
                vec![], std::collections::HashMap::new(), 50) // 50 ms
            .await;
        assert!(svc.get().by_id(id).is_some());

        tokio::time::sleep(Duration::from_millis(200)).await;
        assert!(
            svc.get().by_id(id).is_none(),
            "notification should have expired"
        );
    }

    /// Urgency hint is parsed from the FDO `urgency` (u8) hint.
    #[tokio::test]
    async fn urgency_hint_parsed() {
        let svc = NotificationSubscriber::new();
        let mut hints = std::collections::HashMap::new();
        hints.insert(
            "urgency".into(),
            zbus::zvariant::Value::U8(2).try_into().unwrap(),
        );
        let id = svc
            .notify("A".into(), 0, String::new(), "s".into(), String::new(),
                vec![], hints, 0)
            .await;
        assert_eq!(svc.get().by_id(id).unwrap().urgency, Urgency::Critical);
    }

    /// `dispatch(InvokeAction)` is the path taken by a popup action button.
    /// Per FDO, activating an action closes the notification — verify the
    /// notification is removed and the action key matched before closing.
    #[tokio::test]
    async fn invoke_action_closes_notification() {
        let svc = NotificationSubscriber::new();
        let id = svc
            .notify(
                "A".into(),
                0,
                String::new(),
                "s".into(),
                String::new(),
                vec!["ok".to_string(), "OK".to_string()],
                std::collections::HashMap::new(),
                0,
            )
            .await;
        assert_eq!(svc.get().notifications.len(), 1);

        // Only a matching action key is honored; a bogus key must be ignored.
        svc.dispatch(NotificationCommand::InvokeAction(id, "bogus".into()))
            .await
            .unwrap();
        assert_eq!(svc.get().notifications.len(), 1, "bogus action must be ignored");

        // The real action key closes the notification (just like a click).
        svc.dispatch(NotificationCommand::InvokeAction(id, "ok".into()))
            .await
            .unwrap();
        assert_eq!(svc.get().notifications.len(), 0, "matching action must close");
    }
}
