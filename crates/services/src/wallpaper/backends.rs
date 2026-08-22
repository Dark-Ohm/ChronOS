//! Per-backend command builders, selection, and apply for the four engines
//! awww does not already own (hyprpaper/swaybg/mpvpaper/gslapper) — T349.
//!
//! Command surface is sourced from waytrogen (`reference/waytrogen-main/src/
//! changers/{hyprpaper,swaybg,mpvpaper,gslapper}.rs`, Unlicense — NOTICE).
//! awww's own apply/query lives in [`super`] (it predates the dispatcher).
//!
//! waytrogen discipline: **live exactly one engine at a time.** A change kills
//! every other engine before applying the target (`kill_all_except`).

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use tracing::info;

use super::config;
use super::{Backend, WallpaperCommand};

/// hyprpaper fit mode (waytrogen `HyprpaperFitModes` default = Cover).
pub const HYPRPAPER_FIT: &str = "cover";
/// swaybg mode (waytrogen `SwaybgModes` default = Fill).
pub const SWAYBG_MODE: &str = "fill";
/// swaybg fill color (waytrogen default).
pub const SWAYBG_COLOR: &str = "#000000";
/// gslapper FPS cap (waytrogen `GSllaperSettings::default()`).
const GSLAPPER_FPS: u16 = 30;
/// gslapper cache size in MB (waytrogen default).
const GSLAPPER_CACHE_MB: u32 = 256;
/// gslapper IPC read/write timeout (mirrors the reference).
const IPC_TIMEOUT: Duration = Duration::from_secs(2);
/// gslapper socket startup deadline (mirrors `STARTUP_TIMEOUT`).
const STARTUP_TIMEOUT: Duration = Duration::from_secs(3);
/// gslapper socket poll interval.
const PROBE_INTERVAL: Duration = Duration::from_millis(25);

/// hyprpaper daemon startup deadline (lazy bootstrap, T351).
const HYPRPAPER_START_TIMEOUT: Duration = Duration::from_secs(3);
/// hyprpaper daemon poll interval.
const HYPRPAPER_PROBE_INTERVAL: Duration = Duration::from_millis(50);
/// Settle after the hyprpaper process appears: its IPC endpoint needs a beat
/// before the first `hyprctl hyprpaper` lands (waytrogen settles 200ms too).
const HYPRPAPER_IPC_SETTLE: Duration = Duration::from_millis(200);

/// mpvpaper pause mode (waytrogen `MpvPaperPauseModes`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MpvpaperPause {
    None,
    AutoPause,
    AutoStop,
}

impl Default for MpvpaperPause {
    fn default() -> Self {
        MpvpaperPause::AutoPause
    }
}

/// Autodetect order: foreign engines first (a live one owns the layer, T338),
/// awww last. mpvpaper first — the owner's primary (video) engine.
const AUTODETECT_ORDER: [Backend; 5] = [
    Backend::Mpvpaper,
    Backend::Gslapper,
    Backend::Swaybg,
    Backend::Hyprpaper,
    Backend::Awww,
];

// ─────────────────────────────────────────────────────────────────────────────
// Pure argv builders (argv[0] = binary). Unit-tested; apply only shells out.
// ─────────────────────────────────────────────────────────────────────────────

/// hyprpaper Set — IPC through Hyprland: `hyprctl hyprpaper wallpaper
/// "<monitor>,<path>,<fit>"`. One invocation per monitor.
pub fn hyprpaper_argv(monitor: &str, path: &Path, fit: &str) -> Vec<String> {
    vec![
        "hyprctl".to_string(),
        "hyprpaper".to_string(),
        "wallpaper".to_string(),
        format!("{monitor},{},{}", path.display(), fit),
    ]
}

/// systemd user-unit start for the persistent hyprpaper daemon — preferred
/// over a bare spawn so `Restart=on-failure` supervision applies
/// (architect decision, T351).
pub fn hyprpaper_systemctl_start_argv() -> Vec<String> {
    vec![
        "systemctl".to_string(),
        "--user".to_string(),
        "start".to_string(),
        "hyprpaper".to_string(),
    ]
}

