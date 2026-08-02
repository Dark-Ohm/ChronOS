// crates/app/src/bar/mod.rs
pub use chronos_luau::bar::{BAR_HEIGHT, BarSection, BarWidget, BarWidgetRegistry};

pub mod agent_api;
pub mod appearance;
pub mod layout_config;
mod widgets;

use chronos_services::Service;
use chronos_ui::Theme;

use std::time::Duration;

use gpui::{
    AnyElement, App, Bounds, Context, DisplayId, Render, Size, Window, WindowBackgroundAppearance,
    WindowBounds, WindowKind, WindowOptions, div, layer_shell::*, point, prelude::*, px,
};

use crate::edit_mode;
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
        // Boot the vendored fork animation engine once (idempotent). This is
        // the first view rendered each session, so the `animation_tick` loop
        // that drives every `AnimatedWrapper` starts here.
        gpui_animation::init(window, cx);

        let editing = edit_mode::is_active(cx);
        let registry = cx.global::<BarWidgetRegistry>();
        let left: Vec<AnyElement> = registry
            .widgets_for(BarSection::Left)
            .enumerate()
            .map(|(i, w)| render_widget_slot(w, BarSection::Left, i, editing, window, cx))
            .collect();
        let center: Vec<AnyElement> = registry
            .widgets_for(BarSection::Center)
            .enumerate()
            .map(|(i, w)| render_widget_slot(w, BarSection::Center, i, editing, window, cx))
            .collect();
        let right: Vec<AnyElement> = registry
            .widgets_for(BarSection::Right)
            .enumerate()
            .map(|(i, w)| render_widget_slot(w, BarSection::Right, i, editing, window, cx))
            .collect();

        let theme = Theme::global(cx);
        let mut root = div()
            .size_full()
            .bg(theme.bg.tertiary)
            .border_b_1()
            .border_color(if editing {
                theme.accent.primary
            } else {
                theme.bg.elevated
            })
            .px(px(10.))
            .flex()
            .items_center();

        if editing {
            root = root.child(
                div()
                    .id("bar-edit-badge")
                    .flex_none()
                    .mr(px(8.))
                    .px(px(6.))
                    .py(px(2.))
                    .rounded(px(4.))
                    .bg(theme.accent.primary)
                    .text_color(theme.bg.primary)
                    .text_size(theme.font_sizes.xs)
                    .font_family(theme.font_mono)
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .child("EDIT"),
            );
        }

        root.child(section_div(BarSection::Left, left))
            .child(section_div(BarSection::Center, center))
            .child(section_div(BarSection::Right, right))
    }
}

/// In edit mode: widget + ◀ ▶ move controls. Normal mode: plain render.
fn render_widget_slot(
    w: &dyn BarWidget,
    section: BarSection,
    index: usize,
    editing: bool,
    window: &mut Window,
    cx: &App,
) -> AnyElement {
    let body = w.render(window, cx);
    if !editing {
        return body;
    }
    let theme = Theme::global(cx);
    let left_id = format!("bar-edit-left-{section:?}-{index}");
    let right_id = format!("bar-edit-right-{section:?}-{index}");
    div()
        .flex()
        .items_center()
        .gap(px(2.))
        .px(px(2.))
        .rounded(px(4.))
        .border_1()
        .border_color(theme.accent.primary.opacity(0.45))
        .child(
            div()
                .id(left_id)
                .flex_none()
                .w(px(14.))
                .h(px(18.))
                .flex()
                .items_center()
                .justify_center()
                .cursor_pointer()
                .text_size(px(10.))
                .text_color(theme.text.secondary)
                .hover(|s| s.bg(theme.border.subtle).text_color(theme.text.primary))
                .on_click(move |_ev, _window, cx| {
                    layout_config::move_widget(cx, section, index, -1);
                })
                .child("◀"),
        )
        .child(body)
        .child(
            div()
                .id(right_id)
                .flex_none()
                .w(px(14.))
                .h(px(18.))
                .flex()
                .items_center()
                .justify_center()
                .cursor_pointer()
                .text_size(px(10.))
                .text_color(theme.text.secondary)
                .hover(|s| s.bg(theme.border.subtle).text_color(theme.text.primary))
                .on_click(move |_ev, _window, cx| {
                    layout_config::move_widget(cx, section, index, 1);
                })
                .child("▶"),
        )
        .into_any_element()
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
    // Callers all pass a result of `pult_display_id_or_primary` here, which
    // already implements the full fallback chain (configured uuid →
    // largest by area → primary). Any further `.or_else(|| primary_display())`
    // just re-runs the same chain, so we just trust the id we got.
    let display_size = display_id
        .and_then(|id| cx.find_display(id))
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
    layout_config::spawn_watcher(cx);

    cx.spawn(async move |cx| {
        // Small delay to allow Wayland to enumerate displays.
        cx.background_executor()
            .timer(Duration::from_millis(100))
            .await;

        let _ = cx.update(|cx: &mut App| match crate::monitor::pult_display_id_or_primary(cx) {
            Some(display_id) => {
                tracing::info!("Opening bar on pult display {:?}", display_id);
                open_on_display(Some(display_id), cx);
            }
            None => {
                tracing::info!("No displays found, opening bar on default display");
                open_on_display(None, cx);
            }
        });
    })
    .detach();
}
