//! Tray context-menu popup (DBusMenu).
//!
//! A layer-shell popup anchored TOP|RIGHT that renders the live `MenuNode`
//! tree fetched from a tray item's `com.canonical.dbusmenu` interface.
//!
//! Design mirrors `notifications/`/`osd/`:
//!   * `TrayMenuState` — GPUI global: which service's menu is open, the
//!     fetched `Vec<MenuNode>`, the open window handle, and a generation
//!     token for the auto-close timer.
//!   * `TrayMenuWatcher` — tiny entity hosting `state::watch()` on the tray
//!     service snapshot, so when `FetchMenu` lands the popup repaints.
//!   * `open`/`close`/`toggle` — imperative control from the bar widget's
//!     right-click handler.
//!
//! Anchor TOP|RIGHT (tray lives top-right), margin ~36px below the bar.
//! `KeyboardInteractivity::None` (no Escape handling — rare popup, mouse
//! driven) and **never** Exclusive (popups must not reserve compositor
//! space). We use `remove_window` on close: popups are rare, so reusing the
//! surface (the OSD soft-hide trick) buys nothing and risks the
//! empty-transparent-click captures the task below — a real window that
//! closes cleanly is the correct model here.

pub mod view;

use std::{rc::Rc, time::Duration};

use gpui::{
    AnyWindowHandle, App, AsyncApp, Bounds, Context, DisplayId, Entity, Global, Pixels, Size,
    Subscription, WeakEntity, Window, WindowBackgroundAppearance, WindowBounds, WindowHandle,
    WindowId, WindowKind,
    WindowOptions, layer_shell::*,
    point,
    popup::{
        PopupAnchor, PopupConstraintAdjustment, PopupGravity, PopupNotSupportedError, PopupOptions,
    },
    prelude::*,
    px,
};

use gpui_component::Root;

use chronos_services::{MenuNode, Service, TrayCommand};

use crate::state::{self, AppState};
use crate::tray_menu::view::TrayMenuView;

/// Card width bounds (px) — canon `min-width:230px; max-width:300px`. The
/// surface is usually WIDER than the card: `estimate_menu_width` reserves
/// room for the widest submenu chain so the component's side-by-side
/// submenu never clips against the surface edge (T263 submenu
/// widest-reserve). The surface outside the card is transparent; clicks
/// there hit the component's `on_mouse_down_out` → `DismissEvent` →
/// `close_this` (client-side close, no compositor grab — T264).
pub(crate) const MENU_MIN_W: f32 = 230.;
pub(crate) const MENU_MAX_W: f32 = 300.;
/// Top + right margin (px) so the card sits just below the bar's top edge.
const MENU_MARGIN_TOP: f32 = 36.;
const MENU_MARGIN_RIGHT: f32 = 8.;
/// Auto-close delay after the last open (generation-guarded).
const AUTO_CLOSE_AFTER: Duration = Duration::from_secs(15);
/// Per-row geometry (px): vertical padding + one line of label text.
const ROW_H: f32 = 30.;
/// Floor so a single short menu still has a usable popup height.
const MIN_MENU_H: f32 = 28.;
/// Absolute cap when no display is reachable (shouldn't happen live; the
/// design cap is viewport-relative — `calc(100vh - 16px)` — via the
/// `display_h` argument, see [`estimate_menu_height`]).
const MAX_MENU_H: f32 = 480.;
/// Absolute width cap (px) when no display is reachable — mirror of
/// `MAX_MENU_H`: root card + two nested submenu cards + anchor slack.
const MAX_MENU_W: f32 = 920.;
/// Design `max-height: calc(100vh - 16px)` — breathing margin below the
/// viewport so a max-height menu never touches the screen edge.
const DISPLAY_V_MARGIN: f32 = 16.;
/// Horizontal mirror of `DISPLAY_V_MARGIN` — clamps the widest-reserve
/// surface to the viewport.
const DISPLAY_H_MARGIN: f32 = 16.;
/// Sticky header height (px) — design `.ctx-head`: 14px icon + 4px/8px
/// padding + 1px border-bottom ≈ 30px. Only paid when the menu has a head.
const HEAD_H: f32 = 30.;
/// Items-column chrome around the widest row (px): the component's `items`
/// column `p_1` (4+4) + the card border (1+1).
const CARD_PAD_W: f32 = 10.;
/// Row horizontal padding (px): component `MenuItemElement` `px(8)` on both
/// sides.
const ROW_PAD_W: f32 = 16.;
/// Fixed leading gutter (px) for icon/radio rows — canon `.ci-ic` width.
pub(crate) const GUTTER_W: f32 = 16.;
/// Horizontal gap between gutter, label and shortcut (px).
pub(crate) const ROW_GAP: f32 = 8.;
/// Trailing chevron on submenu rows (px) — component `ChevronRight.xsmall()`.
const SUBMENU_CARET_W: f32 = 12.;
/// The component anchors a submenu 8px INTO the parent card
/// (`left = bounds.width - px(8.)`), so each reserved level must cover that
/// overlap on top of the submenu card's own width.
const SUBMENU_ANCHOR_OVERLAP: f32 = 8.;
/// Per-char advance estimate (px) for a 14px sans label — deterministic
/// stand-in for text-system measurement (there is no text system before the
/// window exists). Deliberately generous (≈0.57em): over-reserving only
/// adds transparent surface, under-reserving pushes a submenu past the
/// surface edge.
const LABEL_ADVANCE: f32 = 8.;
/// Per-char advance estimate (px) for the 12px mono shortcut glyph — mono
/// ≈0.6em plus slack for the wide modifier glyphs (⌃⇧⌥).
const SHORTCUT_ADVANCE: f32 = 9.;

