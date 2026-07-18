//! Desktop-widget terminal spike: layer-shell `Layer::Background` + PTY + VT100.
//!
//! Proves the stack works end-to-end (shell I/O on the desktop). Not a product
//! widget — fixed size/position, no Luau API, no skins, no resize/drag/copy.

mod view;

use std::time::Duration;

use gpui::{
    App, Bounds, DisplayId, Size, WindowBackgroundAppearance, WindowBounds, WindowKind,
    WindowOptions, layer_shell::*, point, prelude::*, px,
};

use crate::desktop_terminal::view::DesktopTerminalView;

/// Fixed spike surface (logical px).
const TERM_WIDTH: f32 = 600.;
const TERM_HEIGHT: f32 = 400.;
/// Below the bar, inset from the left edge.
const MARGIN_TOP: f32 = 80.;
const MARGIN_LEFT: f32 = 48.;

fn pick_display(cx: &App) -> Option<DisplayId> {
    cx.primary_display()
        .map(|d| d.id())
        .or_else(|| cx.displays().into_iter().next().map(|d| d.id()))
}

/// Background layer, top-left with margins. `OnDemand` keyboard so the rest of
/// the desktop keeps input until the user clicks this surface. **Never**
/// `Exclusive` — freezes Hyprland input stack (see launcher docs).
fn window_options(display_id: Option<DisplayId>) -> WindowOptions {
    WindowOptions {
        display_id,
        titlebar: None,
        window_bounds: Some(WindowBounds::Windowed(Bounds {
            origin: point(px(0.), px(0.)),
            size: Size::new(px(TERM_WIDTH), px(TERM_HEIGHT)),
        })),
        app_id: Some("chronos-desktop-terminal".to_string()),
        window_background: WindowBackgroundAppearance::Transparent,
        kind: WindowKind::LayerShell(LayerShellOptions {
            namespace: "desktop-terminal".to_string(),
            layer: Layer::Background,
            anchor: Anchor::TOP | Anchor::LEFT,
            exclusive_zone: None,
            // CSS order: top, right, bottom, left.
            margin: Some((px(MARGIN_TOP), px(0.), px(0.), px(MARGIN_LEFT))),
            keyboard_interactivity: KeyboardInteractivity::OnDemand,
            ..Default::default()
        }),
        ..Default::default()
    }
}

fn open(cx: &mut App) {
    let display_id = pick_display(cx);
    match cx.open_window(window_options(display_id), |_window, cx| {
        cx.new(|cx| DesktopTerminalView::new(cx))
    }) {
        Ok(_) => tracing::info!(
            "desktop_terminal: opened Layer::Background surface ({}×{}, margin top={} left={})",
            TERM_WIDTH,
            TERM_HEIGHT,
            MARGIN_TOP,
            MARGIN_LEFT
        ),
        Err(err) => tracing::error!("desktop_terminal: failed to open window: {err}"),
    }
}

/// Open the spike widget once displays are enumerated.
pub fn init(cx: &mut App) {
    cx.spawn(async move |cx| {
        cx.background_executor()
            .timer(Duration::from_millis(200))
            .await;
        let _ = cx.update(|cx: &mut App| open(cx));
    })
    .detach();
}
