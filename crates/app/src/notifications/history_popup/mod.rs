//! History popup — the persistent log of notifications (feature №14).
//!
//! Opened by clicking the bar's bell widget. Unlike the ephemeral
//! notifications popup (`crate::notifications::view`), this renders the whole
//! in-session history (`NotificationState::history`) and does **not** close on
//! focus loss (same rule as `updates_popup`/`tray_menu`: dismiss is always a
//! conscious action). Opening it marks the history read (`MarkAllRead`) so the
//! bell's unread dot clears.
//!
//! Lifecycle mirrors `updates_popup`: a GPUI global holds the open window
//! handle + a `state::watch` subscription, `open`/`close`/`close_this`/`toggle`
//! follow the same reentrancy-safe pattern (see HANDOFF.md "СИСТЕМНЫЙ БАГ:
//! window.remove_window()").

pub mod view;

use gpui::{
    AnyWindowHandle, App, Bounds, Context, DisplayId, Entity, Global, Pixels, Size, Window,
    WindowBackgroundAppearance, WindowBounds, WindowHandle, WindowKind, WindowOptions,
    layer_shell::*,
    point,
    popup::{
        PopupAnchor, PopupConstraintAdjustment, PopupGravity, PopupNotSupportedError, PopupOptions,
    },
    prelude::*,
    px,
};

use chronos_services::{NotificationCommand, Service};

use crate::state::{self, AppState};

/// Popup width (px) — mockup-fixed.
const POPUP_WIDTH: f32 = 360.;
/// Margins for the LayerShell fallback (no anchoring available).
const POPUP_MARGIN_TOP: f32 = 36.;
const POPUP_MARGIN_RIGHT: f32 = 8.;
/// Card row height budget. Conservative estimate: padding 10+12 + app_name
/// 10.5 + summary 12.5 + body 4-line clamp (11.5*1.45*4≈67) + actions ≈30
/// = ~142, but most cards are shorter. We err on the generous side so the
/// footer "Clear all" is never clipped on initial render — the resize
/// watcher will shrink-to-fit on the next state update.
const ROW_H: f32 = 100.;
/// Footer "Clear all" strip height budget (12px padding * 2 + 8px btn pad
/// * 2 + 12.5px label ≈ 53).
const FOOTER_H: f32 = 53.;
/// Empty-state height budget ("No notifications" centered with 36px padding
/// * 2 + 12px line).
const EMPTY_H: f32 = 84.;
/// Don't grow beyond this — scroll instead.
pub(crate) const MAX_LIST_H: f32 = 480.;

/// Estimate popup height from the live history length so the window is
/// pre-sized close to content (anchor + resize path updates on changes).
fn estimate_popup_height(count: usize) -> f32 {
    if count == 0 {
        EMPTY_H
    } else {
        let list_h = (count as f32 * ROW_H).min(MAX_LIST_H);
        list_h + FOOTER_H
    }
}

/// Global state for the history popup.
#[derive(Default)]
pub struct HistoryPopupState {
    /// Window handle while the popup is open; `None` when closed.
    handle: Option<WindowHandle<view::HistoryPopupView>>,
    /// Watcher entity driving repaints/resize on `NotificationState` changes.
    watcher: Option<Entity<HistoryPopupWatcher>>,
}

impl Global for HistoryPopupState {}

/// Tiny entity hosting the `state::watch()` subscription (same role as
/// `UpdatesPopupWatcher`); it has no state of its own.
pub struct HistoryPopupWatcher {}

fn pick_display(cx: &App) -> Option<DisplayId> {
    crate::monitor::pult_display(cx)
}

