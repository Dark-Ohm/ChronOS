//! Live smoke test for the wallpaper service (awww backend).
//!
//! Steps (per HERMES.md №8 brief):
//!   1. Capture the user's CURRENT wallpaper via `awww query` (so we can restore).
//!   2. Generate a solid-color image in /tmp (never touch ~/Pictures).
//!   3. `Set` it on ONE monitor; confirm via `awww query`.
//!   4. Restore the user's wallpaper (or clear if none was set).
//!
//! `tracing_subscriber` is initialised so the run is NOT blind. The process
//! exits 1 on any failed assertion (zero-result failure criterion).
//!
//! ```sh
//! cargo run -p chronos-services --example wallpaper-smoke
//! ```

use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

use chronos_services::{Service, ServiceStatus, WallpaperCommand, WallpaperSubscriber, AWWW_BIN, AWWW_DAEMON_BIN};

const TEST_IMAGE: &str = "/tmp/chronos-wallpaper-smoke.png";
const TEST_COLOR: &str = "Navy";

fn fail(msg: &str) -> ! {
    eprintln!("[smoke] ❌ FAILED: {msg}");
    std::process::exit(1);
}

fn awww_query_raw() -> Option<String> {
    let out = Command::new(AWWW_BIN).arg("query").output().ok()?;
    if !out.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&out.stdout).into_owned())
}

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let rt = tokio::runtime::Runtime::new().expect("runtime");
    let run = rt.spawn(async {
        let svc = WallpaperSubscriber::new();

        // Wait for the service to become Available (daemon ensured + query done).
        let mut ready = false;
        for _ in 0..60 {
            match svc.status() {
                ServiceStatus::Available => {
                    ready = true;
                    break;
                }
                ServiceStatus::Degraded(_) => {
                    fail("wallpaper service Degraded: awww not available on this host");
                }
                _ => {}
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        if !ready {
            fail("wallpaper service never became Available");
        }

        // 1) Capture current wallpaper BEFORE we touch anything.
        let before_raw = awww_query_raw();
        let before = before_raw
            .as_ref()
            .map(|r| chronos_services::parse_query(r));
        println!(
            "[smoke] BEFORE: current = {:?}, per_monitor = {} outputs",
            before.as_ref().and_then(|s| s.current.clone()),
            before.as_ref().map(|s| s.per_monitor.len()).unwrap_or(0)
        );

        // 2) Generate a solid-color test image in /tmp.
        let status = Command::new("magick")
            .arg("-size")
            .arg("64x64")
            .arg(format!("xc:{TEST_COLOR}"))
            .arg(TEST_IMAGE)
            .status()
            .expect("failed to spawn magick");
        if !status.success() {
            fail("magick failed to generate test image");
        }

        // 3) Pick ONE monitor: first key from per_monitor, else "DP-1".
        let monitor = before
            .as_ref()
            .and_then(|s| s.per_monitor.keys().next().cloned())
            .unwrap_or_else(|| "DP-1".to_string());
        println!("[smoke] setting test wallpaper on monitor: {monitor}");

        svc.dispatch(WallpaperCommand {
            path: PathBuf::from(TEST_IMAGE),
            monitor: Some(monitor.clone()),
            transition: None,
        });

        // Wait for the Set to take effect and confirm via `awww query`.
        let mut confirmed = false;
        for _ in 0..40 {
            tokio::time::sleep(Duration::from_millis(200)).await;
            if let Some(raw) = awww_query_raw() {
                let state = chronos_services::parse_query(&raw);
                if state.per_monitor.get(&monitor) == Some(&PathBuf::from(TEST_IMAGE)) {
                    println!("[smoke] AFTER Set: monitor {monitor} => {}", TEST_IMAGE);
                    confirmed = true;
                    break;
                }
            }
        }
        if !confirmed {
            fail("awww query did not confirm the test wallpaper on the target monitor");
        }

        // 4) Restore the user's wallpaper.
        match &before {
            Some(prev) if prev.current.is_some() => {
                // Re-apply the first known image across all outputs.
                let restore_path = prev.current.clone().unwrap();
                println!("[smoke] restoring user wallpaper: {}", restore_path.display());
                svc.dispatch(WallpaperCommand {
                    path: restore_path,
                    monitor: None,
                    transition: None,
                });
                tokio::time::sleep(Duration::from_millis(500)).await;
            }
            _ => {
                println!("[smoke] no prior wallpaper captured; clearing test layer");
                let _ = Command::new(AWWW_BIN).arg("clear").status();
                tokio::time::sleep(Duration::from_millis(300)).await;
            }
        }

        let _ = std::fs::remove_file(TEST_IMAGE);
        println!("\n✅ wallpaper-smoke PASSED");
    });
    // Hard ceiling: never hang (e.g. if the daemon blocks without a Wayland
    // compositor in this session). `run` is the JoinHandle from `rt.spawn`.
    rt.block_on(async {
        if tokio::time::timeout(Duration::from_secs(25), run)
            .await
            .is_err()
        {
            fail("wallpaper-smoke timed out (daemon likely blocked without a Wayland compositor)");
        }
    });
    // Best-effort cleanup: never leave an orphan daemon.
    let _ = Command::new("pkill").arg("-x").arg(AWWW_DAEMON_BIN).status();
}
