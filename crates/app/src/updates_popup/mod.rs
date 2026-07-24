//! Updates popup — pending-package list + "Upgrade all" button, opened by
//! clicking the bar's updates widget.
//!
//! Mirrors `tray_menu/` for the window-lifecycle bits (layer-shell popup,
//! the `close_this` reentrancy guard around `window.remove_window()` — see
//! HANDOFF.md "СИСТЕМНЫЙ БАГ: window.remove_window()") but is simpler:
//! there is only ever one popup (no per-service keying), and — per ZED.md's
//! explicit warning about the `follow_mouse=1` focus-loss trap (HANDOFF.md /
//! MEMORY.md 2026-07-18: "Cline №9 debounce отклонён") — it does **not**
//! close on keyboard-focus loss at all. Dismiss is always a conscious
//! action: the bar icon toggle, the in-popup "✕" button, or "Upgrade all".

pub mod view;

use gpui::{
    AnyWindowHandle, App, Bounds, Context, DisplayId, Entity, Global, Pixels, Size, Window,
    WindowBackgroundAppearance, WindowBounds, WindowHandle, WindowKind, WindowOptions,
    layer_shell::*, point, prelude::*, px,
    popup::{PopupAnchor, PopupConstraintAdjustment, PopupGravity, PopupNotSupportedError, PopupOptions},
};

use chronos_services::{AurCommand, Service, UpdatesState};

use crate::state::{self, AppState};
use crate::updates_popup::view::UpdatesPopupView;

/// Popup width (px).
const POPUP_WIDTH: f32 = 360.;
const POPUP_MARGIN_TOP: f32 = 36.;
const POPUP_MARGIN_RIGHT: f32 = 8.;

/// Measured from mockup geometry (padding + text + border):
/// header: py(12)*2 + text(~16) + border(1) = ~41
/// row:    py(9)*2 + text(~16) + border(1) = ~35
/// footer: py(12)*2 + btn(py(8)*2 + text(~16) + border(1)) = ~63
const HEADER_H: f32 = 41.;
const ROW_H: f32 = 35.;
const FOOTER_H: f32 = 72.;
const EMPTY_ROW_H: f32 = 40.;
/// Don't grow beyond this — scroll instead.
pub(crate) const MAX_LIST_H: f32 = 340.;

fn estimate_popup_height(count: usize) -> f32 {
    if count == 0 {
        HEADER_H + EMPTY_ROW_H
    } else {
        let list_h = (count as f32 * ROW_H).min(MAX_LIST_H);
        HEADER_H + list_h + FOOTER_H
    }
}

/// Global state for the updates popup.
#[derive(Default)]
pub struct UpdatesPopupState {
    /// Window handle while the popup is open; `None` when closed.
    handle: Option<WindowHandle<UpdatesPopupView>>,
    /// Watcher entity driving repaints/resizes on `UpdatesState` changes.
    watcher: Option<Entity<UpdatesPopupWatcher>>,
}

impl Global for UpdatesPopupState {}

/// Tiny entity that hosts the `state::watch()` subscription — same role as
/// `TrayMenuWatcher`; it has no state of its own.
pub struct UpdatesPopupWatcher {}

fn pick_display(cx: &App) -> Option<DisplayId> {
    crate::monitor::pult_display(cx)
}

/// Layer-shell window options for the popup: TOP | RIGHT, overlay, never
/// exclusive, no keyboard interactivity (mouse-driven, like `tray_menu`).
/// Used as fallback when `AnchoredPopup` isn't supported on this platform.
fn fallback_window_options(display_id: Option<DisplayId>, height: f32) -> WindowOptions {
    WindowOptions {
        display_id,
        titlebar: None,
        window_bounds: Some(WindowBounds::Windowed(Bounds {
            origin: point(px(0.), px(0.)),
            size: Size::new(px(POPUP_WIDTH), px(height)),
        })),
        app_id: Some("chronos-updates-popup".to_string()),
        window_background: WindowBackgroundAppearance::Transparent,
        kind: WindowKind::LayerShell(LayerShellOptions {
            namespace: "updates-popup".to_string(),
            layer: Layer::Overlay,
            anchor: Anchor::TOP | Anchor::RIGHT,
            exclusive_zone: None,
            margin: Some((px(POPUP_MARGIN_TOP), px(POPUP_MARGIN_RIGHT), px(0.), px(0.))),
            keyboard_interactivity: KeyboardInteractivity::None,
            ..Default::default()
        }),
        ..Default::default()
    }
}

/// Anchored popup window options — popup positioned relative to the trigger
/// icon's bounds, extending down-and-left from the icon's bottom-right corner.
fn window_options(anchor_rect: Bounds<Pixels>, parent: AnyWindowHandle, height: f32) -> WindowOptions {
    WindowOptions {
        titlebar: None,
        window_bounds: Some(WindowBounds::Windowed(Bounds {
            origin: point(px(0.), px(0.)),
            size: Size::new(px(POPUP_WIDTH), px(height)),
        })),
        app_id: Some("chronos-updates-popup".to_string()),
        window_background: WindowBackgroundAppearance::Transparent,
        kind: WindowKind::AnchoredPopup(PopupOptions {
            parent,
            anchor_rect,
            anchor: PopupAnchor::BottomRight,
            gravity: PopupGravity::BottomLeft,
            constraint_adjustment: PopupConstraintAdjustment::SLIDE_X
                | PopupConstraintAdjustment::FLIP_X,
            offset: point(px(0.), px(4.)),
            grab: true,
        }),
        ..Default::default()
    }
}

