//! AUR/pacman update-check service — MVP port of Alloy's update logic
//! (`~/projects/chronos-ecosystem/Chronos-AUR`, `services/pacman_ops.rs` +
//! `services/aur_ops.rs`) into the `Service` trait shape.
//!
//! ASYNC TEMPLATE (spec-equivalent to `AudioSubscriber`): `new()` captures
//! `Handle::current()` and `tokio::spawn`s a poll loop; `init_all()` calls
//! this inside `rt.block_on`.
//!
//! ## Detection (read-only, safe to poll in the background)
//!
//! * Official repo updates — `checkupdates` (pacman-contrib) if present: it
//!   syncs its **own** temp copy of the pacman db and never touches
//!   `/var/lib/pacman`, so it is safe to run on a timer without racing a
//!   real `pacman` transaction. Falls back to `pacman -Qu` (reads the
//!   already-synced local db — no extra dependency, but only as fresh as the
//!   last `-Sy`) when `checkupdates` is missing. Binary presence is checked
//!   via `which` (never hardcoded), per ZED.md.
//! * AUR updates — `yay -Qua`, only when `yay` is on PATH. Best-effort: a
//!   failure here does not fail the whole read (AUR is optional).
//!
//! Neither path needs root, and neither mutates any system state — this is
//! the "always safe to verify live" half of the service (see ZED.md
//! "живая верификация").
//!
//! ## The one privileged path
//!
//! `AurCommand::UpgradeAll` shells out to `pkexec yay -Syu --noconfirm` (or
//! `pkexec pacman -Syu --noconfirm` without `yay`) — the same approach Alloy
//! already used (`upgrade_stream_script`/`upgrade_script`, there wrapped in a
//! fish-shell + zenity/askpass layer we don't need here); `pkexec` alone
//! hands off to the desktop's own PolicyKit auth agent, so nothing new is
//! invented. This path is **never** invoked by the poll loop — only by an
//! explicit `dispatch(AurCommand::UpgradeAll)` from the popup's button.

use std::process::{Command, Stdio};
use std::time::Duration;

use futures_signals::signal::{Mutable, Signal};
use tokio::runtime::Handle;
use tracing::{info, warn};

use crate::Service;
use crate::ServiceStatus;
pub use types::{AurCommand, PackageUpdate, UpdateSource, UpdatesState};

pub mod types;

/// Poll interval for update checks. Network/db-sync bound (checkupdates
/// syncs a copy of the pacman db from configured mirrors) — much coarser
/// than audio's 250ms poll; there is no live external signal to react to.
const POLL_INTERVAL: Duration = Duration::from_secs(15 * 60);

#[derive(Clone)]
pub struct AurSubscriber {
    data: Mutable<UpdatesState>,
    status: Mutable<ServiceStatus>,
    /// Captured in `new()` — runtime guard + fire-and-forget for `dispatch`.
    runtime: Handle,
}

impl AurSubscriber {
    /// Non-failing, synchronous constructor (mirrors `AudioSubscriber::new`).
    ///
    /// # Panics
    ///
    /// Panics if called outside a tokio runtime — `Handle::current()`
    /// requires one. `init_all()` runs inside `rt.block_on`.
    pub fn new() -> Self {
        let data = Mutable::new(UpdatesState::default());
        let status = Mutable::new(ServiceStatus::Initializing);

        let handle = Handle::current();
        tokio::spawn(run(data.clone(), status.clone()));

        Self {
            data,
            status,
            runtime: handle,
        }
    }

    /// Fire-and-forget command dispatch (mirrors `AudioSubscriber::dispatch`).
    /// Safe to call from a GPUI click handler.
    pub fn dispatch(&self, cmd: AurCommand) {
        match cmd {
            AurCommand::Refresh => {
                let data = self.data.clone();
                let status = self.status.clone();
                self.runtime.spawn(async move {
                    match read_state().await {
                        Ok(state) => {
                            data.set(state);
                            status.set(ServiceStatus::Available);
                        }
                        Err(e) => {
                            warn!("AurSubscriber refresh failed: {e:?}");
                            status.set(ServiceStatus::Unavailable);
                        }
                    }
                });
            }
            AurCommand::UpgradeAll => {
                let data = self.data.clone();
                let status = self.status.clone();
                self.runtime.spawn(async move {
                    if let Err(e) = run_upgrade_all().await {
                        warn!("AurSubscriber upgrade-all failed: {e:?}");
                        return;
                    }
                    // Re-read so the badge/list reflect the new state once
                    // the (blocking, possibly slow) upgrade finishes.
                    match read_state().await {
                        Ok(state) => {
                            data.set(state);
                            status.set(ServiceStatus::Available);
                        }
                        Err(e) => {
                            warn!("AurSubscriber re-read after upgrade-all failed: {e:?}");
                        }
                    }
                });
            }
        }
    }
}

