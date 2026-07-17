//! Wallpaper service via `awww` (Wayland wallpaper daemon, a maintained swww
//! fork) — see `/usr/bin/awww` and `/usr/bin/awww-daemon` (v0.12.1).
//!
//! ASYNC TEMPLATE (spec §5.1): same shape as `AudioSubscriber`. `new()`
//! captures `Handle::current()` and `tokio::spawn`s a one-shot startup that
//! ensures the daemon is up and restores the currently-set wallpaper; commands
//! are fire-and-forget via `dispatch`. `init_all()` (spec §7) calls this inside
//! `rt.block_on`.
//!
//! Backend knowledge (CLI surface, daemon bootstrap, `--resize`/`--transition-type`
//! flags, enum→string maps) is sourced from the `waytrogen` project
//! (Unlicense / public domain; see `Source/NOTICE`). The awww application
//! itself is NOT embedded — only the subprocess contract.
//!
//! LIMITATION (by design): the reactive state reflects only wallpapers changed
//! through this service. If another process runs `awww img` directly, our
//! `WallpaperState` goes stale until the next `Set`. This is accepted: the
//! shell owns wallpaper changes.

use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;

use futures_signals::signal::{Mutable, Signal};
use tokio::runtime::Handle;
use tracing::{info, warn};

use crate::Service;
use crate::ServiceStatus;
pub use types::{
    Backend, IMAGE_EXTENSIONS, WallpaperCommand, WallpaperState, is_image,
};

pub mod types;

pub const AWWW_BIN: &str = "awww";
pub const AWWW_DAEMON_BIN: &str = "awww-daemon";

/// Default resize mode if the UI does not specify one. awww's own default is
/// `crop`; we make it explicit for determinism.
const DEFAULT_RESIZE: &str = "crop";

/// Bounded retries for the daemon socket to come up after spawn. We poll
/// `awww query` rather than `sleep` on a guessed interval.
const DAEMON_RETRY_LIMIT: usize = 20;

#[derive(Clone)]
pub struct WallpaperSubscriber {
    data: Mutable<WallpaperState>,
    status: Mutable<ServiceStatus>,
    /// Captured in `new()` — runtime guard + fire-and-forget for `dispatch`.
    runtime: Handle,
}

impl WallpaperSubscriber {
    /// Non-failing, synchronous constructor (spec §5.1).
    ///
    /// # Panics
    ///
    /// Panics if called outside a tokio runtime — `Handle::current()` requires
    /// one. `init_all()` (spec §7) calls this inside `rt.block_on`.
    pub fn new() -> Self {
        let data = Mutable::new(WallpaperState::default());
        let status = Mutable::new(ServiceStatus::Initializing);

        // Guard: must run inside `rt.block_on` (spec §5.1 + §7).
        let handle = Handle::current();

        // Restore state: ensure daemon, then query what it displays.
        let data_clone = data.clone();
        let status_clone = status.clone();
        handle.spawn(async move {
            ensure_daemon().await;
            match query_current().await {
                Ok(state) => {
                    if state.current.is_some() || !state.per_monitor.is_empty() {
                        data_clone.set(state);
                    }
                    status_clone.set(ServiceStatus::Available);
                }
                Err(e) => {
                    // awww not installed / daemon failed to start: degraded.
                    // Wallpapers can still be set later once awww is present.
                    warn!("WallpaperSubscriber: awww unavailable ({e}); degraded");
                    status_clone.set(ServiceStatus::Degraded("awww unavailable".into()));
                }
            }
        });

        Self {
            data,
            status,
            runtime: handle,
        }
    }

