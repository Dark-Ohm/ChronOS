//! Shared enter-motion language (T129).
//!
//! ## Two paths
//!
//! 1. **Layer-shell panels** — [`gpui::AnimationExt::with_animation`] works
//!    (live: panels slide in). Helpers: [`enter_animation`],
//!    [`apply_enter_from_left`] / [`apply_enter_from_right`].
//!
//! 2. **Anchored popups** — `with_animation` / `gpui_animation::transition_when`
//!    did not produce visible enter on live Hyprland (instant map). Use
//!    **view-driven** [`arm_enter_progress`] + style from [`enter_t`] each
//!    frame instead (notify loop).
//!
//! Exit fade on `remove_window` is compositor (Hyprland), not these helpers.

use std::time::{Duration, Instant};

use gpui::{Animation, Context, Styled, px};

/// Enter duration.
pub const ENTER_MS: u64 = 260;

/// Frame step for view-driven enter (~120 Hz).
const TICK_MS: u64 = 8;

/// Slide distance (px) at delta=0; zero at delta=1.
pub const SLIDE_PX: f32 = 14.;

/// EaseOutBack sample in 0..=1.
pub fn ease_enter(t: f32) -> f32 {
    gpui::easing::EasingCurve::EaseOutBack(1.5).sample(t.clamp(0.0, 1.0))
}

/// Oneshot enter for layer-shell panels (`with_animation`).
pub fn enter_animation() -> Animation {
    Animation::new(Duration::from_millis(ENTER_MS)).with_easing(ease_enter)
}

/// Opacity + slide from the right (right panel).
pub fn apply_enter_from_right<E: Styled>(el: E, delta: f32) -> E {
    let d = delta.clamp(0.0, 1.0);
    el.opacity(d).left(px(SLIDE_PX * (1.0 - d)))
}

/// Opacity + slide from the left (left panel).
pub fn apply_enter_from_left<E: Styled>(el: E, delta: f32) -> E {
    let d = delta.clamp(0.0, 1.0);
    el.opacity(d).left(px(-SLIDE_PX * (1.0 - d)))
}

/// Opacity + rise from style fields (popups, view-driven `enter_t`).
pub fn apply_enter_rise<E: Styled>(el: E, delta: f32) -> E {
    let d = delta.clamp(0.0, 1.0);
    el.opacity(d).top(px(SLIDE_PX * (1.0 - d)))
}

/// Drive `enter_t` 0→1 with easing via background ticks + `cx.notify`.
///
/// Call once from `View::new` (or first render). `set_t` writes the eased
/// progress into the view.
pub fn arm_enter_progress<V: 'static>(
    cx: &mut Context<V>,
    set_t: impl Fn(&mut V, f32) + Send + 'static,
) {
    cx.spawn(async move |this, cx| {
        let start = Instant::now();
        let dur = Duration::from_millis(ENTER_MS);
        loop {
            let raw = (start.elapsed().as_secs_f32() / dur.as_secs_f32()).min(1.0);
            let eased = ease_enter(raw);
            let ok = this.update(cx, |view, cx| {
                set_t(view, eased);
                cx.notify();
            });
            if ok.is_err() || raw >= 1.0 {
                break;
            }
            cx.background_executor()
                .timer(Duration::from_millis(TICK_MS))
                .await;
        }
    })
    .detach();
}

/// SpringBack for in-card `gpui_animation` toggles (volume device list).
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
    fn ease_enter_endpoints() {
        assert!((ease_enter(0.0) - 0.0).abs() < 1e-5);
        assert!((ease_enter(1.0) - 1.0).abs() < 1e-4);
    }
}
