// crates/app/src/bar/mod.rs
pub use chronos_luau::bar::{BAR_HEIGHT, BarSection, BarWidget, BarWidgetRegistry};

pub mod agent_api;
pub mod appearance;
pub mod layout_config;
mod widgets;

use chronos_services::Service;
use chronos_ui::{Theme, WindowRootExt};

use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use gpui::{
    AnyElement, App, BoxShadow, Bounds, Context, DisplayId, Render, Size, Window,
    WindowBackgroundAppearance, WindowBounds, WindowHandle, WindowKind, WindowOptions, div,
    layer_shell::*, point, prelude::*, px,
};

use crate::edit_mode;
use crate::state::{AppState, watch};

use self::appearance::{BarAppearance, BarEdge, BarElevation, BarWidth};

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
        let right: Vec<(String, AnyElement)> = registry
            .widgets_for(BarSection::Right)
            .enumerate()
            .map(|(i, w)| {
                let name = w.name().to_string();
                let el = render_widget_slot(w, BarSection::Right, i, editing, window, cx);
                (name, el)
            })
            .collect();

        let theme = Theme::global(cx);
        let appearance = layout_config::cached_appearance();
        let mut root = div()
            .window_font(theme)
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
            theme.border.subtle
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
            .child(right_section_div(right))
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

/// Spacing inside one semantic group of the right tray cluster.
const RIGHT_INNER_GAP: f32 = 4.0;
/// Spacing between semantic groups of the right tray cluster (T234):
/// time | status(net/battery/sound) | keyboard layout | mode | project.
const RIGHT_GROUP_GAP: f32 = 14.0;

/// Semantic group id for a right-section widget name. Drives two-level
/// spacing: 4px within a group, 14px between groups. `separator` (0) is a
/// forced break and is dropped from layout — T234 replaces dividers with
/// spacing. Everything not explicitly a stand-alone group falls into the
/// status cluster (1).
fn right_widget_group(name: &str) -> u8 {
    match name {
        "separator" => 0,
        "project" => 2,
        "workspace_mode" => 3,
        "keyboard_layout" => 4,
        "clock" => 5,
        _ => 1,
    }
}

/// Pure grouping of right-section widget names into semantic clusters,
/// preserving order. `separator` forces a break and is dropped. Testable
/// without a GPUI render context.
fn group_right_names(names: &[String]) -> Vec<Vec<String>> {
    let mut groups: Vec<Vec<String>> = Vec::new();
    let mut current: Option<(u8, Vec<String>)> = None;
    for name in names {
        let gid = right_widget_group(name);
        if gid == 0 {
            if let Some((_, g)) = current.take() {
                groups.push(g);
            }
            continue;
        }
        match &mut current {
            Some((cgid, g)) if *cgid == gid => g.push(name.clone()),
            Some((cgid, g)) => {
                let taken = std::mem::replace(g, vec![name.clone()]);
                *cgid = gid;
                groups.push(taken);
            }
            None => current = Some((gid, vec![name.clone()])),
        }
    }
    if let Some((_, g)) = current.take() {
        groups.push(g);
    }
    groups
}

/// Build the right-section container: inner groups (4px) laid out with 14px
/// between groups, pushed to the end. `separator` widgets are dropped.
fn right_section_div(widgets: Vec<(String, AnyElement)>) -> AnyElement {
    let filtered: Vec<(String, AnyElement)> = widgets
        .into_iter()
        .filter(|(n, _)| right_widget_group(n) != 0)
        .collect();
    let names: Vec<String> = filtered.iter().map(|(n, _)| n.clone()).collect();
    let groups = group_right_names(&names);
    let mut it = filtered.into_iter();
    div()
        .flex()
        .flex_1()
        .items_center()
        .justify_end()
        .gap(px(RIGHT_GROUP_GAP))
        .children(groups.into_iter().map(|g| {
            let mut els = Vec::with_capacity(g.len());
            for _ in &g {
                if let Some((_, el)) = it.next() {
                    els.push(el);
                }
            }
            div()
                .flex()
                .items_center()
                .gap(px(RIGHT_INNER_GAP))
                .children(els)
                .into_any_element()
        }))
        .into_any_element()
}