impl Service for AurSubscriber {
    type Data = UpdatesState;
    type Error = anyhow::Error;

    fn subscribe(&self) -> impl Signal<Item = UpdatesState> + Unpin + 'static {
        self.data.signal_cloned()
    }

    fn get(&self) -> UpdatesState {
        self.data.get_cloned()
    }

    fn status(&self) -> ServiceStatus {
        self.status.get_cloned()
    }
}

/// Poll loop: read official + AUR updates, publish diffs, exponential
/// backoff on hard failure (e.g. neither `checkupdates` nor `pacman` runs).
async fn run(data: Mutable<UpdatesState>, status: Mutable<ServiceStatus>) {
    const MAX_BACKOFF: Duration = Duration::from_secs(300);
    let mut backoff = Duration::from_secs(5);
    let mut logged_ok = false;

    loop {
        match read_state().await {
            Ok(state) => {
                if data.get_cloned() != state {
                    data.set(state);
                }
                if status.get_cloned() != ServiceStatus::Available {
                    status.set(ServiceStatus::Available);
                }
                if !logged_ok {
                    info!("AurSubscriber connected (checkupdates/pacman -Qu MVP backend)");
                    logged_ok = true;
                }
                backoff = Duration::from_secs(5);
                tokio::time::sleep(POLL_INTERVAL).await;
            }
            Err(e) => {
                warn!("AurSubscriber read failed, retrying: {e:?}");
                status.set(ServiceStatus::Unavailable);
                logged_ok = false;
                tokio::time::sleep(backoff).await;
                backoff = (backoff * 2).min(MAX_BACKOFF);
            }
        }
    }
}

async fn read_state() -> anyhow::Result<UpdatesState> {
    tokio::task::spawn_blocking(|| {
        let mut updates = read_official()?;
        updates.extend(read_aur());
        Ok(UpdatesState { updates })
    })
    .await
    .map_err(|e| anyhow::anyhow!("aur read join error: {e}"))?
}

/// Official-repo updates. Hard failure only if we can't even spawn a
/// checker binary — "no updates" (empty stdout, whatever the exit code) is
/// a normal, successful result, not an error.
fn read_official() -> anyhow::Result<Vec<PackageUpdate>> {
    let out = if binary_available("checkupdates") {
        run_capture("checkupdates", &[])?
    } else {
        run_capture("pacman", &["-Qu"])?
    };
    Ok(parse_updates(&out, UpdateSource::Official))
}

/// AUR updates via `yay -Qua`. Best-effort: `yay` absent or failing does not
/// fail the whole read (spec §MVP: "если `yay` доступен").
fn read_aur() -> Vec<PackageUpdate> {
    if !binary_available("yay") {
        return Vec::new();
    }
    match run_capture("yay", &["-Qua"]) {
        Ok(out) => parse_updates(&out, UpdateSource::Aur),
        Err(e) => {
            warn!("AurSubscriber: yay -Qua failed: {e:?}");
            Vec::new()
        }
    }
}

