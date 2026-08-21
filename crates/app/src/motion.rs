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

/// Context-menu enter duration — reference `@keyframes ctx-in` is `.12s`.
pub const MENU_ENTER_MS: u64 = 120;

/// Frame step for view-driven enter (~120 Hz).
const TICK_MS: u64 = 8;

/// Slide distance (px) at delta=0; zero at delta=1.
pub const SLIDE_PX: f32 = 14.;

/// EaseOutBack sample in 0..=1.
pub fn ease_enter(t: f32) -> f32 {
    gpui::easing::EasingCurve::EaseOutBack(1.5).sample(t.clamp(0.0, 1.0))
}

/// Context-menu enter easing — `cubic-bezier(.2,.8,.2,1)` from the reference
/// (`Chronos-Context-Menu.dc.html` `@keyframes ctx-in`). Distinct from
/// `ease_enter` (EaseOutBack) — menus rise gently, not with the overshoot
/// popups use.
pub fn ease_menu_enter(t: f32) -> f32 {
    gpui::easing::EasingCurve::CubicBezier(0.2, 0.8, 0.2, 1.0).sample(t.clamp(0.0, 1.0))
}

/// The same `cubic-bezier(.2,.8,.2,1)` curve as a `gpui_animation`
/// [`Transition`] — for `transition_when_else` morphs (accent-bar fade/grow,
/// row hover wash) in context menus. `ease_menu_enter` is a plain fn and
/// can't be passed where the crate wants a `Transition`.
#[derive(Clone, Copy)]
pub struct MenuEase;

impl gpui_animation::transition::Transition for MenuEase {
    fn calculate(&self, t: f32) -> f32 {
        ease_menu_enter(t)
    }
}

/// Rise distance (px) for context-menu enter — matches the reference
/// `translateY(-4px)` at delta=0.
pub const MENU_RISE_PX: f32 = 4.;

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

/// Panel-enter delta for the left content window. T346: the enter
/// animation's zero frame must never be able to strand the content
/// window at `opacity(0)` (a fully transparent layer-shell surface that
/// Hyprland stops driving with frame callbacks looks dead forever). A
/// panel whose ticker is **not** armed renders at `delta = 1` — fully
/// visible — so the only way content stays invisible is if the window
/// itself is gone. Armed panels clamp the ticker's eased progress.
pub fn panel_enter_delta(enter_t: f32, armed: bool) -> f32 {
    if armed {
        enter_t.clamp(0.0, 1.0)
    } else {
        1.0
    }
}

/// Opacity + rise from style fields (popups, view-driven `enter_t`).
pub fn apply_enter_rise<E: Styled>(el: E, delta: f32) -> E {
    let d = delta.clamp(0.0, 1.0);
    el.opacity(d).top(px(SLIDE_PX * (1.0 - d)))
}

/// Opacity + rise for context-menu enter (reference `@keyframes ctx-in`:
/// fade + `translateY(-4px)`, 0.12s, `cubic-bezier(.2,.8,.2,1)`). Scale is
/// omitted — the fork has no compositing `scale()` on elements; the rise +
/// fade reads as the same gentle lift.
pub fn apply_enter_menu<E: Styled>(el: E, delta: f32) -> E {
    let d = delta.clamp(0.0, 1.0);
    el.opacity(d).top(px(MENU_RISE_PX * (1.0 - d)))
}

/// Drive `enter_t` 0→1 with easing via background ticks + `cx.notify`.
///
/// Call once from `View::new` (or first render). `set_t` writes the eased
/// progress into the view. Uses the default popup easing ([`ease_enter`]) and
/// duration ([`ENTER_MS`]); context menus pass their own via
/// [`arm_enter_progress_with`].
pub fn arm_enter_progress<V: 'static>(
    cx: &mut Context<V>,
    set_t: impl Fn(&mut V, f32) + Send + 'static,
) {
    arm_enter_progress_with(cx, Duration::from_millis(ENTER_MS), ease_enter, set_t);
}

/// [`arm_enter_progress`] with an explicit duration + easing — context menus
/// run the reference `@keyframes ctx-in` curve (`ease_menu_enter`, 120 ms)
/// instead of the popups' EaseOutBack.
pub fn arm_enter_progress_with<V: 'static>(
    cx: &mut Context<V>,
    duration: Duration,
    ease: fn(f32) -> f32,
    set_t: impl Fn(&mut V, f32) + Send + 'static,
) {
    cx.spawn(async move |this, cx| {
        let start = Instant::now();
        let dur = duration;
        loop {
            let raw = (start.elapsed().as_secs_f32() / dur.as_secs_f32()).min(1.0);
            let eased = ease(raw);
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

    #[test]
    fn panel_enter_delta_never_transparent_when_unarmed() {
        // T346 regression: an unarmed panel (ticker not yet armed / not
        // running) must render at delta = 1, never at delta = 0 — the
        // zero frame of the old `with_animation` chain could stick at
        // opacity(0) forever on a fresh layer-shell window.
        assert_eq!(panel_enter_delta(0.0, false), 1.0);
        assert_eq!(panel_enter_delta(0.5, false), 1.0);
        assert_eq!(panel_enter_delta(1.0, false), 1.0);
        // Armed panels pass the ticker's eased progress through, clamped.
        assert_eq!(panel_enter_delta(0.0, true), 0.0);
        assert_eq!(panel_enter_delta(0.5, true), 0.5);
        assert_eq!(panel_enter_delta(1.0, true), 1.0);
        assert_eq!(panel_enter_delta(1.2, true), 1.0);
        assert_eq!(panel_enter_delta(-0.2, true), 0.0);
    }

    #[test]
    fn menu_ease_endpoints_and_monotonic() {
        // Cubic-bezier(.2,.8,.2,1) — the reference `ctx-in` curve.
        let e = MenuEase;
        assert!((e.calculate(0.0) - 0.0).abs() < 1e-5);
        assert!((e.calculate(1.0) - 1.0).abs() < 1e-4);
        // Gentle rise — no EaseOutBack overshoot (sample > 1.0 would break
        // the menu's opacity/rise math).
        for t in [0.25, 0.5, 0.75] {
            let v = e.calculate(t);
            assert!(v >= 0.0 && v <= 1.0, "t={t} → {v} must stay in 0..=1");
        }
        // Monotonic: later samples never go below earlier ones.
        let mut prev = 0.0_f32;
        for i in 1..=20 {
            let v = e.calculate(i as f32 / 20.0);
            assert!(v >= prev, "must be monotonic at t={}", i as f32 / 20.0);
            prev = v;
        }
    }
}
