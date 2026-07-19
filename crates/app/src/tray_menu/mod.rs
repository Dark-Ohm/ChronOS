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

use std::time::Duration;

use gpui::{
    App, AsyncApp, Bounds, Context, DisplayId, Entity, Global, Size, Window,
    WindowBackgroundAppearance, WindowBounds, WindowHandle, WindowKind, WindowOptions,
    layer_shell::*, point, prelude::*, px,
};

use chronos_services::{MenuNode, Service, TrayCommand, TraySubscriber};

use crate::state::{self, AppState};
use crate::tray_menu::view::TrayMenuView;

/// Popup width (px). Tall enough for typical indicator menus.
const MENU_WIDTH: f32 = 240.;
/// Top + right margin (px) so the card sits just below the bar's top edge.
const MENU_MARGIN_TOP: f32 = 36.;
const MENU_MARGIN_RIGHT: f32 = 8.;
/// Auto-close delay after the last open (generation-guarded).
const AUTO_CLOSE_AFTER: Duration = Duration::from_secs(15);
/// Per-row geometry (px): vertical padding + one line of label text.
const ROW_H: f32 = 30.;
/// Floor so a single short menu still has a usable popup height.
const MIN_MENU_H: f32 = 28.;
/// Cap so a huge menu can't cover the screen (it scrolls if it somehow hits
/// this, but the surface itself never grows past it).
const MAX_MENU_H: f32 = 480.;

/// Count visible menu rows recursively (inline submenus add their children).
fn count_visible(nodes: &[MenuNode]) -> usize {
    nodes
        .iter()
        .filter(|n| n.visible)
        .map(|n| 1 + count_visible(&n.children))
        .sum()
}

/// Estimate the popup height (px) from the current menu tree.
fn estimate_menu_height(nodes: &[MenuNode]) -> f32 {
    let rows = count_visible(nodes);
    if rows == 0 {
        // Placeholder ("…") state — keep a small surface.
        return MIN_MENU_H;
    }
    (rows as f32 * ROW_H).clamp(MIN_MENU_H, MAX_MENU_H)
}

/// Global state for the tray context-menu popup.
#[derive(Default)]
pub struct TrayMenuState {
    /// Window handle while a menu is open; `None` when closed.
    handle: Option<WindowHandle<TrayMenuView>>,
    /// Watcher entity driving repaints on tray snapshot changes.
    watcher: Option<Entity<TrayMenuWatcher>>,
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
    cx.primary_display()
        .map(|d| d.id())
        .or_else(|| cx.displays().into_iter().next().map(|d| d.id()))
}

/// Layer-shell window options for the menu popup: TOP | RIGHT, overlay,
/// never exclusive, no keyboard interactivity.
fn window_options(display_id: Option<DisplayId>, height: f32) -> WindowOptions {
    WindowOptions {
        display_id,
        titlebar: None,
        window_bounds: Some(WindowBounds::Windowed(Bounds {
            origin: point(px(0.), px(0.)),
            size: Size::new(px(MENU_WIDTH), px(height)),
        })),
        app_id: Some("chronos-tray-menu".to_string()),
        window_background: WindowBackgroundAppearance::Transparent,
        kind: WindowKind::LayerShell(LayerShellOptions {
            namespace: "tray-menu".to_string(),
            layer: Layer::Overlay,
            anchor: Anchor::TOP | Anchor::RIGHT,
            exclusive_zone: None,
            margin: Some((px(MENU_MARGIN_TOP), px(MENU_MARGIN_RIGHT), px(0.), px(0.))),
            keyboard_interactivity: KeyboardInteractivity::None,
            ..Default::default()
        }),
        ..Default::default()
    }
}

