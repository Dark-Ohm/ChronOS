//! System resources service — CPU/RAM via sysinfo + GPU via nvml-wrapper.
//!
//! Same async-poll shape as `UPowerSubscriber`/`CavaSubscriber`: `new()`
//! captures `Handle::current()` (guard against use outside a tokio runtime)
//! and spawns a poll loop that refreshes a `Mutable<SystemResourcesState>`
//! on an interval. NVML is initialized once outside the loop; any failure
//! degrades `gpu_percent` to `None` without killing the CPU/RAM path.

use std::time::Duration;

use futures_signals::signal::{Mutable, Signal};
use sysinfo::System;
use tokio::runtime::Handle;
use tracing::info;

use crate::Service;
use crate::ServiceStatus;
pub use types::SystemResourcesState;

pub mod types;

const POLL_INTERVAL: Duration = Duration::from_secs(1);

/// Pure sampling step — refreshes `sys` and returns the current CPU/RAM
/// percentages. Isolated from the async loop so it's testable without a
/// runtime. `sysinfo::System::refresh_cpu_usage` needs two calls ~200ms apart
/// for a meaningful global CPU percentage on first read; the poll loop
/// below relies on being called repeatedly on `POLL_INTERVAL`, so the very
/// first sample after construction may read 0.0 — this is sysinfo's own
/// documented behavior, not a bug to work around here.
fn sample_cpu_ram(sys: &mut System) -> SystemResourcesState {
    sys.refresh_cpu_usage();
    sys.refresh_memory();
    let cpu_percent = sys.global_cpu_usage();
    let ram_percent = if sys.total_memory() == 0 {
        0.0
    } else {
        (sys.used_memory() as f32 / sys.total_memory() as f32) * 100.0
    };
    SystemResourcesState {
        cpu_percent,
        ram_percent,
        gpu_percent: None,
    }
}

/// Reads GPU utilization percent from device 0 via NVML. `None` on any
/// failure (no Nvidia GPU, driver mismatch, NVML not installed) — the
/// caller treats `None` as "hide the GPU row", never as an error to
/// propagate or a reason to stop polling CPU/RAM.
fn sample_gpu(nvml: Option<&nvml_wrapper::Nvml>) -> Option<f32> {
    let nvml = nvml?;
    let device = nvml.device_by_index(0).ok()?;
    let util = device.utilization_rates().ok()?;
    Some(util.gpu as f32)
}

#[derive(Clone)]
pub struct SystemResourcesSubscriber {
    data: Mutable<SystemResourcesState>,
    status: Mutable<ServiceStatus>,
}

impl SystemResourcesSubscriber {
    /// Non-failing, synchronous constructor.
    ///
    /// # Panics
    ///
    /// Panics if called outside a tokio runtime — same `Handle::current()`
    /// guard as every other subscriber in this crate.
    pub fn new() -> Self {
        let data = Mutable::new(SystemResourcesState::default());
        let status = Mutable::new(ServiceStatus::Initializing);
        let _handle = Handle::current();
        tokio::spawn(run(data.clone(), status.clone()));
        Self { data, status }
    }
}

impl Service for SystemResourcesSubscriber {
    type Data = SystemResourcesState;
    type Error = anyhow::Error;

    fn subscribe(&self) -> impl Signal<Item = SystemResourcesState> + Unpin + 'static {
        self.data.signal_cloned()
    }

    fn get(&self) -> SystemResourcesState {
        self.data.get_cloned()
    }

    fn status(&self) -> ServiceStatus {
        self.status.get_cloned()
    }
}

async fn run(data: Mutable<SystemResourcesState>, status: Mutable<ServiceStatus>) {
    let mut sys = System::new_all();
    // NVML init once outside the loop — never re-init on every tick.
    let nvml = nvml_wrapper::Nvml::init().ok();
    if nvml.is_none() {
        info!("system_resources: NVML unavailable, GPU row will stay hidden");
    }
    status.set(ServiceStatus::Available);
    loop {
        let mut sample = sample_cpu_ram(&mut sys);
        sample.gpu_percent = sample_gpu(nvml.as_ref());
        data.set(sample);
        tokio::time::sleep(POLL_INTERVAL).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sample_cpu_ram_reads_real_host_values_in_range() {
        // Not a fixture test — sysinfo has no fixture path, this reads the
        // real host. Assert only the invariant that must always hold:
        // percentages are within [0, 100]. Live smoke (task 12) is where
        // "the number actually moves under load" gets verified.
        let mut sys = sysinfo::System::new_all();
        // Warm-up: first global_cpu_usage after construction is often 0.0.
        let _ = sample_cpu_ram(&mut sys);
        std::thread::sleep(Duration::from_millis(200));
        let sample = sample_cpu_ram(&mut sys);
        assert!(
            sample.cpu_percent >= 0.0 && sample.cpu_percent <= 100.0,
            "cpu_percent out of range: {}",
            sample.cpu_percent
        );
        assert!(
            sample.ram_percent >= 0.0 && sample.ram_percent <= 100.0,
            "ram_percent out of range: {}",
            sample.ram_percent
        );
        assert!(sample.gpu_percent.is_none());
    }

    #[test]
    fn gpu_sample_none_when_nvml_unavailable_does_not_panic() {
        // On a machine without an Nvidia GPU / driver, Nvml::init() returns
        // Err — this must degrade to None, never panic or bubble an error
        // that kills the poll loop.
        let nvml = nvml_wrapper::Nvml::init();
        let sample = sample_gpu(nvml.as_ref().ok());
        // On THIS dev machine (RTX 3070, confirmed live) nvml initializes,
        // so sample is Some(_) here — the invariant under test is just
        // "never panics and stays within range" for either branch.
        if let Some(pct) = sample {
            assert!(
                pct >= 0.0 && pct <= 100.0,
                "gpu_percent out of range: {pct}"
            );
        }
    }
}
