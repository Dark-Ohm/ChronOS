//! Live D-Bus smoke test for the tray service.
//!
//! Spawns a fake `org.kde.StatusNotifierItem` on the session bus, starts the
//! real `TraySubscriber` (which claims `org.kde.StatusNotifierWatcher`), calls
//! `RegisterStatusNotifierItem`, and verifies the item shows up in `TrayState`
//! with the right title/icon, then verifies `Activate` is delivered to the
//! fake item. Run under a session bus, e.g.:
//!
//! ```sh
//! dbus-run-session cargo run -p chronos-services --example tray-smoke
//! ```
//! (a session bus already exists on this machine, so plain `cargo run` works).

use std::sync::Arc;
use std::sync::Mutex as StdMutex;
use std::time::Duration;

use chronos_services::{Service, TrayCommand, TraySubscriber};
use tokio::runtime::Runtime;
use zbus::interface;
use zbus::proxy;

/// Fake StatusNotifierItem.
struct FakeItem {
    activated: Arc<StdMutex<bool>>,
}

#[interface(name = "org.kde.StatusNotifierItem")]
impl FakeItem {
    #[zbus(property)]
    fn title(&self) -> String {
        "SmokeItem".into()
    }

    #[zbus(property)]
    fn icon_name(&self) -> String {
        "smoke-icon".into()
    }

    #[zbus(property)]
    fn id(&self) -> String {
        "smoke".into()
    }

    async fn activate(&self, _x: i32, _y: i32) {
        *self.activated.lock().unwrap() = true;
        println!("[fake-item] Activate received");
    }
}

#[proxy(
    interface = "org.kde.StatusNotifierWatcher",
    default_path = "/StatusNotifierWatcher",
    default_service = "org.kde.StatusNotifierWatcher",
)]
trait StatusNotifierWatcher {
    async fn register_status_notifier_item(&self, service: &str) -> zbus::Result<()>;
    #[zbus(property)]
    fn registered_status_notifier_items(&self) -> zbus::Result<Vec<String>>;
}

fn main() -> anyhow::Result<()> {
    let rt = Runtime::new()?;
    rt.block_on(async {
        // 1) Start the real tray watcher.
        let tray = TraySubscriber::new();

        // Wait until the watcher has claimed the bus name (or time out).
        let mut ready = false;
        for _ in 0..50 {
            if tray.status() == chronos_services::ServiceStatus::Available {
                ready = true;
                break;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        assert!(ready, "tray watcher never became Available");

        // 2) Start a fake StatusNotifierItem service.
        let activated = Arc::new(StdMutex::new(false));
        let fake = FakeItem {
            activated: activated.clone(),
        };
        let fake_conn = zbus::Connection::session().await?;
        fake_conn
            .object_server()
            .at("/StatusNotifierItem", fake)
            .await?;
        let fake_service = fake_conn.unique_name().map(|n| n.to_string()).unwrap();
        println!("[smoke] fake item bus name: {fake_service}");

        // 3) Register the fake item with our watcher using the AYATANA form:
        //    a bare object path with NO bus name (exactly what udiskie /
        //    nm-applet / blueman send). The watcher must fall back to the
        //    sender's unique name as the D-Bus destination.
        let watcher = StatusNotifierWatcherProxy::new(&fake_conn).await?;
        watcher
            .register_status_notifier_item("/StatusNotifierItem")
            .await?;
        // Canonical key the watcher should produce: {sender unique name}/StatusNotifierItem
        let ayatana_key = format!("{fake_service}/StatusNotifierItem");
        println!("[smoke] RegisterStatusNotifierItem(bare path) sent; expect key {ayatana_key}");

        // 4) Wait for the watcher to pick it up.
        let mut found = false;
        for _ in 0..50 {
            let state = tray.get();
            if let Some(item) = state.find(&ayatana_key) {
                assert_eq!(item.title.as_deref(), Some("SmokeItem"));
                assert_eq!(item.icon_name.as_deref(), Some("smoke-icon"));
                println!(
                    "[smoke] item present: id={} title={:?} icon={:?} label={}",
                    item.id, item.title, item.icon_name, item.label
                );
                found = true;
                break;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        assert!(found, "tray watcher never registered the ayatana-form item");
        let registered = watcher.registered_status_notifier_items().await?;
        assert!(
            registered.iter().any(|s| s == &ayatana_key),
            "RegisteredStatusNotifierItems missing our ayatana-form item"
        );
        println!("[smoke] RegisteredStatusNotifierItems OK: {registered:?}");

        // 5) Activate the item (via the ayatana key) and confirm delivery.
        tray.dispatch(TrayCommand::ActivateItem {
            service: ayatana_key.clone(),
        });
        tokio::time::sleep(Duration::from_millis(300)).await;
        assert!(
            *activated.lock().unwrap(),
            "fake item never received Activate"
        );
        println!("[smoke] Activate delivered to fake item OK");

        println!("\n✅ tray-smoke PASSED");
        Ok(())
    })
}
