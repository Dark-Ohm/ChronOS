//! Dock: pinned launcher panel, always visible at screen bottom.
//!
//! Opens as a layer-shell surface (like the bar, not a popup). Shows a
//! horizontal row of icons from `ApplicationsSubscriber` filtered to a
//! hardcoded pinned list. Click launches the app via `launcher::launch`.

mod view;

use std::time::Duration;

use gpui::{
    App, Bounds, DisplayId, Render, Size, Window, WindowBackgroundAppearance, WindowBounds,
    WindowKind, WindowOptions, div, layer_shell::*, point, prelude::*, px,
};

use crate::dock::view::DockView;
use crate::state::{watch, AppState};

const DOCK_HEIGHT: f32 = 56.;

/// Window options for a bottom-anchored dock on the given display.
fn window_options(display_id: Option<DisplayId>, cx: &App) -> WindowOptions {
    let display_size = display_id
        .and_then(|id| cx.find_display(id))
        .or_else(|| cx.primary_display())
        .map(|display| display.bounds().size)
        .unwrap_or_else(|| Size::new(px(1920.), px(1080.)));

    // Dock is centered horizontally, not full-width.
    let dock_width = px(400.);

    WindowOptions {
        display_id,
        titlebar: None,
        window_bounds: Some(WindowBounds::Windowed(Bounds {
            origin: point(
                (display_size.width - dock_width) / 2.,
                display_size.height - px(DOCK_HEIGHT),
            ),
            size: Size::new(dock_width, px(DOCK_HEIGHT)),
        })),
        app_id: Some("chronos-dock".to_string()),
        window_background: WindowBackgroundAppearance::Transparent,
        kind: WindowKind::LayerShell(LayerShellOptions {
            namespace: "dock".to_string(),
            layer: Layer::Top,
            anchor: Anchor::BOTTOM,
            exclusive_zone: Some(px(DOCK_HEIGHT)),
            margin: None,
            keyboard_interactivity: KeyboardInteractivity::None,
            ..Default::default()
        }),
        ..Default::default()
    }
}

fn open_on_display(display_id: Option<DisplayId>, cx: &mut App) -> bool {
    match cx.open_window(window_options(display_id, cx), move |_, cx| {
        cx.new(|cx| DockView::new(cx))
    }) {
        Ok(_) => true,
        Err(err) => {
            tracing::warn!("Failed to open dock window: {}", err);
            false
        }
    }
}

/// Opens one dock window per display. Called once at startup from `main.rs`.
pub fn init(cx: &mut App) {
    cx.spawn(async move |cx| {
        // Small delay to allow Wayland to enumerate displays.
        cx.background_executor()
            .timer(Duration::from_millis(150))
            .await;

        let _ = cx.update(|cx: &mut App| {
            let displays = cx.displays();
            if displays.is_empty() {
                tracing::info!("No displays found, opening dock on default display");
                open_on_display(None, cx);
            } else {
                tracing::info!("Opening dock on {} displays", displays.len());
                for d in displays {
                    open_on_display(Some(d.id()), cx);
                }
            }
        });
    })
    .detach();
}
