//! Wallpaper service — multi-backend dispatcher (T349). Drives `awww` (a
//! maintained swww fork — see `/usr/bin/awww` and `/usr/bin/awww-daemon`
//! v0.12.1) plus hyprpaper / swaybg / mpvpaper / gslapper. The active engine
//! is resolved once at startup (config override → autodetect → awww) and
//! cached in `WallpaperState.backend`; `dispatch` routes every command through
//! it, killing all other engines first (waytrogen's one-live-engine rule).
//!
//! ASYNC TEMPLATE (spec §5.1): same shape as `AudioSubscriber`. `new()`
//! captures `Handle::current()` and `tokio::spawn`s a one-shot startup that
//! ensures the daemon is up and restores the currently-set wallpaper; commands
//! are fire-and-forget via `dispatch`. `init_all()` (spec §7) calls this inside
//! `rt.block_on`.
//!
//! Backend knowledge (CLI surface, daemon bootstrap, `--resize`/`--transition-type`
//! flags, enum→string maps) is sourced from the `waytrogen` project
//! (Unlicense / public domain; see `Source/NOTICE`). The engine applications
//! themselves are NOT embedded — only the subprocess contract.
//!
//! LIMITATION (by design): the reactive state reflects only wallpapers changed
//! through this service. If another process changes the wallpaper directly, our
//! `WallpaperState` goes stale until the next `Set`. This is accepted: the
//! shell owns wallpaper changes.

pub mod backends;
pub mod config;
pub mod types;

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

pub const AWWW_BIN: &str = "awww";
pub const AWWW_DAEMON_BIN: &str = "awww-daemon";

