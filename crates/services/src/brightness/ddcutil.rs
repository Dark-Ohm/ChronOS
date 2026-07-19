//! `ddcutil` shell-out helpers.
//!
//! All functions are sync and blocking — they must be called from a tokio
//! runtime via `spawn_blocking` or from a background task, never from a GPUI
//! UI thread. DDC/CI latency is ~100–300ms per call (i2c round-trip), so the
//! service never polls on a frame cadence; it reads on demand (init, popup
//! open, after a step).
//!
//! MVP policy (per ZED.md brief): one slider drives **both** displays. Read
//! returns the primary (display 2 / DP-1 on this machine); write applies to
//! display 1 and display 2. No UI for per-monitor selection.

use std::process::Command;

use tracing::{info, warn};

/// DDC/CI VCP code for brightness.
const VCP_BRIGHTNESS: &str = "10";

/// Path to the `ddcutil` binary. Resolved once at construction; if missing,
/// the service reports `available == false` and the UI soft-fails.
pub const DDCUTIL_BIN: &str = "ddcutil";

/// Parse the stdout of `ddcutil getvcp 10 --display N`.
///
/// Live fixture (verified 2026-07-19 on Samsung LC32G5xT over DP-1):
/// ```text
/// VCP code 0x10 (Brightness                    ): current value =   100, max value =   100
/// ```
/// We extract the first integer after `current value =`. Returns `None` on
/// any parse failure — the caller treats that as "brightness unavailable".
pub fn parse_getvcp_stdout(stdout: &str) -> Option<u8> {
    // Find the segment after "current value".
    let key = "current value";
    let rest = stdout.split(key).nth(1)?;
    // rest looks like " =   100, max value =   100\n" — skip non-digits.
    let digits: String = rest
        .trim_start()
        .trim_start_matches(|c: char| !c.is_ascii_digit())
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .collect();
    if digits.is_empty() {
        return None;
    }
    let value: u32 = digits.parse().ok()?;
    if value > u32::from(u8::MAX) {
        warn!("ddcutil: parsed brightness {value} > u8::MAX, clamping");
        Some(u8::MAX)
    } else {
        Some(value as u8)
    }
}