/// swaybg Set — one `swaybg` process. `monitor: None` → "All" (no `-o` block).
pub fn swaybg_argv(monitor: Option<&str>, path: &Path, mode: &str, color: &str) -> Vec<String> {
    let mut args = vec!["swaybg".to_string()];
    if let Some(mon) = monitor {
        args.push("-o".to_string());
        args.push(mon.to_string());
    }
    args.extend([
        "-i".to_string(),
        path.display().to_string(),
        "-m".to_string(),
        mode.to_string(),
        "-c".to_string(),
        color.to_string(),
    ]);
    args
}

/// mpvpaper Set — one process per monitor; `*` = All. `-o` omitted when there
/// are no additional mpv options; `-n` only when a slideshow interval is set.
pub fn mpvpaper_argv(
    monitor: Option<&str>,
    path: &Path,
    mpv_opts: &str,
    pause: MpvpaperPause,
    slideshow_secs: Option<u32>,
) -> Vec<String> {
    let mut args = vec!["mpvpaper".to_string()];
    if !mpv_opts.is_empty() {
        args.push("-o".to_string());
        args.push(mpv_opts.to_string());
    }
    match pause {
        MpvpaperPause::None => {}
        MpvpaperPause::AutoPause => args.push("--auto-pause".to_string()),
        MpvpaperPause::AutoStop => args.push("--auto-stop".to_string()),
    }
    if let Some(secs) = slideshow_secs {
        args.push("-n".to_string());
        args.push(secs.to_string());
    }
    args.push(monitor.unwrap_or("*").to_string());
    args.push(path.display().to_string());
    args.push("-f".to_string());
    args
}

/// gslapper launch argv (waytrogen defaults: `fill` scale, loop video, no
/// pause, fps 30, transition none, cache 256). `monitor` is already canonical
/// (`*` for All). `-I <socket>` puts the IPC socket in ChronOS's own namespace.
pub fn gslapper_launch_argv(monitor: &str, socket: &Path, path: &Path) -> Vec<String> {
    vec![
        "gslapper".to_string(),
        "-I".to_string(),
        socket.display().to_string(),
        "-o".to_string(),
        "fill no-audio loop".to_string(),
        "-r".to_string(),
        GSLAPPER_FPS.to_string(),
        "--transition-type".to_string(),
        "none".to_string(),
        "--transition-duration".to_string(),
        "0.5".to_string(),
        "--cache-size".to_string(),
        GSLAPPER_CACHE_MB.to_string(),
        monitor.to_string(),
        path.display().to_string(),
    ]
}

// ─────────────────────────────────────────────────────────────────────────────
// gslapper socket namespace (ChronOS owns its own, not waytrogen's)
// ─────────────────────────────────────────────────────────────────────────────

/// `All` → `*`, matching waytrogen's canonical monitor spelling.
fn canonical_monitor(monitor: Option<&str>) -> &str {
    monitor.unwrap_or("*")
}

/// FNV-1a 64-bit, ported from waytrogen (`changers/gslapper.rs::stable_output_id`):
/// keeps socket names short and stable across sessions.
fn stable_output_id(output: &str) -> u64 {
    output.bytes().fold(0xcbf29ce484222325, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x100000001b3)
    })
}

fn runtime_dir() -> PathBuf {
    std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/run/user/1000"))
        .join("chronos")
}

/// gslapper IPC socket for a monitor. ChronOS namespace (`$XDG_RUNTIME_DIR/
/// chronos/`), NOT waytrogen's — two managers must never share a socket.
pub fn gslapper_socket_path(monitor: &str) -> PathBuf {
    runtime_dir().join(format!(
        "gslapper-{:016x}.sock",
        stable_output_id(monitor)
    ))
}

// ─────────────────────────────────────────────────────────────────────────────
// Detection + selection
// ─────────────────────────────────────────────────────────────────────────────

