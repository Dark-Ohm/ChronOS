//! Live check: hold the tray watcher up and print the real tray state so a
//! real SNI app (e.g. `udiskie --appindicator`) can be started against it.
//!
//! ```sh
//! cargo run -p chronos-services --example tray-live-check &
//! killall udiskie; udiskie --appindicator &
//! # watch the log for "item added: .../udiskie"
//! ```

use std::time::Duration;

use chronos_services::{Service, TraySubscriber};
use tokio::runtime::Runtime;

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let rt = Runtime::new()?;
    rt.block_on(async {
        let tray = TraySubscriber::new();
        for _ in 0..50 {
            if tray.status() == chronos_services::ServiceStatus::Available {
                break;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        println!("[live-check] watcher Available; start `udiskie --appindicator` now");

        // Monitor for ~20s, printing state every second.
        for _ in 0..20 {
            let state = tray.get();
            if state.is_empty() {
                println!("[live-check] TrayState: (empty)");
            } else {
                for item in &state.items {
                    println!(
                        "[live-check] TrayState: id={} title={:?} icon={:?}",
                        item.id, item.title, item.icon_name
                    );
                }
            }
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
        println!("[live-check] done");
        Ok(())
    })
}
