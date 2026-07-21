//! Right side panel view — MPRIS card (task 9); spectrum (task 10) and
//! power row (task 11) append as further children of `render()`.
//!
//! ## `on_hover` / animation split (fork rule)
//! Our gpui fork stores a **single** `Option` hover handler per element and
//! `debug_assert!`s if `.on_hover` is set twice. Consequences:
//! - Root node: **only** the peek close-debounce `on_hover` (this file).
//! - `gpui-animation`'s `.transition_on_hover` also installs `on_hover` →
//!   **never** combine it with the root debounce. Peek motion uses
//!   state-driven `.transition_when` on an **inner** wrapper that has no
//!   manual `on_hover`.
//! - Hover strip is a separate window (`hover_strip.rs`) with its own
//!   single `on_hover`.

use std::time::Duration;

use chronos_services::{MprisState, Service};
use chronos_ui::Theme;
use gpui::{AsyncApp, Context, IntoElement, Render, Window, div, prelude::*, px};
use gpui_animation::animation::TransitionExt;
use gpui_animation::transition::general::Linear;

use crate::side_panel_right::mpris_card::render_mpris_card;
use crate::state::{self, AppState};

/// Delay before peek-close after mouse leaves panel (or strip). Long enough
/// to cross the 4px strip → panel gap without flicker; short enough to feel
/// snappy on real leave-to-desktop.
const PEEK_LEAVE_DEBOUNCE: Duration = Duration::from_millis(280);

/// Slide/fade-in duration when the panel window first appears.
const REVEAL_MS: u64 = 180;

pub struct SidePanelRightView {
    mpris: MprisState,
    /// State-driven reveal for `transition_when` (not hover-driven).
    revealed: bool,
}

impl SidePanelRightView {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let signal = AppState::mpris(cx).subscribe();
        state::watch(cx, signal, |this: &mut Self, data: MprisState, cx| {
            this.mpris = data;
            cx.notify();
        });

        // Kick reveal after first paint so `transition_when(true)` has a
        // from→to pair (opacity 0 → 1). Entity update may return Result on
        // this fork — only close the task if the window is gone.
        cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(Duration::from_millis(16))
                .await;
            if this
                .update(cx, |this, cx| {
                    this.revealed = true;
                    cx.notify();
                })
                .is_err()
            {
                // view dropped (peek closed before first frame)
            }
        })
        .detach();

        Self {
            mpris: AppState::mpris(cx).get(),
            revealed: false,
        }
    }
}

impl Render for SidePanelRightView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = *Theme::global(cx);
        let revealed = self.revealed;

        // ── OUTER: sole `on_hover` on this window (debounce) ──────────────
        // Do NOT put `.with_transition` / `.transition_on_hover` here.
        div()
            .id("side-panel-right-root")
            .size_full()
            .on_hover(|hovered, _window, cx| {
                if *hovered {
                    crate::side_panel_right::hold_peek(cx);
                } else {
                    crate::side_panel_right::schedule_release_peek(cx);
                }
            })
            // ── INNER: visual shell + optional state-driven animation ───
            // AnimatedWrapper installs its own `on_hover` on the child it
            // wraps — that is this inner node only. Never attach another
            // `on_hover` here.
            .child(
                div()
                    .id("side-panel-body")
                    .with_transition("side-panel-body")
                    .size_full()
                    .bg(theme.bg.secondary)
                    .border_l_1()
                    .border_color(theme.border.default)
                    .p(px(20.))
                    .flex()
                    .flex_col()
                    .gap(px(20.))
                    .opacity(if revealed { 1.0 } else { 0.0 })
                    .transition_when(
                        revealed,
                        Duration::from_millis(REVEAL_MS),
                        Linear,
                        |s| s.opacity(1.0),
                    )
                    .child(render_mpris_card(&self.mpris, &theme, cx)),
            )
    }
}

/// Re-export for callers that want the constant without digging.
#[allow(dead_code)]
pub(crate) fn peek_leave_debounce() -> Duration {
    PEEK_LEAVE_DEBOUNCE
}

/// App-level debounce used by both strip leave and panel leave.
/// Generation-guarded (same pattern as `tray_menu::schedule_autoclose`).
pub(crate) fn schedule_release_from_app(cx: &mut gpui::App, generation: u64) {
    cx.spawn(async move |app_cx: &mut AsyncApp| {
        app_cx
            .background_executor()
            .timer(PEEK_LEAVE_DEBOUNCE)
            .await;
        app_cx.update(|app_cx| {
            if app_cx
                .global::<crate::side_panel_right::SidePanelRightState>()
                .peek_generation
                != generation
            {
                return;
            }
            crate::side_panel_right::close_peek_if_not_pinned(app_cx);
        });
    })
    .detach();
}
