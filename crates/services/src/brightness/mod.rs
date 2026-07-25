//! Brightness service via `ddcutil` (DDC/CI over i2c-dev).
//!
//! No D-Bus bus for monitor brightness — shells out to `ddcutil` (verified
//! 2026-07-19: ddcutil 2.2.7, i2c-dev, two displays). `brightnessctl` is
//! **not** used (LED only on this box).
//!
//! ## Dispatch model (fixed 2026-07-25)
//!
//! DDC is **serial and slow** (100–300ms per display × N). Spawning a new
//! `write_all`+`getvcp` task on every drag tick / ± click caused:
//! - out-of-order re-reads → UI "jumps for minutes"
//! - i2c contention → failed `getvcp` → `available: false` → broken widget
//!
//! Now: **latest-wins coalesce** (like audio volume) + **optimistic**
//! `data.set` before DDC + **no re-read after Set** (trust the write;
//! `Refresh` still re-reads). Transient DDC failures never clear `available`.

use std::time::Duration;

use futures_signals::signal::{Mutable, Signal};
use tokio::runtime::Handle;
use tokio::sync::watch;
use tracing::{debug, info, warn};

use crate::Service;
use crate::ServiceStatus;
pub use types::{BrightnessCommand, BrightnessState};

pub mod ddcutil;
pub mod types;

pub use ddcutil::{
    DDCUTIL_BIN, detect_displays, get_brightness, parse_getvcp_stdout, read_primary,
    set_brightness, write_all,
};

/// Brightness step applied on `BrightnessCommand::Step` (±5%).
pub const STEP: i8 = 5;

/// Cap a hung i2c write so the coalesce task can move on.
const DDC_TIMEOUT: Duration = Duration::from_secs(2);

#[derive(Clone)]
pub struct BrightnessSubscriber {
    data: Mutable<BrightnessState>,
    status: Mutable<ServiceStatus>,
    displays: Mutable<Vec<u32>>,
    runtime: Handle,
    /// Latest-wins target brightness for `Set` / `Step` (coalesce task).
    set_tx: watch::Sender<Option<u8>>,
}

impl BrightnessSubscriber {
    /// # Panics
    /// Outside a tokio runtime (`Handle::current()`).
    pub fn new() -> Self {
        let data = Mutable::new(BrightnessState::default());
        let status = Mutable::new(ServiceStatus::Initializing);
        let displays = Mutable::new(Vec::new());

        let handle = Handle::current();
        tokio::spawn(run_init(data.clone(), status.clone(), displays.clone()));

        let (set_tx, set_rx) = watch::channel::<Option<u8>>(None);
        tokio::spawn(run_set_coalesce(
            set_rx,
            data.clone(),
            displays.clone(),
        ));

        Self {
            data,
            status,
            displays,
            runtime: handle,
            set_tx,
        }
    }

    /// Fire-and-forget. Safe from GPUI click handlers.
    ///
    /// `Set` / `Step`: optimistic state + coalesce channel (no parallel DDC).
    /// `Refresh`: separate task (detect + read), does not join the set queue.
    pub fn dispatch(&self, cmd: BrightnessCommand) {
        match cmd {
            BrightnessCommand::Set(target) => {
                let value = target.min(100);
                self.optimistic_set(value);
                let _ = self.set_tx.send(Some(value));
                debug!("brightness: queue Set({value})");
            }
            BrightnessCommand::Step(delta) => {
                let current = self.data.get_cloned();
                if !current.available {
                    return;
                }
                let next = (i32::from(current.value) + i32::from(delta)).clamp(0, 100) as u8;
                self.optimistic_set(next);
                let _ = self.set_tx.send(Some(next));
                debug!("brightness: queue Step → Set({next})");
            }
            BrightnessCommand::Refresh => {
                let data = self.data.clone();
                let displays = self.displays.clone();
                let status = self.status.clone();
                self.runtime.spawn(async move {
                    if let Err(e) = refresh(&data, &displays, &status).await {
                        warn!("BrightnessSubscriber Refresh failed: {e:?}");
                    }
                });
            }
        }
    }

    fn optimistic_set(&self, value: u8) {
        let current = self.data.get_cloned();
        // Never flip available→false on a Set path.
        if current.available {
            self.data.set(BrightnessState {
                value,
                available: true,
            });
        } else {
            // Still unavailable — keep flag; value is cosmetic.
            self.data.set(BrightnessState {
                value,
                available: false,
            });
        }
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

/// Startup probe only (no poll loop).
async fn run_init(
    data: Mutable<BrightnessState>,
    status: Mutable<ServiceStatus>,
    displays: Mutable<Vec<u32>>,
) {
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
    if available {
        status.set(ServiceStatus::Available);
        info!(
            "BrightnessSubscriber connected: {value}% on {} displays",
            displays.get_cloned().len()
        );
    } else {
        status.set(ServiceStatus::Degraded(
            "no DDC displays — ddcutil/i2c unavailable".to_string(),
        ));
        info!("BrightnessSubscriber soft-fail: no DDC displays detected");
    }
}

/// Latest-wins DDC writer. One `write_all` at a time; further Sets overwrite
/// the pending target. **No getvcp after write** — concurrent re-reads were
/// the jump/storm bug. Trust the value we asked for; `Refresh` re-syncs.
async fn run_set_coalesce(
    mut rx: watch::Receiver<Option<u8>>,
    data: Mutable<BrightnessState>,
    displays: Mutable<Vec<u32>>,
) {
    loop {
        if rx.changed().await.is_err() {
            return;
        }

        let Some(mut target) = *rx.borrow_and_update() else {
            continue;
        };

        loop {
            let detected = displays.get_cloned();
            if detected.is_empty() {
                break;
            }
            let cloned = detected.clone();
            let write_target = target;
            let write_ok = tokio::time::timeout(
                DDC_TIMEOUT,
                tokio::task::spawn_blocking(move || write_all(&cloned, write_target)),
            )
            .await
            .ok()
            .and_then(|j| j.ok())
            .unwrap_or(false);

            if write_ok {
                // Confirm optimistic value; keep available=true even if a later
                // Refresh fails — do not poison the UI mid-session.
                let cur = data.get_cloned();
                data.set(BrightnessState {
                    value: write_target,
                    available: cur.available || true,
                });
            } else {
                warn!("brightness coalesce: write_all failed for {write_target}%");
                // Leave data as-is (still optimistic from dispatch).
            }

            // Drain newer targets that arrived during the write.
            if !rx.has_changed().unwrap_or(false) {
                break;
            }
            match *rx.borrow_and_update() {
                Some(next) => target = next,
                None => break,
            }
        }
    }
}

async fn refresh(
    data: &Mutable<BrightnessState>,
    displays: &Mutable<Vec<u32>>,
    status: &Mutable<ServiceStatus>,
) -> anyhow::Result<()> {
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
    if available {
        status.set(ServiceStatus::Available);
    }
    // If refresh fails availability, keep Degraded — don't force Available.
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn step_clamps_to_zero() {
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
        assert_eq!(STEP, 5);
    }
}
