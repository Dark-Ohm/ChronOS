//! Shared enter-motion language (T129).
//!
//! Scale is not available on `div` style in this fork — enter uses
//! **opacity + translate** via `gpui_animation` (`State::translate_x/y`,
//! requires `.relative()` on the animated element). Easing: EaseOutBack
//! wrapped as [`SpringBack`] (same recipe as volume_popup).

use std::time::Duration;

use gpui::{Pixels, px};
use gpui_animation::transition::Transition;

/// Default enter duration (ms) — matches volume device list (~260).
pub const ENTER_MS: u64 = 240;

/// Delay before flipping `revealed` so the first paint is the closed pose.
pub const REVEAL_DELAY_MS: u64 = 16;

/// Horizontal slide distance at rest pose (toward panel edge / screen).
pub const SLIDE_PX: f32 = 10.;

/// Spring-overshoot easing for declarative transitions.
#[derive(Clone, Copy)]
pub struct SpringBack(pub f32);

impl Default for SpringBack {
    fn default() -> Self {
        Self(1.5)
    }
}

impl Transition for SpringBack {
    fn calculate(&self, t: f32) -> f32 {
        gpui::easing::EasingCurve::EaseOutBack(self.0).sample(t)
    }
}

pub fn enter_duration() -> Duration {
    Duration::from_millis(ENTER_MS)
}

pub fn reveal_delay() -> Duration {
    Duration::from_millis(REVEAL_DELAY_MS)
}

/// Closed-pose horizontal offset: left panel slides from left (negative),
/// right panel from right (positive). Popups: slight up is better — use
/// [`enter_slide_y`].
pub fn enter_slide_x(from_left: bool) -> Pixels {
    if from_left {
        px(-SLIDE_PX)
    } else {
        px(SLIDE_PX)
    }
}

pub fn enter_slide_y() -> Pixels {
    px(SLIDE_PX)
}

/// Closed opacity for the base style (before / when not revealed).
pub fn closed_opacity() -> f32 {
    0.0
}

/// Open-pose modifier for `transition_when` — opacity 1 + zero translate.
/// Inline at call sites if the crate path for `State` is awkward; this is
/// the canonical shape:
/// ```ignore
/// .transition_when(revealed, enter_duration(), SpringBack::default(), |s| {
///     s.opacity(1.0).translate(px(0.), px(0.))
/// })
/// ```

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spring_back_endpoints() {
        let e = SpringBack(1.5);
        assert!((e.calculate(0.0) - 0.0).abs() < 1e-5);
        assert!((e.calculate(1.0) - 1.0).abs() < 1e-4);
    }

    #[test]
    fn slide_directions() {
        assert!(f32::from(enter_slide_x(true)) < 0.0);
        assert!(f32::from(enter_slide_x(false)) > 0.0);
    }
}