/// Layer-shell window options for the popup — fallback when `AnchoredPopup`
/// is not supported on the current platform. TOP | RIGHT overlay, never
/// exclusive, no keyboard interactivity (mouse-driven, like `updates_popup`).
/// Same geometry as `updates_popup::fallback_window_options`.
fn fallback_window_options(display_id: Option<DisplayId>, height: f32) -> WindowOptions {
    WindowOptions {
        display_id,
        titlebar: None,
        window_bounds: Some(WindowBounds::Windowed(Bounds {
            origin: point(px(0.), px(0.)),
            size: Size::new(px(POPUP_WIDTH), px(height)),
        })),
        app_id: Some("chronos-notif-history-popup".to_string()),
        window_background: WindowBackgroundAppearance::Transparent,
        kind: WindowKind::LayerShell(LayerShellOptions {
            namespace: "notif-history-popup".to_string(),
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

/// Anchored popup window options — popup positioned relative to the bell
/// icon's bounds, extending down-and-left from the icon's bottom-right
/// corner. Same anchor/gravity/constraint/offset as `updates_popup` (T117
/// proven pair — do not invent new geometry).
fn window_options(
    anchor_rect: Bounds<Pixels>,
    parent: AnyWindowHandle,
    height: f32,
) -> WindowOptions {
    WindowOptions {
        titlebar: None,
        window_bounds: Some(WindowBounds::Windowed(Bounds {
            origin: point(px(0.), px(0.)),
            size: Size::new(px(POPUP_WIDTH), px(height)),
        })),
        app_id: Some("chronos-notif-history-popup".to_string()),
        window_background: WindowBackgroundAppearance::Transparent,
        kind: WindowKind::AnchoredPopup(PopupOptions {
            parent,
            anchor_rect,
            anchor: PopupAnchor::BottomRight,
            gravity: PopupGravity::BottomLeft,
            constraint_adjustment: PopupConstraintAdjustment::SLIDE_X
                | PopupConstraintAdjustment::FLIP_X,
            offset: point(px(0.), px(4.)),
            // T264 A2: no compositor grab — see the note in
            // `volume_popup::window_options`.
            grab: false,
        }),
        ..Default::default()
    }
}

/// Open the popup anchored to the bell (idempotent — no-op if already open).
/// Marks the history read so the bell's unread dot clears the moment the
/// inbox is viewed. Same reentrancy discipline as `updates_popup::open`.
pub fn open(cx: &mut App, anchor_rect: Bounds<Pixels>, parent: AnyWindowHandle) {
    let svc = AppState::notification(cx).clone();
    cx.background_spawn(async move {
        let _ = svc.dispatch(NotificationCommand::MarkAllRead).await;
    })
    .detach();

    if cx.global::<HistoryPopupState>().handle.is_some() {
        return;
    }

    let count = AppState::notification(cx).get().history.len();
    let height = estimate_popup_height(count);

    let result = cx.open_window(window_options(anchor_rect, parent, height), |_, app_cx| {
        app_cx.new(|view_cx| view::HistoryPopupView::new(view_cx))
    });

    // AnchoredPopup may not be supported on this platform/backend — fall
    // back to fixed-corner LayerShell (mirrors `updates_popup::open`).
    let result = match result {
        Err(err) => {
            if err.downcast_ref::<PopupNotSupportedError>().is_some() {
                tracing::warn!(
                    "history_popup: AnchoredPopup not supported on this platform, falling back to fixed-corner LayerShell"
                );
                let display_id = pick_display(cx);
                cx.open_window(fallback_window_options(display_id, height), |_, app_cx| {
                    app_cx.new(|view_cx| view::HistoryPopupView::new(view_cx))
                })
            } else {
                Err(err)
            }
        }
        ok => ok,
    };

    match result {
        Ok(new_handle) => {
            cx.global_mut::<HistoryPopupState>().handle = Some(new_handle);
        }
        Err(err) => tracing::warn!("history_popup: failed to open popup: {err}"),
    }
}

/// Close the popup (clears state + destroys the window). Safe to call from
/// contexts that do NOT already hold `&mut Window` for this popup (bar widget
/// click, external toggle) — uses `handle.update`. Logs failures instead of
/// `let _ =` (plan Global Constraints).
pub fn close(cx: &mut App) {
    if let Some(handle) = cx.global_mut::<HistoryPopupState>().handle.take() {
        if let Err(e) = handle.update(cx, |_, window: &mut Window, _| window.remove_window()) {
            tracing::warn!("history_popup: close remove_window failed (already dead?): {e}");
        }
    }
}

/// Close the popup from inside a callback that already holds `&mut Window` for
/// this popup's window-id. A blind `close(cx)` would re-enter `handle.update`
/// on the same id and silently fail — see HANDOFF.md "СИСТЕМНЫЙ БАГ:
/// window.remove_window()". Clear the tracked handle and call
/// `remove_window()` on the live reference directly (same as
/// `updates_popup::close_this`).
pub(crate) fn close_this(window: &mut Window, cx: &mut App) {
    let this = window.window_handle();
    let tracked = cx
        .global::<HistoryPopupState>()
        .handle
        .as_ref()
        .map(|h| **h == this)
        .unwrap_or(false);
    if tracked {
        cx.global_mut::<HistoryPopupState>().handle.take(); // clear BEFORE remove
    }
    window.remove_window(); // direct, no reentrant handle.update
}

/// Toggle: click on the bar bell closes an open popup, opens a closed one.
/// Called from the bell widget's `on_mouse_down(Left)`, which holds `&mut
/// Window` for the BAR's window, not the popup's — so closing here correctly
/// goes through `close(cx)` (`handle.update`), not `close_this`.
pub fn toggle(
    anchor_rect: Bounds<Pixels>,
    parent: AnyWindowHandle,
    _window: &mut Window,
    cx: &mut App,
) {
    let is_open = cx.global::<HistoryPopupState>().handle.is_some();
    if is_open {
        close(cx);
    } else {
        open(cx, anchor_rect, parent);
    }
}

/// Wire the history popup to the live notification service. Called once from
/// `main.rs` (after `notifications::init`). On each `NotificationState`
/// change, resizes + notifies the popup window (mirrors `updates_popup::init`).
pub fn init(cx: &mut App) {
    cx.set_global(HistoryPopupState::default());

    let signal = AppState::notification(cx).subscribe();

    let watcher = cx.new(|cx| {
        state::watch(
            cx,
            signal,
            |_this: &mut HistoryPopupWatcher,
             state: chronos_services::NotificationState,
             cx: &mut Context<HistoryPopupWatcher>| {
                let handle = cx.global::<HistoryPopupState>().handle.clone();
                if let Some(handle) = handle {
                    let height = estimate_popup_height(state.history.len());
                    let resize_ok = handle.update(cx, |_, window: &mut Window, _| {
                        window.resize(Size::new(px(POPUP_WIDTH), px(height)));
                    });
                    if resize_ok.is_err() {
                        cx.global_mut::<HistoryPopupState>().handle.take();
                    } else {
                        if let Err(e) = handle.update(cx, |_, _window, view_cx| view_cx.notify()) {
                            tracing::warn!("history_popup: notify update failed: {e}");
                        }
                        cx.refresh_windows();
                    }
                }
            },
        );
        HistoryPopupWatcher {}
    });

    cx.global_mut::<HistoryPopupState>().watcher = Some(watcher);
    tracing::info!("history_popup: subscribed to notification service");
}
