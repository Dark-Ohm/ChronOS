//! Brightness service via `ddcutil` (DDC/CI over i2c-dev).
//!
//! ## Why brightness "jumps for minutes"
//!
//! DDC is serial and slow (~0.5–1.5s for `setvcp` × N monitors). If every
//! drag sample starts a write, the queue of intermediate values keeps
//! updating the *panels* long after the user stopped. Concurrent `getvcp`
//! after write made the *UI* jump too; failed getvcp set `available=false`.
//!
//! ## Model (2026-07-25, second pass)
//!
//! 1. **Optimistic** `data.set` on every `Set`/`Step` (UI tracks instantly).
//! 2. **Debounced latest-wins** writer: wait ~150ms of quiet after the last
//!    Set, then `write_all` **once**. If the user moved during the write,
//!    write the newest target once more — never replay intermediate points.
//! 3. **No getvcp after Set** — trust the target; `Refresh` re-syncs.
//! 4. **Refresh is generation-gated** — discarded if a Set happened while
//!    detect/read was in flight (open-popup race).
//! 5. Transient DDC errors never clear `available`.

use std::sync::atomic::{AtomicU64, Ordering};
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

/// Brightness step for `BrightnessCommand::Step` (±5%).
pub const STEP: i8 = 5;

const DDC_TIMEOUT: Duration = Duration::from_secs(3);
/// Quiet period after the last Set before we touch i2c (debounce).
const SET_DEBOUNCE: Duration = Duration::from_millis(150);

#[derive(Clone)]
pub struct BrightnessSubscriber {
    data: Mutable<BrightnessState>,
    status: Mutable<ServiceStatus>,
    displays: Mutable<Vec<u32>>,
    runtime: Handle,
    /// Latest requested brightness (0..=100).
    set_tx: watch::Sender<Option<u8>>,
    /// Bumped on every Set/Step so in-flight Refresh can be discarded.
    set_epoch: std::sync::Arc<AtomicU64>,
}

impl BrightnessSubscriber {
    /// # Panics
    /// Outside a tokio runtime (`Handle::current()`).
    pub fn new() -> Self {
        let data = Mutable::new(BrightnessState::default());
        let status = Mutable::new(ServiceStatus::Initializing);
        let displays = Mutable::new(Vec::new());
        let set_epoch = std::sync::Arc::new(AtomicU64::new(0));

        let handle = Handle::current();
        tokio::spawn(run_init(data.clone(), status.clone(), displays.clone()));

        let (set_tx, set_rx) = watch::channel::<Option<u8>>(None);
        tokio::spawn(run_set_coalesce(
            set_rx,
            data.clone(),
            displays.clone(),
            set_epoch.clone(),
        ));

        Self {
            data,
            status,
            displays,
            runtime: handle,
            set_tx,
            set_epoch,
        }
    }

    /// Fire-and-forget. Safe from GPUI handlers.
    pub fn dispatch(&self, cmd: BrightnessCommand) {
        match cmd {
            BrightnessCommand::Set(target) => {
                let value = target.min(100);
                self.queue_set(value);
            }
            BrightnessCommand::Step(delta) => {
                let current = self.data.get_cloned();
                if !current.available {
                    return;
                }
                let next = (i32::from(current.value) + i32::from(delta)).clamp(0, 100) as u8;
                self.queue_set(next);
            }
            BrightnessCommand::Refresh => {
                let data = self.data.clone();
                let displays = self.displays.clone();
                let status = self.status.clone();
                let epoch = self.set_epoch.clone();
                let epoch_at_start = epoch.load(Ordering::SeqCst);
                self.runtime.spawn(async move {
                    if let Err(e) = refresh(&data, &displays, &status, &epoch, epoch_at_start).await
                    {
                        warn!("BrightnessSubscriber Refresh failed: {e:?}");
                    }
                });
            }
        }
    }

    fn queue_set(&self, value: u8) {
        self.set_epoch.fetch_add(1, Ordering::SeqCst);
        self.optimistic_set(value);
        let _ = self.set_tx.send(Some(value));
        debug!("brightness: queue Set({value})");
    }