/// Count visible root rows. Submenu children render in a separate anchored
/// overlay and must not inflate the root popup's surface height.
fn count_visible(nodes: &[MenuNode]) -> usize {
    nodes.iter().filter(|n| n.visible).count()
}

/// Height of the pult display, if reachable — drives the design's
/// viewport-relative menu cap (`calc(100vh - 16px)`).
fn pult_display_height(cx: &gpui::App) -> Option<f32> {
    crate::monitor::pult_display_info(cx).map(|d| f32::from(d.bounds().size.height))
}

/// Width of the pult display, if reachable — clamps the widest-reserve
/// surface to the viewport (mirror of [`pult_display_height`]).
fn pult_display_width(cx: &gpui::App) -> Option<f32> {
    crate::monitor::pult_display_info(cx).map(|d| f32::from(d.bounds().size.width))
}

/// Estimate the popup height (px) from the current menu tree.
///
/// Cap is `min(rows * ROW_H + head, display_h − 16)` per design
/// `max-height: calc(100vh - 16px)`; the inner scroll-guard takes over past
/// that. `MAX_MENU_H` only remains as a fallback when the display is
/// unreachable. `has_head` adds the sticky header row (`.ctx-head`).
fn estimate_menu_height(nodes: &[MenuNode], display_h: Option<f32>, has_head: bool) -> f32 {
    let rows = count_visible(nodes);
    let head = if has_head { HEAD_H } else { 0.0 };
    if rows == 0 {
        // Placeholder ("…") state — keep a small surface.
        return MIN_MENU_H + head;
    }
    let est = rows as f32 * ROW_H + head;
    match display_h {
        Some(h) => est.clamp(MIN_MENU_H, (h - DISPLAY_V_MARGIN).max(MIN_MENU_H)),
        None => est.clamp(MIN_MENU_H, MAX_MENU_H),
    }
}

/// Estimated content width (px) of one visible row — mirrors the view's row
/// composition: leading gutter (custom rows always render the gutter box;
/// native rows reserve it via the `Icon::empty()` shim when the level has
/// any gutter row), 14px sans label (`whitespace-nowrap`; an empty label
/// renders the "…" placeholder), mono shortcut glyph on leaf rows, trailing
/// chevron on submenu rows. Separators carry no text width. Pure and
/// deterministic — no text system exists before the window is created.
fn row_content_width(node: &MenuNode, level_has_gutter: bool) -> f32 {
    if node.separator {
        return 0.;
    }
    let is_submenu = !node.children.is_empty();
    // Proxy for the view's `any_gutter_row` (which resolves icon themes —
    // I/O a pure estimate must not do): an unresolved icon over-reserves
    // the gutter, which only costs transparent pixels.
    let gutter = if node.icon_name.is_some()
        || node.toggle.is_some()
        || node.shortcut.is_some()
        || level_has_gutter
    {
        GUTTER_W + ROW_GAP
    } else {
        0.
    };
    let label_chars = if node.label.is_empty() {
        1 // renders as the "…" placeholder
    } else {
        node.label.chars().count()
    };
    // Submenu rows render label + chevron only (the view's `append_node`
    // returns before the shortcut branch when children exist).
    let shortcut_w = if is_submenu {
        0.
    } else {
        node.shortcut
            .as_deref()
            .and_then(view::shortcut_to_glyph)
            .map(|g| ROW_GAP + g.chars().count() as f32 * SHORTCUT_ADVANCE)
            .unwrap_or(0.)
    };
    let caret_w = if is_submenu {
        ROW_GAP + SUBMENU_CARET_W
    } else {
        0.
    };
    ROW_PAD_W + gutter + label_chars as f32 * LABEL_ADVANCE + shortcut_w + caret_w
}

