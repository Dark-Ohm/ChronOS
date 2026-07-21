//! Right side panel — lazy layer-shell overlay, hover-peek (task 8) or
//! pinned (bar-widget click / hotkey, this task). Window lifecycle
//! mirrors `system_popup/`/`volume_popup/`: `Layer::Overlay`,
//! `KeyboardInteractivity::None`, `close_this` reentrancy guard
//! (`ARCHITECTURE.md §4.1` — never re-entrant `handle.update` for
//! `remove_window()` from inside that window's own callback).
//!
//! **No Esc-to-close** — matches the real convention already in this
//! codebase (`volume_popup`/`system_popup` have no Esc handler either,
//! `KeyboardInteractivity::None` doesn't deliver key events). Dismiss is
//! re-toggle / click-away (pinned) / mouse-leave debounce (peek).

mod hover_strip;
mod mpris_card;
mod power_row;
mod spectrum_row;
pub mod view;

use chronos_luau::bar::BAR_HEIGHT;
use gpui::{
    App, Bounds, DisplayId, Global, Size, Window, WindowBackgroundAppearance, WindowBounds,
    WindowHandle, WindowKind, WindowOptions, layer_shell::*, point, prelude::*, px,
};

use crate::side_panel_right::view::SidePanelRightView;

const PANEL_WIDTH: f32 = 300.;

/// Gap from the panel to the display top **and** bottom. Equals bar height
/// so the panel sits flush under the bar (no overlap) with the same air
/// above the bottom bezel. Do **not** use TOP|BOTTOM stretch + dual
/// margins on Hyprland Overlay — exclusive zone + stretch skews the gaps
/// (measured unequal). Fixed height + TOP|RIGHT is the reliable path.
const PANEL_EDGE_GAP: f32 = BAR_HEIGHT;

#[derive(Default)]
pub struct SidePanelRightState {
    handle: Option<WindowHandle<SidePanelRightView>>,
    /// `true` when opened by hotkey/bar-click (`toggle` / `open_pinned`) —
    /// stays open until re-toggled. `false` when opened by hover — closes
    /// on mouse-leave debounce unless a pin request arrives while peeked.
    pinned: bool,
    /// Bumped on hover-enter (strip or panel). Leave schedules a close
    /// only if this value is still unchanged after the debounce window.
    peek_generation: u64,
}

impl Global for SidePanelRightState {}

/// Pure decision: should a peek-leave request close the panel?
fn should_close_on_peek_leave(state: &SidePanelRightState) -> bool {
    !state.pinned
}

/// Cursor entered strip or panel — cancel any pending peek-close.
pub(crate) fn hold_peek(cx: &mut App) {
    let state = cx.global_mut::<SidePanelRightState>();
    state.peek_generation = state.peek_generation.wrapping_add(1);
}

/// Cursor left strip or panel — close after debounce if still unpinned
/// and no later enter bumped the generation.
pub(crate) fn schedule_release_peek(cx: &mut App) {
    let generation = cx.global::<SidePanelRightState>().peek_generation;
    view::schedule_release_from_app(cx, generation);
}

fn display_height(display_id: Option<DisplayId>, cx: &App) -> f32 {
    display_id
        .and_then(|id| cx.find_display(id))
        .or_else(|| cx.primary_display())
        .map(|d| f32::from(d.bounds().size.height))
        .unwrap_or(1080.)
}

fn window_options(display_id: Option<DisplayId>, cx: &App) -> WindowOptions {
    let display_h = display_height(display_id, cx);
    // Equal top/bottom air: height = display − 2×gap.
    let panel_h = (display_h - 2. * PANEL_EDGE_GAP).max(100.);
    WindowOptions {
        display_id,
        titlebar: None,
        window_bounds: Some(WindowBounds::Windowed(Bounds {
            origin: point(px(0.), px(0.)),
            size: Size::new(px(PANEL_WIDTH), px(panel_h)),
        })),
        app_id: Some("chronos-side-panel-right".to_string()),
        window_background: WindowBackgroundAppearance::Transparent,
        kind: WindowKind::LayerShell(LayerShellOptions {
            namespace: "side_panel_right".to_string(),
            layer: Layer::Overlay,
            // TOP|RIGHT only (not BOTTOM): fixed height + top margin gives
            // symmetric vertical inset without Hyprland stretch skew.
            anchor: Anchor::TOP | Anchor::RIGHT,
            exclusive_zone: None,
            // (top, right, bottom, left)
            // Top margin 0: bar exclusive zone already places TOP-anchored
            // Overlay under the bar (y=BAR_HEIGHT). Height = display−2×gap
            // so bottom air equals top air.
            margin: None,
            keyboard_interactivity: KeyboardInteractivity::None,
            ..Default::default()
        }),
        ..Default::default()
    }
}

