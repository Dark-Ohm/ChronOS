//! Shared, render-frequency-immune network byte-rate sampling.
//!
//! Extracted from `crates/app/src/bar/widgets/network.rs` so the bar widget
//! and the right side panel's network spectrum meter read from one
//! implementation instead of two copies of procfs parsing drifting apart.
//!
//! ## Render immunity
//!
//! Callers (bar widget, side panel) may invoke `update_speed` many times
//! per frame. The time gate below only updates the procfs snapshot once
//! per `min_interval`; between updates the cached speed pair is returned
//! unchanged.

use std::io;
use std::path::Path;
use std::time::{Duration, Instant};

/// Minimum time between procfs snapshot updates.
pub const SAMPLE_INTERVAL: Duration = Duration::from_secs(1);

/// Snapshot of aggregate byte counters for speed computation.
pub struct NetSample {
    pub rx: u64,
    pub tx: u64,
    pub time: Instant,
}

/// Mutable state for one sampler instance (bar and panel each hold their own).
pub struct NetState {
    pub sample: Option<NetSample>,
    pub cached_dl: f64,
    pub cached_ul: f64,
}

impl Default for NetState {
    fn default() -> Self {
        Self {
            sample: None,
            cached_dl: 0.0,
            cached_ul: 0.0,
        }
    }
}

/// Result of a speed update (bytes/sec).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NetSpeed {
    pub dl: f64,
    pub ul: f64,
}

/// Reads total rx/tx bytes from all non-loopback interfaces.
pub fn read_interface_bytes() -> io::Result<(u64, u64)> {
    let net_dir = Path::new("/sys/class/net");
    let mut total_rx = 0u64;
    let mut total_tx = 0u64;

    for entry in std::fs::read_dir(net_dir)? {
        let entry = entry?;
        let ifname = entry.file_name();
        let ifname = ifname.to_string_lossy();

        if ifname == "lo" {
            continue;
        }

        if let Ok(rx_str) =
            std::fs::read_to_string(entry.path().join("statistics").join("rx_bytes"))
        {
            if let Ok(rx) = rx_str.trim().parse::<u64>() {
                total_rx += rx;
            }
        }
        if let Ok(tx_str) =
            std::fs::read_to_string(entry.path().join("statistics").join("tx_bytes"))
        {
            if let Ok(tx) = tx_str.trim().parse::<u64>() {
                total_tx += tx;
            }
        }
    }

    Ok((total_rx, total_tx))
}