/// Canon card width (px) of one menu level: widest row + column chrome,
/// clamped to `min-width:230 / max-width:300` — the same bounds the
/// component enforces at render (`min_w`/`max_w`). The sticky head row is
/// excluded on purpose: its title is `min_w(0).overflow_hidden()` — it
/// clips instead of widening the card.
fn level_card_width(nodes: &[MenuNode]) -> f32 {
    let level_has_gutter = nodes.iter().any(|n| {
        n.visible && !n.separator && (n.icon_name.is_some() || n.toggle.is_some())
    });
    let widest = nodes
        .iter()
        .filter(|n| n.visible)
        .map(|n| row_content_width(n, level_has_gutter))
        .fold(0., f32::max);
    (CARD_PAD_W + widest).clamp(MENU_MIN_W, MENU_MAX_W)
}

/// Reserve (px) for the widest open submenu chain beyond the root card.
/// Only one submenu chain is open at a time and each level sits inside the
/// surface next to its parent card, so the reserve is the max over
/// root→leaf submenu paths of the summed level card widths (+ the
/// component's anchor overlap per level). Zero without submenus — the
/// surface then equals the root card width.
fn submenu_chain_reserve(nodes: &[MenuNode]) -> f32 {
    nodes
        .iter()
        .filter(|n| n.visible && !n.separator && !n.children.is_empty())
        .map(|n| {
            level_card_width(&n.children)
                + SUBMENU_ANCHOR_OVERLAP
                + submenu_chain_reserve(&n.children)
        })
        .fold(0., f32::max)
}

/// Estimate the popup surface width (px) from the full menu tree — the
/// widest-reserve half of the T263 submenu decision: the layout is fetched
/// whole before the popup shows, so the surface is sized up front with room
/// for the widest submenu chain. Pure + display-clamped, mirroring
/// [`estimate_menu_height`].
fn estimate_menu_width(nodes: &[MenuNode], display_w: Option<f32>) -> f32 {
    let surface = level_card_width(nodes) + submenu_chain_reserve(nodes);
    match display_w {
        Some(w) => surface.min((w - DISPLAY_H_MARGIN).max(MENU_MIN_W)),
        None => surface.min(MAX_MENU_W),
    }
}

/// Global state for the tray context-menu popup.
#[derive(Default)]
pub struct TrayMenuState {
    /// Window handle while a menu is open; `None` when closed. The window
    /// root is a `gpui_component::Root` (component widgets panic on
    /// `window.root()` otherwise).
    handle: Option<WindowHandle<Root>>,
    /// Weak handle to the live `TrayMenuView`, so the watcher can repaint it
    /// (which rebuilds the `PopupMenu` when `FetchMenu` lands).
    view: Option<WeakEntity<TrayMenuView>>,
    /// Watcher entity driving repaints on tray snapshot changes.
    watcher: Option<Entity<TrayMenuWatcher>>,
    /// Clears stale popup state when the compositor closes an xdg-popup via
    /// `PopupDone` (outside click), bypassing our explicit close paths.
    window_closed_subscription: Option<Subscription>,
    /// Transparent layer-surface that receives clicks outside the popup while
    /// the native popup intentionally has `grab: false` (T264).
    click_catcher: Option<AnyWindowHandle>,
    /// The service whose menu is currently open (its `TrayItem.id`).
    open_service: Option<String>,
    /// Fetched menu tree for `open_service`.
    nodes: Vec<MenuNode>,
    /// Bumped on every open/close so stale auto-close timers no-op.
    close_generation: u64,
}

impl Global for TrayMenuState {}

impl TrayMenuState {
    /// The service whose menu is currently shown, if any.
    pub fn open_service(&self) -> Option<&str> {
        self.open_service.as_deref()
    }

    /// The fetched menu tree (read-only, for the view).
    pub fn nodes(&self) -> &[MenuNode] {
        &self.nodes
    }
}

/// Tiny entity that hosts the `state::watch()` subscription. It has no state
/// of its own — `watch` needs an entity/Context to spawn its update loop.
pub struct TrayMenuWatcher {}

fn pick_display(cx: &App) -> Option<DisplayId> {
    crate::monitor::pult_display(cx)
}

fn open_click_catcher(
    cx: &mut App,
    anchor_rect: Bounds<Pixels>,
    width: f32,
    height: f32,
) -> anyhow::Result<AnyWindowHandle> {
    crate::popup_click_catcher::open_for_popup(
        cx,
        anchor_rect,
        Size::new(px(width), px(height)),
        Rc::new(|window, cx| close_from_click_catcher(window, cx)),
    )
}