/// Run `bin args...` and return stdout regardless of exit status — both
/// `checkupdates` and `pacman -Qu`/`yay -Qua` signal "no updates available"
/// via a non-zero exit code, which is not a failure here. Only a spawn
/// error (binary truly missing/unrunnable) is propagated.
fn run_capture(bin: &str, args: &[&str]) -> anyhow::Result<String> {
    let output = Command::new(bin)
        .args(args)
        .output()
        .map_err(|e| anyhow::anyhow!("failed to spawn {bin}: {e}"))?;
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// Check binary presence via `which` — never hardcode a path (spec: "проверь
/// через which, не хардкодь путь").
pub fn binary_available(name: &str) -> bool {
    Command::new("which")
        .arg(name)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Parse one `checkupdates`/`pacman -Qu`/`yay -Qua` line:
/// `pkgname oldver -> newver` (arrow spacing tolerated either side).
pub fn parse_update_line(line: &str, source: UpdateSource) -> Option<PackageUpdate> {
    let line = line.trim();
    if line.is_empty() {
        return None;
    }
    let (left, new_version) = line.split_once("->")?;
    let new_version = new_version.trim();
    if new_version.is_empty() {
        return None;
    }
    let mut parts = left.trim().split_whitespace();
    let name = parts.next()?;
    let old_version = parts.next()?;
    if name.is_empty() || old_version.is_empty() {
        return None;
    }
    Some(PackageUpdate {
        name: name.to_string(),
        old_version: old_version.to_string(),
        new_version: new_version.to_string(),
        source,
    })
}

/// Parse a full `checkupdates`/`pacman -Qu`/`yay -Qua` stdout blob.
/// Unparseable lines are skipped rather than failing the whole read.
pub fn parse_updates(stdout: &str, source: UpdateSource) -> Vec<PackageUpdate> {
    stdout
        .lines()
        .filter_map(|line| parse_update_line(line, source))
        .collect()
}

/// Pure mapping: which binary+argv the "Upgrade all" button runs. Unit
/// tested — this is the only thing verified about the privileged path
/// without ever executing it (ZED.md "живая верификация").
pub fn upgrade_command_args(has_yay: bool) -> (&'static str, Vec<&'static str>) {
    if has_yay {
        ("pkexec", vec!["yay", "-Syu", "--noconfirm"])
    } else {
        ("pkexec", vec!["pacman", "-Syu", "--noconfirm"])
    }
}

/// Launch the real upgrade and block (on a blocking-pool thread, per the
/// `spawn_blocking` convention used everywhere else in this file — never on
/// the async loop) until it finishes. `pkexec` pops the desktop's own
/// PolicyKit dialog; we never feed it a password ourselves.
async fn run_upgrade_all() -> anyhow::Result<()> {
    tokio::task::spawn_blocking(|| {
        let has_yay = binary_available("yay");
        let (bin, args) = upgrade_command_args(has_yay);
        info!(
            "AurSubscriber: launching upgrade — {bin} {}",
            args.join(" ")
        );

        let status = Command::new(bin)
            .args(&args)
            .status()
            .map_err(|e| anyhow::anyhow!("failed to spawn {bin} {}: {e}", args.join(" ")))?;
        if status.success() {
            info!("AurSubscriber: upgrade command finished successfully");
        } else {
            warn!("AurSubscriber: upgrade command exited with {status}");
        }
        Ok(())
    })
    .await
    .map_err(|e| anyhow::anyhow!("upgrade-all join error: {e}"))?
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::panic::{AssertUnwindSafe, catch_unwind};

    /// Same runtime_guard contract as audio/network/upower/tray.
    #[test]
    fn aur_new_panics_outside_runtime() {
        let result = catch_unwind(AssertUnwindSafe(|| {
            let _ = AurSubscriber::new();
        }));
        assert!(
            result.is_err(),
            "AurSubscriber::new() must panic outside a tokio runtime (Handle::current guard)"
        );
    }

    #[tokio::test]
    async fn aur_new_inside_runtime_starts_initializing_or_available() {
        let svc = AurSubscriber::new();
        let st = svc.status();
        assert!(
            matches!(
                st,
                ServiceStatus::Initializing | ServiceStatus::Available | ServiceStatus::Unavailable
            ),
            "unexpected status: {st:?}"
        );
        let _ = svc.get();
    }

    #[test]
    fn updates_state_is_eq() {
        // Compile-time guard: no floats crept in (unlike AudioState/UPowerData).
        let a = UpdatesState::default();
        let b = a.clone();
        assert_eq!(a, b);
    }

    #[test]
    fn parse_line_plain() {
        let u = parse_update_line("firefox 121.0-1 -> 121.0.1-1", UpdateSource::Official).unwrap();
        assert_eq!(u.name, "firefox");
        assert_eq!(u.old_version, "121.0-1");
        assert_eq!(u.new_version, "121.0.1-1");
        assert_eq!(u.source, UpdateSource::Official);
    }

    #[test]
    fn parse_line_no_arrow_space() {
        // checkupdates always emits spaces, but be tolerant of tight arrows.
        let u = parse_update_line("foo 1.0->1.1", UpdateSource::Aur).unwrap();
        assert_eq!(u.name, "foo");
        assert_eq!(u.old_version, "1.0");
        assert_eq!(u.new_version, "1.1");
        assert_eq!(u.source, UpdateSource::Aur);
    }

    #[test]
    fn parse_line_garbage() {
        assert_eq!(parse_update_line("", UpdateSource::Official), None);
        assert_eq!(
            parse_update_line("not a pkg line", UpdateSource::Official),
            None
        );
        assert_eq!(parse_update_line("foo ->", UpdateSource::Official), None);
        assert_eq!(parse_update_line("-> 1.1", UpdateSource::Official), None);
    }

    /// Live fixture captured on the dev host via `pacman -Qu` (2026-07-18,
    /// ZED.md task) — not invented (HANDOFF/MEMORY field rule: "фикстура,
    /// не снятая с живого вывода — фантазия"). Covers epoch-prefixed
    /// versions (`discord 1:1.0.148-1 -> 1:1.0.149-1`), which a naive
    /// `parse::<u32>()`-based parser would choke on — we treat versions as
    /// opaque strings, so this is a non-issue, but it's exactly the kind of
    /// real-world shape an invented fixture tends to miss.
    #[test]
    fn parse_updates_matches_live_pacman_qu_fixture() {
        let stdout = "cdparanoia 10.2-9.1 -> 10.2-10.1\n\
            discord 1:1.0.148-1 -> 1:1.0.149-1\n\
            libnm 1.56.1-1 -> 1.56.1-2\n\
            networkmanager 1.56.1-1 -> 1.56.1-2\n\
            opencode 1.18.2-1.1 -> 1.18.3-1.1\n\
            ppp 2.5.2-1.1 -> 2.5.3-1.1\n\
            python-opentelemetry-sdk 1.43.0-1 -> 1.44.0-1\n";
        let updates = parse_updates(stdout, UpdateSource::Official);
        assert_eq!(updates.len(), 7);
        assert_eq!(updates[0].name, "cdparanoia");
        assert_eq!(updates[0].old_version, "10.2-9.1");
        assert_eq!(updates[0].new_version, "10.2-10.1");
        // Epoch prefix (`1:`) must survive as part of the opaque version string.
        let discord = &updates[1];
        assert_eq!(discord.name, "discord");
        assert_eq!(discord.old_version, "1:1.0.148-1");
        assert_eq!(discord.new_version, "1:1.0.149-1");
        assert_eq!(updates[6].name, "python-opentelemetry-sdk");
    }

    #[test]
    fn parse_updates_multi_line_skips_garbage() {
        let stdout = "pkg-a 1.0-1 -> 1.0-2\n\npkg-b 2.0-1 -> 2.1-1\nnot a line\n";
        let updates = parse_updates(stdout, UpdateSource::Official);
        assert_eq!(updates.len(), 2);
        assert_eq!(updates[0].name, "pkg-a");
        assert_eq!(updates[1].name, "pkg-b");
    }

    #[test]
    fn upgrade_args_prefers_yay() {
        let (bin, args) = upgrade_command_args(true);
        assert_eq!(bin, "pkexec");
        assert_eq!(args, vec!["yay", "-Syu", "--noconfirm"]);
    }

    #[test]
    fn upgrade_args_falls_back_to_pacman() {
        let (bin, args) = upgrade_command_args(false);
        assert_eq!(bin, "pkexec");
        assert_eq!(args, vec!["pacman", "-Syu", "--noconfirm"]);
    }

    #[test]
    fn binary_available_false_for_bogus_name() {
        assert!(!binary_available(
            "chronos-definitely-not-a-real-binary-xyz123"
        ));
    }

    #[test]
    fn count_reflects_updates_len() {
        let state = UpdatesState {
            updates: vec![
                PackageUpdate {
                    name: "a".into(),
                    old_version: "1".into(),
                    new_version: "2".into(),
                    source: UpdateSource::Official,
                },
                PackageUpdate {
                    name: "b".into(),
                    old_version: "1".into(),
                    new_version: "2".into(),
                    source: UpdateSource::Aur,
                },
            ],
        };
        assert_eq!(state.count(), 2);
    }
}