fn open_window(cx: &mut App, pinned: bool) {
    if cx.global::<SidePanelRightState>().handle.is_some() {
        if pinned {
            // Already open as peek → upgrade to pin without re-open.
            cx.global_mut::<SidePanelRightState>().pinned = true;
            tracing::info!("side_panel_right: upgraded peek → pinned");
        }
        return;
    }
    let display_id = crate::monitor::pult_display(cx);
    match cx.open_window(window_options(display_id, cx), |_, view_cx| {
        view_cx.new(|cx| SidePanelRightView::new(cx))
    }) {
        Ok(handle) => {
            let state = cx.global_mut::<SidePanelRightState>();
            state.handle = Some(handle);
            state.pinned = pinned;
            tracing::info!(
                "side_panel_right: opened ({})",
                if pinned { "pinned" } else { "peek" }
            );
        }
        Err(err) => tracing::warn!(
            "side_panel_right: failed to open ({}): {err}",
            if pinned { "pinned" } else { "peek" }
        ),
    }
}

/// Open pinned (idempotent — no-op if already open; upgrades peek → pin).
pub fn open_pinned(cx: &mut App) {
    open_window(cx, true);
}

/// Open in peek mode (hover entered the strip). No-op if already open in
/// either mode (does not demote pin to peek).
pub fn open_peek(cx: &mut App) {
    open_window(cx, false);
}

/// Mouse left the strip and the panel. Closes only if not pinned.
pub fn close_peek_if_not_pinned(cx: &mut App) {
    if !should_close_on_peek_leave(cx.global::<SidePanelRightState>()) {
        return;
    }
    close(cx);
}

/// Close from outside (bar toggle / hotkey).
///
/// Note the `match` instead of `let _ =`: `system_popup`/`volume_popup`
/// swallow this Err today, and a swallowed `handle.update` Err is exactly
/// what hid the ghost-window bug for a full session (HANDOFF.md
/// 2026-07-18). New code does not inherit that wart — an Err here means
/// the handle was taken but the window never closed, i.e. a ghost.
pub fn close(cx: &mut App) {
    if let Some(handle) = cx.global_mut::<SidePanelRightState>().handle.take() {
        cx.global_mut::<SidePanelRightState>().pinned = false;
        match handle.update(cx, |_, window: &mut Window, _| window.remove_window()) {
            Ok(()) => {
                tracing::info!("side_panel_right: closed");
            }
            Err(e) => tracing::warn!(
                "side_panel_right: close() could not reach the window ({e}) — possible ghost"
            ),
        }
    } else {
        cx.global_mut::<SidePanelRightState>().pinned = false;
    }
}

/// Close from inside a callback that already holds `&mut Window` for this
/// panel. Must not re-enter `handle.update` on the same id (ghost-window
/// guard, `ARCHITECTURE.md §4.1`).
#[allow(dead_code)] // used by tasks 8–11 (peek leave, dismiss controls)
pub(crate) fn close_this(window: &mut Window, cx: &mut App) {
    let this = window.window_handle();
    let tracked = cx
        .global::<SidePanelRightState>()
        .handle
        .as_ref()
        .map(|h| **h == this)
        .unwrap_or(false);
    if tracked {
        let state = cx.global_mut::<SidePanelRightState>();
        state.handle.take();
        state.pinned = false;
    }
    window.remove_window();
    tracing::info!("side_panel_right: close_this");
}

/// Bar-widget click / hotkey target.
pub fn toggle(_window: &mut Window, cx: &mut App) {
    if cx.global::<SidePanelRightState>().handle.is_some() {
        close(cx);
    } else {
        open_pinned(cx);
    }
}

pub fn init(cx: &mut App) {
    cx.set_global(SidePanelRightState::default());
    // Defer the strip one tick so `cx.displays()` / pult uuid match what
    // `bar::init` sees a moment later. Opening the strip synchronously in
    // `main` before the bar historically landed it on the wrong output
    // (HDMI-A-1) while the panel+bar bound to DP-1 (pult).
    cx.spawn(async move |cx| {
        cx.background_executor()
            .timer(std::time::Duration::from_millis(50))
            .await;
        cx.update(|cx| {
            hover_strip::init_hover_strip(cx);
            // Optional smoke: pin-open for grim without hover/ydotool.
            // Not product wiring — only when env is set.
            if std::env::var_os("CHRONOS_SMOKE_SIDE_PANEL").is_some() {
                open_pinned(cx);
            }
        });
    })
    .detach();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn peek_close_request_is_noop_while_pinned() {
        let mut state = SidePanelRightState::default();
        state.pinned = true;
        assert!(!should_close_on_peek_leave(&state));
    }

    #[test]
    fn peek_close_request_closes_when_not_pinned() {
        let mut state = SidePanelRightState::default();
        state.pinned = false;
        assert!(should_close_on_peek_leave(&state));
    }
}
