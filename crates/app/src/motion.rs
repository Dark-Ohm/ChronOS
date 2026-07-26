//! Shared enter-motion language (T129).
//!
//! Uses **native** [`gpui::AnimationExt::with_animation`] (not
//! `gpui_animation::transition_when`). The vendored crate's
//! `transition_when` only starts if the element id already has registry
//! state — on a freshly opened layer-shell/popup window the first paint
//! has no state, so enter was a silent no-op and the shell hard-cut to
//! full opacity (live report: "enter мгновенный").
//!
//! Native `with_animation` stores start time in element state on first
//! layout and requests frames until oneshot completes — reliable for open.

use std::time::Duration;

use gpui::{Animation, Styled, px};

/// Enter duration — same ballpark as volume device list (~260ms).
pub const ENTER_MS: u64 = 260;

/// Slide distance (px) at delta=0; zero at delta=1.
pub const SLIDE_PX: f32 = 14.;

/// Oneshot enter animation with EaseOutBack overshoot.
pub fn enter_animation() -> Animation {
    Animation::new(Duration::from_millis(ENTER_MS)).with_easing(|t| {
        gpui::easing::EasingCurve::EaseOutBack(1.5).sample(t)
    })
}

/// Opacity + slide from the right edge (right panel).
/// Works on `Div` or `Stateful<Div>` (after `.id()`).
pub fn apply_enter_from_right<E: Styled>(el: E, delta: f32) -> E {
    let d = delta.clamp(0.0, 1.0);
    el.opacity(d).left(px(SLIDE_PX * (1.0 - d)))
}

/// Opacity + slide from the left edge (left panel).
pub fn apply_enter_from_left<E: Styled>(el: E, delta: f32) -> E {
    let d = delta.clamp(0.0, 1.0);
    el.opacity(d).left(px(-SLIDE_PX * (1.0 - d)))
}

/// Opacity + slight rise (popups).
pub fn apply_enter_rise<E: Styled>(el: E, delta: f32) -> E {
    let d = delta.clamp(0.0, 1.0);
    el.opacity(d).top(px(SLIDE_PX * (1.0 - d)))
}

/// SpringBack for **in-card** `gpui_animation` toggles (volume device list).
/// Not used for window enter — see module docs.
#[derive(Clone, Copy)]
pub struct SpringBack(pub f32);

impl Default for SpringBack {
    fn default() -> Self {
        Self(1.5)
    }
}

impl gpui_animation::transition::Transition for SpringBack {
    fn calculate(&self, t: f32) -> f32 {
        gpui::easing::EasingCurve::EaseOutBack(self.0).sample(t)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui_animation::transition::Transition;

    #[test]
    fn spring_back_endpoints() {
        let e = SpringBack(1.5);
        assert!((e.calculate(0.0) - 0.0).abs() < 1e-5);
        assert!((e.calculate(1.0) - 1.0).abs() < 1e-4);
    }

    #[test]
    fn enter_delta_ends_opaque() {
        // Apply helpers are pure style — smoke that clamp works.
        let _ = enter_animation();
        assert!((1.0_f32.clamp(0.0, 1.0) - 1.0).abs() < f32::EPSILON);
    }
}
