//! System tray service: `org.kde.StatusNotifierWatcher` D-Bus **server** plus
//! per-item `org.kde.StatusNotifierItem` **client** proxies.
//!
//! This is the SECOND server-side zbus use in ChronOS (the notification daemon
//! was the first). Written from scratch against the FDO StatusNotifier spec and
//! our own `Service` trait. The donor (`reference/gpui-shell-main`) is
//! **unlicensed** (all-rights-reserved), so this is a rewrite-by-pattern: the
//! FDO method/signal *signatures* are the spec (not copyrightable), the *logic*
//! is ours. Zero lines copied.
//!
//! Architecture mirrors `notification`/`upower`/`network`: `new()` is a
//! non-failing sync constructor that captures `Handle::current()` (panicking if
//! called outside a tokio runtime — same guard) and spawns the connect/serve
//! loop. `init_all()` (crates/services/src/lib.rs) calls this inside
//! `rt.block_on`.
//!
//! ## CRITICAL zbus executor caveat (HANDOFF §факты)
//!
//! The zbus object server dispatches interface handlers on ITS OWN executor
//! thread, NOT on tokio. Therefore `tokio::spawn` / `tokio::sync::Mutex` inside
//! an interface handler **panics**. The recipe (also used by notification/):
//! `std::sync::Mutex` for shared state + `Handle::current()` captured in `new()`,
//! and only the *reactive* Mutable (std primitives) is mutated from handlers.

use std::sync::Arc;
use std::sync::Mutex as StdMutex;

use futures_signals::signal::{Mutable, Signal};
use tokio::runtime::Handle;
use tracing::{info, warn};
use zbus::fdo::DBusProxy;
use zbus::message::Header;
use zbus::object_server::SignalEmitter;
use zbus::{Connection, interface, proxy};

use crate::Service;
use crate::ServiceStatus;
pub use types::{TrayCommand, TrayIcon, TrayItem, TrayState};
pub mod types;

/// Well-known name we claim on the session bus (the StatusNotifierWatcher).
const WELL_KNOWN_NAME: &str = "org.kde.StatusNotifierWatcher";
/// Object path we register the watcher interface at.
const OBJECT_PATH: &str = "/StatusNotifierWatcher";
/// Default item object path (apps conventionally expose their item here).
const DEFAULT_ITEM_PATH: &str = "/StatusNotifierItem";

/// RGBA icon pixmap as carried by `StatusNotifierItem.IconPixmap` (signature
/// `(iiay)`): width, height, raw ARGB bytes. We flip ARGB→RGBA when storing.
#[derive(Clone, Debug, zbus::zvariant::Type, zbus::zvariant::Value)]
pub struct IconPixmap {
    pub width: i32,
    pub height: i32,
    pub bytes: Vec<u8>,
}

/// Client-side proxy to a registered `org.kde.StatusNotifierItem`.
#[proxy(
    interface = "org.kde.StatusNotifierItem",
    default_path = "/StatusNotifierItem",
)]
trait StatusNotifierItem {
    #[zbus(property)]
    fn title(&self) -> zbus::Result<String>;

    #[zbus(property)]
    fn id(&self) -> zbus::Result<String>;

    #[zbus(property)]
    fn icon_name(&self) -> zbus::Result<String>;

    #[zbus(property)]
    fn icon_pixmap(&self) -> zbus::Result<Vec<IconPixmap>>;

    /// Left-click activation.
    fn activate(&self, x: i32, y: i32) -> zbus::Result<()>;

    /// Middle-click activation.
    fn secondary_activate(&self, x: i32, y: i32) -> zbus::Result<()>;

    /// Right-click context menu.
    fn context_menu(&self, x: i32, y: i32) -> zbus::Result<()>;
}

/// The subscriber: owns reactive state + the live D-Bus connection. The
/// interface `impl StatusNotifierWatcher` below is registered on the
/// connection's object server (the `self` there is `TraySubscriber`).
#[derive(Clone)]
pub struct TraySubscriber {
    data: Mutable<TrayState>,
    status: Mutable<ServiceStatus>,
    /// Live connection, set once the serve loop registers the object. Held here
    /// so it is never dropped while the daemon is alive.
    conn: Mutable<Option<Connection>>,
    /// Registered item ids (std Mutex: handlers run on the zbus executor
    /// thread, not a tokio context — no reactor available).
    inner: Arc<StdMutex<WatcherInner>>,
    /// Tokio runtime handle captured in `new()` (inside `rt.block_on`). The
    /// capture itself is the runtime guard (panics if constructed outside a
    /// runtime). Used to spawn fire-and-forget D-Bus calls from `dispatch`.
    runtime: Handle,
}