/// Update network speed state from fresh byte counters.
///
/// `min_interval` controls the time gate — if less time has elapsed since
/// the last sample, cached speeds are returned without touching the
/// snapshot. Pass `SAMPLE_INTERVAL` in production; inject shorter
/// durations in tests.
pub fn update_speed(
    state: &mut NetState,
    rx: u64,
    tx: u64,
    now: Instant,
    min_interval: Duration,
) -> NetSpeed {
    match state.sample {
        Some(ref prev) => {
            let elapsed = now.duration_since(prev.time);
            if elapsed < min_interval {
                return NetSpeed {
                    dl: state.cached_dl,
                    ul: state.cached_ul,
                };
            }
            let elapsed_secs = elapsed.as_secs_f64();
            let dl = (rx.saturating_sub(prev.rx)) as f64 / elapsed_secs;
            let ul = (tx.saturating_sub(prev.tx)) as f64 / elapsed_secs;
            state.sample = Some(NetSample { rx, tx, time: now });
            state.cached_dl = dl;
            state.cached_ul = ul;
            NetSpeed { dl, ul }
        }
        None => {
            state.sample = Some(NetSample { rx, tx, time: now });
            state.cached_dl = 0.0;
            state.cached_ul = 0.0;
            NetSpeed { dl: 0.0, ul: 0.0 }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_sample_returns_zero_and_stores_snapshot() {
        let mut state = NetState::default();
        let now = Instant::now();
        let speed = update_speed(&mut state, 1000, 500, now, SAMPLE_INTERVAL);
        assert_eq!(speed, NetSpeed { dl: 0.0, ul: 0.0 });
        assert!(state.sample.is_some());
    }

    #[test]
    fn second_sample_after_interval_computes_real_delta() {
        let mut state = NetState::default();
        let t0 = Instant::now();
        update_speed(&mut state, 1000, 500, t0, Duration::from_millis(10));
        let t1 = t0 + Duration::from_secs(1);
        let speed = update_speed(&mut state, 2000, 1500, t1, Duration::from_millis(10));
        assert_eq!(speed, NetSpeed { dl: 1000.0, ul: 1000.0 });
    }

    #[test]
    fn sample_within_min_interval_returns_cached_value_not_fresh_delta() {
        // Regression guard for the exact bug fixed 2026-07-20 in
        // bar/widgets/network.rs: a render() call microseconds after the
        // last sample must not compute a near-zero delta.
        let mut state = NetState::default();
        let t0 = Instant::now();
        update_speed(&mut state, 1000, 500, t0, Duration::from_secs(1));
        let t1 = t0 + Duration::from_millis(5);
        let speed = update_speed(&mut state, 1001, 501, t1, Duration::from_secs(1));
        assert_eq!(speed, NetSpeed { dl: 0.0, ul: 0.0 });
    }

    // Ported verbatim in intent from the pre-extraction tests in
    // `crates/app/src/bar/widgets/network.rs` (renamed for the new field
    // names). Do NOT drop these when deleting the originals in Step 5 —
    // they cover the two cases the three tests above do not: repeated
    // same-frame calls, and u64 counter wraparound.

    #[test]
    fn repeated_calls_in_one_frame_never_collapse_the_cached_value() {
        let mut state = NetState::default();
        let t0 = Instant::now();

        let r1 = update_speed(&mut state, 0, 0, t0, SAMPLE_INTERVAL);
        assert_eq!(r1, NetSpeed { dl: 0.0, ul: 0.0 });

        // Simulate many render() calls inside the same frame.
        for _ in 0..10 {
            let r = update_speed(&mut state, 0, 0, t0, SAMPLE_INTERVAL);
            assert_eq!(r.dl, 0.0, "cached value collapsed between repeated calls");
            assert_eq!(r.ul, 0.0, "cached value collapsed between repeated calls");
        }

        // Advance past the gate with real counters.
        let t1 = t0 + SAMPLE_INTERVAL + Duration::from_millis(1);
        let r2 = update_speed(&mut state, 1_000_000, 500_000, t1, SAMPLE_INTERVAL);
        assert!(r2.dl > 900_000.0 && r2.dl < 1_100_000.0);
        assert!(r2.ul > 450_000.0 && r2.ul < 550_000.0);

        // Same instant → cache wins, even with wildly different counters.
        let r3 = update_speed(&mut state, 9_999_999, 8_888_888, t1, SAMPLE_INTERVAL);
        assert_eq!(r3.dl, r2.dl, "cached value changed despite different counters");
        assert_eq!(r3.ul, r2.ul, "cached value changed despite different counters");
    }

    #[test]
    fn counter_wraparound_yields_zero_then_recovers() {
        let mut state = NetState::default();
        let t0 = Instant::now();

        // First sample at near-wrap counters.
        update_speed(&mut state, u64::MAX, u64::MAX, t0, SAMPLE_INTERVAL);

        // Counters wrapped: `saturating_sub` must floor at 0, not underflow.
        let t1 = t0 + SAMPLE_INTERVAL;
        let wrapped = update_speed(&mut state, 100, 200, t1, SAMPLE_INTERVAL);
        assert_eq!(wrapped.dl, 0.0, "dl must be 0 on the wrap tick");
        assert_eq!(wrapped.ul, 0.0, "ul must be 0 on the wrap tick");

        // Next tick recovers normally from the new baseline.
        let t2 = t1 + SAMPLE_INTERVAL;
        let recovered = update_speed(&mut state, 100_000, 200_000, t2, SAMPLE_INTERVAL);
        assert!((recovered.dl - 99_900.0).abs() < 100.0);
        assert!((recovered.ul - 199_800.0).abs() < 100.0);
    }
}
