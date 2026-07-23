//! Invisible 4px hit-test strip on the left screen edge. A permanently
//! open, zero-content layer-shell window whose only job is to receive
//! `on_hover` — GPUI delivers mouse-enter/leave to any window under the
//! cursor regardless of visible content, so a fully transparent 4px-wide
//! strip is a legitimate hit-test surface, not a hack. This sidesteps
//! compositor-level pointer polling entirely.

use chronos_luau::bar::BAR_HEIGHT;
use gpui::{
    App, Bounds, DisplayId, IntoElement, Render, Size, Window, WindowBackgroundAppearance,
    WindowBounds, WindowKind, WindowOptions, div, layer_shell::*, point, prelude::*, px,
};

const STRIP_WIDTH: f32 = 4.;
/// Match panel vertical inset (`mod.rs` PANEL_EDGE_GAP).
const STRIP_EDGE_GAP: f32 = BAR_HEIGHT;

struct HoverStripView {}

impl Render for HoverStripView {
    fn render(&mut self, _window: &mut Window, _cx: &mut gpui::Context<Self>) -> impl IntoElement {
        // Transparent hit surface. Enter → peek open + hold generation.
        // Leave → schedule release (panel enter will bump generation and
        // cancel the close if the user crossed onto the panel).
        div()
            .size_full()
            .id("side-panel-left-hover-strip")
            .on_hover(|hovered, _window, cx| {
                if *hovered {
                    super::hold_peek(cx);
                    super::open_peek(cx);
                } else {
                    super::schedule_release_peek(cx);
                }
            })
    }
}

fn strip_window_options(display_id: Option<DisplayId>, cx: &App) -> WindowOptions {
    let display_h = display_id
        .and_then(|id| cx.find_display(id))
        .or_else(|| cx.primary_display())
        .map(|d| f32::from(d.bounds().size.height))
        .unwrap_or(1080.);
    // Top gap only (under bar); reach display bottom like the panel.
    let strip_h = (display_h - STRIP_EDGE_GAP).max(100.);
    WindowOptions {
        display_id,
        titlebar: None,
        window_bounds: Some(WindowBounds::Windowed(Bounds {
            origin: point(px(0.), px(0.)),
            size: Size::new(px(STRIP_WIDTH), px(strip_h)),
        })),
        app_id: Some("chronos-side-panel-left-hover-strip".to_string()),
        window_background: WindowBackgroundAppearance::Transparent,
        kind: WindowKind::LayerShell(LayerShellOptions {
            namespace: "side_panel_left_hover_strip".to_string(),
            layer: Layer::Overlay,
            anchor: Anchor::TOP | Anchor::LEFT,
            exclusive_zone: None,
            margin: None,
            keyboard_interactivity: KeyboardInteractivity::None,
            ..Default::default()
        }),
        ..Default::default()
    }
}

/// Open the permanent hover-detector strip. Called once from
/// `side_panel_left::init`, never toggled or closed for the life of the
/// process.
pub fn init_hover_strip(cx: &mut App) {
    let display_id = crate::monitor::pult_display(cx);
    tracing::info!("side_panel_left: hover strip on display_id={display_id:?}");
    match cx.open_window(strip_window_options(display_id, cx), |_, view_cx| {
        view_cx.new(|_| HoverStripView {})
    }) {
        Ok(_) => tracing::info!("side_panel_left: hover strip opened"),
        Err(err) => tracing::warn!("side_panel_left: failed to open hover strip: {err}"),
    }
}
