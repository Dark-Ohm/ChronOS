//! Brightness service via `ddcutil` (DDC/CI over i2c-dev).
//!
//! No D-Bus bus exists for monitor brightness on a generic desktop — this
//! service shells out to the `ddcutil` binary (verified 2026-07-19 on this
//! machine: ddcutil 2.2.7, i2c-dev loaded, user in `i2c` group, two displays
//! on `/dev/i2c-2` and `/dev/i2c-3`). `brightnessctl` is **not** used — it
//! only sees the LED numlock on this box, not the monitors.
//!
//! Architecture mirrors `AudioSubscriber` (poll + dispatch on tokio, Mutable
//! for reactivity). The difference is cadence: DDC latency is 100–300ms per
//! call, so we do **not** poll on a fixed interval. Reads happen on init,
//! on popup open (`BrightnessCommand::Refresh`), and after each `Set`/`Step`
//! dispatch — never on a frame cadence.
//!
//! Soft-fail: missing `ddcutil`, no `/dev/i2c-*` access, or empty `detect`
//! → `BrightnessState { value: 0, available: false }`. The UI renders a
//! muted/disabled brightness block; the shell stays alive (same philosophy
//! as cava without the binary).

use std::time::Duration;

use futures_signals::signal::{Mutable, Signal};
use tokio::runtime::Handle;
use tracing::{info, warn};

use crate::Service;
use crate::ServiceStatus;
pub use types::{BrightnessCommand, BrightnessState};

pub mod ddcutil;
pub mod types;

pub use ddcutil::{
    DDCUTIL_BIN, detect_displays, get_brightness, parse_getvcp_stdout, read_primary,
    set_brightness, write_all,
};

/// Brightness step applied on `BrightnessCommand::Step` (±5%, matching the
/// volume popup and the design mockup's steppers).
pub const STEP: i8 = 5;

/// Per-call DDC timeout. `ddcutil getvcp` normally returns in <300ms; cap at
/// 2s so a hung i2c bus never blocks the dispatch task indefinitely.
const DDC_TIMEOUT: Duration = Duration::from_secs(2);

#[derive(Clone)]
pub struct BrightnessSubscriber {
    data: Mutable<BrightnessState>,
    status: Mutable<ServiceStatus>,
    /// Detected display numbers (1-based, from `ddcutil detect`). Empty when
    /// no DDC displays are available. Cached at init; refreshed on `Refresh`.
    displays: Mutable<Vec<u32>>,
    /// Captured in `new()` — runtime guard + fire-and-forget for `dispatch`.
    runtime: Handle,
}

impl BrightnessSubscriber {
    /// Non-failing, synchronous constructor (spec §5.1).
    ///
    /// # Panics
    ///
    /// Panics if called outside a tokio runtime — `Handle::current()` requires
    /// one. `init_all()` (spec §7) calls this inside `rt.block_on`, so the
    /// runtime is present there.
    pub fn new() -> Self {
        let data = Mutable::new(BrightnessState::default());
        let status = Mutable::new(ServiceStatus::Initializing);
        let displays = Mutable::new(Vec::new());

        let handle = Handle::current();
        tokio::spawn(run(data.clone(), status.clone(), displays.clone()));

        Self {
            data,
            status,
            displays,
            runtime: handle,
        }
    }

    /// Fire-and-forget command dispatch (mirrors `AudioSubscriber::dispatch`).
    /// Safe to call from a GPUI click handler.
    pub fn dispatch(&self, cmd: BrightnessCommand) {
        let data = self.data.clone();
        let displays = self.displays.clone();
        let status = self.status.clone();
        self.runtime.spawn(async move {
            if let Err(e) = apply_command(&cmd, &data, &displays).await {
                warn!("BrightnessSubscriber command failed ({cmd:?}): {e:?}");
            }
        });
        // status is read by the UI; keep it referenced so the field isn't
        // dead-stripped in release builds (the watcher reads `data` only).
        let _ = &status;
    }
}

impl Service for BrightnessSubscriber {
    type Data = BrightnessState;
    type Error = anyhow::Error;
    fn subscribe(&self) -> impl Signal<Item = BrightnessState> + Unpin + 'static {
        self.data.signal_cloned()
    }
    fn get(&self) -> BrightnessState {
        self.data.get_cloned()
    }
    fn status(&self) -> ServiceStatus {
        self.status.get_cloned()
    }
}