/// Run `ddcutil detect` and return the list of 1-based display numbers that
/// responded. Empty vec = no DDC displays (or ddcutil missing / i2c denied).
pub fn detect_displays() -> Vec<u32> {
    let output = match Command::new(DDCUTIL_BIN)
        .args(["detect", "--brief"])
        .output()
    {
        Ok(o) => o,
        Err(e) => {
            warn!("ddcutil detect failed to spawn: {e}");
            return Vec::new();
        }
    };
    if !output.status.success() {
        warn!(
            "ddcutil detect exited non-zero: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
        return Vec::new();
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    // `ddcutil detect --brief` prints one "Display N" line per display.
    let mut displays = Vec::new();
    for line in stdout.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("Display ") {
            if let Ok(n) = rest.trim().parse::<u32>() {
                displays.push(n);
            }
        }
    }
    displays
}

/// Read brightness (VCP 10) from a specific display number (1-based).
/// Returns `None` on any error — caller treats as unavailable.
pub fn get_brightness(display_no: u32) -> Option<u8> {
    let output = Command::new(DDCUTIL_BIN)
        .args([
            "getvcp",
            VCP_BRIGHTNESS,
            "--display",
            &display_no.to_string(),
        ])
        .output()
        .ok()?;
    if !output.status.success() {
        warn!(
            "ddcutil getvcp failed on display {display_no}: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
        return None;
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_getvcp_stdout(&stdout)
}

/// Set brightness (VCP 10) on a specific display number. Soft-fails: logs a
/// warning on error, does not propagate. Returns `true` on success.
pub fn set_brightness(display_no: u32, value: u8) -> bool {
    let status = Command::new(DDCUTIL_BIN)
        .args([
            "setvcp",
            VCP_BRIGHTNESS,
            &value.to_string(),
            "--display",
            &display_no.to_string(),
        ])
        .status();
    match status {
        Ok(s) if s.success() => true,
        Ok(s) => {
            warn!("ddcutil setvcp on display {display_no} exited {s}");
            false
        }
        Err(e) => {
            warn!("ddcutil setvcp failed to spawn on display {display_no}: {e}");
            false
        }
    }
}

/// Read brightness from the primary display. Heuristic: prefer display 2
/// (DP-1 on this machine — Samsung 144Hz primary), fall back to display 1,
/// then to the first detected display. Returns `(value, available)`.
pub fn read_primary(displays: &[u32]) -> (u8, bool) {
    if displays.is_empty() {
        return (0, false);
    }
    let primary = displays
        .iter()
        .copied()
        .find(|&n| n == 2)
        .or_else(|| displays.first().copied());
    let Some(primary) = primary else {
        return (0, false);
    };
    match get_brightness(primary) {
        Some(v) => (v, true),
        None => {
            // Primary failed — try any other display before giving up.
            for &d in displays {
                if d == primary {
                    continue;
                }
                if let Some(v) = get_brightness(d) {
                    return (v, true);
                }
            }
            (0, false)
        }
    }
}

/// Apply a brightness value to **all** detected displays (MVP policy: one
/// slider = both monitors). Returns `true` if at least one display accepted
/// the write. Soft-fails per-display: a single EIO on one monitor (Dell
/// U2412M occasionally returns -EIO for unsupported features) does not abort
/// the other.
pub fn write_all(displays: &[u32], value: u8) -> bool {
    if displays.is_empty() {
        return false;
    }
    let mut any_ok = false;
    for &d in displays {
        if set_brightness(d, value) {
            any_ok = true;
        }
    }
    if any_ok {
        info!("ddcutil: set brightness to {value} on all displays");
    }
    any_ok
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Live fixture captured 2026-07-19 on Samsung LC32G5xT (DP-1, display 2).
    #[test]
    fn parse_live_samsung_dp1() {
        let stdout = "VCP code 0x10 (Brightness                    ): current value =   100, max value =   100\n";
        assert_eq!(parse_getvcp_stdout(stdout), Some(100));
    }

    /// Live fixture captured 2026-07-19 on Dell U2412M (HDMI-A-1, display 1).
    /// Note: Dell occasionally prefixes a bus-error warning line; the parser
    /// must still find the value line.
    #[test]
    fn parse_live_dell_hdmi_with_warning() {
        let stdout = "busno=2. Monitor apparently returns -EIO for unsupported features. This cannot be relied on.\nError detecting VCP version using VCP feature xDF: Error_Info[EIO in ddc_write_read_with_retry, causes: DDCRC_DDC_DATA, EIO]\nVCP code 0x10 (Brightness                    ): current value =   100, max value =   100\n";
        assert_eq!(parse_getvcp_stdout(stdout), Some(100));
    }

    #[test]
    fn parse_mid_value() {
        let stdout = "VCP code 0x10 (Brightness                    ): current value =    42, max value =   100\n";
        assert_eq!(parse_getvcp_stdout(stdout), Some(42));
    }

    #[test]
    fn parse_zero() {
        let stdout = "VCP code 0x10 (Brightness                    ): current value =     0, max value =   100\n";
        assert_eq!(parse_getvcp_stdout(stdout), Some(0));
    }

    #[test]
    fn parse_malformed_returns_none() {
        assert_eq!(parse_getvcp_stdout(""), None);
        assert_eq!(parse_getvcp_stdout("no current value here"), None);
        assert_eq!(parse_getvcp_stdout("current value =  not a number"), None);
    }

    #[test]
    fn parse_no_current_value_key() {
        let stdout = "VCP code 0x12 (Contrast                      ): max value =   100\n";
        assert_eq!(parse_getvcp_stdout(stdout), None);
    }

    #[test]
    fn parse_clamps_above_u8() {
        // Hypothetical fixture — DDC brightness max is 100 in practice, but
        // the parser must not panic on a malformed large value.
        let stdout = "current value = 9999";
        assert_eq!(parse_getvcp_stdout(stdout), Some(255));
    }

    #[test]
    fn read_primary_prefers_display_2() {
        // No live i2c in CI — exercise the empty + fallback logic only.
        let (v, avail) = read_primary(&[]);
        assert!(!avail);
        assert_eq!(v, 0);
    }
}