/// Returns window options for the bar on the given display.
///
/// Edge (anchor), fraction width, and floating margins are baked into the
/// open-time WindowOptions. The fork does not have live `set_anchor`/
/// `set_margin`, so changes to these fields trigger a window recreate
/// (T207: destroy + reopen — the bar reappears in <1 s, no process restart).
/// Height, exclusive zone, radius, and elevation apply live.
fn window_options(display_id: Option<DisplayId>, cx: &App) -> WindowOptions {
    let display_size = display_id
        .and_then(|id| cx.find_display(id))
        .map(|display| display.bounds().size)
        .unwrap_or_else(|| Size::new(px(1920.), px(1080.)));

    let appearance = layout_config::cached_appearance();
    let height = px(appearance.height);

    // Edge → anchor. For full-width bars we stretch horizontally (LEFT|RIGHT);
    // fraction/hug bars anchor to a single horizontal edge and use margins
    // for alignment (the compositor would stretch LEFT|RIGHT to display width,
    // rendering the fraction width ineffective).
    let is_full = appearance.width == BarWidth::Full;
    let edge_anchor = match appearance.edge {
        BarEdge::Bottom => Anchor::BOTTOM,
        _ => Anchor::TOP,
    };
    let anchor = if is_full {
        Anchor::LEFT | Anchor::RIGHT | edge_anchor
    } else {
        edge_anchor
    };

    let bar_width = match appearance.width {
        BarWidth::Fraction(f) => px(f32::from(display_size.width) * f),
        _ => display_size.width,
    };

    let exclusive_zone = if appearance.exclusive {
        Some(height)
    } else {
        Some(px(0.))
    };

    // Margins: for non-full bars, horizontal margins position the pill
    // (align: start → left margin 0; center → split the gap; end → right
    // margin 0). Floating bars add the user-configured margin on top.
    let margin = if is_full && !appearance.floating {
        None
    } else {
        let user_x = if appearance.floating {
            px(appearance.margin.x)
        } else {
            px(0.)
        };
        let user_y = if appearance.floating {
            px(appearance.margin.y)
        } else {
            px(0.)
        };
        let leftover_w = f32::from(display_size.width) - f32::from(bar_width);
        let (left_m, right_m) = if is_full {
            (user_x, user_x)
        } else {
            match appearance.align {
                appearance::BarAlign::Start => (user_x, px(leftover_w).max(user_x)),
                appearance::BarAlign::Center => {
                    let half = px((leftover_w / 2.0).max(0.0));
                    (half.max(user_x), half.max(user_x))
                }
                appearance::BarAlign::End => (px(leftover_w).max(user_x), user_x),
            }
        };
        Some((
            px(appearance.margin.y).max(user_y),  // top
            right_m,                                // right
            px(appearance.margin.y).max(user_y),  // bottom
            left_m,                                 // left
        ))
    };

    WindowOptions {
        display_id,
        titlebar: None,
        window_bounds: Some(WindowBounds::Windowed(Bounds {
            origin: point(px(0.), px(0.)),
            size: Size::new(bar_width, height),
        })),
        app_id: Some("chronos-bar".to_string()),
        window_background: WindowBackgroundAppearance::Transparent,
        kind: WindowKind::LayerShell(LayerShellOptions {
            namespace: "bar".to_string(),
            layer: Layer::Top,
            anchor,
            exclusive_zone,
            margin,
            keyboard_interactivity: KeyboardInteractivity::None,
            ..Default::default()
        }),
        ..Default::default()
    }
}