/// Layer-shell window options for the menu popup: TOP | RIGHT, overlay,
/// never exclusive, `OnDemand` keyboard (the canon drives the menu with
/// `navIdx`/`paintNav` — arrow/enter/escape navigation is required, and
/// `None` would swallow every key). Fixed-corner fallback when `AnchoredPopup`
/// isn't supported on this platform (mirrors `volume_popup`).
fn fallback_window_options(display_id: Option<DisplayId>, width: f32, height: f32) -> WindowOptions {
    WindowOptions {
        display_id,
        titlebar: None,
        window_bounds: Some(WindowBounds::Windowed(Bounds {
            origin: point(px(0.), px(0.)), size: Size::new(px(width), px(height)),
        })),
        app_id: Some("chronos-tray-menu".to_string()),
        window_background: WindowBackgroundAppearance::Transparent,
        kind: WindowKind::LayerShell(LayerShellOptions {
            namespace: "tray-menu".to_string(),
            layer: Layer::Overlay,
            anchor: Anchor::TOP | Anchor::RIGHT,
            exclusive_zone: None,
            margin: Some((px(MENU_MARGIN_TOP), px(MENU_MARGIN_RIGHT), px(0.), px(0.))),
            keyboard_interactivity: KeyboardInteractivity::OnDemand,
            ..Default::default()
        }),
        ..Default::default()
    }
}

/// Anchored popup — positioned at the tray icon's bounds (the canon
/// `positionRoot` opens at the click point and clamps to the viewport).
/// Gravity: below-and-right of the icon's bottom-left corner; the
/// constraint flags slide/flip when the icon sits near a screen edge so the
/// menu never lands off-screen (canon 8px clamp).
fn window_options(
    anchor_rect: Bounds<Pixels>,
    parent: AnyWindowHandle,
    width: f32,
    height: f32,
) -> WindowOptions {
    WindowOptions {
        titlebar: None,
        window_bounds: Some(WindowBounds::Windowed(Bounds {
            origin: point(px(0.), px(0.)), size: Size::new(px(width), px(height)),
        })),
        app_id: Some("chronos-tray-menu".to_string()),
        window_background: WindowBackgroundAppearance::Transparent,
        kind: WindowKind::AnchoredPopup(PopupOptions {
            parent,
            anchor_rect,
            anchor: PopupAnchor::BottomLeft,
            gravity: PopupGravity::BottomRight,
            constraint_adjustment: PopupConstraintAdjustment::SLIDE_X
                | PopupConstraintAdjustment::SLIDE_Y
                | PopupConstraintAdjustment::FLIP_X
                | PopupConstraintAdjustment::FLIP_Y,
            offset: point(px(0.), px(4.)),
            // T264: Hyprland 0.56.x can retain this xdg-popup's seat grab
            // after client-side destruction and then drop compositor-wide
            // pointer input. PopupMenu still dismisses on click-away/Escape,
            // so this safety-critical menu must not request a compositor grab.
            grab: false,
        }),
        ..Default::default()
    }
}