#[derive(Default)]
struct WatcherInner {
    /// Service strings currently registered (dedup set).
    items: Vec<String>,
}

impl TraySubscriber {
    /// Non-failing, synchronous constructor (spec §5.1) — same shape as
    /// `NotificationSubscriber::new()` / `UPowerSubscriber::new()`.
    ///
    /// # Panics
    ///
    /// Panics if called outside a tokio runtime — `Handle::current()` requires
    /// one. `init_all()` (spec §7) calls this inside `rt.block_on`.
    pub fn new() -> Self {
        let data = Mutable::new(TrayState::default());
        let status = Mutable::new(ServiceStatus::Initializing);
        let conn = Mutable::new(None);

        // Guard: must run inside `rt.block_on` (spec §5.1 + §7).
        let _handle = Handle::current();

        let svc = Self {
            data: data.clone(),
            status: status.clone(),
            conn: conn.clone(),
            inner: Arc::new(StdMutex::new(WatcherInner::default())),
            runtime: Handle::current(),
        };

        tokio::spawn(run(svc.clone(), data, status, conn, svc.inner.clone()));
        svc
    }

    /// Issue an imperative command (currently: ActivateItem). Fire-and-forget:
    /// the async D-Bus call is spawned on the tokio runtime captured in `new()`
    /// (via the `Handle`), so this is safe to call from non-tokio contexts such
    /// as a GPUI click handler (mirrors `CompositorSubscriber::dispatch`).
    pub fn dispatch(&self, cmd: TrayCommand) {
        match cmd {
            TrayCommand::ActivateItem { service } => {
                let svc = self.clone();
                self.runtime.spawn(async move {
                    svc.activate_item(&service).await;
                });
            }
        }
    }

    /// Build a proxy to an item and invoke `Activate(0, 0)`.
    async fn activate_item(&self, service: &str) {
        let Some(conn) = self.conn.get_cloned() else {
            warn!("tray: no connection, cannot activate {service}");
            return;
        };
        let (dest, path) = split_service(service);
        let builder = match StatusNotifierItemProxy::builder(&conn)
            .destination(dest.clone())
            .ok()
            .and_then(|b| b.path(path.clone()).ok())
        {
            Some(b) => b,
            None => {
                warn!("tray: failed to build proxy for {service}");
                return;
            }
        };
        let proxy = match builder.build().await {
            Ok(p) => p,
            Err(e) => {
                warn!("tray: failed to build proxy for {service}: {e:?}");
                return;
            }
        };
        if let Err(e) = proxy.activate(0, 0).await {
            warn!("tray: Activate failed for {service}: {e:?}");
        } else {
            info!("tray: activated {service}");
        }
    }

    /// Add (or refresh) a tray item by reading its properties over D-Bus.
    /// Runs on the zbus executor (async handler context) — only std primitives.
    async fn add_item(&self, service: &str, conn: &Connection) {
        let (dest, path) = split_service(service);
        let builder = match StatusNotifierItemProxy::builder(conn)
            .destination(dest.clone())
            .ok()
            .and_then(|b| b.path(path.clone()).ok())
        {
            Some(b) => b,
            None => {
                warn!("tray: failed to build item proxy for {service}");
                return;
            }
        };
        let proxy = match builder.build().await {
            Ok(p) => p,
            Err(e) => {
                warn!("tray: failed to build item proxy for {service}: {e:?}");
                return;
            }
        };

        let title = proxy.title().await.ok();
        let icon_name = proxy.icon_name().await.ok().filter(|n| !n.is_empty());
        let icon_pixmap = proxy
            .icon_pixmap()
            .await
            .ok()
            .and_then(|pix| {
                pix.into_iter()
                    .max_by_key(|p| (p.width, p.height))
                    .map(|mut p| {
                        // Convert ARGB → RGBA so the buffer is ready for GPU upload
                        // later (pixmap rendering is deferred — see OPENCODE report).
                        for pixel in p.bytes.chunks_exact_mut(4) {
                            pixel.rotate_left(1);
                        }
                        p.bytes
                    })
            });

        let label = TrayItem::derive_label(&title, &icon_name);
        let item = TrayItem {
            id: service.to_string(),
            title,
            icon_name,
            icon_pixmap,
            label,
        };

        // Push or refresh in reactive state (std Mutex-free: Mutable uses
        // std primitives, safe on the zbus executor thread).
        {
            let mut guard = self.data.lock_mut();
            if let Some(existing) = guard.items.iter_mut().find(|i| i.id == service) {
                *existing = item;
            } else {
                guard.items.push(item);
            }
        }

        // Track in the watcher's dedup set.
        {
            let mut inner = self.inner.lock().unwrap();
            if !inner.items.iter().any(|s| s == service) {
                inner.items.push(service.to_string());
            }
        }
    }