/// Screen-space horizontal extent `[x0, x1]` of the bar on a display of the
/// given width (px, left edge = 0). Powers the T217 panel junction: panels
/// decide whether their top edge sits under the bar by comparing against
/// this. Mirrors the margin/anchor math in `window_options` so the published
/// extent matches what the compositor actually places.
fn bar_screen_x_extent(display_w: f32, appearance: &BarAppearance) -> (f32, f32) {
    // Full-width, non-floating bar stretches to both screen edges → covers
    // every panel strip, so panels butt against it with square corners.
    if appearance.width == BarWidth::Full && !appearance.floating {
        return (0.0, display_w);
    }
    let bar_w = match appearance.width {
        // Floating full bar is inset by margin.x on both edges (the window
        // width is `Full`, but the compositor shrinks it by the margins).
        BarWidth::Full => (display_w - 2.0 * appearance.margin.x).max(0.0),
        BarWidth::Fraction(f) => display_w * f,
        BarWidth::Hug => display_w,
    };
    let leftover = (display_w - bar_w).max(0.0);
    let user_x = appearance.margin.x;
    let offset = if appearance.floating {
        // Floating bars inset every edge by margin.x on top of alignment.
        match appearance.align {
            appearance::BarAlign::Start => user_x,
            appearance::BarAlign::Center => (leftover / 2.0).max(user_x),
            appearance::BarAlign::End => leftover.max(user_x),
        }
    } else {
        match appearance.align {
            appearance::BarAlign::Start => user_x,
            appearance::BarAlign::Center => leftover / 2.0,
            appearance::BarAlign::End => leftover,
        }
    };
    (offset, offset + bar_w)
}

/// Publish the live bar geometry (height + radius + horizontal extent) to
/// `crate::state`. The height part preserves the T200 contract; radius and
/// extent drive the T217 panel junction. With no display enumerated yet the
/// safe default is published: full-width square bar (panels keep square
/// corners — pre-T217 chrome).
fn publish_bar_geometry(cx: &mut App) {
    let appearance = layout_config::cached_appearance();
    match crate::monitor::pult_display_id_or_primary(cx).and_then(|id| cx.find_display(id)) {
        Some(display) => {
            let display_w = f32::from(display.bounds().size.width);
            let (x0, x1) = bar_screen_x_extent(display_w, &appearance);
            crate::state::set_bar_geometry(appearance.radius, x0, x1);
        }
        None => crate::state::set_bar_geometry(0.0, 0.0, f32::INFINITY),
    }
    crate::state::set_bar_height_px(appearance.height);
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

/// Snapshot of the fields that require a window recreate when they change
/// (edge, width mode, align, floating, margin — no live `set_anchor`/
/// `set_margin` in the fork). Tracked so `apply_appearance` can decide:
/// live path vs destroy+reopen (T207).
#[derive(Debug, Clone, Copy, PartialEq)]
struct AnchorFields {
    edge: BarEdge,
    width: BarWidth,
    align: appearance::BarAlign,
    floating: bool,
    margin_x: f32,
    margin_y: f32,
}

impl AnchorFields {
    fn from_appearance(a: &appearance::BarAppearance) -> Self {
        Self {
            edge: a.edge,
            width: a.width,
            align: a.align,
            floating: a.floating,
            margin_x: a.margin.x,
            margin_y: a.margin.y,
        }
    }
}

static LAST_ANCHOR: OnceLock<Mutex<Option<AnchorFields>>> = OnceLock::new();

fn last_anchor() -> &'static Mutex<Option<AnchorFields>> {
    LAST_ANCHOR.get_or_init(|| Mutex::new(None))
}

/// Close the bar window (idempotent — no-op if already closed).
fn close_bar(cx: &mut App) {
    if let Some(handle) = bar_window().lock().unwrap_or_else(|e| e.into_inner()).take() {
        match handle.update(cx, |_, window: &mut Window, _| window.remove_window()) {
            Ok(()) => tracing::info!("bar: closed for recreate"),
            Err(e) => tracing::warn!("bar: close for recreate could not reach window ({e})"),
        }
    }
}

