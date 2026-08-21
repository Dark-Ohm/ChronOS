//! Live smoke test for the T349 multi-backend dispatcher — the mpvpaper path.
//!
//! Unlike `wallpaper-smoke.rs` (awww query-driven), this exercises the new
//! machinery: config override → backend resolution, then a `Set` routed through
//! mpvpaper (kill-all-except + `mpvpaper --auto-pause <mon|*> <path> -f`).
//!
//! The run is self-contained and restores everything it touches:
//!   1. Backs up `~/.config/chronos/wallpaper.toml` (if any) and writes
//!      `[wallpaper] backend = "mpvpaper"`.
//!   2. Constructs the subscriber and asserts the resolved backend is mpvpaper.
//!   3. Dispatches a `Set` of the target media; waits for `pidof mpvpaper`.
//!   4. On exit (success OR failure) kills mpvpaper and restores the config.
//!
//! ```sh
//! cargo run -p chronos-services --example wallpaper-multibackend-smoke /path/to/video.mp4
//! # (defaults to the first *.mp4 under ~/Pictures/Wallpapers)
//! ```

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use chronos_services::{Backend, Service, WallpaperCommand, WallpaperSubscriber};

fn config_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("~/.config"))
        .join("chronos")
        .join("wallpaper.toml")
}

/// First `*.mp4` under `~/Pictures/Wallpapers`, or `None` if the folder is empty.
fn default_target() -> Option<PathBuf> {
    let dir = PathBuf::from(std::env::var("HOME").ok()?).join("Pictures/Wallpapers");
    let mut mp4s: Vec<PathBuf> = std::fs::read_dir(&dir)
        .ok()?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.is_file() && p.extension().and_then(|e| e.to_str()) == Some("mp4"))
        .collect();
    mp4s.sort();
    mp4s.into_iter().next()
}

fn pidof(bin: &str) -> bool {
    Command::new("pidof")
        .arg(bin)
        .output()
        .is_ok_and(|o| o.status.success())
}

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let target = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .or_else(default_target)
        .unwrap_or_else(|| {
            eprintln!("[smoke] no target media given and none found under ~/Pictures/Wallpapers");
            std::process::exit(2);
        });

    let cfg = config_path();
    let backup = std::fs::read_to_string(&cfg).ok();
    if let Some(dir) = cfg.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let _ = std::fs::write(&cfg, "[wallpaper]\nbackend = \"mpvpaper\"\n");

    let outcome = run_smoke(&target);

    // On success, hold mpvpaper up for a few seconds so an external capture
    // (`grim` / `hyprctl layers`) can see the live wallpaper before restore.
    // Defaults to 0 (no hold); set e.g. `SMOKE_HOLD_SECS=8` to capture.
    if outcome.is_ok() {
        let secs: u64 = std::env::var("SMOKE_HOLD_SECS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        if secs > 0 {
            println!("[smoke] holding {secs}s for external capture");
            std::thread::sleep(Duration::from_secs(secs));
        }
    }

    // Restore unconditionally — never leave a spawned mpvpaper or a stray
    // config override behind (success OR failure).
    let _ = Command::new("pkill").arg("-9").arg("mpvpaper").status();
    match backup {
        Some(prev) => {
            let _ = std::fs::write(&cfg, prev);
        }
        None => {
            let _ = std::fs::remove_file(&cfg);
        }
    }

    match outcome {
        Ok(()) => println!("\n✅ wallpaper-multibackend-smoke PASSED"),
        Err(msg) => {
            eprintln!("[smoke] ❌ FAILED: {msg}");
            std::process::exit(1);
        }
    }
}

fn run_smoke(target: &Path) -> Result<(), String> {
    let rt = tokio::runtime::Runtime::new().expect("runtime");
    rt.block_on(async {
        let svc = WallpaperSubscriber::new();

        // 1) Config override must resolve to mpvpaper.
        let backend = svc.get().backend;
        if backend != Backend::Mpvpaper {
            return Err(format!("expected backend Mpvpaper, got {backend:?}"));
        }
        println!("[smoke] resolved backend: {backend:?}");

        // 2) Set the target media across all outputs.
        println!("[smoke] Set → {}", target.display());
        svc.dispatch(WallpaperCommand {
            path: target.to_path_buf(),
            monitor: None,
            transition: None,
        });

        // 3) mpvpaper is restart-based; wait for the process to come up.
        let mut alive = false;
        for _ in 0..40 {
            tokio::time::sleep(Duration::from_millis(250)).await;
            if pidof("mpvpaper") {
                alive = true;
                break;
            }
        }
        if !alive {
            return Err("mpvpaper did not spawn after Set".to_string());
        }
        println!("[smoke] mpvpaper is alive");

        // 4) Reactive state reflects the set media + backend (dispatch's
        //    fire-and-forget task updates `Mutable` after apply succeeds).
        let mut reflected = false;
        for _ in 0..20 {
            tokio::time::sleep(Duration::from_millis(100)).await;
            let state = svc.get();
            if state.current.as_deref() == Some(target) && state.backend == Backend::Mpvpaper {
                reflected = true;
                break;
            }
        }
        if !reflected {
            return Err(format!(
                "state did not reflect the set media: {:?}",
                svc.get()
            ));
        }
        println!("[smoke] state reflects: backend=Mpvpaper current={}", target.display());
        Ok(())
    })
}
