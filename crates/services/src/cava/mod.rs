//! Cava audio visualizer — long-lived child process, ascii raw frames.
//!
//! Soft-fails when `cava` is missing or exits: keeps an empty/zero frame and
//! reconnects with exponential backoff (same shape as other service loops).
//! Does not panics or take the shell down.

use std::path::PathBuf;
use std::process::Stdio;
use std::time::Duration;

use futures_signals::signal::{Mutable, Signal};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::runtime::Handle;
use tracing::{info, warn};

use crate::{Service, ServiceStatus};

pub mod types;
pub use types::CavaState;

/// Bars requested in the chronos cava config (matches design mockup).
pub const BAR_COUNT: usize = 24;
/// Matches config `ascii_max_range`.
pub const ASCII_MAX: u8 = 100;

/// Minimal cava config for pipewire → ascii raw on stdout.
const CAVA_CONFIG: &str = r#"
[general]
bars = 24
framerate = 30
autosens = 1
sensitivity = 100

[input]
method = pipewire

[output]
method = raw
raw_target = /dev/stdout
data_format = ascii
ascii_max_range = 100
bar_delimiter = 59
frame_delimiter = 10
channels = mono
"#;

#[derive(Clone)]
pub struct CavaSubscriber {
    data: Mutable<CavaState>,
    status: Mutable<ServiceStatus>,
}

impl CavaSubscriber {
    /// Non-failing constructor. Spawns the cava reader loop on the current
    /// tokio runtime (must be called inside `rt.block_on`, like other services).
    ///
    /// # Panics
    ///
    /// If no tokio runtime is active (`Handle::current()`).
    pub fn new() -> Self {
        let data = Mutable::new(vec![0u8; BAR_COUNT]);
        let status = Mutable::new(ServiceStatus::Initializing);
        let _handle = Handle::current();
        tokio::spawn(run(data.clone(), status.clone()));
        Self { data, status }
    }
}

impl Service for CavaSubscriber {
    type Data = CavaState;
    type Error = anyhow::Error;

    fn subscribe(&self) -> impl Signal<Item = CavaState> + Unpin + 'static {
        self.data.signal_cloned()
    }

    fn get(&self) -> CavaState {
        self.data.get_cloned()
    }

    fn status(&self) -> ServiceStatus {
        self.status.get_cloned()
    }
}

/// Parse one ascii raw frame: `n;n;...;n;` or `n;n;...;n` → bar heights 0..=100.
///
/// Pure; unit-tested against a real machine capture.
pub fn parse_cava_ascii_line(line: &str) -> Option<Vec<u8>> {
    let line = line.trim();
    if line.is_empty() {
        return None;
    }
    let mut bars = Vec::with_capacity(BAR_COUNT);
    for part in line.split(';') {
        if part.is_empty() {
            // trailing delimiter after last bar
            continue;
        }
        let v: u32 = part.parse().ok()?;
        bars.push(v.min(u32::from(ASCII_MAX)) as u8);
    }
    if bars.is_empty() {
        None
    } else {
        Some(bars)
    }
}

fn write_config() -> anyhow::Result<PathBuf> {
    let dir = std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    let path = dir.join("chronos-cava.conf");
    std::fs::write(&path, CAVA_CONFIG.trim_start())?;
    Ok(path)
}

async fn run(data: Mutable<CavaState>, status: Mutable<ServiceStatus>) {
    const MAX_BACKOFF: Duration = Duration::from_secs(60);
    let mut backoff = Duration::from_secs(1);
    let mut logged_spawn_fail = false;

    loop {
        match run_once(&data, &status, &mut logged_spawn_fail).await {
            Ok(()) => {
                // EOF / process exit — treat like a soft failure, grow backoff
                // so a crash-looping binary does not hammer every second.
                status.set(ServiceStatus::Unavailable);
                data.set(vec![0u8; BAR_COUNT]);
                if !logged_spawn_fail {
                    warn!("cava: process ended, restarting with backoff");
                    logged_spawn_fail = true;
                }
            }
            Err(e) => {
                status.set(ServiceStatus::Unavailable);
                data.set(vec![0u8; BAR_COUNT]);
                if !logged_spawn_fail {
                    warn!("cava: unavailable ({e:?}); soft-fail, will retry");
                    logged_spawn_fail = true;
                }
            }
        }
        tokio::time::sleep(backoff).await;
        backoff = (backoff * 2).min(MAX_BACKOFF);
    }
}

async fn run_once(
    data: &Mutable<CavaState>,
    status: &Mutable<ServiceStatus>,
    logged_spawn_fail: &mut bool,
) -> anyhow::Result<()> {
    let config = write_config()?;
    let mut child = Command::new("cava")
        .arg("-p")
        .arg(&config)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .map_err(|e| anyhow::anyhow!("spawn cava: {e}"))?;

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| anyhow::anyhow!("cava stdout missing"))?;
    let mut reader = BufReader::new(stdout).lines();

    status.set(ServiceStatus::Available);
    if *logged_spawn_fail {
        info!("cava: process started");
        *logged_spawn_fail = false;
    } else {
        info!("cava: process started (bars={BAR_COUNT}, ascii raw)");
    }

    while let Some(line) = reader.next_line().await? {
        if let Some(bars) = parse_cava_ascii_line(&line) {
            // Always publish: visualizer needs every frame even if identical.
            data.set(bars);
        }
    }

    // Drain exit status (ignore code — either way we restart).
    let _ = child.wait().await;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::panic::{AssertUnwindSafe, catch_unwind};

    #[test]
    fn parse_real_capture_line() {
        // Live capture on this machine 2026-07-19 with speaker-test + cava 0.10.7
        // config bars=24 ascii_max_range=100 mono raw stdout.
        let line = "0;0;1;1;1;2;2;4;7;19;100;12;7;5;4;3;3;3;2;2;2;2;2;2;";
        let bars = parse_cava_ascii_line(line).expect("parses");
        assert_eq!(bars.len(), 24);
        assert_eq!(bars[0], 0);
        assert_eq!(bars[10], 100);
        assert_eq!(bars[11], 12);
        assert_eq!(bars[23], 2);
    }

    #[test]
    fn parse_without_trailing_delimiter() {
        let line = "1;2;3";
        let bars = parse_cava_ascii_line(line).unwrap();
        assert_eq!(bars, vec![1, 2, 3]);
    }

    #[test]
    fn parse_clamps_above_max() {
        let bars = parse_cava_ascii_line("0;150;200").unwrap();
        assert_eq!(bars, vec![0, 100, 100]);
    }

    #[test]
    fn parse_empty_and_garbage() {
        assert!(parse_cava_ascii_line("").is_none());
        assert!(parse_cava_ascii_line("   ").is_none());
        assert!(parse_cava_ascii_line("a;b;c").is_none());
    }

    #[test]
    fn cava_new_panics_outside_runtime() {
        let result = catch_unwind(AssertUnwindSafe(|| {
            let _ = CavaSubscriber::new();
        }));
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn cava_new_inside_runtime() {
        let svc = CavaSubscriber::new();
        let st = svc.status();
        assert!(matches!(
            st,
            ServiceStatus::Initializing | ServiceStatus::Available | ServiceStatus::Unavailable
        ));
        let frame = svc.get();
        assert_eq!(frame.len(), BAR_COUNT);
    }
}