    /// Fire-and-forget command dispatch (mirrors `AudioSubscriber::dispatch`).
    /// Safe to call from a GPUI click handler.
    pub fn dispatch(&self, cmd: WallpaperCommand) {
        let data = self.data.clone();
        let status = self.status.clone();
        self.runtime.spawn(async move {
            match apply_command(&cmd).await {
                Ok(()) => {
                    // Reflect locally: update current and per_monitor for the
                    // targeted (or all) outputs so UI does not wait for a poll.
                    let mut state = data.get_cloned();
                    state.backend = Backend::Awww;
                    if let Some(mon) = &cmd.monitor {
                        state.per_monitor.insert(mon.clone(), cmd.path.clone());
                    } else {
                        // All outputs unless we learn otherwise on next query.
                        // Collect keys first to avoid borrowing while mutating.
                        let mons: Vec<String> = state.per_monitor.keys().cloned().collect();
                        for mon in mons {
                            state.per_monitor.insert(mon, cmd.path.clone());
                        }
                        // If per_monitor was empty we still know the image.
                    }
                    state.current = Some(cmd.path.clone());
                    data.set(state);
                    status.set(ServiceStatus::Available);
                }
                Err(e) => {
                    warn!("WallpaperSubscriber command failed ({cmd:?}): {e:?}");
                }
            }
        });
    }

    /// Kill any running backend so a new one can take the wallpaper layer.
    ///
    /// Only `Awww` is implemented; the others are stubbed per the framework
    /// (they are not built in this MVP).
    pub fn kill_backend(&self, backend: Backend) {
        match backend {
            Backend::Awww => {
                // awww runs per-output daemons; `awww clear` removes the image.
                // We don't block the caller: best-effort, fire-and-forget.
                self.runtime.spawn(async {
                    let _ = Command::new(AWWW_BIN).arg("clear").output();
                });
            }
            other => warn!("kill_backend: backend not implemented: {}", other.as_str()),
        }
    }

    /// Extensions awww can display (for file pickers / validation).
    pub fn accepted_formats(&self) -> &'static [&'static str] {
        IMAGE_EXTENSIONS
    }
}

impl Default for WallpaperSubscriber {
    fn default() -> Self {
        Self::new()
    }
}

impl Service for WallpaperSubscriber {
    type Data = WallpaperState;
    type Error = anyhow::Error;

    fn subscribe(&self) -> impl Signal<Item = WallpaperState> + Unpin + 'static {
        self.data.signal_cloned()
    }

    fn get(&self) -> WallpaperState {
        self.data.get_cloned()
    }

    fn status(&self) -> ServiceStatus {
        self.status.get_cloned()
    }
}

/// Ensure the awww daemon is running. Idempotent: if `awww-daemon` is already
/// alive (via `pidof`), do nothing; otherwise spawn it and wait for its socket
/// by polling `awww query` (no blind `sleep`).
async fn ensure_daemon() {
    if daemon_alive() {
        return;
    }
    info!("WallpaperSubscriber: starting {AWWW_DAEMON_BIN}");
    // Detach stdio so a stuck daemon cannot hold the caller's pipes open.
    let _ = Command::new(AWWW_DAEMON_BIN)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn();

    // Wait for the socket to come up by retrying `awww query`. Bound each
    // query so a half-up daemon cannot block us forever.
    for _ in 0..DAEMON_RETRY_LIMIT {
        if daemon_alive() {
            let probe = tokio::task::spawn_blocking(|| {
                Command::new(AWWW_BIN).arg("query").output()
            });
            match tokio::time::timeout(std::time::Duration::from_secs(2), probe).await {
                Ok(Ok(Ok(out))) if out.status.success() => return,
                _ => {}
            }
        }
        tokio::time::sleep(std::time::Duration::from_millis(150)).await;
    }
    warn!("WallpaperSubscriber: {AWWW_DAEMON_BIN} did not come up in time");
}

/// `true` if `awww-daemon` is alive (from `pidof`).
fn daemon_alive() -> bool {
    Command::new("pidof")
        .arg(AWWW_DAEMON_BIN)
        .output()
        .is_ok_and(|o| o.status.success())
}

