//! Start menu — the second launcher surface (T265-H).
//!
//! The bar's Start button (was `launcher::toggle`) opens a compact menu that
//! shares the OSD launcher's data model: the same `applications` service,
//! `FuzzySearch` index, frecency, `launcher.toml` config and favorites/recents
//! resolution. It is NOT a second launcher state — the view reuses the pure
//! model modules (`search`, `favorites`, `grid`, `launch`, `system_actions`).
//!
//! Layer: the menu is a **Layer::Overlay** surface, not an `AnchoredPopup`.
//! A popup parented to the Top bar renders in the Top layer and gets covered
//! by the Overlay side panels; Overlay is the only layer above them. On open
//! we leave the left panel open (menu must paint over it) and close the
//! right panel only when its geometry would intersect the menu — no two
//! Overlay surfaces may fight over one rectangle. `grab` does not exist for a
//! layer surface; dismissal is ours: click-away (shared click-catcher), Esc,
//! re-click Start, or launch.

pub mod view;

use std::rc::Rc;

use gpui::{
    AnyWindowHandle, App, Bounds, DisplayId, Global, Pixels, Size, Window,
    WindowBackgroundAppearance, WindowBounds, WindowHandle, WindowKind, WindowOptions,
    layer_shell::*, point, prelude::*, px,
};

use gpui_component::Root;
use gpui_component::input::InputState;

use crate::start_menu::view::StartMenuView;

/// Menu width — mockup `min(720px, 94vw)` (fixed; the pult is wide enough).
pub(crate) const START_MENU_WIDTH: f32 = 720.;
/// Menu height — fixed; the app grid scrolls inside.
pub(crate) const START_MENU_HEIGHT: f32 = 520.;
/// Flush to the output's left edge. The HTML mockup's 16px is stage padding
/// (Windows-bottom scene), not Hyprland geometry.
pub(crate) const START_MENU_MARGIN_LEFT: f32 = 0.;

/// Tracks the open start-menu window + its transparent click-catcher.
#[derive(Default)]
pub struct StartMenuState {
    handle: Option<WindowHandle<Root>>,
    click_catcher: Option<AnyWindowHandle>,
}

impl Global for StartMenuState {}

fn pick_display(cx: &App) -> Option<DisplayId> {
    crate::monitor::pult_display_id_or_primary(cx)
}

/// The menu's on-screen bounds. Deterministic: TOP|LEFT anchored with an
/// explicit margin and the `-1` exclusive-zone escape hatch (same contract as
/// `side_panel_left::content_window_options`), so the bar's exclusive zone
/// does NOT double-offset it. Top == live bar height, left == `START_MENU_MARGIN_LEFT`.
fn menu_bounds(cx: &App) -> Bounds<Pixels> {
    Bounds::new(
        point(px(START_MENU_MARGIN_LEFT), px(crate::state::bar_height_px())),
        Size::new(px(START_MENU_WIDTH), px(START_MENU_HEIGHT)),
    )
}

/// Pure overlap test for the right panel: does a panel of `panel_width` px
/// sitting at the right edge of a `display_width`-wide output reach the
/// menu's right edge (`menu_right`)?
fn right_panel_overlaps(display_width: f32, panel_width: f32, menu_right: f32) -> bool {
    panel_width > (display_width - menu_right)
}

/// Whether the open right panel would intersect the menu (narrow outputs).
/// Rail-only (width <= `RAIL_ONLY_WIDTH`) never overlaps a 736px menu on any
/// realistic output; an open content panel can.
fn right_panel_intersects_menu(cx: &App) -> bool {
    let Some(display) = crate::monitor::pult_display_info(cx) else {
        return false;
    };
    let display_w = f32::from(display.bounds().size.width);
    let state = cx.global::<crate::side_panel_right::SidePanelRightState>();
    right_panel_overlaps(
        display_w,
        state.width,
        START_MENU_MARGIN_LEFT + START_MENU_WIDTH,
    )
}

fn window_options(display_id: Option<DisplayId>) -> WindowOptions {
    WindowOptions {
        display_id,
        titlebar: None,
        window_bounds: Some(WindowBounds::Windowed(Bounds {
            origin: point(px(0.), px(0.)),
            size: Size::new(px(START_MENU_WIDTH), px(START_MENU_HEIGHT)),
        })),
        app_id: Some("chronos-start-menu".to_string()),
        window_background: WindowBackgroundAppearance::Transparent,
        kind: WindowKind::LayerShell(LayerShellOptions {
            namespace: "chronos-start-menu".to_string(),
            layer: Layer::Overlay,
            anchor: Anchor::TOP | Anchor::LEFT,
            // `-1` opts out of the bar's automatic top offset so the explicit
            // margin below is authoritative (side-panel content contract).
            exclusive_zone: Some(px(-1.)),
            margin: Some((
                px(crate::state::bar_height_px()),
                px(0.),
                px(0.),
                px(START_MENU_MARGIN_LEFT),
            )),
            // OnDemand: the search field needs keyboard; Exclusive is forbidden
            // (wedges Hyprland's input stack — T264 class). The compositor only
            // grants a seat after a click, so typing starts after the user
            // clicks into the field (or the surface, per compositor policy).
            keyboard_interactivity: KeyboardInteractivity::OnDemand,
            ..Default::default()
        }),
        ..Default::default()
    }
}

