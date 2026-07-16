//! Live smoke harness for the org.freedesktop.Notifications daemon.
//!
//! Builds + runs from the services crate (which compiles cleanly). Mirrors the
//! app's `rt.block_on(init_all())` shape so `NotificationSubscriber::new()`
//! resolves `Handle::current()` inside a live tokio runtime.
//!
//! Usage:
//!   1. stop any other notification daemon:  mako / dunst / swaync
//!   2. RUST_LOG=info cargo run -p chronos-services --example notification-smoke
//!   3. (in another terminal) notify-send "test" "body"
//!      gdbus call --session --dest org.freedesktop.Notifications \
//!        --object-path /org/freedesktop/Notifications \
//!        --method org.freedesktop.Notifications.GetServerInformation
//!      busctl --user status org.freedesktop.Notifications

use chronos_services::{init_all, Service};
use futures_signals::signal::SignalExt;
use futures_util::stream::StreamExt;
use tracing::info;
use tracing_subscriber::EnvFilter;

#[tokio::main(flavor = "multi_thread", worker_threads = 2)]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    // `init_all()` calls `NotificationSubscriber::new()` which requires a live
    // tokio runtime (Handle::current guard). `#[tokio::main]` provides it,
    // matching the app's `rt.block_on(init_all())` contract.
    let services = init_all();
    let notif = services.notification.clone();

    info!(status = ?notif.status(), "daemon constructed");

    // Reactively print every notification snapshot (proves the D-Bus -> state path).
    let mut stream = notif.subscribe().to_stream();
    tokio::spawn(async move {
        while let Some(state) = stream.next().await {
            info!(
                active = state.notifications.len(),
                any_critical = state.any_critical,
                "NotificationState updated"
            );
            for n in &state.notifications {
                info!(id = n.id, app = %n.app_name, urgency = ?n.urgency, "{}: {}", n.summary, n.body);
            }
        }
    });

    info!("serving org.freedesktop.Notifications on the session bus. Ctrl-C / SIGTERM to exit.");
    // Idle forever (the daemon's own spawned task keeps the object server
    // alive on the connection's executor). The process exits on Ctrl-C.
    std::future::pending::<()>().await;
    info!("shutting down");
}
