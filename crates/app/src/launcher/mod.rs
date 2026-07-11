//! Launcher module: desktop entry cache, fuzzy search, overlay view, launch.

pub mod cache;
pub mod entry;
pub mod launch;
pub mod search;
pub mod view;

use gpui::{
    layer_shell::*, point, prelude::*, px, App, Bounds, DisplayId, Global, Size, Window,
    WindowBackgroundAppearance, WindowBounds, WindowHandle, WindowKind, WindowOptions,
};

use crate::launcher::view::LauncherView;

const LAUNCHER_WIDTH: f32 = 600.;
const LAUNCHER_HEIGHT: f32 = 400.;

/// Tracks the open launcher window so `toggle` can open/close it.
#[derive(Default)]
struct LauncherState {
    handle: Option<WindowHandle<LauncherView>>,
}

impl Global for LauncherState {}

/// Window options for a centered overlay on the given display.
///
/// Centering is done via layer-shell anchors + margins (stretch to the full
/// display, then inset by the margin on every side). We avoid
/// `KeyboardInteractivity::Exclusive`: on some compositors an exclusive
/// layer-surface that never ack's keyboard focus can wedge the input stack
/// and take the session down. `OnDemand` + `activate_window()` gives the
/// launcher focus without that risk.
fn window_options(display_id: Option<DisplayId>, cx: &App) -> WindowOptions {
    let display_size = display_id
        .and_then(|id| cx.find_display(id))
        .or_else(|| cx.primary_display())
        .map(|display| display.bounds().size)
        .unwrap_or_else(|| Size::new(px(1920.), px(1080.)));

    let margin_x = ((display_size.width - px(LAUNCHER_WIDTH)) / 2.).max(px(0.));
    let margin_y = ((display_size.height - px(LAUNCHER_HEIGHT)) / 2.).max(px(0.));

    WindowOptions {
        display_id,
        titlebar: None,
        window_bounds: Some(WindowBounds::Windowed(Bounds {
            origin: point(px(0.), px(0.)),
            size: Size::new(px(LAUNCHER_WIDTH), px(LAUNCHER_HEIGHT)),
        })),
        app_id: Some("chronos-launcher".to_string()),
        window_background: WindowBackgroundAppearance::Transparent,
        kind: WindowKind::LayerShell(LayerShellOptions {
            namespace: "launcher".to_string(),
            layer: Layer::Overlay,
            anchor: Anchor::TOP | Anchor::BOTTOM | Anchor::LEFT | Anchor::RIGHT,
            exclusive_zone: None,
            margin: Some((margin_y, margin_x, margin_y, margin_x)),
            keyboard_interactivity: KeyboardInteractivity::OnDemand,
            ..Default::default()
        }),
        ..Default::default()
    }
}

/// Open the launcher overlay. No-op if it is already open.
pub fn open(cx: &mut App) {
    let handle = cx.global::<LauncherState>().handle.clone();
    let already_open = handle
        .as_ref()
        .map(|h| h.is_active(cx).unwrap_or(false))
        .unwrap_or(false);
    if already_open {
        return;
    }

    let display_id = cx
        .primary_display()
        .map(|d| d.id())
        .or_else(|| cx.displays().into_iter().next().map(|d| d.id()));

    match cx.open_window(window_options(display_id, cx), |_, cx| {
        cx.new(|cx| LauncherView::new(cx))
    }) {
        Ok(handle) => {
            cx.global_mut::<LauncherState>().handle = Some(handle);
        }
        Err(err) => tracing::warn!("Failed to open launcher window: {}", err),
    }
}

/// Close the launcher overlay if it is open.
pub fn close(cx: &mut App) {
    if let Some(handle) = cx.global_mut::<LauncherState>().handle.take() {
        let _ = handle.update(cx, |_, window: &mut Window, _| window.remove_window());
    }
}

/// Toggle the launcher overlay open/closed.
pub fn toggle(cx: &mut App) {
    let handle = cx.global::<LauncherState>().handle.clone();
    let is_open = handle
        .as_ref()
        .map(|h| h.is_active(cx).unwrap_or(false))
        .unwrap_or(false);
    if is_open {
        close(cx);
    } else {
        open(cx);
    }
}

/// Register the launcher's global state. Called once at startup from `main.rs`.
pub fn init(cx: &mut App) {
    cx.set_global(LauncherState::default());
}