/// Open (or switch to) the menu for `service`. Fetches the tree via
/// `FetchMenu` and opens the popup anchored to the tray icon's bounds
/// (falls back to a fixed-corner LayerShell popup when `AnchoredPopup`
/// isn't supported on this platform).
pub fn open(cx: &mut App, anchor_rect: Bounds<Pixels>, parent: AnyWindowHandle, service: String) {
    // Re-fetch the menu (idempotent; cheap) so stale trees don't linger.
    AppState::tray(cx).dispatch(TrayCommand::FetchMenu {
        service: service.clone(),
    });

    // Snapshot the freshly-fetched (or cached) tree for the service.
    let nodes = AppState::tray(cx)
        .get()
        .find(&service)
        .and_then(|item| item.menu.clone())
        .unwrap_or_default();

    // The tray item's non-empty title (if any) drives the sticky header row.
    let has_head = AppState::tray(cx)
        .get()
        .find(&service)
        .map(|item| item.title.as_deref().map(|t| !t.is_empty()).unwrap_or(false))
        .unwrap_or(false);

    let generation = {
        let state = cx.global_mut::<TrayMenuState>();
        state.open_service = Some(service);
        state.nodes = nodes;
        state.close_generation = state.close_generation.wrapping_add(1);
        state.close_generation
    };

    let handle = cx.global::<TrayMenuState>().handle.clone();
    let display_h = pult_display_height(cx);
    let display_w = pult_display_width(cx);
    match handle {
        Some(existing) => {
            let width = estimate_menu_width(&cx.global::<TrayMenuState>().nodes, display_w);
            let height =
                estimate_menu_height(&cx.global::<TrayMenuState>().nodes, display_h, has_head);
            let _ = existing.update(cx, |_, window: &mut gpui::Window, _| {
                window.resize(Size::new(px(width), px(height)));
            });
            let view = cx.global::<TrayMenuState>().view.clone();
            if let Some(view) = view.and_then(|view| view.upgrade()) {
                let _ = view.update(cx, |_, view_cx| view_cx.notify());
            }
        }
        None => {
            let width = estimate_menu_width(&cx.global::<TrayMenuState>().nodes, display_w);
            let height =
                estimate_menu_height(&cx.global::<TrayMenuState>().nodes, display_h, has_head);
            // The window root MUST be a component `Root` — the hosted
            // `PopupMenu` (and any component widget inside it) panics on
            // `window.root()` otherwise. `bg(transparent)` keeps the rounded
            // card corners showing the desktop instead of Root's solid fill.
            let click_catcher = open_click_catcher(cx, anchor_rect, width, height).ok();
            let mut opened_view: Option<WeakEntity<TrayMenuView>> = None;
            let mut open = |cx: &mut App, options: WindowOptions| {
                cx.open_window(options, |window, view_cx| {
                    let view = view_cx.new(|view_cx| TrayMenuView::new(view_cx));
                    opened_view = Some(view.downgrade());
                    view_cx.new(|view_cx| {
                        Root::new(view, window, view_cx)
                            .bordered(false)
                            .bg(gpui::transparent_black())
                    })
                })
            };
            let result = match open(cx, window_options(anchor_rect, parent, width, height)) {
                Err(err) => {
                    if let Some(catcher) = click_catcher {
                        let _ = catcher.update(cx, |_, window, _| window.remove_window());
                    }
                    if err.downcast_ref::<PopupNotSupportedError>().is_some() {
                        tracing::warn!(
                            "tray_menu: AnchoredPopup not supported on this platform, falling back to fixed-corner LayerShell"
                        );
                        let display_id = pick_display(cx);
                        open(cx, fallback_window_options(display_id, width, height))
                    } else {
                        Err(err)
                    }
                }
                ok => ok,
            };
            match result {
                Ok(new_handle) => {
                    let window_id = new_handle.window_id();
                    let window_closed_subscription = cx.on_window_closed(move |cx, closed_id| {
                        let tracked = cx
                            .global::<TrayMenuState>()
                            .handle
                            .as_ref()
                            .map(|handle| handle.window_id());
                        if closed_id != window_id || !window_closed_is_tracked(tracked, closed_id) {
                            return;
                        }
                        let catcher = {
                            let state = cx.global_mut::<TrayMenuState>();
                            state.handle = None;
                            state.view = None;
                            state.open_service = None;
                            state.nodes.clear();
                            state.close_generation = state.close_generation.wrapping_add(1);
                            state.window_closed_subscription = None;
                            state.click_catcher.take()
                        };
                        if let Some(catcher) = catcher {
                            let _ = catcher.update(cx, |_, window, _| window.remove_window());
                        }
                    });
                    let view = opened_view.clone();
                    {
                        let state = cx.global_mut::<TrayMenuState>();
                        state.handle = Some(new_handle);
                        state.view = opened_view;
                        state.window_closed_subscription = Some(window_closed_subscription);
                        state.click_catcher = click_catcher;
                    }
                    // `FetchMenu` may have landed while `open_window` was
                    // creating the surface, before `handle/view` were stored.
                    // In that race the watcher updated `nodes` but could not
                    // notify the view. Close the race explicitly here.
                    if let Some(view) = view.and_then(|view| view.upgrade()) {
                        let _ = view.update(cx, |_, view_cx| view_cx.notify());
                    }
                }
                Err(err) => tracing::warn!("tray_menu: failed to open popup: {err}"),
            }
        }
    }

    schedule_autoclose(cx, generation);
}

/// Close the popup (clears state + destroys the window).
pub fn close(cx: &mut App) {
    let (popup, catcher) = {
        let state = cx.global_mut::<TrayMenuState>();
        state.open_service = None;
        state.nodes = Vec::new();
        state.close_generation = state.close_generation.wrapping_add(1);
        state.window_closed_subscription = None;
        state.view = None;
        (state.handle.take(), state.click_catcher.take())
    };
    if let Some(handle) = popup {
        let _ = handle.update(cx, |_, window: &mut gpui::Window, _| window.remove_window());
    }
    if let Some(handle) = catcher {
        let _ = handle.update(cx, |_, window, _| window.remove_window());
    }
}