/// Wallpaper backends that waytrogen can drive but this service does NOT own.
/// If one of these holds the background layer, spawning an empty `awww-daemon`
/// over it wins the Hyprland layer-level z-order fight (last mapper wins) and
/// blackens the user's wallpaper — the T338 startup stomp.
///
/// awww itself is shared with waytrogen, so a live `awww-daemon` is never
/// treated as foreign: we query it instead of spawning over it. The gallery
/// app (`waytrogen`) is also deliberately absent — it only holds the layer
/// indirectly through one of these daemons.
const FOREIGN_BACKEND_BINS: &[&str] = &["hyprpaper", "swaybg", "mpvpaper", "gslapper"];

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
        // Resolve the active engine once (config override → autodetect → awww).
        // A handful of fast `pidof`/config reads; `new()` runs inside
        // `rt.block_on` (spec §7) so the synchronous probe is acceptable here.
        let backend = backends::resolve_backend();

        let mut initial = WallpaperState::default();
        initial.backend = backend;
        let data = Mutable::new(initial);
        let status = Mutable::new(ServiceStatus::Initializing);

        // Guard: must run inside `rt.block_on` (spec §5.1 + §7).
        let handle = Handle::current();

        // Restore state: ensure daemon, then query what it displays.
        let data_clone = data.clone();
        let status_clone = status.clone();
        handle.spawn(async move {
            match backend {
                // awww: respect foreign ownership at startup (T338), then
                // reflect what awww currently displays.
                Backend::Awww => {
                    if ensure_daemon().await == StartOutcome::SkippedForeignBackend {
                        // A foreign backend owns the wallpaper layer; leave it
                        // untouched and report that the shell is not driving it.
                        status_clone
                            .set(ServiceStatus::Degraded("wallpaper managed externally".into()));
                        return;
                    }
                    match query_current().await {
                        // We only reflect awww's current state, never drive it.
                        // Do NOT call `awww restore` on an empty read: it
                        // reapplies awww's own on-disk cache, which can drift
                        // from the user's actual selection (caught live
                        // 2026-08-04; waytrogen's config.json vs awww's cache).
                        Ok(state) => {
                            data_clone.set(state);
                            status_clone.set(ServiceStatus::Available);
                        }
                        Err(e) => {
                            // awww not installed / daemon failed to start.
                            warn!("WallpaperSubscriber: awww unavailable ({e}); degraded");
                            status_clone.set(ServiceStatus::Degraded("awww unavailable".into()));
                        }
                    }
                }
                // The other engines have no query surface we own; we drive
                // them on the next Set. Report available with the engine
                // known and the current path unknown.
                other => {
                    info!(
                        "WallpaperSubscriber: active backend {other:?} (no query); ready to drive it"
                    );
                    status_clone.set(ServiceStatus::Available);
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
        // Route through the active engine resolved at startup (config override
        // → autodetect → awww).
        let backend = self.data.get_cloned().backend;
        self.runtime.spawn(async move {
            match apply_command(&cmd, backend).await {
                Ok(()) => {
                    // Reflect locally: update current and per_monitor for the
                    // targeted (or all) outputs so UI does not wait for a poll.
                    let mut state = data.get_cloned();
                    state.backend = backend;
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

    /// Kill any running backend so a new one can take the wallpaper layer
    /// (T349: all five engines are killable — `pkill -9` for the subprocess
    /// backends, IPC `stop` for gslapper).
    pub fn kill_backend(&self, backend: Backend) {
        backends::kill_backend_fn(backend, None);
    }

    /// Re-query `awww query` and update reactive state.
    ///
    /// Use after an external process (e.g. waytrogen gallery) changes the
    /// wallpaper behind our back. Fire-and-forget: spawns on the captured
    /// runtime, updates `Mutable` on completion.
    pub fn refresh(&self) {
        let data = self.data.clone();
        let status = self.status.clone();
        self.runtime.spawn(async move {
            match query_current().await {
                Ok(state) => {
                    if state.current.is_some() || !state.per_monitor.is_empty() {
                        data.set(state);
                    }
                    status.set(ServiceStatus::Available);
                }
                Err(e) => {
                    warn!("WallpaperSubscriber::refresh failed: {e}");
                    // Honest "daemon dead": awww no longer owns the layer
                    // (e.g. mpvpaper killed it). Clear stale state so the UI
                    // does not keep showing a wallpaper the daemon is no
                    // longer serving.
                    status.set(ServiceStatus::Degraded("awww daemon dead".into()));
                    data.set(WallpaperState::default());
                }
            }
        });
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

/// What [`ensure_daemon`] decided at startup.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum StartOutcome {
    /// `awww-daemon` was already alive; nothing was spawned.
    AlreadyAlive,
    /// We spawned the daemon and its socket came up.
    Started,
    /// A foreign backend owns the wallpaper layer; we left the desktop
    /// untouched rather than stomping it.
    SkippedForeignBackend,
}

/// Ensure the awww daemon is running for the STARTUP path. Idempotent: if
/// `awww-daemon` is already alive (via `pidof`), do nothing; if a foreign
/// backend holds the wallpaper layer, skip the spawn entirely (T338);
/// otherwise spawn it and wait for its socket by polling `awww query` (no
/// blind `sleep`).
async fn ensure_daemon() -> StartOutcome {
    if daemon_alive() {
        return StartOutcome::AlreadyAlive;
    }
    if foreign_backend_alive() {
        info!(
            "WallpaperSubscriber: foreign backend owns the wallpaper layer; not starting {AWWW_DAEMON_BIN}"
        );
        return StartOutcome::SkippedForeignBackend;
    }
    spawn_daemon().await;
    StartOutcome::Started
}

/// Spawn awww-daemon regardless of a foreign backend. Used by explicit user
/// wallpaper commands (`Set`/`Next`): the user chose awww, so it may take the
/// layer. The startup path (`new`) must NOT call this — it respects foreign
/// ownership via [`ensure_daemon`].
async fn ensure_daemon_forced() {
    if daemon_alive() {
        return;
    }
    spawn_daemon().await;
}

/// Spawn `awww-daemon --no-cache` and poll `awww query` until its socket is up.
async fn spawn_daemon() {
    info!("WallpaperSubscriber: starting {AWWW_DAEMON_BIN}");
    // Detach stdio so a stuck daemon cannot hold the caller's pipes open.
    // `--no-cache`: a fresh daemon spawn otherwise self-restores its
    // per-output on-disk cache (`~/.cache/awww/<ver>/<output>`) for every
    // output it has ever painted, including ones waytrogen has since
    // reassigned to another backend (e.g. DP-1 -> gslapper, see T244).
    // That auto-restore remaps a background layer-shell surface on top of
    // the other backend's, and Hyprland gives z-order to whoever mapped
    // last -> DP-1 goes black. We only ever want awww painting outputs the
    // UI explicitly targets via `dispatch`, never from its own cache.
    let _ = Command::new(AWWW_DAEMON_BIN)
        .arg("--no-cache")
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
    backends::process_alive(AWWW_DAEMON_BIN)
}

/// `true` if any foreign wallpaper backend is alive (from `pidof`).
fn foreign_backend_alive() -> bool {
    FOREIGN_BACKEND_BINS
        .iter()
        .any(|bin| backends::process_alive(bin))
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

/// Route a command through the active backend, killing every other engine
/// first — one live engine at a time (waytrogen's `change()` discipline).
async fn apply_command(cmd: &WallpaperCommand, backend: Backend) -> anyhow::Result<()> {
    backends::kill_all_except(backend, cmd.monitor.as_deref());
    backends::apply_backend(cmd, backend).await
}

/// awww Set: ensure the daemon is up (forced — the user chose awww), then
/// `awww img ...`. Kept here because awww's daemon bootstrap predates the
/// dispatcher; `backends::apply_backend` forwards the `Awww` arm to it.
pub(crate) async fn apply_awww(cmd: &WallpaperCommand) -> anyhow::Result<()> {
    ensure_daemon_forced().await;
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

    #[test]
    fn foreign_backend_bins_excludes_awww_and_covers_foreign_backends() {
        // awww is shared with waytrogen, so a live daemon must never be
        // treated as foreign; every non-awww backend must be, or startup
        // would stomp it (T338).
        assert!(!FOREIGN_BACKEND_BINS.contains(&AWWW_BIN));
        assert!(!FOREIGN_BACKEND_BINS.contains(&AWWW_DAEMON_BIN));
        for backend in [
            Backend::Hyprpaper,
            Backend::Swaybg,
            Backend::Mpvpaper,
            Backend::Gslapper,
        ] {
            assert!(
                FOREIGN_BACKEND_BINS.contains(&backend.as_str()),
                "foreign backend {} must be detected so startup never stomps it",
                backend.as_str()
            );
        }
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
