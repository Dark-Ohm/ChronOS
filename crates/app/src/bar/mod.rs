// crates/app/src/bar/mod.rs
pub use chronos_luau::bar::{BAR_HEIGHT, BarSection, BarWidget, BarWidgetRegistry};

pub mod agent_api;
pub mod appearance;
pub mod layout_config;
mod widgets;

use chronos_services::Service;
use chronos_ui::Theme;

use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use gpui::{
    AnyElement, App, BoxShadow, Bounds, Context, DisplayId, Render, Size, Window,
    WindowBackgroundAppearance, WindowBounds, WindowHandle, WindowKind, WindowOptions, div,
    layer_shell::*, point, prelude::*, px,
};

use crate::edit_mode;
use crate::state::{AppState, watch};

use self::appearance::{BarEdge, BarElevation, BarWidth};

struct Bar;

/// The live bar window handle — captured at open so `apply_appearance` can
/// resize / re-zone the surface on hot-reload without `remove_window`+reopen
/// (T200; ghost-window guard per `wayland-window-lifecycle`).
static BAR_WINDOW: OnceLock<Mutex<Option<WindowHandle<Bar>>>> = OnceLock::new();

fn bar_window() -> &'static Mutex<Option<WindowHandle<Bar>>> {
    BAR_WINDOW.get_or_init(|| Mutex::new(None))
}

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
        let appearance = layout_config::cached_appearance();
        let mut root = div()
            .size_full()
            .bg(theme.bg.tertiary)
            .px(px(10.))
            .flex()
            .items_center();
        // Border side follows the edge (top bar → bottom border, bottom bar →
        // top border). Vertical edges are not applied yet (T200 v1).
        if appearance.edge == BarEdge::Bottom {
            root = root.border_t_1();
        } else {
            root = root.border_b_1();
        }
        root = root.border_color(if editing {
            theme.accent.primary
        } else {
            theme.bg.elevated
        });
        if appearance.radius > 0.0 {
            root = root.rounded(px(appearance.radius)).overflow_hidden();
        }
        if appearance.elevation != BarElevation::None {
            root = root.shadow(elevation_shadow(appearance.elevation, theme));
        }

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

