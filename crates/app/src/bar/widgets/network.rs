//! Network widget for the bar — activity dot + speedometer ↓/↑.
//!
//! Reads `/sys/class/net/<iface>/statistics/rx_bytes` and `tx_bytes` directly
//! from procfs (all interfaces except `lo`), computes speeds over 1-second
//! intervals, and shows a green/gray/red activity dot plus two-line
//! download/upload speeds. No subprocesses, no async I/O — pure fs reads on the
//! ticker.
//!
//! ## Render immunity
//!
//! GPUI calls `render()` multiple times per frame (measure/layout/paint) and the
//! bar repaints on every service signal. The widget uses a **time gate**:
//! the procfs snapshot is only updated once per `SAMPLE_INTERVAL` (1 s).
//! Between updates, the last computed speed pair is cached and returned
//! unchanged, so any number of render calls within a frame all produce the same
//! value — never zero from a sub-second delta.

use std::io;
use std::path::Path;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use gpui::{AnyElement, App, Hsla, Window, div, prelude::*, px};

use chronos_luau::bar::{BarSection, BarWidget, BarWidgetRegistry};
use chronos_services::{ConnectivityState, Service};
use chronos_ui::Theme;

use crate::state::AppState;

/// Minimum time between procfs snapshot updates.
const SAMPLE_INTERVAL: Duration = Duration::from_secs(1);

/// Speed below which the link is considered idle (< 1 KB/s).
const IDLE_THRESHOLD: u64 = 1024;

/// Snapshot of aggregate byte counters for speed computation.
struct Sample {
    rx: u64,
    tx: u64,
    time: Instant,
}

/// Mutable state inside the widget (interior mutability for `&self` render).
struct NetworkState {
    /// Base snapshot for delta (None on first tick).
    sample: Option<Sample>,
    /// Last computed download speed (bytes/sec), cached between updates.
    cached_dl: f64,
    /// Last computed upload speed (bytes/sec), cached between updates.
    cached_ul: f64,
}

impl Default for NetworkState {
    fn default() -> Self {
        Self {
            sample: None,
            cached_dl: 0.0,
            cached_ul: 0.0,
        }
    }
}

// ---------------------------------------------------------------------------
// Procfs helpers
// ---------------------------------------------------------------------------

