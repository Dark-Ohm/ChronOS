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

/// Delay after first paint before flipping `revealed`.
///
/// `transition_when` only starts if the element id already has a state in
/// the registry (created on first `AnimatedWrapper` paint). If `revealed`
/// is true on the *first* paint, `state_mut` is None and the animation is
/// silently skipped — panel pops in at the open pose. Schedule reveal only
/// after the closed pose has been painted once.
pub const REVEAL_DELAY_MS: u64 = 48;

/// Horizontal slide distance at rest pose (toward panel edge / screen).
pub const SLIDE_PX: f32 = 12.;

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

/// Schedule `revealed = true` after [`reveal_delay`], from the **first
/// paint** (not from `new`). See [`REVEAL_DELAY_MS`].
///
/// Call once when `!reveal_armed`, then set `reveal_armed = true`.
pub fn arm_reveal<V: 'static>(
    cx: &mut gpui::Context<V>,
    set_revealed: impl Fn(&mut V) + Send + 'static,
) {
    cx.spawn(async move |this, cx| {
        cx.background_executor().timer(reveal_delay()).await;
        let _ = this.update(cx, |this, cx| {
            set_revealed(this);
            cx.notify();
        });
    })
    .detach();
}

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
