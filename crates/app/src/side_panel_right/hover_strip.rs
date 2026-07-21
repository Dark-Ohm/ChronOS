//! Invisible 4px hit-test strip on the right screen edge. A permanently
//! open, zero-content layer-shell window whose only job is to receive
//! `on_hover` — GPUI delivers mouse-enter/leave to any window under the
//! cursor regardless of visible content, so a fully transparent 4px-wide
//! strip is a legitimate hit-test surface, not a hack. This sidesteps
//! compositor-level pointer polling entirely (the alternative considered
//! in the spec's open question §8.1 and rejected here as unnecessary
//! complexity once GPUI's own hover event was confirmed sufficient).

use gpui::{
    App, Bounds, DisplayId, IntoElement, Render, Size, Window, WindowBackgroundAppearance,
    WindowBounds, WindowKind, WindowOptions, div, layer_shell::*, point, prelude::*, px,
};

const STRIP_WIDTH: f32 = 4.;

struct HoverStripView {}

impl Render for HoverStripView {
    fn render(&mut self, _window: &mut Window, _cx: &mut gpui::Context<Self>) -> impl IntoElement {
        // Transparent hit surface. Enter → peek open + hold generation.
        // Leave → schedule release (panel enter will bump generation and
        // cancel the close if the user crossed onto the panel).
        div().size_full().id("side-panel-hover-strip").on_hover(|hovered, _window, cx| {
            if *hovered {
                super::hold_peek(cx);
                super::open_peek(cx);
            } else {
                super::schedule_release_peek(cx);
            }
        })
    }
}

fn strip_window_options(display_id: Option<DisplayId>) -> WindowOptions {
    WindowOptions {
        display_id,
        titlebar: None,
        window_bounds: Some(WindowBounds::Windowed(Bounds {
            origin: point(px(0.), px(0.)),
            size: Size::new(px(STRIP_WIDTH), px(0.)),
        })),
        app_id: Some("chronos-side-panel-hover-strip".to_string()),
        window_background: WindowBackgroundAppearance::Transparent,
        kind: WindowKind::LayerShell(LayerShellOptions {
            namespace: "side_panel_hover_strip".to_string(),
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

/// Open the permanent hover-detector strip. Called once from
/// `side_panel_right::init`, never toggled or closed for the life of the
/// process.
pub fn init_hover_strip(cx: &mut App) {
    let display_id = crate::monitor::pult_display(cx);
    tracing::info!("side_panel_right: hover strip on display_id={display_id:?}");
    match cx.open_window(strip_window_options(display_id), |_, view_cx| {
        view_cx.new(|_| HoverStripView {})
    }) {
        Ok(_) => tracing::info!("side_panel_right: hover strip opened"),
        Err(err) => tracing::warn!("side_panel_right: failed to open hover strip: {err}"),
    }
}