/// Close the popup from inside a callback that already holds `&mut Window`
/// for this popup's window-id. A blind `close(cx)` would re-enter
/// `handle.update` on the same id, which silently fails while the callback is
/// running (the window slot is empty during dispatch), leaving a ghost popup.
/// So clear the tracked handle and call `remove_window()` on the live
/// reference directly — the pattern `launcher` already uses (Cline, №8).
pub(crate) fn close_this(window: &mut Window, cx: &mut App) {
    let this = window.window_handle();
    let tracked = cx
        .global::<TrayMenuState>()
        .handle
        .as_ref()
        .map(|h| **h == this)
        .unwrap_or(false);
    let catcher = {
        let state = cx.global_mut::<TrayMenuState>();
        if tracked {
            state.handle.take(); // clear BEFORE remove
            state.view = None;
            state.window_closed_subscription = None;
        }
        state.click_catcher.take()
    };
    let state = cx.global_mut::<TrayMenuState>();
    state.open_service = None;
    state.nodes = Vec::new();
    state.close_generation = state.close_generation.wrapping_add(1);
    if let Some(catcher) = catcher {
        let _ = catcher.update(cx, |_, catcher_window, _| catcher_window.remove_window());
    }
    window.remove_window(); // direct, no reentrant handle.update
}

