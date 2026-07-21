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
//!
//! Sampling logic lives in `chronos_services::net_stats` (shared with the right
//! side panel).

use std::sync::Mutex;
use std::time::Instant;

use gpui::{AnyElement, App, Hsla, Window, div, prelude::*, px};

use chronos_luau::bar::{BarSection, BarWidget, BarWidgetRegistry};
use chronos_services::net_stats::{
    NetState as NetworkState, SAMPLE_INTERVAL, read_interface_bytes, update_speed,
};
use chronos_services::{ConnectivityState, Service};
use chronos_ui::Theme;

use crate::state::AppState;

/// Speed below which the link is considered idle (< 1 KB/s).
const IDLE_THRESHOLD: u64 = 1024;

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
                (result.dl, result.ul)
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
}