/// Open the transparent full-output click-catcher with a hole over the menu.
fn open_click_catcher(cx: &mut App) -> anyhow::Result<AnyWindowHandle> {
    let display = crate::monitor::pult_display_info(cx)
        .ok_or_else(|| anyhow::anyhow!("no display for start-menu click-catcher"))?;
    let output_size = display.bounds().size;
    let hole = menu_bounds(cx);
    crate::popup_click_catcher::open(
        cx,
        Some(display.id()),
        output_size,
        crate::popup_click_catcher::outside_input_regions(output_size, hole),
        Rc::new(|window, cx| close_from_click_catcher(window, cx)),
    )
}

/// Open the start menu. No-op if already open.
pub fn open(cx: &mut App) {
    if cx.global::<StartMenuState>().handle.is_some() {
        return;
    }

    // Singleton with the Sound/Calendar popups: they anchor to the bar's
    // top-right corner and carry their own full-output click-catchers —
    // leave none of them stacked under the Overlay menu (or each other).
    crate::volume_popup::close(cx);
    crate::calendar_popup::close(cx);

    // Do not close the left panel: the menu is Overlay and must sit *on top*
    // of it (owner errata). The right panel still closes when geometries
    // would intersect — two Overlay surfaces in one rectangle is the
    // remaining z-fight we refuse to guess.
    if right_panel_intersects_menu(cx) {
        crate::side_panel_right::close(cx);
    }

    let click_catcher = open_click_catcher(cx).ok();
    let display_id = pick_display(cx);

    let result = cx.open_window(window_options(display_id), |window, cx| {
        let input = cx.new(|cx| InputState::new(window, cx));
        input.update(cx, |s, cx| {
            s.set_placeholder("Search applications…", window, cx);
        });
        let entity = cx.new(|cx| StartMenuView::new(cx, input.clone(), window));
        // Focus the field internally so typing works as soon as the compositor
        // grants the OnDemand seat (click into the surface).
        input.update(cx, |s, cx| s.focus(window, cx));
        cx.new(|cx| {
            Root::new(entity, window, cx)
                .bordered(false)
                .bg(gpui::transparent_black())
        })
    });

    match result {
        Ok(handle) => {
            let state = cx.global_mut::<StartMenuState>();
            state.handle = Some(handle);
            state.click_catcher = click_catcher;
        }
        Err(err) => {
            if let Some(catcher) = click_catcher {
                let _ = catcher.update(cx, |_, window, _| window.remove_window());
            }
            tracing::warn!("start_menu: failed to open: {err}");
        }
    }
}

/// Close the menu from outside its callbacks (Start button toggle). Uses
/// `handle.update`; removes both the menu and the click-catcher.
pub fn close(cx: &mut App) {
    let (handle, catcher) = {
        let state = cx.global_mut::<StartMenuState>();
        (state.handle.take(), state.click_catcher.take())
    };
    if let Some(handle) = handle {
        let _ = handle.update(cx, |_, window: &mut Window, _| window.remove_window());
    }
    if let Some(catcher) = catcher {
        let _ = catcher.update(cx, |_, window, _| window.remove_window());
    }
}

/// Close from inside the menu's own callback (Esc / launch). Direct
/// `remove_window` on the live reference — never re-entrant `handle.update`.
pub(crate) fn close_this(window: &mut Window, cx: &mut App) {
    let this = window.window_handle();
    let catcher = {
        let state = cx.global_mut::<StartMenuState>();
        if state.handle.as_ref().map(|h| **h == this).unwrap_or(false) {
            state.handle.take();
        }
        state.click_catcher.take()
    };
    if let Some(catcher) = catcher {
        let _ = catcher.update(cx, |_, catcher_window, _| catcher_window.remove_window());
    }
    window.remove_window();
}

/// Close from the transparent click-catcher's own callback.
fn close_from_click_catcher(window: &mut Window, cx: &mut App) {
    let this = window.window_handle();
    let (menu, catcher) = {
        let state = cx.global_mut::<StartMenuState>();
        (state.handle.take(), state.click_catcher.take())
    };
    if let Some(menu) = menu {
        let _ = menu.update(cx, |_, menu_window, _| menu_window.remove_window());
    }
    if catcher == Some(this) {
        window.remove_window();
    } else if let Some(catcher) = catcher {
        let _ = catcher.update(cx, |_, catcher_window, _| catcher_window.remove_window());
    }
}

/// Start-button toggle. Caller's window is the bar, not the menu → `close`.
pub fn toggle(cx: &mut App) {
    if cx.global::<StartMenuState>().handle.is_some() {
        close(cx);
    } else {
        open(cx);
    }
}

/// Register the start-menu global. Called once from `main.rs`.
pub fn init(cx: &mut App) {
    cx.set_global(StartMenuState::default());
    tracing::info!("start_menu: initialized");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn menu_dimensions_reasonable() {
        assert!(START_MENU_WIDTH > 400.0 && START_MENU_WIDTH < 900.0);
        assert!(START_MENU_HEIGHT > 300.0 && START_MENU_HEIGHT < 700.0);
        assert!(START_MENU_MARGIN_LEFT >= 0.0);
    }

    #[test]
    fn right_panel_overlaps_only_when_it_reaches_menu() {
        let menu_right = START_MENU_MARGIN_LEFT + START_MENU_WIDTH; // 720
        // 2560-wide output: a 960px right panel starts at x=1600 — no overlap.
        assert!(!right_panel_overlaps(2560.0, 960.0, menu_right));
        assert!(!right_panel_overlaps(2560.0, 40.0, menu_right));
        // 1366-wide output: a 960px panel starts at x=406 < 736 — overlap.
        assert!(right_panel_overlaps(1366.0, 960.0, menu_right));
        // 1366-wide output with rail-only panel: 40px at x=1326 — no overlap.
        assert!(!right_panel_overlaps(1366.0, 40.0, menu_right));
    }
}
