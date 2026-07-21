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
//! re-toggle / click-away (pinned) / mouse-leave debounce (peek, task 8).

pub mod view;

use gpui::{
    App, Bounds, DisplayId, Global, Size, Window, WindowBackgroundAppearance, WindowBounds,
    WindowHandle, WindowKind, WindowOptions, layer_shell::*, point, prelude::*, px,
};

use crate::side_panel_right::view::SidePanelRightView;

const PANEL_WIDTH: f32 = 300.;

#[derive(Default)]
pub struct SidePanelRightState {
    handle: Option<WindowHandle<SidePanelRightView>>,
}

impl Global for SidePanelRightState {}

fn window_options(display_id: Option<DisplayId>) -> WindowOptions {
    WindowOptions {
        display_id,
        titlebar: None,
        window_bounds: Some(WindowBounds::Windowed(Bounds {
            origin: point(px(0.), px(0.)),
            // Height is filled by TOP|BOTTOM anchor; compositor owns that axis.
            size: Size::new(px(PANEL_WIDTH), px(0.)),
        })),
        app_id: Some("chronos-side-panel-right".to_string()),
        window_background: WindowBackgroundAppearance::Transparent,
        kind: WindowKind::LayerShell(LayerShellOptions {
            namespace: "side_panel_right".to_string(),
            layer: Layer::Overlay,
            anchor: Anchor::TOP | Anchor::BOTTOM | Anchor::RIGHT,
            exclusive_zone: Some(px(0.)),
            margin: None,
            keyboard_interactivity: KeyboardInteractivity::None,
            ..Default::default()
        }),
        ..Default::default()
    }
}

/// Open pinned (idempotent — no-op if already open).
pub fn open_pinned(cx: &mut App) {
    if cx.global::<SidePanelRightState>().handle.is_some() {
        return;
    }
    let display_id = crate::monitor::pult_display(cx);
    match cx.open_window(window_options(display_id), |_, view_cx| {
        view_cx.new(|cx| SidePanelRightView::new(cx))
    }) {
        Ok(handle) => {
            cx.global_mut::<SidePanelRightState>().handle = Some(handle);
            tracing::info!("side_panel_right: opened pinned");
        }
        Err(err) => tracing::warn!("side_panel_right: failed to open: {err}"),
    }
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
        match handle.update(cx, |_, window: &mut Window, _| window.remove_window()) {
            Ok(()) => {
                tracing::info!("side_panel_right: closed");
            }
            Err(e) => tracing::warn!(
                "side_panel_right: close() could not reach the window ({e}) — possible ghost"
            ),
        }
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
        cx.global_mut::<SidePanelRightState>().handle.take();
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
}
