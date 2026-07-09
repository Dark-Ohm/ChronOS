// crates/app/src/bar/mod.rs
pub mod sections;
pub mod widget;

use std::time::Duration;

use gpui::{
    App, AnyElement, Bounds, Context, DisplayId, Render, Size, Window,
    WindowBackgroundAppearance, WindowBounds, WindowKind, WindowOptions, div, layer_shell::*,
    point, prelude::*, px, rgb,
};

use sections::{BarSection, BAR_COLOR, BAR_HEIGHT};
use widget::BarWidgetRegistry;

struct Bar;

impl Render for Bar {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let registry = cx.global::<BarWidgetRegistry>();
        let left: Vec<AnyElement> = registry
            .widgets_for(BarSection::Left)
            .map(|w| w.render(window, cx))
            .collect();
        let center: Vec<AnyElement> = registry
            .widgets_for(BarSection::Center)
            .map(|w| w.render(window, cx))
            .collect();
        let right: Vec<AnyElement> = registry
            .widgets_for(BarSection::Right)
            .map(|w| w.render(window, cx))
            .collect();

        div()
            .size_full()
            .bg(rgb(BAR_COLOR))
            .flex()
            .items_center()
            .child(section_div(BarSection::Left, left))
            .child(section_div(BarSection::Center, center))
            .child(section_div(BarSection::Right, right))
    }
}

/// Wrap a section's widgets in a flex container aligned per section.
fn section_div(section: BarSection, widgets: Vec<AnyElement>) -> AnyElement {
    match section {
        BarSection::Left => div()
            .flex()
            .flex_1()
            .justify_start()
            .gap(px(8.))
            .children(widgets)
            .into_any_element(),
        BarSection::Center => div()
            .flex()
            .flex_none()
            .justify_center()
            .gap(px(8.))
            .children(widgets)
            .into_any_element(),
        BarSection::Right => div()
            .flex()
            .flex_1()
            .justify_end()
            .gap(px(8.))
            .children(widgets)
            .into_any_element(),
    }
}

/// Returns window options for a top-anchored bar on the given display.
fn window_options(display_id: Option<DisplayId>, cx: &App) -> WindowOptions {
    let display_size = display_id
        .and_then(|id| cx.find_display(id))
        .or_else(|| cx.primary_display())
        .map(|display| display.bounds().size)
        .unwrap_or_else(|| Size::new(px(1920.), px(1080.)));

    WindowOptions {
        display_id,
        titlebar: None,
        window_bounds: Some(WindowBounds::Windowed(Bounds {
            origin: point(px(0.), px(0.)),
            size: Size::new(display_size.width, px(BAR_HEIGHT)),
        })),
        app_id: Some("chronos-bar".to_string()),
        window_background: WindowBackgroundAppearance::Transparent,
        kind: WindowKind::LayerShell(LayerShellOptions {
            namespace: "bar".to_string(),
            layer: Layer::Top,
            anchor: Anchor::LEFT | Anchor::RIGHT | Anchor::TOP,
            exclusive_zone: Some(px(BAR_HEIGHT)),
            margin: None,
            keyboard_interactivity: KeyboardInteractivity::None,
            ..Default::default()
        }),
        ..Default::default()
    }
}

fn open_on_display(display_id: Option<DisplayId>, cx: &mut App) -> bool {
    match cx.open_window(window_options(display_id, cx), move |_, cx| cx.new(|_| Bar)) {
        Ok(_) => true,
        Err(err) => {
            tracing::warn!("Failed to open bar window: {}", err);
            false
        }
    }
}

/// Opens one bar window per display and installs the empty widget registry.
/// Called once at startup from `main.rs`.
pub fn init(cx: &mut App) {
    cx.set_global(BarWidgetRegistry::default());

    cx.spawn(async move |cx| {
        // Small delay to allow Wayland to enumerate displays.
        cx.background_executor()
            .timer(Duration::from_millis(100))
            .await;

        let _ = cx.update(|cx: &mut App| {
            let displays = cx.displays();
            if displays.is_empty() {
                tracing::info!("No displays found, opening bar on default display");
                open_on_display(None, cx);
            } else {
                tracing::info!("Opening bar on {} displays", displays.len());
                for d in displays {
                    open_on_display(Some(d.id()), cx);
                }
            }
        });
    })
    .detach();
}