/// Open the popup (idempotent — no-op if already open). Also fires a
/// `Refresh` so the list is current even if the last poll tick is stale
/// (mirrors `tray_menu::open`'s re-fetch-on-open).
pub fn open(cx: &mut App, anchor_rect: Bounds<Pixels>, parent: AnyWindowHandle) {
    AppState::aur(cx).dispatch(AurCommand::Refresh);

    if cx.global::<UpdatesPopupState>().handle.is_some() {
        return;
    }

    let count = AppState::aur(cx).get().count();
    let height = estimate_popup_height(count);

    let result = cx.open_window(window_options(anchor_rect, parent, height), |_, app_cx| {
        app_cx.new(|view_cx| UpdatesPopupView::new(view_cx))
    });

    let result = match result {
        Err(err) => {
            if err.downcast_ref::<PopupNotSupportedError>().is_some() {
                tracing::warn!("updates_popup: AnchoredPopup not supported on this platform, falling back to fixed-corner LayerShell");
                let display_id = pick_display(cx);
                cx.open_window(fallback_window_options(display_id, height), |_, app_cx| {
                    app_cx.new(|view_cx| UpdatesPopupView::new(view_cx))
                })
            } else {
                Err(err)
            }
        }
        ok => ok,
    };

    match result {
        Ok(new_handle) => {
            cx.global_mut::<UpdatesPopupState>().handle = Some(new_handle);
        }
        Err(err) => tracing::warn!("updates_popup: failed to open popup: {err}"),
    }
}

/// Close the popup (clears state + destroys the window). Safe to call from
/// contexts that do NOT already hold `&mut Window` for this popup (bar
/// widget click, external toggle) — uses `handle.update`.
pub fn close(cx: &mut App) {
    if let Some(handle) = cx.global_mut::<UpdatesPopupState>().handle.take() {
        if let Err(e) = handle.update(cx, |_, window: &mut Window, _| window.remove_window()) {
            tracing::warn!("updates_popup: close remove_window failed (already dead?): {e}");
        }
    }
}

/// Close the popup from inside a callback that already holds `&mut Window`
/// for this popup's window-id (the in-popup "✕" / "Upgrade all" buttons). A
/// blind `close(cx)` would re-enter `handle.update` on the same id, which
/// silently fails while the callback is running (the window slot is empty
/// during dispatch) and leaves a ghost popup — see HANDOFF.md "СИСТЕМНЫЙ
/// БАГ: window.remove_window()". Clear the tracked handle and call
/// `remove_window()` on the live reference directly instead — the pattern
/// `launcher`/`tray_menu` already use.
pub(crate) fn close_this(window: &mut Window, cx: &mut App) {
    let this = window.window_handle();
    let tracked = cx
        .global::<UpdatesPopupState>()
        .handle
        .as_ref()
        .map(|h| **h == this)
        .unwrap_or(false);
    if tracked {
        cx.global_mut::<UpdatesPopupState>().handle.take(); // clear BEFORE remove
    }
    window.remove_window(); // direct, no reentrant handle.update
}

/// Toggle: click on the bar icon closes an open popup, opens a closed one.
/// Called from the bar widget's `on_mouse_down`, which holds `&mut Window` for
/// the BAR's window, not the popup's — so closing an already-open popup here
/// correctly goes through `close(cx)` (`handle.update`), not `close_this`.
pub fn toggle(anchor_rect: Bounds<Pixels>, parent: AnyWindowHandle, _window: &mut Window, cx: &mut App) {
    let is_open = cx.global::<UpdatesPopupState>().handle.is_some();
    if is_open {
        close(cx);
    } else {
        open(cx, anchor_rect, parent);
    }
}

/// Dispatch "Upgrade all". Called from inside the popup's own `on_click`,
/// which already holds `&mut Window` for the popup's window. The popup
/// stays open so the user can see the upgrade status (button is blocked,
/// "Upgrading…" text shown); it closes only on explicit dismiss or after
/// the upgrade completes and the user clicks close.
pub(crate) fn upgrade_all(_window: &mut Window, cx: &mut App) {
    AppState::aur(cx).dispatch(AurCommand::UpgradeAll);
    tracing::info!("updates_popup: dispatched UpgradeAll");
}

/// Wire the updates popup to the live aur service. Called once from
/// `main.rs`.
pub fn init(cx: &mut App) {
    cx.set_global(UpdatesPopupState::default());

    let signal = AppState::aur(cx).subscribe();

    let watcher = cx.new(|cx| {
        state::watch(
            cx,
            signal,
            |_this: &mut UpdatesPopupWatcher,
             updates_state: UpdatesState,
             cx: &mut Context<UpdatesPopupWatcher>| {
                let handle = cx.global::<UpdatesPopupState>().handle.clone();
                if let Some(handle) = handle {
                    let height = estimate_popup_height(updates_state.count());
                    let resize_ok = handle.update(cx, |_, window: &mut Window, _| {
                        window.resize(Size::new(px(POPUP_WIDTH), px(height)));
                    });
                    if resize_ok.is_err() {
                        // Window is dead — clear the stale handle to stop spamming.
                        cx.global_mut::<UpdatesPopupState>().handle.take();
                    } else {
                        let _ = handle.update(cx, |_, _window, view_cx| view_cx.notify());
                    }
                }
            },
        );
        UpdatesPopupWatcher {}
    });

    cx.global_mut::<UpdatesPopupState>().watcher = Some(watcher);
    tracing::info!("updates_popup: subscribed to aur service");
}