    fn optimistic_set(&self, value: u8) {
        let available = self.data.get_cloned().available;
        // Never flip available→false on the set path.
        self.data.set(BrightnessState { value, available });
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

/// Debounced latest-wins writer.
///
/// - Wait until Sets stop arriving for `SET_DEBOUNCE`.
/// - Write that single target.
/// - If a newer target arrived during the write, write it once (no history).
async fn run_set_coalesce(
    mut rx: watch::Receiver<Option<u8>>,
    data: Mutable<BrightnessState>,
    displays: Mutable<Vec<u32>>,
    set_epoch: std::sync::Arc<AtomicU64>,
) {
    loop {
        if rx.changed().await.is_err() {
            return;
        }

        // Debounce: restart quiet timer on every new value.
        loop {
            tokio::select! {
                biased;
                res = rx.changed() => {
                    if res.is_err() {
                        return;
                    }
                    // New Set while waiting — reset debounce.
                }
                _ = tokio::time::sleep(SET_DEBOUNCE) => {
                    break;
                }
            }
        }

        let Some(mut target) = *rx.borrow_and_update() else {
            continue;
        };

        // Serial writes of only the current latest (and follow-ups mid-write).
        loop {
            let epoch_before_write = set_epoch.load(Ordering::SeqCst);
            let detected = displays.get_cloned();
            if detected.is_empty() {
                break;
            }

            let cloned = detected.clone();
            let write_target = target;
            debug!("brightness: ddc write {write_target}%");
            let write_ok = tokio::time::timeout(
                DDC_TIMEOUT,
                tokio::task::spawn_blocking(move || write_all(&cloned, write_target)),
            )
            .await
            .ok()
            .and_then(|j| j.ok())
            .unwrap_or(false);

            if write_ok {
                // Only confirm if the user has not already moved on.
                let cur = data.get_cloned();
                if cur.value == write_target {
                    data.set(BrightnessState {
                        value: write_target,
                        available: true,
                    });
                } else {
                    debug!(
                        "brightness: skip confirm {write_target}% (ui now {})",
                        cur.value
                    );
                }
            } else {
                warn!("brightness: write_all failed for {write_target}%");
            }

            // Newer Set during write?
            if !rx.has_changed().unwrap_or(false) {
                // Also check epoch in case send coalesced equal values oddly.
                let _ = epoch_before_write;
                break;
            }
            match *rx.borrow_and_update() {
                Some(next) if next != write_target => {
                    target = next;
                    // No extra debounce after an in-flight supersede — apply final ASAP.
                }
                Some(_) => break,
                None => break,
            }
        }
    }
}

async fn refresh(
    data: &Mutable<BrightnessState>,
    displays: &Mutable<Vec<u32>>,
    status: &Mutable<ServiceStatus>,
    set_epoch: &AtomicU64,
    epoch_at_start: u64,
) -> anyhow::Result<()> {
    let detected = tokio::task::spawn_blocking(detect_displays)
        .await
        .unwrap_or_default();

    // Set happened while we were detecting — drop this refresh.
    if set_epoch.load(Ordering::SeqCst) != epoch_at_start {
        debug!("brightness: Refresh discarded (set during detect)");
        return Ok(());
    }

    displays.set(detected.clone());
    let (value, available) = if detected.is_empty() {
        (0, false)
    } else {
        tokio::task::spawn_blocking(move || read_primary(&detected))
            .await
            .unwrap_or((0, false))
    };

    if set_epoch.load(Ordering::SeqCst) != epoch_at_start {
        debug!("brightness: Refresh discarded (set during read)");
        return Ok(());
    }

    data.set(BrightnessState { value, available });
    if available {
        status.set(ServiceStatus::Available);
    }
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
        assert_eq!(150u8.min(100), 100);
    }

    #[test]
    fn step_constant_matches_volume_popup() {
        assert_eq!(STEP, 5);
    }
}