    /// Remove a tray item from reactive state + dedup set.
    fn remove_item(&self, service: &str) {
        {
            let mut guard = self.data.lock_mut();
            guard.items.retain(|i| i.id != service);
        }
        {
            let mut inner = self.inner.lock().unwrap();
            inner.items.retain(|s| s != service);
        }
    }

    /// Initial discovery of already-registered items (covers apps that
    /// registered before this watcher started). One-shot, runs on the zbus
    /// executor at serve time.
    async fn discover_existing(&self, conn: &Connection) {
        let dbus = match DBusProxy::new(conn).await {
            Ok(d) => d,
            Err(e) => {
                warn!("tray: dbus proxy for discovery failed: {e:?}");
                return;
            }
        };
        let Ok(names) = dbus.list_names().await else {
            return;
        };
        for name in names {
            let s = name.as_str();
            if s.contains("StatusNotifierItem") {
                self.add_item(s, conn).await;
            }
        }
    }
}

impl Default for TraySubscriber {
    fn default() -> Self {
        Self::new()
    }
}

impl Service for TraySubscriber {
    type Data = TrayState;
    type Error = anyhow::Error;

    fn subscribe(&self) -> impl Signal<Item = TrayState> + Unpin + 'static {
        self.data.signal_cloned()
    }
    fn get(&self) -> TrayState {
        self.data.get_cloned()
    }
    fn status(&self) -> ServiceStatus {
        self.status.get_cloned()
    }
}

/// The zbus interface implementation (server side, zbus 5.17). Registered on
/// this `TraySubscriber` instance.
#[interface(name = "org.kde.StatusNotifierWatcher")]
impl TraySubscriber {
    /// FDO `RegisterStatusNotifierItem`. `service` identifies the item (its bus
    /// name, e.g. `:1.234` or `org.kde.StatusNotifierItem-1234-1`).
    async fn register_status_notifier_item(
        &mut self,
        service: String,
        #[zbus(header)] header: Header<'_>,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) {
        // Apps normally pass their own bus name; fall back to the sender's
        // unique name if the argument is empty.
        let sender = header.sender().map(|s| s.to_string());
        let service = if service.is_empty() {
            match sender {
                Some(s) => s,
                None => {
                    warn!("tray: RegisterStatusNotifierItem with no service and no sender");
                    return;
                }
            }
        } else {
            service
        };

        // Dedup: already tracked → no-op (still re-emit harmless signal).
        let already = {
            let inner = self.inner.lock().unwrap();
            inner.items.iter().any(|s| s == &service)
        };
        if already {
            return;
        }

        // Emit BEFORE fetching props so listeners learn of the item promptly.
        if let Err(e) = Self::status_notifier_item_registered(&emitter, &service).await {
            warn!("tray: emit Registered failed: {e:?}");
        }

        // Fetch item properties over the connection the emitter carries.
        let conn = emitter.connection().clone();
        self.add_item(&service, &conn).await;
    }

    /// FDO `RegisterStatusNotifierHost`. We are the host; accept no-ops.
    async fn register_status_notifier_host(&mut self, _service: String) {}

    /// FDO property `RegisteredStatusNotifierItems`.
    #[zbus(property)]
    fn registered_status_notifier_items(&self) -> Vec<String> {
        self.inner.lock().unwrap().items.clone()
    }

    /// FDO property `IsStatusNotifierHostRegistered`.
    #[zbus(property)]
    fn is_status_notifier_host_registered(&self) -> bool {
        true
    }

    /// FDO property `ProtocolVersion`.
    #[zbus(property)]
    fn protocol_version(&self) -> i32 {
        0
    }

    /// FDO signal `StatusNotifierItemRegistered`.
    #[zbus(signal)]
    async fn status_notifier_item_registered(
        emitter: &SignalEmitter<'_>,
        service: &str,
    ) -> zbus::Result<()>;