/// Pure mapping of [`WallpaperCommand`] → `awww img` argv (no binary name).
///
/// Unit-tested; `apply_command` only shells out. Matches waytrogen's
/// `change_awww_wallpaper`: `--resize <mode>`, optional `--outputs <MON>`,
/// optional `--transition-type <T>`, then the image path.
pub fn command_to_awww_args(cmd: &WallpaperCommand) -> Vec<String> {
    let mut args = vec![
        "img".to_string(),
        "--resize".to_string(),
        DEFAULT_RESIZE.to_string(),
    ];
    if let Some(mon) = &cmd.monitor {
        args.push("--outputs".to_string());
        args.push(mon.clone());
    }
    if let Some(transition) = &cmd.transition {
        args.push("--transition-type".to_string());
        args.push(transition.clone());
    }
    args.push(cmd.path.to_string_lossy().into_owned());
    args
}

async fn apply_command(cmd: &WallpaperCommand) -> anyhow::Result<()> {
    ensure_daemon().await;
    let args = command_to_awww_args(cmd);
    tokio::task::spawn_blocking(move || {
        let output = Command::new(AWWW_BIN)
            .args(&args)
            .output()
            .map_err(|e| anyhow::anyhow!("failed to spawn `{AWWW_BIN}`: {e}"))?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("`{AWWW_BIN} {}` failed: {}", args.join(" "), stderr.trim());
        }
        Ok(())
    })
    .await
    .map_err(|e| anyhow::anyhow!("wallpaper command join error: {e}"))?
}

/// Parse `awww query` output into reactive state.
///
/// `awww query` prints one line per output, e.g.
/// `eDP-1: 1920x1080, scale: 1, currently displaying: image: /pics/a.png`.
/// We populate `per_monitor` from every line and `current` from the first
/// `image:` we find.
pub fn parse_query(query_output: &str) -> WallpaperState {
    let mut per_monitor: HashMap<String, PathBuf> = HashMap::new();
    let mut current: Option<PathBuf> = None;

    for line in query_output.lines() {
        // awww 0.12.1 emits lines with a LEADING ": " (e.g.
        // ": DP-1: 2560x1440, ..."). Strip any leading ':'/' ' before the
        // first real token so the output name parses regardless.
        let line = line.trim_start_matches([':', ' ']);
        // Output name is the token before the first ':'.
        let (output, rest) = match line.split_once(':') {
            Some((o, r)) => (o.trim().to_string(), r),
            None => continue,
        };
        if output.is_empty() || !rest.contains("currently displaying") {
            continue;
        }
        // Only an explicit "image: " line is a wallpaper. A "color: RRGGBB"
        // monitor (no image) must NOT be treated as a path. The phrase sits
        // mid-line, so locate it with `find` rather than `strip_prefix`.
        let Some(pos) = rest.find("currently displaying: image: ") else {
            continue;
        };
        let path = rest[pos + "currently displaying: image: ".len()..].trim().to_string();
        if path.is_empty() {
            continue;
        }
        let path_buf = PathBuf::from(path);
        per_monitor.insert(output, path_buf.clone());
        if current.is_none() {
            current = Some(path_buf);
        }
    }

    WallpaperState {
        current,
        per_monitor,
        backend: Backend::Awww,
    }
}