/// `true` if `pidof <bin>` reports a live process.
pub fn process_alive(bin: &str) -> bool {
    Command::new("pidof")
        .arg(bin)
        .output()
        .is_ok_and(|o| o.status.success())
}

/// `true` if this backend's process is alive (`pidof`).
pub fn backend_alive(backend: Backend) -> bool {
    process_alive(backend.process_bin())
}

/// Resolve the active backend: config override → autodetect live engine →
/// awww. Called once at startup; the result is cached in `WallpaperState`.
pub fn resolve_backend() -> Backend {
    if let Some(backend) = config::load_config().backend_override() {
        info!("wallpaper: backend override from wallpaper.toml: {backend:?}");
        return backend;
    }
    for backend in AUTODETECT_ORDER {
        if backend_alive(backend) {
            info!("wallpaper: autodetected active backend: {backend:?}");
            return backend;
        }
    }
    Backend::Awww
}

// ─────────────────────────────────────────────────────────────────────────────
// Kill (waytrogen `kill_all_changers_except` / per-backend `kill`)
// ─────────────────────────────────────────────────────────────────────────────

/// Kill one backend so another can take the wallpaper layer.
/// gslapper is stopped via IPC `stop` (not `pkill`, per waytrogen); the rest
/// are `pkill -9 <bin>`.
pub fn kill_backend_fn(backend: Backend, monitor: Option<&str>) {
    match backend {
        // gslapper is stopped via IPC (never `pkill`, per waytrogen): one
        // monitor's socket when targeted, all of ours otherwise.
        Backend::Gslapper => {
            let _ = match monitor {
                Some(mon) => gslapper_stop(&gslapper_socket_path(mon)),
                None => stop_all_gslappers(),
            };
        }
        other => {
            let bin = other.process_bin().to_string();
            let _ = Command::new("pkill").arg("-9").arg(&bin).status();
        }
    }
}