/// Reads total rx/tx bytes from all non-loopback interfaces.
fn read_interface_bytes() -> io::Result<(u64, u64)> {
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

// ---------------------------------------------------------------------------
// Speed computation (time-gated, testable via parameter injection)
// ---------------------------------------------------------------------------

/// Result of a speed update.
struct SpeedSample {
    dl_speed: f64,
    ul_speed: f64,
}

/// Update network speed state from fresh byte counters.
///
/// Returns the current (download, upload) speeds in bytes/sec.
/// `min_interval` controls the time gate — if less time has elapsed since the
/// last sample, cached speeds are returned without updating the snapshot.
/// Pass `SAMPLE_INTERVAL` in production; inject shorter durations in tests to
/// verify behaviour without waiting.
fn update_speed(
    state: &mut NetworkState,
    rx: u64,
    tx: u64,
    now: Instant,
    min_interval: Duration,
) -> SpeedSample {
    match state.sample {
        Some(ref prev) => {
            let elapsed = now.duration_since(prev.time);
            if elapsed < min_interval {
                // Time gate: return cached speeds without touching the snapshot.
                return SpeedSample {
                    dl_speed: state.cached_dl,
                    ul_speed: state.cached_ul,
                };
            }
            let elapsed_secs = elapsed.as_secs_f64();
            let dl = (rx.saturating_sub(prev.rx)) as f64 / elapsed_secs;
            let ul = (tx.saturating_sub(prev.tx)) as f64 / elapsed_secs;
            state.sample = Some(Sample { rx, tx, time: now });
            state.cached_dl = dl;
            state.cached_ul = ul;
            SpeedSample { dl_speed: dl, ul_speed: ul }
        }
        None => {
            // First tick: store snapshot, return 0.
            state.sample = Some(Sample { rx, tx, time: now });
            state.cached_dl = 0.0;
            state.cached_ul = 0.0;
            SpeedSample {
                dl_speed: 0.0,
                ul_speed: 0.0,
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Format helpers (pure, testable)
// ---------------------------------------------------------------------------

/// Human-readable speed string with stable width (exactly 4 characters).
///
/// Examples: `"  0 "`, `"999 "`, `"1.0K"`, `" 34K"`, `"999K"`, `"1.0M"`, `" 12M"`
fn format_speed(bytes_per_sec: f64) -> String {
    let (value, suffix) = if bytes_per_sec >= 1_000_000.0 {
        (bytes_per_sec / 1_000_000.0, "M")
    } else if bytes_per_sec >= 1_000.0 {
        (bytes_per_sec / 1_000.0, "K")
    } else {
        (bytes_per_sec, " ")
    };

    if suffix == " " {
        // 0-999 bytes -> "  0 " .. "999 "
        format!("{:>3} ", value as u64)
    } else if value >= 100.0 {
        // 100-999 -> "999K", "999M"
        format!("{:>3}{}", value as u64, suffix)
    } else if value >= 10.0 {
        // 10-99.9 -> " 34K", " 12M"
        format!(" {:>2.0}{}", value, suffix)
    } else {
        // 0-9.99 -> "1.0K", "5.0M"
        format!("{:>3.1}{}", value, suffix)
    }
}

/// Determine the indicator dot colour based on traffic speed and connectivity.
fn indicator_color(
    dl_speed: f64,
    ul_speed: f64,
    connectivity: ConnectivityState,
    theme: &Theme,
) -> Hsla {
    if connectivity == ConnectivityState::None {
        return theme.status.error;
    }
    if dl_speed + ul_speed < IDLE_THRESHOLD as f64 {
        theme.text.disabled
    } else {
        theme.status.success
    }
}

/// Pure description of widget state, decoupled from GPUI.
#[derive(Debug, PartialEq)]
struct SpeedView {
    dl: String,
    ul: String,
    idle: bool,
    disconnected: bool,
}

fn compute_view(connectivity: ConnectivityState, dl_speed: f64, ul_speed: f64) -> SpeedView {
    let disconnected = connectivity == ConnectivityState::None;
    let idle = !disconnected && dl_speed + ul_speed < IDLE_THRESHOLD as f64;
    let (dl, ul) = if disconnected {
        (0.0, 0.0)
    } else {
        (dl_speed, ul_speed)
    };
    SpeedView {
        dl: format_speed(dl),
        ul: format_speed(ul),
        idle,
        disconnected,
    }
}

// ---------------------------------------------------------------------------
// Widget
// ---------------------------------------------------------------------------

pub struct NetworkWidget {
    /// Mutable state guarded by interior mutability for `&self` render.
    state: Mutex<NetworkState>,
}

impl NetworkWidget {
    pub fn new() -> Self {
        Self {
            state: Mutex::new(NetworkState::default()),
        }
    }
}

impl Default for NetworkWidget {
    fn default() -> Self {
        Self::new()
    }
}

impl BarWidget for NetworkWidget {
    fn name(&self) -> &str {
        "network"
    }

    fn section(&self) -> BarSection {
        BarSection::Right
    }

    fn render(&self, _window: &mut Window, cx: &App) -> AnyElement {
        let net = AppState::network(cx);
        let data = net.get();
        let theme = Theme::global(cx);

        // Read procfs counters and compute speeds (time-gated, cached).
        let (dl_speed, ul_speed) = match read_interface_bytes() {
            Ok((rx, tx)) => {
                // unwrap_or_else for poisoned mutex: in single-threaded GPUI
                // the lock is never truly poisoned, but panicking in render
                // would tear down the bar. Silently recover.
                let mut guard = self.state.lock().unwrap_or_else(|e| e.into_inner());
                let result = update_speed(&mut *guard, rx, tx, Instant::now(), SAMPLE_INTERVAL);
                (result.dl_speed, result.ul_speed)
            }
            Err(_) => {
                // Read error — return cached values.
                // No logging: transient procfs errors (e.g. race with interface
                // creation) self-heal on the next 1s tick. Spamming every
                // second would drown real warnings.
                let guard = self.state.lock().unwrap_or_else(|e| e.into_inner());
                (guard.cached_dl, guard.cached_ul)
            }
        };

        let view = compute_view(data.connectivity, dl_speed, ul_speed);
        let dot_color = indicator_color(dl_speed, ul_speed, data.connectivity, theme);
        let speed_color = if view.disconnected {
            theme.text.disabled
        } else {
            theme.text.secondary
        };

        div()
            .flex()
            .items_center()
            .gap(px(4.))
            .child(
                div()
                    .w(px(6.))
                    .h(px(6.))
                    .rounded_full()
                    .bg(dot_color)
                    .flex_none(),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .items_end()
                    .child(
                        div()
                            .child(format!("\u{2193} {}", view.dl))
                            .text_color(speed_color)
                            .text_size(theme.font_sizes.xs)
                            .font_family(theme.font_mono),
                    )
                    .child(
                        div()
                            .child(format!("\u{2191} {}", view.ul))
                            .text_color(speed_color)
                            .text_size(theme.font_sizes.xs)
                            .font_family(theme.font_mono),
                    ),
            )
            .into_any_element()
    }
}

/// Register the network widget with the global bar registry.
pub fn register(cx: &mut App) {
    cx.global_mut::<BarWidgetRegistry>()
        .register(Box::new(NetworkWidget::new()));
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- format_speed -------------------------------------------------------

    #[test]
    fn format_zero() {
        assert_eq!(format_speed(0.0), "  0 ");
    }

    #[test]
    fn format_bytes() {
        assert_eq!(format_speed(5.0), "  5 ");
        assert_eq!(format_speed(999.0), "999 ");
    }

    #[test]
    fn format_kilobytes() {
        assert_eq!(format_speed(1_000.0), "1.0K");
        assert_eq!(format_speed(34_000.0), " 34K");
        assert_eq!(format_speed(999_000.0), "999K");
    }

    #[test]
    fn format_megabytes() {
        assert_eq!(format_speed(1_000_000.0), "1.0M");
        assert_eq!(format_speed(12_000_000.0), " 12M");
        assert_eq!(format_speed(999_000_000.0), "999M");
    }

    #[test]
    fn format_stable_width() {
        for v in &[
            0.0, 5.0, 999.0, 1_000.0, 34_000.0, 999_000.0, 1_000_000.0, 99_000_000.0,
        ] {
            assert_eq!(format_speed(*v).len(), 4, "failed for {v}");
        }
    }

    // -- compute_view -------------------------------------------------------

    #[test]
    fn view_disconnected() {
        let view = compute_view(ConnectivityState::None, 0.0, 0.0);
        assert!(view.disconnected);
        assert!(!view.idle);
        assert_eq!(view.dl, "  0 ");
        assert_eq!(view.ul, "  0 ");
    }

    #[test]
    fn view_idle_with_connectivity() {
        let view = compute_view(ConnectivityState::Full, 0.0, 0.0);
        assert!(!view.disconnected);
        assert!(view.idle);
    }

    #[test]
    fn view_active_traffic() {
        let view = compute_view(ConnectivityState::Full, 50_000.0, 20_000.0);
        assert!(!view.disconnected);
        assert!(!view.idle);
        assert_eq!(view.dl, " 50K");
        assert_eq!(view.ul, " 20K");
    }

    #[test]
    fn view_below_threshold_idle() {
        let view = compute_view(ConnectivityState::Full, 500.0, 523.0);
        assert!(!view.disconnected);
        assert!(view.idle);
    }

    // -- indicator_color ----------------------------------------------------

    #[test]
    fn indicator_disconnected_is_error() {
        let theme = Theme::default();
        assert_eq!(
            indicator_color(0.0, 0.0, ConnectivityState::None, &theme),
            theme.status.error
        );
    }

    #[test]
    fn indicator_idle_is_disabled() {
        let theme = Theme::default();
        assert_eq!(
            indicator_color(0.0, 0.0, ConnectivityState::Full, &theme),
            theme.text.disabled
        );
    }

    #[test]
    fn indicator_active_is_success() {
        let theme = Theme::default();
        assert_eq!(
            indicator_color(50_000.0, 20_000.0, ConnectivityState::Full, &theme),
            theme.status.success
        );
    }

    #[test]
    fn indicator_below_threshold_is_disabled() {
        let theme = Theme::default();
        assert_eq!(
            indicator_color(500.0, 523.0, ConnectivityState::Full, &theme),
            theme.text.disabled
        );
    }

    #[test]
    fn indicator_at_threshold_is_success() {
        let theme = Theme::default();
        assert_eq!(
            indicator_color(1024.0, 0.0, ConnectivityState::Full, &theme),
            theme.status.success
        );
    }

    // -- update_speed (time-gated speed computation) -----------------------

    #[test]
    fn update_speed_first_call_returns_zero() {
        let mut state = NetworkState::default();
        let now = Instant::now();
        let result = update_speed(&mut state, 100_000, 50_000, now, SAMPLE_INTERVAL);
        assert_eq!(result.dl_speed, 0.0);
        assert_eq!(result.ul_speed, 0.0);
        // Snapshot should be stored for next tick.
        assert!(state.sample.is_some());
    }

    #[test]
    fn update_speed_immunity_to_frequency() {
        let mut state = NetworkState::default();
        let t0 = Instant::now();

        // First call (cold start).
        let r1 = update_speed(&mut state, 0, 0, t0, SAMPLE_INTERVAL);
        assert_eq!(r1.dl_speed, 0.0);

        // Simulate many render calls within the same "frame".
        for _ in 0..10 {
            let r = update_speed(&mut state, 0, 0, t0, SAMPLE_INTERVAL);
            assert_eq!(r.dl_speed, 0.0, "cached value collapsed between repeated calls");
            assert_eq!(r.ul_speed, 0.0, "cached value collapsed between repeated calls");
        }

        // Advance time past the gate and provide real counters.
        let t1 = t0 + SAMPLE_INTERVAL + Duration::from_millis(1);
        let r2 = update_speed(&mut state, 1_000_000, 500_000, t1, SAMPLE_INTERVAL);
        assert!(r2.dl_speed > 900_000.0 && r2.dl_speed < 1_100_000.0);
        assert!(r2.ul_speed > 450_000.0 && r2.ul_speed < 550_000.0);

        // Same time → cached value, not zero.
        let r3 = update_speed(&mut state, 1_000_000, 500_000, t1, SAMPLE_INTERVAL);
        assert_eq!(r3.dl_speed, r2.dl_speed, "cached value changed between gated calls");
        assert_eq!(r3.ul_speed, r2.ul_speed, "cached value changed between gated calls");

        // Different counters, same time → cache still wins.
        let r4 = update_speed(&mut state, 9_999_999, 8_888_888, t1, SAMPLE_INTERVAL);
        assert_eq!(r4.dl_speed, r2.dl_speed, "cached value changed despite different counters");
        assert_eq!(r4.ul_speed, r2.ul_speed, "cached value changed despite different counters");
    }

    #[test]
    fn update_speed_computes_correct_speed() {
        let mut state = NetworkState::default();
        let t0 = Instant::now();

        // Cold start.
        update_speed(&mut state, 0, 0, t0, SAMPLE_INTERVAL);

        // After exactly SAMPLE_INTERVAL: 50,000 bytes transferred.
        let t1 = t0 + SAMPLE_INTERVAL;
        let result = update_speed(&mut state, 50_000, 10_000, t1, SAMPLE_INTERVAL);
        assert_eq!(result.dl_speed, 50_000.0);
        assert_eq!(result.ul_speed, 10_000.0);
    }

    #[test]
    fn update_speed_handles_counter_wrap() {
        let mut state = NetworkState::default();
        let t0 = Instant::now();

        // First sample with high (near-wrapping) counters.
        update_speed(&mut state, u64::MAX, u64::MAX, t0, SAMPLE_INTERVAL);

        // After SAMPLE_INTERVAL: counters appear to have wrapped.
        let t1 = t0 + SAMPLE_INTERVAL;
        let result = update_speed(&mut state, 100, 200, t1, SAMPLE_INTERVAL);
        assert_eq!(result.dl_speed, 0.0, "dl should be 0 on wrap tick");
        assert_eq!(result.ul_speed, 0.0, "ul should be 0 on wrap tick");

        // Next tick recovers normally.
        let t2 = t1 + SAMPLE_INTERVAL;
        let result = update_speed(&mut state, 100_000, 200_000, t2, SAMPLE_INTERVAL);
        assert!((result.dl_speed - 99_900.0).abs() < 100.0);
        assert!((result.ul_speed - 199_800.0).abs() < 100.0);
    }

}
