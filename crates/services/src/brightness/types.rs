//! Brightness data types.
//!
//! Brightness is delivered via `ddcutil` (DDC/CI over i2c-dev) — there is no
//! D-Bus bus for this on a generic desktop. The service shells out to
//! `ddcutil getvcp 10` / `ddcutil setvcp 10 <N>` and parses the stdout.

/// State snapshot for the brightness service.
///
/// `available == false` means `ddcutil` is missing, `/dev/i2c-*` is
/// inaccessible, or `ddcutil detect` returned no displays — the UI must
/// render a muted/disabled brightness block in that case (soft-fail, same
/// philosophy as cava without the binary).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BrightnessState {
    /// 0..=100 — current brightness of the primary display.
    /// When `available == false`, this is 0 and must not be shown as a real
    /// value.
    pub value: u8,
    /// Whether a usable DDC display was detected.
    pub available: bool,
}

/// Commands dispatched to the brightness service (fire-and-forget).
#[derive(Clone, Copy, Debug)]
pub enum BrightnessCommand {
    /// Set brightness to an absolute value (0..=100). Clamped internally.
    Set(u8),
    /// Step brightness by a signed delta (e.g. +5 / -5). Clamped internally.
    Step(i8),
    /// Re-read the current brightness from the display. Dispatched on popup
    /// open so the slider reflects the live monitor state.
    Refresh,
}