async fn query_current() -> anyhow::Result<WallpaperState> {
    tokio::task::spawn_blocking(|| {
        let output = Command::new(AWWW_BIN)
            .arg("query")
            .output()
            .map_err(|e| anyhow::anyhow!("failed to spawn `{AWWW_BIN} query`: {e}"))?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("`{AWWW_BIN} query` failed: {}", stderr.trim());
        }
        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(parse_query(&stdout))
    })
    .await
    .map_err(|e| anyhow::anyhow!("wallpaper query join error: {e}"))?
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::panic::{AssertUnwindSafe, catch_unwind};

    /// Same runtime_guard contract as audio/network/upower/tray.
    #[test]
    fn wallpaper_new_panics_outside_runtime() {
        let result = catch_unwind(AssertUnwindSafe(|| {
            let _ = WallpaperSubscriber::new();
        }));
        assert!(
            result.is_err(),
            "WallpaperSubscriber::new() must panic outside a tokio runtime (Handle::current guard)"
        );
    }

    #[tokio::test]
    async fn wallpaper_new_inside_runtime_starts() {
        let sub = WallpaperSubscriber::new();
        matches!(
            sub.status(),
            ServiceStatus::Initializing | ServiceStatus::Available | ServiceStatus::Degraded(_)
        );
    }

    #[test]
    fn command_to_awww_args_all_outputs() {
        let cmd = WallpaperCommand {
            path: PathBuf::from("/pics/a.png"),
            monitor: None,
            transition: None,
        };
        assert_eq!(
            command_to_awww_args(&cmd),
            vec![
                "img".to_string(),
                "--resize".to_string(),
                "crop".to_string(),
                "/pics/a.png".to_string(),
            ]
        );
    }

    #[test]
    fn command_to_awww_args_one_monitor_and_transition() {
        let cmd = WallpaperCommand {
            path: PathBuf::from("/pics/b.jpg"),
            monitor: Some("DP-1".into()),
            transition: Some("fade".into()),
        };
        assert_eq!(
            command_to_awww_args(&cmd),
            vec![
                "img".to_string(),
                "--resize".to_string(),
                "crop".to_string(),
                "--outputs".to_string(),
                "DP-1".to_string(),
                "--transition-type".to_string(),
                "fade".to_string(),
                "/pics/b.jpg".to_string(),
            ]
        );
    }

    #[test]
    fn backend_as_str() {
        assert_eq!(Backend::Awww.as_str(), "awww");
        assert_eq!(Backend::Hyprpaper.as_str(), "hyprpaper");
        assert_eq!(Backend::Gslapper.as_str(), "gslapper");
    }

    // --- Fixtures from live `awww query` output (captured on this host) ---

    #[test]
    fn parse_query_fills_per_monitor_and_current() {
        // Real awww 0.12.1 output (leading ": ", one image + one color monitor).
        let out = ": HDMI-A-1: 1920x1200, scale: 1, currently displaying: color: 000000\n\
                   : DP-1: 2560x1440, scale: 1, currently displaying: image: /tmp/chronos-wallpaper-smoke.png\n";
        let state = parse_query(out);
        // Only the image monitor lands in per_monitor; color monitors are skipped.
        assert_eq!(state.current, Some(PathBuf::from("/tmp/chronos-wallpaper-smoke.png")));
        assert_eq!(state.per_monitor.len(), 1);
        assert_eq!(
            state.per_monitor.get("DP-1"),
            Some(&PathBuf::from("/tmp/chronos-wallpaper-smoke.png"))
        );
        assert!(state.per_monitor.get("HDMI-A-1").is_none());
        assert_eq!(state.backend, Backend::Awww);
    }

    #[test]
    fn parse_query_handles_no_image() {
        // Real awww 0.12.1 line for a monitor showing only a color (no image).
        let out = ": HDMI-A-1: 1920x1200, scale: 1, currently displaying: color: 000000\n";
        let state = parse_query(out);
        assert_eq!(state.current, None);
        assert!(state.per_monitor.is_empty());
    }

    #[test]
    fn parse_query_handles_spaces_in_path() {
        // Leading ": " AND a space in the image path.
        let out = ": DP-1: 2560x1440, scale: 1, currently displaying: image: /pics/my wall.png\n";
        let state = parse_query(out);
        assert_eq!(state.current, Some(PathBuf::from("/pics/my wall.png")));
        assert_eq!(state.per_monitor.get("DP-1"), Some(&PathBuf::from("/pics/my wall.png")));
    }

    #[test]
    fn parse_query_ignores_unrelated_lines() {
        // Lines without "currently displaying" must be ignored (leading ": " ok).
        let out = ": DP-1: 3840x2160, scale: 1\nawww v0.12.1\n";
        let state = parse_query(out);
        assert_eq!(state.current, None);
        assert!(state.per_monitor.is_empty());
    }

    #[test]
    fn is_image_matches_common_extensions() {
        assert!(is_image(std::path::Path::new("a.PNG")));
        assert!(is_image(std::path::Path::new("a.webp")));
        assert!(!is_image(std::path::Path::new("a.txt")));
        assert!(!is_image(std::path::Path::new("noext")));
    }
}