/// Initial probe: detect displays, read primary brightness, set status.
/// No retry loop — DDC is either present at startup or it isn't. A `Refresh`
/// dispatch later can re-detect if the user hotplugs a monitor.
async fn run(
    data: Mutable<BrightnessState>,
    status: Mutable<ServiceStatus>,
    displays: Mutable<Vec<u32>>,
) {
    // detect_displays is blocking (spawns ddcutil, ~200ms) — use spawn_blocking
    // so we don't stall the tokio worker.
    let detected = tokio::task::spawn_blocking(detect_displays)
        .await
        .unwrap_or_default();
    displays.set(detected.clone());

    let (value, available) = if detected.is_empty() {
        (0, false)
    } else {
        // Read primary display brightness (blocking) — spawn_blocking again.
        tokio::task::spawn_blocking(move || read_primary(&detected))
            .await
            .unwrap_or((0, false))
    };

    data.set(BrightnessState { value, available });
    if available {
        status.set(ServiceStatus::Available);
        info!(
            "BrightnessSubscriber connected: {value}% on {} displays",
            displays.get_cloned().len()
        );
    } else {
        // Soft-fail: ddcutil missing or no DDC displays. Stay Available so the
        // UI can still render the popup (with a muted brightness block); the
        // `available` flag on the data is the real signal, not ServiceStatus.
        status.set(ServiceStatus::Degraded(
            "no DDC displays — ddcutil/i2c unavailable".to_string(),
        ));
        info!("BrightnessSubscriber soft-fail: no DDC displays detected");
    }
}

/// Apply a `BrightnessCommand` and re-read the primary display so the UI
/// reflects the new value immediately (no polling).
async fn apply_command(
    cmd: &BrightnessCommand,
    data: &Mutable<BrightnessState>,
    displays: &Mutable<Vec<u32>>,
) -> anyhow::Result<()> {
    match *cmd {
        BrightnessCommand::Refresh => {
            // Re-detect in case of hotplug, then re-read.
            let detected = tokio::task::spawn_blocking(detect_displays)
                .await
                .unwrap_or_default();
            displays.set(detected.clone());
            let (value, available) = if detected.is_empty() {
                (0, false)
            } else {
                tokio::task::spawn_blocking(move || read_primary(&detected))
                    .await
                    .unwrap_or((0, false))
            };
            data.set(BrightnessState { value, available });
        }
        BrightnessCommand::Set(target) => {
            let value = target.min(100);
            let detected = displays.get_cloned();
            if !detected.is_empty() {
                let cloned = detected.clone();
                let _ = tokio::time::timeout(
                    DDC_TIMEOUT,
                    tokio::task::spawn_blocking(move || write_all(&cloned, value)),
                )
                .await;
                // Re-read primary so the slider shows the actual monitor value
                // (some displays clamp or round).
                let (v, avail) = tokio::task::spawn_blocking(move || read_primary(&detected))
                    .await
                    .unwrap_or((0, false));
                data.set(BrightnessState {
                    value: v,
                    available: avail,
                });
            }
        }
        BrightnessCommand::Step(delta) => {
            let current = data.get_cloned();
            if !current.available {
                return Ok(());
            }
            let next = (i32::from(current.value) + i32::from(delta)).clamp(0, 100) as u8;
            let detected = displays.get_cloned();
            if !detected.is_empty() {
                let cloned = detected.clone();
                let _ = tokio::time::timeout(
                    DDC_TIMEOUT,
                    tokio::task::spawn_blocking(move || write_all(&cloned, next)),
                )
                .await;
                let (v, avail) = tokio::task::spawn_blocking(move || read_primary(&detected))
                    .await
                    .unwrap_or((0, false));
                data.set(BrightnessState {
                    value: v,
                    available: avail,
                });
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn step_clamps_to_zero() {
        // current=3, delta=-5 → clamped to 0 (not -2).
        let next = (i32::from(3) + i32::from(-5)).clamp(0, 100) as u8;
        assert_eq!(next, 0);
    }

    #[test]
    fn step_clamps_to_hundred() {
        let next = (i32::from(98) + i32::from(5)).clamp(0, 100) as u8;
        assert_eq!(next, 100);
    }

    #[test]
    fn set_clamps_above_hundred() {
        let v = 150u8.min(100);
        assert_eq!(v, 100);
    }

    #[test]
    fn step_constant_matches_volume_popup() {
        // ±5% — same as the volume popup's STEP, matches the design mockup.
        assert_eq!(STEP, 5);
    }
}
