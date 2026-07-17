//! System tray service: `org.kde.StatusNotifierWatcher` D-Bus **server** plus
//! per-item `org.kde.StatusNotifierItem` **client** proxies.

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
pub use types::{
    MenuNode, MenuToggleType, TrayCommand, TrayIcon, TrayItem, TrayPixmap, TrayState,
    strip_mnemonic,
};
pub mod menu;
pub mod types;

const WELL_KNOWN_NAME: &str = "org.kde.StatusNotifierWatcher";
const OBJECT_PATH: &str = "/StatusNotifierWatcher";
const DEFAULT_ITEM_PATH: &str = "/StatusNotifierItem";

#[derive(Clone, Debug, zbus::zvariant::Type, zbus::zvariant::Value)]
pub struct IconPixmap {
    pub width: i32,
    pub height: i32,
    pub bytes: Vec<u8>,
}

fn convert_icon_pixmap(mut p: IconPixmap) -> TrayPixmap {
    for pixel in p.bytes.chunks_exact_mut(4) {
        pixel.rotate_left(1);
    }
    TrayPixmap {
        width: p.width.max(0) as u32,
        height: p.height.max(0) as u32,
        data: p.bytes,
    }
}

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
    #[zbus(property)]
    fn menu(&self) -> zbus::Result<zbus::zvariant::OwnedObjectPath>;
    fn activate(&self, x: i32, y: i32) -> zbus::Result<()>;
    fn secondary_activate(&self, x: i32, y: i32) -> zbus::Result<()>;
    fn context_menu(&self, x: i32, y: i32) -> zbus::Result<()>;
}

#[derive(Clone)]
pub struct TraySubscriber {
    data: Mutable<TrayState>,
    status: Mutable<ServiceStatus>,
    conn: Mutable<Option<Connection>>,
    inner: Arc<StdMutex<WatcherInner>>,
    runtime: Handle,
}

#[derive(Default)]
struct WatcherInner {
    items: Vec<String>,
}

impl TraySubscriber {
    pub fn new() -> Self {
        let data = Mutable::new(TrayState::default());
        let status = Mutable::new(ServiceStatus::Initializing);
        let conn = Mutable::new(None);
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

    pub fn dispatch(&self, cmd: TrayCommand) {
        match cmd {
            TrayCommand::ActivateItem { service } => {
                let svc = self.clone();
                self.runtime.spawn(async move {
                    svc.activate_item(&service).await;
                });
            }
            TrayCommand::FetchMenu { service } => {
                let svc = self.clone();
                self.runtime.spawn(async move {
                    svc.fetch_menu(&service).await;
                });
            }
            TrayCommand::MenuClicked { service, id } => {
                let svc = self.clone();
                self.runtime.spawn(async move {
                    svc.menu_clicked(&service, id).await;
                });
            }
        }
    }

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

    async fn fetch_menu(&self, service: &str) {
        let Some(conn) = self.conn.get_cloned() else {
            warn!("tray: no connection, cannot fetch menu for {service}");
            return;
        };
        let (dest, _item_path) = split_service(service);
        let menu_path = {
            let guard = self.data.lock_ref();
            guard
                .items
                .iter()
                .find(|i| i.id == service)
                .and_then(|i| i.menu_path.clone())
        };
        let Some(menu_path) = menu_path else {
            warn!("tray: {service} has no Menu path");
            return;
        };
        match menu::fetch_tree(&conn, &dest, &menu_path).await {
            Ok(nodes) => {
                let count = nodes.len();
                {
                    let mut guard = self.data.lock_mut();
                    if let Some(item) = guard.items.iter_mut().find(|i| i.id == service) {
                        item.menu = Some(nodes);
                        info!("tray: fetched menu for {service} ({count} nodes)");
                    } else {
                        warn!("tray: menu arrived for vanished item {service}");
                    }
                }
            }
            Err(e) => {
                warn!("tray: fetch_menu failed for {service} (menu at {menu_path}): {e:?}");
            }
        }
    }

    async fn menu_clicked(&self, service: &str, id: i32) {
        let Some(conn) = self.conn.get_cloned() else {
            warn!("tray: no connection, cannot send clicked to {service}");
            return;
        };
        let (dest, _item_path) = split_service(service);
        let menu_path = {
            let guard = self.data.lock_ref();
            guard
                .items
                .iter()
                .find(|i| i.id == service)
                .and_then(|i| i.menu_path.clone())
        };
        let Some(menu_path) = menu_path else {
            warn!("tray: {service} has no Menu path for Event");
            return;
        };
        if let Err(e) = menu::send_clicked(&conn, &dest, &menu_path, id).await {
            warn!("tray: Event(clicked) failed for {service} id={id}: {e:?}");
        }
    }

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
                    .map(convert_icon_pixmap)
            });
        let menu_path = proxy.menu().await.ok().map(|p| p.to_string());

        let label = TrayItem::derive_label(&title, &icon_name);
        let log_title = title.clone();
        let log_icon = icon_name.clone();
        let log_menu = menu_path.clone();
        let item = TrayItem {
            id: service.to_string(),
            title,
            icon_name,
            icon_pixmap,
            label,
            menu_path,
            menu: None,
        };

        {
            let mut guard = self.data.lock_mut();
            if let Some(existing) = guard.items.iter_mut().find(|i| i.id == service) {
                let prev_menu = existing.menu.take();
                *existing = item;
                existing.menu = prev_menu;
                info!("tray: item refreshed: {service} (title={log_title:?}, icon={log_icon:?}, menu={log_menu:?})");
            } else {
                guard.items.push(item);
                info!("tray: item added: {service} (title={log_title:?}, icon={log_icon:?}, menu={log_menu:?})");
            }
        }

        {
            let mut inner = self.inner.lock().unwrap();
            if !inner.items.iter().any(|s| s == service) {
                inner.items.push(service.to_string());
            }
        }
    }

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

