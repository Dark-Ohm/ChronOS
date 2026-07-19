// crates/app/src/bar/mod.rs
pub use chronos_luau::bar::{BAR_HEIGHT, BarSection, BarWidget, BarWidgetRegistry};

mod widgets;

use chronos_services::Service;
use chronos_ui::Theme;

use std::time::Duration;

use gpui::{
    AnyElement, App, Bounds, Context, DisplayId, Render, Size, Window, WindowBackgroundAppearance,
    WindowBounds, WindowKind, WindowOptions, div, layer_shell::*, point, prelude::*, px,
};

use crate::state::{AppState, watch};

struct Bar;

impl Bar {
    fn new(cx: &mut Context<Self>) -> Self {
        // Subscribe to all service signals — any update repaints the bar.
        watch(cx, AppState::compositor(cx).subscribe(), |_, _, cx| {
            cx.notify();
        });
        watch(cx, AppState::network(cx).subscribe(), |_, _, cx| {
            cx.notify();
        });
        watch(cx, AppState::upower(cx).subscribe(), |_, _, cx| {
            cx.notify();
        });
        watch(cx, AppState::notification(cx).subscribe(), |_, _, cx| {
            cx.notify();
        });
        watch(cx, AppState::audio(cx).subscribe(), |_, _, cx| {
            cx.notify();
        });
        watch(cx, AppState::mpris(cx).subscribe(), |_, _, cx| {
            cx.notify();
        });
        watch(cx, AppState::cava(cx).subscribe(), |_, _, cx| {
            cx.notify();
        });

        // 1-second ticker for clock and other time-dependent widgets.
        // Uses the background executor, not tokio.
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(Duration::from_secs(1)).await;
                let _ = this.update(cx, |_, cx| cx.notify());
            }
        })
        .detach();

        Self
    }
}

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

        let theme = Theme::global(cx);
        div()
            .size_full()
            .bg(theme.bg.tertiary)
            .border_b_1()
            .border_color(theme.bg.elevated)
            .px(px(10.))
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
        // Gaps follow the mockup: left groups 12px apart, right controls 4px.
        BarSection::Left => div()
            .flex()
            .flex_1()
            .items_center()
            .justify_start()
            .gap(px(12.))
            .children(widgets)
            .into_any_element(),
        BarSection::Center => div()
            .flex()
            .flex_none()
            .items_center()
            .justify_center()
            .gap(px(8.))
            .children(widgets)
            .into_any_element(),
        BarSection::Right => div()
            .flex()
            .flex_1()
            .items_center()
            .justify_end()
            .gap(px(4.))
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
    match cx.open_window(window_options(display_id, cx), move |_, cx| {
        cx.new(|cx| Bar::new(cx))
    }) {
        Ok(_) => true,
        Err(err) => {
            tracing::warn!("Failed to open bar window: {}", err);
            false
        }
    }
}

/// Opens one bar window on the pult (control) display.
/// Called once at startup from `main.rs`.
pub fn init(cx: &mut App) {
    cx.set_global(BarWidgetRegistry::default());
    widgets::register_builtin(cx);

    cx.spawn(async move |cx| {
        // Small delay to allow Wayland to enumerate displays.
        cx.background_executor()
            .timer(Duration::from_millis(100))
            .await;

        let _ = cx.update(|cx: &mut App| {
            match crate::monitor::pult_display(cx) {
                Some(display_id) => {
                    tracing::info!("Opening bar on pult display {:?}", display_id);
                    open_on_display(Some(display_id), cx);
                }
                None => {
                    tracing::info!("No displays found, opening bar on default display");
                    open_on_display(None, cx);
                }
            }
        });
    })
    .detach();
}
