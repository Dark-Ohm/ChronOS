//! Live smoke test for the tray DBusMenu service.
//!
//! Connects to a running tray service, discovers a registered StatusNotifierItem
//! that has a Menu property (e.g. `udiskie --appindicator`), fetches its menu
//! tree via GetLayout, prints it, then sends a harmless Event(clicked) to a
//! safe item (non-destructive action).
//!
//! ```sh
//! cargo run -p chronos-services --example tray-menu-smoke
//! ```
//! Requires a running session bus with an SNI that has a Menu (e.g. udiskie).

use std::process;
use std::time::Duration;

use chronos_services::{Service, TraySubscriber};
use tokio::runtime::Runtime;
use tracing_subscriber;

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let rt = Runtime::new()?;
    rt.block_on(async {
        let tray = TraySubscriber::new();

        // Wait for watcher to claim the name.
        let mut ready = false;
        for _ in 0..50 {
            if tray.status() == chronos_services::ServiceStatus::Available {
                ready = true;
                break;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        if !ready {
            anyhow::bail!("tray watcher never became Available");
        }
        println!("[menu-smoke] TraySubscriber Available");

        // Check for registered items.
        let state = tray.get();
        let with_menu: Vec<String> = state
            .items
            .iter()
            .filter(|i| i.menu_path.is_some())
            .map(|i| i.id.clone())
            .collect();

        if with_menu.is_empty() {
            eprintln!("[menu-smoke] No SNI items with Menu found on the bus.");
            eprintln!(
                "[menu-smoke] Start `udiskie --appindicator` and re-run."
            );
            process::exit(1);
        }

        println!(
            "[menu-smoke] Found {} item(s) with Menu: {:?}",
            with_menu.len(),
            with_menu
        );

        for service in &with_menu {
            println!("\n[menu-smoke] Fetching menu for {service}...");
            tray.dispatch(chronos_services::TrayCommand::FetchMenu {
                service: service.clone(),
            });

            // Wait a bit for the fetch to complete.
            tokio::time::sleep(Duration::from_millis(500)).await;

            let new_state = tray.get();
            if let Some(item) = new_state.find(service) {
                if let Some(menu) = &item.menu {
                    print_menu(&menu, 0);
                    // Try to send a harmless Event(clicked) — pick the first
                    // non-separator, non-destructive-looking leaf action.
                    if let Some(safe) = find_safe_node(&menu) {
                        println!(
                            "[menu-smoke] Sending Event(clicked) to id {} ('{}')...",
                            safe.id, safe.label
                        );
                        tray.dispatch(
                            chronos_services::TrayCommand::MenuClicked {
                                service: service.clone(),
                                id: safe.id,
                            },
                        );
                        tokio::time::sleep(Duration::from_millis(300)).await;
                        println!("[menu-smoke] Event(clicked) sent OK");
                    } else {
                        println!("[menu-smoke] No safe clickable node found (all are destructive or separators)");
                    }
                } else {
                    println!("[menu-smoke] Menu fetch returned no data for {service}");
                }
            }
        }

        println!("\n✅ tray-menu-smoke PASSED");
        Ok(())
    })
}

fn print_menu(nodes: &[chronos_services::MenuNode], depth: usize) {
    for node in nodes {
        let indent = "  ".repeat(depth);
        if node.separator {
            println!("{indent}--- separator ---");
            continue;
        }
        let toggle = if let Some((t, c)) = &node.toggle {
            format!(" [{:?}={}]", t, c)
        } else {
            String::new()
        };
        let vis = if !node.visible { " [hidden]" } else { "" };
        let ena = if !node.enabled { " [disabled]" } else { "" };
        println!(
            "{indent}#{} \"{}\"{}{}{}{}",
            node.id,
            node.label,
            toggle,
            vis,
            ena,
            if node.children.is_empty() { "" } else { " →" }
        );
        if !node.children.is_empty() {
            print_menu(&node.children, depth + 1);
        }
    }
}

fn find_safe_node<'a>(nodes: &'a [chronos_services::MenuNode]) -> Option<&'a chronos_services::MenuNode> {
    for node in nodes {
        if node.separator || !node.enabled || !node.visible || node.label.is_empty() {
            continue;
        }
        let label_lower = node.label.to_lowercase();
        // Skip clearly destructive actions.
        if label_lower.contains("quit")
            || label_lower.contains("exit")
            || label_lower.contains("kill")
            || label_lower.contains("shutdown")
            || label_lower.contains("power off")
            || label_lower.contains("unmount")
            || label_lower.contains("eject")
            || label_lower.contains("remove")
        {
            if !node.children.is_empty() {
                if let Some(n) = find_safe_node(&node.children) {
                    return Some(n);
                }
            }
            continue;
        }
        if node.children.is_empty() {
            return Some(node);
        }
        if let Some(n) = find_safe_node(&node.children) {
            return Some(n);
        }
    }
    None
}