#[interface(name = "org.kde.StatusNotifierWatcher")]
impl TraySubscriber {
    async fn register_status_notifier_item(
        &mut self,
        service: String,
        #[zbus(header)] header: Header<'_>,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) {
        let sender = header.sender().map(|s| s.to_string());
        let service = match normalize_registration(&service, sender.as_deref()) {
            Some(s) => s,
            None => {
                warn!("tray: RegisterStatusNotifierItem with no service and no sender");
                return;
            }
        };

        let already = {
            let inner = self.inner.lock().unwrap();
            inner.items.iter().any(|s| s == &service)
        };
        if already {
            return;
        }

        if let Err(e) = Self::status_notifier_item_registered(&emitter, &service).await {
            warn!("tray: emit Registered failed: {e:?}");
        }

        let conn = emitter.connection().clone();
        self.add_item(&service, &conn).await;
    }

    async fn register_status_notifier_host(&mut self, _service: String) {}

    #[zbus(property)]
    fn registered_status_notifier_items(&self) -> Vec<String> {
        self.inner.lock().unwrap().items.clone()
    }

    #[zbus(property)]
    fn is_status_notifier_host_registered(&self) -> bool {
        true
    }

    #[zbus(property)]
    fn protocol_version(&self) -> i32 {
        0
    }

    #[zbus(signal)]
    async fn status_notifier_item_registered(
        emitter: &SignalEmitter<'_>,
        service: &str,
    ) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn status_notifier_item_unregistered(
        emitter: &SignalEmitter<'_>,
        service: &str,
    ) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn status_notifier_host_registered(emitter: &SignalEmitter<'_>) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn status_notifier_host_unregistered(emitter: &SignalEmitter<'_>) -> zbus::Result<()>;
}

fn split_service(service: &str) -> (String, String) {
    if let Some(idx) = service.find('/') {
        (service[..idx].to_string(), service[idx..].to_string())
    } else {
        (service.to_string(), DEFAULT_ITEM_PATH.to_string())
    }
}

fn normalize_registration(service: &str, sender: Option<&str>) -> Option<String> {
    if service.is_empty() {
        sender.map(|s| s.to_string())
    } else if service.starts_with('/') {
        let sender = sender?;
        Some(format!("{sender}{service}"))
    } else {
        Some(service.to_string())
    }
}

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
            let new_owner = args.new_owner.as_ref().map(|o| o.as_str()).unwrap_or("");
            if new_owner.is_empty() {
                let name = args.name.as_str().to_string();
                unregister_by_name(&svc, &name, &conn).await;
            }
        }
    });
}

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