/// Close both surfaces from the transparent click-catcher's own callback.
pub(crate) fn close_from_click_catcher(window: &mut Window, cx: &mut App) {
    let this = window.window_handle();
    let (popup, catcher) = {
        let state = cx.global_mut::<TrayMenuState>();
        state.open_service = None;
        state.nodes.clear();
        state.close_generation = state.close_generation.wrapping_add(1);
        state.window_closed_subscription = None;
        state.view = None;
        (state.handle.take(), state.click_catcher.take())
    };
    if let Some(popup) = popup {
        let _ = popup.update(cx, |_, popup_window, _| popup_window.remove_window());
    }
    if catcher == Some(this) {
        window.remove_window();
    } else if let Some(catcher) = catcher {
        let _ = catcher.update(cx, |_, catcher_window, _| catcher_window.remove_window());
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ToggleAction {
    Close,
    OpenFresh,
}

fn toggle_action(open_service: Option<&str>, requested_service: &str) -> ToggleAction {
    match open_service {
        Some(open) if open == requested_service => ToggleAction::Close,
        _ => ToggleAction::OpenFresh,
    }
}

fn window_closed_is_tracked(tracked: Option<WindowId>, closed: WindowId) -> bool {
    tracked == Some(closed)
}

/// Toggle: clicking the same service's tray icon closes the popup; clicking a
/// different one opens/switches. Returns the new open state (`true` = open).
/// Caller (the bar widget) is the bar window — pass its handle as `parent`
/// and the icon's bounds as `anchor_rect`.
pub fn toggle(
    anchor_rect: Bounds<Pixels>,
    parent: AnyWindowHandle,
    _window: &mut Window,
    cx: &mut App,
    service: String,
) -> bool {
    let open_service = cx
        .global::<TrayMenuState>()
        .open_service
        .clone();
    match toggle_action(open_service.as_deref(), &service) {
        ToggleAction::Close => {
            close(cx);
            false
        }
        ToggleAction::OpenFresh => {
            // An xdg-popup's parent/anchor are creation-time state. Reusing
            // the existing window when switching tray items leaves it
            // anchored to the previous icon, so destroy and remap it.
            if open_service.is_some() {
                close(cx);
            }
            open(cx, anchor_rect, parent, service);
            true
        }
    }
}

/// Start the 15s auto-close timer (generation-guarded so only the latest
/// open's timer fires).
fn schedule_autoclose(cx: &mut App, generation: u64) {
    cx.spawn(async move |app_cx: &mut AsyncApp| {
        app_cx.background_executor().timer(AUTO_CLOSE_AFTER).await;
        app_cx.update(|app_cx| {
            if app_cx.global::<TrayMenuState>().close_generation != generation {
                return;
            }
            close(app_cx);
        });
    })
    .detach();
}

/// Wire the tray-menu popup to the live tray service. Called once from
/// `main.rs`.
pub fn init(cx: &mut App) {
    cx.set_global(TrayMenuState::default());

    let signal = AppState::tray(cx).subscribe();

    let watcher = cx.new(|cx| {
        state::watch(
            cx,
            signal,
            |_this: &mut TrayMenuWatcher,
             state: chronos_services::TrayState,
             cx: &mut Context<TrayMenuWatcher>| {
                // When FetchMenu lands for the open service, repaint + resize.
                let (open_service, nodes) = {
                    let g = cx.global::<TrayMenuState>();
                    match &g.open_service {
                        Some(svc) => {
                            let nodes = state
                                .find(svc)
                                .and_then(|item| item.menu.clone())
                                .unwrap_or_default();
                            (Some(svc.clone()), nodes)
                        }
                        None => (None, Vec::new()),
                    }
                };
                if let Some(_svc) = &open_service {
                    let (handle, view) = {
                        let g = cx.global_mut::<TrayMenuState>();
                        g.nodes = nodes;
                        (g.handle.clone(), g.view.clone())
                    };
                    if let Some(handle) = handle {
                        let display_h = pult_display_height(cx);
                        let display_w = pult_display_width(cx);
                        let has_head = open_service
                            .as_deref()
                            .and_then(|svc| {
                                state
                                    .find(svc)
                                    .and_then(|item| item.title.as_deref())
                                    .map(|t| !t.is_empty())
                            })
                            .unwrap_or(false);
                        let width = estimate_menu_width(
                            &cx.global::<TrayMenuState>().nodes,
                            display_w,
                        );
                        let height = estimate_menu_height(
                            &cx.global::<TrayMenuState>().nodes,
                            display_h,
                            has_head,
                        );
                        let _ = handle.update(cx, |_, window: &mut gpui::Window, _| {
                            window.resize(Size::new(px(width), px(height)));
                        });
                        // Repaint the view: `render` diffs the tree and
                        // rebuilds the `PopupMenu` entity (no set-items API).
                        if let Some(view) = view.and_then(|v| v.upgrade()) {
                            let _ = view.update(cx, |_, view_cx| view_cx.notify());
                        }
                    }
                }
            },
        );
        TrayMenuWatcher {}
    });

    cx.global_mut::<TrayMenuState>().watcher = Some(watcher);
    tracing::info!("tray_menu: subscribed to tray service");
}

/// Dispatch a menu-item click to the tray service.
///
/// Called from a `PopupMenuItem` `on_click` callback. The window is NOT
/// closed here: `PopupMenu` emits `DismissEvent` right after the handler
/// runs (confirm), and the view's `DismissEvent` subscription calls
/// `close_this` — one close path, no double `remove_window`.
pub fn click_item(cx: &mut App, id: i32) {
    let Some(service) = cx.global::<TrayMenuState>().open_service.clone() else {
        return;
    };
    AppState::tray(cx).dispatch(TrayCommand::MenuClicked { service, id });
    tracing::info!("tray_menu: clicked menu item id={id}");
}

#[cfg(test)]
mod tests {
    use super::{
        MAX_MENU_W, MENU_MAX_W, MENU_MIN_W, ToggleAction, count_visible, estimate_menu_width,
        toggle_action, window_closed_is_tracked, window_options,
    };
    use chronos_services::MenuNode;
    use gpui::{
        AppContext, Bounds, Context, Render, TestAppContext, Window, WindowId, WindowKind, div,
    };

    struct EmptyView;

    impl Render for EmptyView {
        fn render(
            &mut self,
            _window: &mut Window,
            _cx: &mut Context<Self>,
        ) -> impl gpui::IntoElement {
            div()
        }
    }

    #[test]
    fn same_service_closes_the_open_popup() {
        assert_eq!(toggle_action(Some("steam"), "steam"), ToggleAction::Close);
    }

    #[test]
    fn different_service_requires_a_fresh_anchor() {
        assert_eq!(
            toggle_action(Some("steam"), "discord"),
            ToggleAction::OpenFresh
        );
    }

    #[test]
    fn closed_state_opens_a_fresh_popup() {
        assert_eq!(toggle_action(None, "steam"), ToggleAction::OpenFresh);
    }

    #[gpui::test]
    fn anchored_popup_does_not_request_a_compositor_grab(cx: &mut TestAppContext) {
        let parent = cx.update(|cx| {
            cx.open_window(Default::default(), |_window, cx| cx.new(|_| EmptyView))
                .expect("test parent window")
        });
        let options = window_options(Bounds::default(), parent.into(), 300.0, 120.0);
        let WindowKind::AnchoredPopup(options) = options.kind else {
            panic!("tray menu must remain an anchored popup");
        };

        assert!(!options.grab, "T264 forbids compositor popup grabs");
    }

    #[test]
    fn compositor_close_matches_only_the_tracked_window() {
        let tracked = WindowId::from(7);
        assert!(window_closed_is_tracked(Some(tracked), WindowId::from(7)));
        assert!(!window_closed_is_tracked(Some(tracked), WindowId::from(8)));
        assert!(!window_closed_is_tracked(None, WindowId::from(7)));
    }

    #[test]
    fn submenu_children_do_not_inflate_root_height() {
        let child = menu_node(2, vec![]);
        let root = menu_node(1, vec![child]);
        assert_eq!(count_visible(&[root]), 1);
    }

    #[test]
    fn empty_tree_gets_the_min_card_and_no_reserve() {
        assert_eq!(estimate_menu_width(&[], None), MENU_MIN_W);
    }

    #[test]
    fn flat_short_menu_has_no_submenu_reserve() {
        let nodes = vec![menu_node(1, vec![]), menu_node(2, vec![])];
        // Widest row: 16 row pad + 6 chars × 8 = 64 → +10 chrome = 74 → min
        // card; no submenus → no reserve.
        assert_eq!(estimate_menu_width(&nodes, None), MENU_MIN_W);
    }

    #[test]
    fn long_label_clamps_the_card_to_max() {
        let mut node = menu_node(1, vec![]);
        node.label = "x".repeat(80);
        // 10 chrome + 16 row pad + 80 × 8 = 666 → clamped to the 300 max card.
        assert_eq!(estimate_menu_width(&[node], None), MENU_MAX_W);
    }

    #[test]
    fn submenu_reserves_the_widest_child_card() {
        let mut child = menu_node(2, vec![]);
        child.label = "w".repeat(30);
        let mut sub = menu_node(1, vec![child]);
        sub.label = "sub".to_string();
        // Root: min card 230. Child level: 10 chrome + 16 row pad + 30 × 8
        // = 266 (within bounds) → reserve 266 + 8 anchor overlap = 274.
        assert_eq!(estimate_menu_width(&[sub], None), 230. + 266. + 8.);
    }

    #[test]
    fn nested_submenus_sum_the_open_chain() {
        let mut leaf = menu_node(3, vec![]);
        leaf.label = "w".repeat(30);
        let inner = menu_node(2, vec![leaf]);
        let outer = menu_node(1, vec![inner]);
        // Root: 230. Outer children level: min card 230 → 238 with overlap.
        // Inner children level: 266 → 274 with overlap.
        assert_eq!(estimate_menu_width(&[outer], None), 230. + 238. + 274.);
    }

    #[test]
    fn surface_width_clamps_to_the_display() {
        let mut child = menu_node(2, vec![]);
        child.label = "w".repeat(30);
        let sub = menu_node(1, vec![child]);
        // Unclamped the surface would be 504; a 400px display caps it at
        // 400 − 16.
        assert_eq!(estimate_menu_width(&[sub], Some(400.)), 384.);
    }

    #[test]
    fn unreachable_display_falls_back_to_the_absolute_cap() {
        // Chain of four max-width submenu levels under a min-card root:
        // 230 + 4 × (300 + 8) = 1462 → capped.
        let mut node = menu_node(0, vec![]);
        node.label = "x".repeat(80);
        for id in 1..5 {
            let mut wide = menu_node(id * 100, vec![]);
            wide.label = "x".repeat(80);
            node = menu_node(id, vec![wide, node]);
        }
        assert_eq!(estimate_menu_width(&[node], None), MAX_MENU_W);
    }

    #[test]
    fn invisible_nodes_do_not_widen_the_surface() {
        let mut hidden = menu_node(1, vec![]);
        hidden.label = "x".repeat(80);
        hidden.visible = false;
        assert_eq!(estimate_menu_width(&[hidden], None), MENU_MIN_W);
    }

    #[test]
    fn separators_carry_no_width() {
        let mut sep = menu_node(1, vec![]);
        sep.separator = true;
        sep.label = "x".repeat(80);
        assert_eq!(estimate_menu_width(&[sep], None), MENU_MIN_W);
    }

    #[test]
    fn icons_and_shortcuts_feed_the_estimate() {
        let mut plain = menu_node(1, vec![]);
        plain.label = "l".repeat(24);
        // Plain: 10 chrome + 16 row pad + 24 × 8 = 218 → min card 230.
        assert_eq!(estimate_menu_width(&[plain.clone()], None), MENU_MIN_W);
        let mut with_icon = plain.clone();
        with_icon.icon_name = Some("app".to_string());
        // +24 gutter → 242.
        assert_eq!(estimate_menu_width(&[with_icon], None), 242.);
        let mut with_shortcut = plain;
        with_shortcut.shortcut = Some(vec![vec!["Control".to_string(), "X".to_string()]]);
        // Custom row: gutter 24 + shortcut gap 8 + glyph "⌃X" 2 × 9 → 268.
        assert_eq!(estimate_menu_width(&[with_shortcut], None), 268.);
    }

    fn menu_node(id: i32, children: Vec<MenuNode>) -> MenuNode {
        MenuNode {
            id,
            label: format!("item {id}"),
            enabled: true,
            visible: true,
            separator: false,
            toggle: None,
            icon_name: None,
            shortcut: None,
            children,
        }
    }
}