/// Open (or switch to) the menu for `service`. Fetches the tree via
/// `FetchMenu` and opens the layer-shell surface.
pub fn open(cx: &mut App, service: String) {
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

    let state = cx.global_mut::<TrayMenuState>();
    state.open_service = Some(service);
    state.nodes = nodes;
    state.close_generation = state.close_generation.wrapping_add(1);
    let generation = state.close_generation;
    drop(state);

    let handle = cx.global::<TrayMenuState>().handle.clone();
    match handle {
        Some(existing) => {
            let height = estimate_menu_height(&cx.global::<TrayMenuState>().nodes);
            let _ = existing.update(cx, |_, window: &mut gpui::Window, _| {
                window.resize(Size::new(px(MENU_WIDTH), px(height)));
            });
            let _ = existing.update(cx, |_, _window, view_cx| {
                view_cx.notify();
            });
        }
        None => {
            let display_id = pick_display(cx);
            let height = estimate_menu_height(&cx.global::<TrayMenuState>().nodes);
            match cx.open_window(window_options(display_id, height), |_, app_cx| {
                app_cx.new(|view_cx| TrayMenuView::new(view_cx))
            }) {
                Ok(new_handle) => {
                    cx.global_mut::<TrayMenuState>().handle = Some(new_handle);
                }
                Err(err) => tracing::warn!("tray_menu: failed to open popup: {err}"),
            }
        }
    }

    schedule_autoclose(cx, generation);
}

/// Close the popup (clears state + destroys the window).
pub fn close(cx: &mut App) {
    let state = cx.global_mut::<TrayMenuState>();
    state.open_service = None;
    state.nodes = Vec::new();
    state.close_generation = state.close_generation.wrapping_add(1);
    if let Some(handle) = state.handle.take() {
        let _ = handle.update(cx, |_, window: &mut gpui::Window, _| window.remove_window());
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
    if tracked {
        cx.global_mut::<TrayMenuState>().handle.take(); // clear BEFORE remove
    }
    let state = cx.global_mut::<TrayMenuState>();
    state.open_service = None;
    state.nodes = Vec::new();
    state.close_generation = state.close_generation.wrapping_add(1);
    window.remove_window(); // direct, no reentrant handle.update
}

/// Toggle: clicking the same service's tray icon closes the popup; clicking a
/// different one opens/switches. Returns the new open state (`true` = open).
pub fn toggle(cx: &mut App, service: String) -> bool {
    let already = cx
        .global::<TrayMenuState>()
        .open_service
        .as_ref()
        .map(|s| s == &service)
        .unwrap_or(false);
    if already {
        close(cx);
        false
    } else {
        open(cx, service);
        true
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
                    let handle = {
                        let g = cx.global_mut::<TrayMenuState>();
                        g.nodes = nodes;
                        g.handle.clone()
                    };
                    if let Some(handle) = handle {
                        let height = estimate_menu_height(
                            &cx.global::<TrayMenuState>().nodes,
                        );
                        let _ = handle.update(cx, |_, window: &mut gpui::Window, _| {
                            window.resize(Size::new(px(MENU_WIDTH), px(height)));
                        });
                        let _ = handle.update(cx, |_, _window, view_cx| view_cx.notify());
                    }
                }
            },
        );
        TrayMenuWatcher {}
    });

    cx.global_mut::<TrayMenuState>().watcher = Some(watcher);
    tracing::info!("tray_menu: subscribed to tray service");
}

/// Dispatch a menu-item click to the tray service and close the popup.
///
/// Called from inside the popup's own `on_click` callback, which already
/// holds `&mut Window` for this window. Closing MUST go through that live
/// reference — calling `close(cx)` here would re-enter `handle.update` on the
/// same window-id, which silently fails (`App::update_window_id` keeps the
/// slot `cx.windows[id]` empty during the callback) and leaves a ghost popup.
pub fn click_item(window: &mut Window, cx: &mut App, id: i32) {
    let Some(service) = cx.global::<TrayMenuState>().open_service.clone() else {
        return;
    };
    AppState::tray(cx).dispatch(TrayCommand::MenuClicked { service, id });
    tracing::info!("tray_menu: clicked menu item id={id}");
    close_this(window, cx);
}