/// Kill every engine except `target` before it takes the layer.
pub fn kill_all_except(target: Backend, monitor: Option<&str>) {
    for backend in AUTODETECT_ORDER {
        if backend != target {
            kill_backend_fn(backend, monitor);
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Apply (Set/Next) for the four new engines
// ─────────────────────────────────────────────────────────────────────────────

/// Route a `WallpaperCommand` through a non-awww backend. awww is handled by
/// `super::apply_awww` (it owns the daemon bootstrap).
pub async fn apply_backend(cmd: &WallpaperCommand, backend: Backend) -> anyhow::Result<()> {
    match backend {
        Backend::Awww => super::apply_awww(cmd).await,
        Backend::Hyprpaper => apply_hyprpaper(cmd).await,
        Backend::Swaybg => apply_swaybg(cmd).await,
        Backend::Mpvpaper => apply_mpvpaper(cmd).await,
        Backend::Gslapper => apply_gslapper(cmd).await,
    }
}

/// Run a full argv (argv[0] = binary) to completion, failing on non-zero exit.
async fn run_argv(argv: &[String]) -> anyhow::Result<()> {
    let argv = argv.to_vec();
    tokio::task::spawn_blocking(move || {
        let Some((bin, args)) = argv.split_first() else {
            anyhow::bail!("wallpaper command argv is empty");
        };
        let output = Command::new(bin)
            .args(args)
            .output()
            .map_err(|e| anyhow::anyhow!("failed to spawn `{bin}`: {e}"))?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("`{bin} {}` failed: {}", args.join(" "), stderr.trim());
        }
        Ok(())
    })
    .await
    .map_err(|e| anyhow::anyhow!("wallpaper command join error: {e}"))?
}

/// Spawn a detached process (swaybg/mpvpaper/gslapper are long-lived).
async fn spawn_argv_detached(argv: &[String]) -> anyhow::Result<()> {
    let argv = argv.to_vec();
    tokio::task::spawn_blocking(move || {
        let Some((bin, args)) = argv.split_first() else {
            anyhow::bail!("wallpaper command argv is empty");
        };
        Command::new(bin)
            .args(args)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .map(|_| ())
            .map_err(|e| anyhow::anyhow!("failed to spawn `{bin}`: {e}"))
    })
    .await
    .map_err(|e| anyhow::anyhow!("wallpaper spawn join error: {e}"))?
}

/// Lazy-bootstrap the persistent hyprpaper daemon: alive check → systemd user
/// unit → bare-spawn fallback → bounded readiness poll (architect decision,
/// T351; waytrogen parity). Blocking — call from `spawn_blocking`.
fn ensure_hyprpaper_daemon() -> anyhow::Result<()> {
    if process_alive("hyprpaper") {
        return Ok(());
    }

    let argv = hyprpaper_systemctl_start_argv();
    let unit_started = Command::new(&argv[0])
        .args(&argv[1..])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
        && wait_for_hyprpaper_process();

    if !unit_started {
        // No unit on this machine, or the unit came up dead.
        Command::new("hyprpaper")
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .map_err(|e| anyhow::anyhow!("failed to spawn hyprpaper daemon: {e}"))?;
        if !wait_for_hyprpaper_process() {
            anyhow::bail!("hyprpaper daemon did not come up within {HYPRPAPER_START_TIMEOUT:?}");
        }
    }

    info!(
        "wallpaper: hyprpaper daemon {}",
        if unit_started {
            "started via systemctl --user"
        } else {
            "spawned directly"
        }
    );
    std::thread::sleep(HYPRPAPER_IPC_SETTLE);
    Ok(())
}

fn wait_for_hyprpaper_process() -> bool {
    let deadline = Instant::now() + HYPRPAPER_START_TIMEOUT;
    loop {
        if process_alive("hyprpaper") {
            return true;
        }
        if Instant::now() >= deadline {
            return false;
        }
        std::thread::sleep(HYPRPAPER_PROBE_INTERVAL);
    }
}

async fn apply_hyprpaper(cmd: &WallpaperCommand) -> anyhow::Result<()> {
    tokio::task::spawn_blocking(ensure_hyprpaper_daemon)
        .await
        .map_err(|e| anyhow::anyhow!("hyprpaper bootstrap join error: {e}"))??;

    match &cmd.monitor {
        Some(mon) => {
            let argv = hyprpaper_argv(mon, &cmd.path, HYPRPAPER_FIT);
            run_argv(&argv).await
        }
        // "All": hyprpaper has no wildcard — loop over live monitors.
        None => {
            for mon in monitor_names().await? {
                let argv = hyprpaper_argv(&mon, &cmd.path, HYPRPAPER_FIT);
                run_argv(&argv).await?;
            }
            Ok(())
        }
    }
}

async fn apply_swaybg(cmd: &WallpaperCommand) -> anyhow::Result<()> {
    // restart-based: kill any previous swaybg, then one process.
    let _ = Command::new("pkill").arg("-9").arg("swaybg").status();
    let argv = swaybg_argv(cmd.monitor.as_deref(), &cmd.path, SWAYBG_MODE, SWAYBG_COLOR);
    spawn_argv_detached(&argv).await
}

async fn apply_mpvpaper(cmd: &WallpaperCommand) -> anyhow::Result<()> {
    // restart-based: kill any previous mpvpaper, then one process per monitor.
    let _ = Command::new("pkill").arg("-9").arg("mpvpaper").status();
    let argv = mpvpaper_argv(
        cmd.monitor.as_deref(),
        &cmd.path,
        "",
        MpvpaperPause::default(),
        None,
    );
    spawn_argv_detached(&argv).await
}

async fn apply_gslapper(cmd: &WallpaperCommand) -> anyhow::Result<()> {
    let monitor = canonical_monitor(cmd.monitor.as_deref()).to_string();
    let socket = gslapper_socket_path(&monitor);

    let path = cmd.path.clone();
    let socket_for_block = socket.clone();
    let monitor_for_block = monitor;
    tokio::task::spawn_blocking(move || {
        gslapper_change(&monitor_for_block, &socket_for_block, &path)
    })
    .await
    .map_err(|e| anyhow::anyhow!("gslapper apply join error: {e}"))?
}

/// Apply one media path through a live gslapper socket; spawn if it is absent,
/// restart on a media-type change (the reference's `change_gslapper_wallpaper`).
fn gslapper_change(monitor: &str, socket: &Path, path: &Path) -> anyhow::Result<()> {
    let Some(dir) = socket.parent() else {
        anyhow::bail!("gslapper socket has no parent dir");
    };
    std::fs::create_dir_all(dir)?;

    if !socket.exists() {
        return gslapper_spawn(monitor, socket, path);
    }

    match gslapper_ipc(socket, &format!("change {}", path.display())) {
        Ok(_) => Ok(()),
        Err(e) if e.to_string().contains("cannot update path") => {
            gslapper_stop(socket)?;
            gslapper_spawn(monitor, socket, path)
        }
        Err(e) => Err(e),
    }
}

/// Blocking gslapper IPC request (one command, one response line).
fn gslapper_ipc(socket: &Path, command: &str) -> anyhow::Result<String> {
    let mut stream = UnixStream::connect(socket)
        .map_err(|e| anyhow::anyhow!("gslapper: connect {} failed: {e}", socket.display()))?;
    stream.set_read_timeout(Some(IPC_TIMEOUT))?;
    stream.set_write_timeout(Some(IPC_TIMEOUT))?;
    stream.write_all(command.as_bytes())?;
    stream.write_all(b"\n")?;

    let mut response = String::new();
    BufReader::new(stream).read_line(&mut response)?;
    let response = response.trim_end_matches(['\r', '\n']);
    if response.is_empty() {
        anyhow::bail!("gslapper: empty IPC response");
    }
    if let Some(error) = response.strip_prefix("ERROR:") {
        anyhow::bail!("{}", error.trim());
    }
    Ok(response.to_owned())
}

/// Stop a gslapper via IPC `stop` (never `pkill`, per waytrogen).
fn gslapper_stop(socket: &Path) -> anyhow::Result<()> {
    if !socket.exists() {
        return Ok(());
    }
    match gslapper_ipc(socket, "stop") {
        Ok(_) => {}
        // A connect failure means the process already died; just reap the stale
        // socket file and move on.
        Err(_) if UnixStream::connect(socket).is_err() => {
            let _ = std::fs::remove_file(socket);
            return Ok(());
        }
        Err(e) => return Err(e),
    }
    let _ = std::fs::remove_file(socket);
    Ok(())
}

/// Stop every gslapper socket in ChronOS's namespace (best-effort).
fn stop_all_gslappers() -> anyhow::Result<()> {
    let dir = runtime_dir();
    if !dir.is_dir() {
        return Ok(());
    }
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default();
        if name.starts_with("gslapper-") && name.ends_with(".sock") {
            let _ = gslapper_stop(&path);
        }
    }
    Ok(())
}

/// Spawn gslapper and wait (bounded) for its IPC socket to come up.
fn gslapper_spawn(monitor: &str, socket: &Path, path: &Path) -> anyhow::Result<()> {
    let argv = gslapper_launch_argv(monitor, socket, path);
    let Some((bin, args)) = argv.split_first() else {
        anyhow::bail!("gslapper launch argv is empty");
    };
    Command::new(bin)
        .args(args)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|e| anyhow::anyhow!("gslapper: spawn failed: {e}"))?;

    let deadline = Instant::now() + STARTUP_TIMEOUT;
    loop {
        if gslapper_ipc(socket, "query").is_ok() {
            return Ok(());
        }
        if Instant::now() >= deadline {
            anyhow::bail!("gslapper IPC socket did not become ready");
        }
        std::thread::sleep(PROBE_INTERVAL);
    }
}