/// Live-apply `cached_appearance()` to the bar window. Height, exclusive
/// zone, and input region are live. Edge, width mode, floating, and margin
/// changes trigger a window recreate (destroy + reopen — the bar reappears
/// in <1 s, no process restart). Called from `layout_config::apply` on
/// every `bar.toml` change (300 ms debounce) and once after open.
/// Idempotent.
pub fn apply_appearance(cx: &mut App) {
    let appearance = layout_config::cached_appearance();
    let current_anchor = AnchorFields::from_appearance(&appearance);

    // Publish radius + horizontal extent for the panel junction first — the
    // decision only depends on the (sanitized) appearance + pult display,
    // not on the window being open yet.
    publish_bar_geometry(cx);

    // If anchor-dependent fields changed → destroy + reopen the window.
    // The fork has no live `set_anchor`/`set_margin`; this is the honest
    // cold-path (T207 product path).
    // If anchor-dependent fields changed → destroy + reopen the window.
    // The fork has no live `set_anchor`/`set_margin`; this is the honest
    // cold-path (T207 product path). Only update `last_anchor` on success
    // — a failed reopen leaves the old state so the next attempt retries.
    let needs_recreate = {
        let last = last_anchor().lock().unwrap_or_else(|e| e.into_inner());
        *last != Some(current_anchor)
    };
    if needs_recreate {
        let display_id = crate::monitor::pult_display_id_or_primary(cx);
        close_bar(cx);
        if open_on_display(display_id, cx) {
            *last_anchor().lock().unwrap_or_else(|e| e.into_inner()) = Some(current_anchor);
            tracing::info!(
                ?current_anchor,
                "bar: recreated window for anchor-dependent change"
            );
        } else {
            tracing::error!(
                "bar: failed to reopen after anchor change — bar will be missing until next restart"
            );
            return;
        }
    }

    let Some(handle) = *bar_window().lock().unwrap_or_else(|e| e.into_inner()) else {
        tracing::debug!("bar: no window yet, appearance apply deferred");
        return;
    };

    match handle.update(cx, |_bar, window, cx| {
        let current = window.bounds().size;
        window.resize(Size::new(current.width, px(appearance.height)));
        if appearance.exclusive {
            window.set_exclusive_zone(px(appearance.height));
        } else {
            window.set_exclusive_zone(px(0.));
        }
        // Pill-shaped bar (fraction or floating): limit input to the visible
        // content area so clicks outside the pill fall through to the desktop.
        // For non-stretched (fraction) bars the window IS the pill, so the
        // full-bounds region = visible area. For floating bars the compositor
        // positions the surface at the margin offset; the full surface is the
        // pill area, same logic applies.
        if !(appearance.width == BarWidth::Full && !appearance.floating) {
            window.set_input_region(Some(&[Bounds::new(
                point(px(0.), px(0.)),
                Size::new(current.width, px(appearance.height)),
            )]));
        } else {
            window.set_input_region(None);
        }
        cx.notify();
    }) {
        Ok(()) => {
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
    // Publish the configured geometry before any panel opens (strips open
    // ~50 ms after start; bar window at ~100 ms) — panels must see the
    // right gap and corner rule from the first frame.
    publish_bar_geometry(cx);

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
            // (resize/zone). Seeds LAST_ANCHOR so subsequent hot-reload
            // changes can detect anchor-dependent deltas.
            apply_appearance(cx);
        });
    })
    .detach();
}

#[cfg(test)]
mod tests {
    use super::*;
use chronos_ui::{Theme, WindowRootExt};

    #[test]
    fn elevation_none_has_no_shadow() {
        let theme = Theme::default();
        assert!(elevation_shadow(BarElevation::None, &theme).is_empty());
        assert_eq!(elevation_shadow(BarElevation::Soft, &theme).len(), 1);
        assert_eq!(elevation_shadow(BarElevation::Strong, &theme).len(), 2);
    }