    /// FDO signal `StatusNotifierItemUnregistered`.
    #[zbus(signal)]
    async fn status_notifier_item_unregistered(
        emitter: &SignalEmitter<'_>,
        service: &str,
    ) -> zbus::Result<()>;

    /// FDO signal `StatusNotifierHostRegistered`.
    #[zbus(signal)]
    async fn status_notifier_host_registered(emitter: &SignalEmitter<'_>) -> zbus::Result<()>;

    /// FDO signal `StatusNotifierHostUnregistered`.
    #[zbus(signal)]
    async fn status_notifier_host_unregistered(emitter: &SignalEmitter<'_>) -> zbus::Result<()>;
}

/// Split a `service` string into (destination, object-path). If the string
/// contains a `/`, the part before it is the destination and the rest is the
/// path; otherwise the whole string is the destination and the default item
/// path is used.
fn split_service(service: &str) -> (String, String) {
    if let Some(idx) = service.find('/') {
        (service[..idx].to_string(), service[idx..].to_string())
    } else {
        (service.to_string(), DEFAULT_ITEM_PATH.to_string())
    }
}

/// Connect to the session bus, claim the well-known name, register the watcher
/// interface, perform initial discovery, then serve until the bus connection
/// drops. On disconnect, back off and retry (spec §5.1 shape).
async fn run(
    svc: TraySubscriber,
    data: Mutable<TrayState>,
    status: Mutable<ServiceStatus>,
    conn_slot: Mutable<Option<Connection>>,
    inner: Arc<StdMutex<WatcherInner>>,
) {
    const MAX_BACKOFF: std::time::Duration = std::time::Duration::from_secs(30);
    let mut backoff = std::time::Duration::from_secs(1);

    loop {
        match serve(&svc, &data, &status, &conn_slot, &inner).await {
            Ok(()) => {
                warn!("tray: watcher disconnected; retrying");
            }
            Err(e) => {
                warn!("tray: watcher connect failed, retrying: {e:?}");
            }
        }
        status.set(ServiceStatus::Unavailable);
        tokio::time::sleep(backoff).await;
        backoff = (backoff * 2).min(MAX_BACKOFF);
    }
}

/// Spawn a watcher (on the tokio runtime) that removes tray items whose D-Bus
/// name loses ownership (app exit / crash) and emits the FDO
/// `StatusNotifierItemUnregistered` signal. This is safe here because we are
/// inside the tokio-spawned `run`/`serve` future — NOT inside a zbus object-
/// server handler, where `tokio::spawn` would panic.
fn spawn_owner_watch(svc: TraySubscriber, conn: Connection) {
    tokio::spawn(async move {
        let dbus = match DBusProxy::new(&conn).await {
            Ok(d) => d,
            Err(e) => {
                warn!("tray: owner-watch dbus proxy failed: {e:?}");
                return;
            }
        };
        let mut stream = match dbus.receive_name_owner_changed().await {
            Ok(s) => s,
            Err(e) => {
                warn!("tray: owner-watch stream failed: {e:?}");
                return;
            }
        };
        use futures_util::StreamExt;
        while let Some(sig) = stream.next().await {
            let Ok(args) = sig.args() else { continue };
            // Name lost ownership when the new owner is empty.
            let new_owner = args.new_owner.as_ref().map(|o| o.as_str()).unwrap_or("");
            if new_owner.is_empty() {
                let name = args.name.as_str().to_string();
                unregister_by_name(&svc, &name, &conn).await;
            }
        }
    });
}

/// Remove any tracked item whose service string equals `name` or starts with
/// `name/` (i.e. the item's bus name just lost ownership), then emit the FDO
/// `StatusNotifierItemUnregistered` signal.
async fn unregister_by_name(svc: &TraySubscriber, name: &str, conn: &Connection) {
    let lost: Vec<String> = {
        let inner = svc.inner.lock().unwrap();
        inner
            .items
            .iter()
            .filter(|s| *s == name || s.starts_with(&format!("{name}/")))
            .cloned()
            .collect()
    };
    if lost.is_empty() {
        return;
    }
    for service in &lost {
        svc.remove_item(service);
        if let Ok(emitter) = SignalEmitter::new(conn, OBJECT_PATH) {
            let _ = TraySubscriber::status_notifier_item_unregistered(&emitter, service).await;
        }
    }
    info!("tray: unregistered {} item(s) for vanished name {name}", lost.len());
}

/// One connection's lifetime: claim name, register interface, initial discovery,
/// serve until the connection closes (held alive by `std::future::pending`).
async fn serve(
    svc: &TraySubscriber,
    _data: &Mutable<TrayState>,
    status: &Mutable<ServiceStatus>,
    conn_slot: &Mutable<Option<Connection>>,
    _inner: &Arc<StdMutex<WatcherInner>>,
) -> anyhow::Result<()> {
    let conn = Connection::session().await?;

    // Register our interface at the object path FIRST, then claim the
    // well-known name — avoids the zbus warning about "Requesting name before
    // setting up the object server".
    conn.object_server()
        .at(OBJECT_PATH, svc.clone())
        .await
        .map_err(|e| anyhow::anyhow!("object_server().at failed: {e}"))?;

    conn.request_name(WELL_KNOWN_NAME).await?;

    // Signal readiness and keep the connection alive.
    conn_slot.set(Some(conn.clone()));
    status.set(ServiceStatus::Available);
    info!("org.kde.StatusNotifierWatcher registered");

    // Initial discovery of items that registered before we started.
    svc.discover_existing(&conn).await;

    // Watch for item bus-names vanishing (app exit / crash). This task runs on
    // the *tokio* runtime (we are inside `tokio::spawn` here), which is legal —
    // only the zbus object-server *interface handlers* run on the zbus executor
    // thread. On name loss we remove the item and emit the FDO
    // `StatusNotifierItemUnregistered` signal.
    spawn_owner_watch(svc.clone(), conn.clone());

    // Keep the object server alive. The server runs on `conn`'s own internal
    // executor; holding `conn` prevents the connection from dropping, so it
    // serves until the process exits.
    std::future::pending::<()>().await;
    conn_slot.set(None);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Edge case demanded by the Lead Architect: `TraySubscriber::new()` calls
    /// `Handle::current()`, which panics outside a tokio runtime. Pins the
    /// runtime guard so it is never silently removed (spec §5.1 + §7).
    #[test]
    fn new_panics_outside_runtime() {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _ = TraySubscriber::new();
        }));
        assert!(
            result.is_err(),
            "TraySubscriber::new() must panic outside a tokio runtime"
        );
    }

    /// Inside a runtime, `new()` must NOT panic and must start Initializing.
    #[tokio::test]
    async fn new_starts_initializing_in_runtime() {
        let svc = TraySubscriber::new();
        assert_eq!(svc.status(), ServiceStatus::Initializing);
    }

    /// `split_service` handles both bare bus names and name+path forms.
    #[test]
    fn split_service_forms() {
        assert_eq!(
            split_service(":1.234"),
            (":1.234".to_string(), DEFAULT_ITEM_PATH.to_string())
        );
        assert_eq!(
            split_service("org.kde.StatusNotifierItem-1234-1/Menu"),
            (
                "org.kde.StatusNotifierItem-1234-1".to_string(),
                "/Menu".to_string()
            )
        );
    }

    /// `TrayItem::derive_label` prefers title then icon_name then `?`.
    #[test]
    fn derive_label_priority() {
        assert_eq!(TrayItem::derive_label(&Some("Network".into()), &None), "N");
        assert_eq!(
            TrayItem::derive_label(&None, &Some("network-wired".into())),
            "N"
        );
        assert_eq!(TrayItem::derive_label(&None, &None), "?");
        assert_eq!(TrayItem::derive_label(&Some("".into()), &None), "?");
    }

    /// `TrayState` add/remove logic via the public subscriber state.
    #[tokio::test]
    async fn add_and_remove_item() {
        let svc = TraySubscriber::new();
        // Simulate an item appearing by mutating reactive state directly
        // (the D-Bus path is covered by the live smoke test).
        {
            let mut guard = svc.data.lock_mut();
            guard.items.push(TrayItem {
                id: ":1.99".into(),
                title: Some("Wireless".into()),
                icon_name: Some("network-wireless".into()),
                icon_pixmap: None,
                label: "W".into(),
            });
        }
        assert_eq!(svc.get().len(), 1);
        assert!(svc.get().find(":1.99").is_some());

        svc.remove_item(":1.99");
        assert_eq!(svc.get().len(), 0);
    }

    /// `dispatch(ActivateItem)` is a no-op (no panic) when no connection yet.
    #[tokio::test]
    async fn dispatch_activate_without_connection() {
        let svc = TraySubscriber::new();
        // Must not panic even though conn is None.
        svc.dispatch(TrayCommand::ActivateItem {
            service: ":1.99".into(),
        });
    }
}