/// Live monitor names via `hyprctl monitors` (for hyprpaper "All" fan-out).
async fn monitor_names() -> anyhow::Result<Vec<String>> {
    tokio::task::spawn_blocking(|| {
        let output = Command::new("hyprctl")
            .arg("monitors")
            .output()
            .map_err(|e| anyhow::anyhow!("failed to run `hyprctl monitors`: {e}"))?;
        if !output.status.success() {
            anyhow::bail!("`hyprctl monitors` failed");
        }
        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut names = Vec::new();
        for line in stdout.lines() {
            let line = line.trim_start();
            if let Some(rest) = line.strip_prefix("Monitor ") {
                if let Some(name) = rest.split_whitespace().next() {
                    names.push(name.to_string());
                }
            }
        }
        if names.is_empty() {
            anyhow::bail!("`hyprctl monitors` returned no monitor names");
        }
        Ok(names)
    })
    .await
    .map_err(|e| anyhow::anyhow!("hyprctl monitors join error: {e}"))?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hyprpaper_argv_matches_reference() {
        let argv = hyprpaper_argv("DP-1", Path::new("/pics/a.png"), "cover");
        assert_eq!(
            argv,
            vec![
                "hyprctl",
                "hyprpaper",
                "wallpaper",
                "DP-1,/pics/a.png,cover",
            ]
        );
    }

    #[test]
    fn hyprpaper_systemctl_start_argv_matches_unit_name() {
        assert_eq!(
            hyprpaper_systemctl_start_argv(),
            vec!["systemctl", "--user", "start", "hyprpaper"]
        );
    }

    /// T351 live proof of the `Some(monitor)` branch + lazy daemon bootstrap:
    /// kills any running hyprpaper, applies to one monitor, asserts the daemon
    /// was raised. Needs a real Hyprland session — hence `#[ignore]`.
    #[test]
    #[ignore = "drives the real hyprpaper daemon via hyprctl; needs a live Hyprland session"]
    fn live_apply_hyprpaper_single_monitor_bootstraps_daemon() {
        let _ = Command::new("pkill").args(["-x", "hyprpaper"]).status();
        std::thread::sleep(Duration::from_millis(500));
        assert!(!process_alive("hyprpaper"), "precondition: daemon dead");

        let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
        rt.block_on(async {
            let cmd = WallpaperCommand {
                path: PathBuf::from("/tmp/t351-red.png"),
                monitor: Some("DP-1".into()),
                transition: None,
            };
            apply_hyprpaper(&cmd)
                .await
                .expect("apply_hyprpaper single-monitor");
        });
        assert!(process_alive("hyprpaper"), "daemon must be alive after Set");
    }

    /// T352 live legs for the point-Set divergence: ChronOS `apply_swaybg`
    /// restarts ONE process carrying only the current command's monitor, so a
    /// Set on another monitor drops the first monitor's image (waytrogen
    /// would rebuild all known `-o` blocks from its registry). Run the two
    /// tests in order, grimming between, to document the fact.
    #[test]
    #[ignore = "drives real swaybg on a live Hyprland session; needs /tmp/t351-red.png"]
    fn live_apply_swaybg_point_set_dp1() {
        let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
        rt.block_on(async {
            let cmd = WallpaperCommand {
                path: PathBuf::from("/tmp/t351-red.png"),
                monitor: Some("DP-1".into()),
                transition: None,
            };
            apply_swaybg(&cmd).await.expect("apply_swaybg DP-1");
        });
        std::thread::sleep(Duration::from_millis(700));
        assert!(process_alive("swaybg"), "swaybg must be alive after Set");
    }

    #[test]
    #[ignore = "run after live_apply_swaybg_point_set_dp1; needs /tmp/t351-blue.png"]
    fn live_apply_swaybg_point_set_hdmi_second() {
        let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
        rt.block_on(async {
            let cmd = WallpaperCommand {
                path: PathBuf::from("/tmp/t351-blue.png"),
                monitor: Some("HDMI-A-1".into()),
                transition: None,
            };
            apply_swaybg(&cmd).await.expect("apply_swaybg HDMI-A-1");
        });
        std::thread::sleep(Duration::from_millis(700));
        assert!(process_alive("swaybg"), "swaybg must be alive after Set");
    }

    #[test]
    fn swaybg_argv_all_omits_output_block() {
        let argv = swaybg_argv(None, Path::new("/pics/a.png"), "fill", "#000000");
        assert_eq!(
            argv,
            vec!["swaybg", "-i", "/pics/a.png", "-m", "fill", "-c", "#000000"]
        );
    }

    #[test]
    fn swaybg_argv_one_monitor_prepends_output() {
        let argv = swaybg_argv(Some("HDMI-A-1"), Path::new("/p/a.png"), "fit", "#123456");
        assert_eq!(
            argv,
            vec![
                "swaybg",
                "-o",
                "HDMI-A-1",
                "-i",
                "/p/a.png",
                "-m",
                "fit",
                "-c",
                "#123456",
            ]
        );
    }

    #[test]
    fn mpvpaper_argv_all_with_defaults() {
        let argv = mpvpaper_argv(None, Path::new("/v/a.mp4"), "", MpvpaperPause::AutoPause, None);
        assert_eq!(
            argv,
            vec!["mpvpaper", "--auto-pause", "*", "/v/a.mp4", "-f"]
        );
    }

    #[test]
    fn mpvpaper_argv_one_monitor_with_opts_and_slideshow() {
        let argv = mpvpaper_argv(
            Some("DP-1"),
            Path::new("/v/a.mp4"),
            "loop",
            MpvpaperPause::None,
            Some(30),
        );
        assert_eq!(
            argv,
            vec!["mpvpaper", "-o", "loop", "-n", "30", "DP-1", "/v/a.mp4", "-f"]
        );
    }

    #[test]
    fn gslapper_launch_argv_matches_reference_defaults() {
        let argv = gslapper_launch_argv(
            "DP-1",
            Path::new("/run/user/1000/chronos/gslapper-0000000000000000.sock"),
            Path::new("/v/a.mp4"),
        );
        assert!(argv.windows(2).any(|p| p == ["-r", "30"]));
        assert!(argv.windows(2).any(|p| p == ["--cache-size", "256"]));
        assert!(argv.iter().any(|a| a == "fill no-audio loop"));
        assert!(!argv.iter().any(|a| a == "-f")); // mpvpaper's flag, not gslapper's
        assert_eq!(argv.last().map(String::as_str), Some("/v/a.mp4"));
    }

    #[test]
    fn gslapper_socket_path_is_in_chronos_namespace_and_output_specific() {
        let dp = gslapper_socket_path("DP-1");
        let hdmi = gslapper_socket_path("HDMI-A-1");
        assert_ne!(dp, hdmi);
        assert!(dp.to_string_lossy().contains("/chronos/gslapper-"));
        assert!(dp.to_string_lossy().ends_with(".sock"));
        assert!(!dp.to_string_lossy().contains("/waytrogen/"));
    }

    #[test]
    fn stable_output_id_is_deterministic() {
        assert_eq!(stable_output_id("*"), stable_output_id("*"));
        assert_ne!(stable_output_id("DP-1"), stable_output_id("HDMI-A-1"));
    }

    #[test]
    fn backend_process_bin_maps_awww_to_daemon() {
        assert_eq!(Backend::Awww.process_bin(), "awww-daemon");
        assert_eq!(Backend::Mpvpaper.process_bin(), "mpvpaper");
    }

    #[test]
    fn backend_parse_is_case_insensitive_and_exhaustive() {
        for (raw, expected) in [
            ("awww", Backend::Awww),
            ("HyprPaper", Backend::Hyprpaper),
            ("SWAYBG", Backend::Swaybg),
            ("Mpvpaper", Backend::Mpvpaper),
            ("gslapper", Backend::Gslapper),
        ] {
            assert_eq!(Backend::parse(raw), Some(expected), "raw={raw}");
        }
        assert_eq!(Backend::parse("bogus"), None);
        assert_eq!(Backend::parse(""), None);
    }

    #[test]
    fn only_video_backends_report_video_support() {
        assert!(Backend::Mpvpaper.supports_video());
        assert!(Backend::Gslapper.supports_video());
        assert!(!Backend::Awww.supports_video());
        assert!(!Backend::Hyprpaper.supports_video());
        assert!(!Backend::Swaybg.supports_video());
    }
}