    #[test]
    fn bar_screen_x_extent_full_non_floating_covers_display() {
        let a = BarAppearance {
            edge: BarEdge::Top,
            ..Default::default()
        };
        assert_eq!(bar_screen_x_extent(2560.0, &a), (0.0, 2560.0));
    }

    #[test]
    fn bar_screen_x_extent_fraction_centered() {
        let a = BarAppearance {
            width: BarWidth::Fraction(0.7),
            align: appearance::BarAlign::Center,
            ..Default::default()
        };
        // 2560 * 0.7 = 1792, leftover 768 → centered at x=384.
        assert_eq!(bar_screen_x_extent(2560.0, &a), (384.0, 2176.0));
    }

    #[test]
    fn bar_screen_x_extent_fraction_end_touches_right_edge() {
        let a = BarAppearance {
            width: BarWidth::Fraction(0.7),
            align: appearance::BarAlign::End,
            ..Default::default()
        };
        assert_eq!(bar_screen_x_extent(2560.0, &a), (768.0, 2560.0));
    }

    #[test]
    fn bar_screen_x_extent_floating_insets_every_edge() {
        let a = BarAppearance {
            width: BarWidth::Fraction(0.7),
            align: appearance::BarAlign::Center,
            floating: true,
            margin: appearance::BarMargin { x: 12.0, y: 8.0 },
            ..Default::default()
        };
        // leftover/2 = 384, max(user_x = 12) → 384. Bar ends at 384+1792.
        assert_eq!(bar_screen_x_extent(2560.0, &a), (384.0, 2176.0));
        // A center-float whose gap would push it under margin.x keeps the margin.
        let a = BarAppearance {
            width: BarWidth::Fraction(0.95),
            align: appearance::BarAlign::Center,
            floating: true,
            margin: appearance::BarMargin { x: 12.0, y: 8.0 },
            ..Default::default()
        };
        // leftover = 128, half = 64 ≥ 12 → still 64.
        assert_eq!(bar_screen_x_extent(2560.0, &a), (64.0, 2496.0));
    }

    // -- T234 right tray cluster grouping ------------------------------------

    fn names(s: &[&str]) -> Vec<String> {
        s.iter().map(|n| n.to_string()).collect()
    }

    #[test]
    fn group_right_breaks_on_semantic_change() {
        // Default config order: project | workspace_mode | volume | network |
        // keyboard_layout | tray | updates | system | notification_bell |
        // battery | clock (separators dropped).
        let g = group_right_names(&names(&[
            "project",
            "workspace_mode",
            "separator",
            "volume",
            "network",
            "keyboard_layout",
            "tray",
            "updates",
            "system",
            "notification_bell",
            "separator",
            "battery",
            "clock",
        ]));
        // 7 clusters: project / mode / net / layout / status / battery / clock.
        assert_eq!(g.len(), 7);
        assert_eq!(g[0], vec!["project"]);
        assert_eq!(g[1], vec!["workspace_mode"]);
        assert_eq!(g[2], vec!["volume", "network"]);
        assert_eq!(g[3], vec!["keyboard_layout"]);
        assert_eq!(
            g[4],
            vec!["tray", "updates", "system", "notification_bell"]
        );
        assert_eq!(g[5], vec!["battery"]);
        assert_eq!(g[6], vec!["clock"]);
    }

    #[test]
    fn group_right_drops_separators() {
        let g = group_right_names(&names(&["clock", "separator", "network"]));
        assert_eq!(g.len(), 2);
        assert_eq!(g[0], vec!["clock"]);
        assert_eq!(g[1], vec!["network"]);
    }

    #[test]
    fn group_right_merges_same_group_across_runs() {
        let g = group_right_names(&names(&["volume", "network", "battery"]));
        assert_eq!(g.len(), 1);
        assert_eq!(g[0], vec!["volume", "network", "battery"]);
    }
}