async fn serve(
    svc: &TraySubscriber,
    _data: &Mutable<TrayState>,
    status: &Mutable<ServiceStatus>,
    conn_slot: &Mutable<Option<Connection>>,
    _inner: &Arc<StdMutex<WatcherInner>>,
) -> anyhow::Result<()> {
    let conn = Connection::session().await?;

    conn.object_server()
        .at(OBJECT_PATH, svc.clone())
        .await
        .map_err(|e| anyhow::anyhow!("object_server().at failed: {e}"))?;

    conn.request_name(WELL_KNOWN_NAME).await?;

    conn_slot.set(Some(conn.clone()));
    status.set(ServiceStatus::Available);
    info!("org.kde.StatusNotifierWatcher registered");

    svc.discover_existing(&conn).await;

    spawn_owner_watch(svc.clone(), conn.clone());

    std::future::pending::<()>().await;
    conn_slot.set(None);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[tokio::test]
    async fn new_starts_initializing_in_runtime() {
        let svc = TraySubscriber::new();
        assert_eq!(svc.status(), ServiceStatus::Initializing);
    }

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
        assert_eq!(
            split_service(":1.50/org/ayatana/NotificationItem/udiskie"),
            (
                ":1.50".to_string(),
                "/org/ayatana/NotificationItem/udiskie".to_string()
            )
        );
    }

    #[test]
    fn normalize_registration_forms() {
        assert_eq!(
            normalize_registration("", Some(":1.7")),
            Some(":1.7".to_string())
        );
        assert_eq!(
            normalize_registration("org.kde.StatusNotifierItem-1234-1/Menu", Some(":1.7")),
            Some("org.kde.StatusNotifierItem-1234-1/Menu".to_string())
        );
        assert_eq!(
            normalize_registration("/org/ayatana/NotificationItem/udiskie", Some(":1.50")),
            Some(":1.50/org/ayatana/NotificationItem/udiskie".to_string())
        );
        assert_eq!(normalize_registration("", None), None);
        assert_eq!(normalize_registration("/some/path", None), None);
    }

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

    #[tokio::test]
    async fn add_and_remove_item() {
        let svc = TraySubscriber::new();
        {
            let mut guard = svc.data.lock_mut();
            guard.items.push(TrayItem {
                id: ":1.99".into(),
                title: Some("Wireless".into()),
                icon_name: Some("network-wireless".into()),
                icon_pixmap: None,
                label: "W".into(),
                menu_path: None,
                menu: None,
            });
        }
        assert_eq!(svc.get().len(), 1);
        assert!(svc.get().find(":1.99").is_some());

        svc.remove_item(":1.99");
        assert_eq!(svc.get().len(), 0);
    }

    #[tokio::test]
    async fn dispatch_activate_without_connection() {
        let svc = TraySubscriber::new();
        svc.dispatch(TrayCommand::ActivateItem {
            service: ":1.99".into(),
        });
    }

    #[test]
    fn convert_icon_pixmap_preserves_dims_and_argb_to_rgba() {
        let p = IconPixmap {
            width: 2,
            height: 1,
            bytes: vec![0xFF, 0x10, 0x20, 0x30, 0xFF, 0x40, 0x50, 0x60],
        };
        let tp = convert_icon_pixmap(p);
        assert_eq!(tp.width, 2);
        assert_eq!(tp.height, 1);
        assert_eq!(tp.data.len(), 8);
        assert_eq!(
            tp.data,
            vec![0x10, 0x20, 0x30, 0xFF, 0x40, 0x50, 0x60, 0xFF]
        );
    }

    #[test]
    fn convert_icon_pixmap_clamps_negative_dims() {
        let p = IconPixmap {
            width: -1,
            height: -1,
            bytes: vec![],
        };
        let tp = convert_icon_pixmap(p);
        assert_eq!(tp.width, 0);
        assert_eq!(tp.height, 0);
        assert!(tp.data.is_empty());
    }
}