/// Returns window options for the bar on the given display.
///
/// Cold-path appearance: `edge` (anchor) and `height` (+exclusive zone) come
/// from `cached_appearance()`. Width mode / margins / align need live
/// `set_anchor`/`set_margin` (not in fork, T198) — v1 keeps full-width
/// stretch; `apply_appearance` warns when a file asks for more.
fn window_options(display_id: Option<DisplayId>, cx: &App) -> WindowOptions {
    // Callers all pass a result of `pult_display_id_or_primary` here, which
    // already implements the full fallback chain (configured uuid →
    // largest by area → primary). Any further `.or_else(|| primary_display())`
    // just re-runs the same chain, so we just trust the id we got.
    let display_size = display_id
        .and_then(|id| cx.find_display(id))
        .map(|display| display.bounds().size)
        .unwrap_or_else(|| Size::new(px(1920.), px(1080.)));

    let appearance = layout_config::cached_appearance();
    let height = px(appearance.height);
    // Edge is cold-path: anchor is fixed at open (no live set_anchor in the
    // fork). Vertical edges parse+store but are not applied (later wave).
    let anchor = match appearance.edge {
        BarEdge::Bottom => Anchor::LEFT | Anchor::RIGHT | Anchor::BOTTOM,
        _ => Anchor::LEFT | Anchor::RIGHT | Anchor::TOP,
    };
    let exclusive_zone = if appearance.exclusive {
        Some(height)
    } else {
        Some(px(0.))
    };

    WindowOptions {
        display_id,
        titlebar: None,
        window_bounds: Some(WindowBounds::Windowed(Bounds {
            origin: point(px(0.), px(0.)),
            size: Size::new(display_size.width, height),
        })),
        app_id: Some("chronos-bar".to_string()),
        window_background: WindowBackgroundAppearance::Transparent,
        kind: WindowKind::LayerShell(LayerShellOptions {
            namespace: "bar".to_string(),
            layer: Layer::Top,
            anchor,
            exclusive_zone,
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
        Ok(handle) => {
            *bar_window()
                .lock()
                .unwrap_or_else(|e| e.into_inner()) = Some(handle);
            true
        }
        Err(err) => {
            tracing::warn!("Failed to open bar window: {}", err);
            false
        }
    }
}

/// Map `elevation` to drop shadows for the bar strip.
///
/// Frosted-blur mapping (`elevation_blur_layer`) is deferred: the bar
/// repaints on every service update (audio/network/mpris/cava push high
/// frequencies), and `window.paint_blur` per frame on a full-width strip is a
/// real cost for a 144 fps shell. Shadows are cheap and visible on the edge.
fn elevation_shadow(elevation: BarElevation, theme: &Theme) -> Vec<BoxShadow> {
    match elevation {
        BarElevation::None => Vec::new(),
        BarElevation::Soft => vec![BoxShadow::new(px(0.), px(3.), theme.bg.primary.opacity(0.4))
            .blur_radius(px(10.))],
        BarElevation::Strong => vec![
            BoxShadow::new(px(0.), px(8.), theme.bg.primary.opacity(0.55)).blur_radius(px(20.)),
            BoxShadow::new(px(0.), px(3.), theme.bg.primary.opacity(0.35)).blur_radius(px(8.)),
        ],
    }
}

/// Deferred-field warnings, deduplicated per value (the watcher fires on
/// every `bar.toml` touch; a stuck config must not spam the log).
static DEFERRED_WARNED: OnceLock<Mutex<Option<(BarWidth, BarEdge)>>> = OnceLock::new();

/// Warn (once per value) that `width`/`edge` need a restart — no live
/// `set_anchor`/`set_margin` in the fork (T198), cold-path only.
fn warn_deferred_fields(width: BarWidth, edge: BarEdge) {
    let slot = DEFERRED_WARNED.get_or_init(|| Mutex::new(None));
    let mut last = slot.lock().unwrap_or_else(|e| e.into_inner());
    if *last == Some((width, edge)) {
        return;
    }
    *last = Some((width, edge));
    drop(last);
    if width != BarWidth::Full {
        tracing::warn!(
            "bar: width mode change requires shell restart (no live set_anchor in fork)"
        );
    }
    if edge != BarEdge::Top {
        tracing::warn!(
            "bar: edge change requires shell restart (no live set_anchor in fork)"
        );
    }
}

/// Live-apply `cached_appearance()` to the bar window — height, exclusive
/// zone, input region. Called from `layout_config::apply` on every `bar.toml`
/// change (300 ms debounce) and once after open — no process restart.
/// Idempotent.
///
/// Width mode / edge changes need anchor+margin (fork `set_anchor`/
/// `set_margin` do not exist yet, T198) — cold-path only, restart to apply;
/// we log that here so a config edit surfaces it.
pub fn apply_appearance(cx: &mut App) {
    let appearance = layout_config::cached_appearance();
    warn_deferred_fields(appearance.width, appearance.edge);

    let Some(handle) = *bar_window().lock().unwrap_or_else(|e| e.into_inner()) else {
        tracing::debug!("bar: no window yet, appearance apply deferred");
        return;
    };

    match handle.update(cx, |_bar, window, cx| {
        // Height is live on the free axis; width stays compositor-set for a
        // stretched (full) bar.
        let current = window.bounds().size;
        window.resize(Size::new(current.width, px(appearance.height)));
        if appearance.exclusive {
            window.set_exclusive_zone(px(appearance.height));
        } else {
            window.set_exclusive_zone(px(0.));
        }
        // v1 surface == visible pill → full-surface input region is correct.
        // Explicit call keeps intent documented (API exists — T198 note).
        window.set_input_region(None);
        cx.notify();
    }) {
        Ok(()) => {
            // Publish only after the window actually resized — the panel gap
            // must follow the applied (not merely configured) height.
            crate::state::set_bar_height_px(appearance.height);
            tracing::debug!("bar: appearance applied");
        }
        Err(e) => tracing::warn!("bar: appearance apply could not reach window ({e})"),
    }
}

/// Opens one bar window on the pult (control) display.
/// Called once at startup from `main.rs`.
pub fn init(cx: &mut App) {
    cx.set_global(BarWidgetRegistry::default());
    widgets::register_builtin(cx);
    layout_config::spawn_watcher(cx);
    // Publish the configured height before any panel opens (strips open
    // ~50 ms after start; bar window at ~100 ms) — panels must see the
    // right gap from the first frame.
    crate::state::set_bar_height_px(layout_config::cached_appearance().height);

    cx.spawn(async move |cx| {
        // Small delay to allow Wayland to enumerate displays.
        cx.background_executor()
            .timer(Duration::from_millis(100))
            .await;

        let _ = cx.update(|cx: &mut App| {
            match crate::monitor::pult_display_id_or_primary(cx) {
                Some(display_id) => {
                    tracing::info!("Opening bar on pult display {:?}", display_id);
                    open_on_display(Some(display_id), cx);
                }
                None => {
                    tracing::info!("No displays found, opening bar on default display");
                    open_on_display(None, cx);
                }
            }
            // Idempotent: apply the configured appearance right after open
            // (resize/zone) and surface deferred-field warnings.
            apply_appearance(cx);
        });
    })
    .detach();
}

#[cfg(test)]
mod tests {
    use super::*;
    use chronos_ui::Theme;

    #[test]
    fn elevation_none_has_no_shadow() {
        let theme = Theme::default();
        assert!(elevation_shadow(BarElevation::None, &theme).is_empty());
        assert_eq!(elevation_shadow(BarElevation::Soft, &theme).len(), 1);
        assert_eq!(elevation_shadow(BarElevation::Strong, &theme).len(), 2);
    }
}
