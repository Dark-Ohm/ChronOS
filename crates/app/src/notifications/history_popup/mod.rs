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
    App, Bounds, Context, DisplayId, Entity, Global, Size, Window, WindowBackgroundAppearance,
    WindowBounds, WindowHandle, WindowKind, WindowOptions, layer_shell::*, point, prelude::*, px,
};

use chronos_services::{NotificationCommand, Service};

use crate::state::{self, AppState};

/// Popup width (px).
const POPUP_WIDTH: f32 = 360.;
/// Top + right margin (px) — same as `updates_popup` so it sits just under the
/// bar's top edge, aligned to the right.
const POPUP_MARGIN_TOP: f32 = 36.;
const POPUP_MARGIN_RIGHT: f32 = 8.;
/// Header row height budget (px).
const HEADER_H: f32 = 36.;
/// Hard pixel clip on the history-card list. The window height is a fixed cap
/// (the `max_h` clip is the real guarantee — same philosophy as №12/№13).
const LIST_MAX_H: f32 = 380.;
/// Total window height cap = header + clipped list.
const POPUP_HEIGHT: f32 = HEADER_H + LIST_MAX_H;

/// Global state for the history popup.
#[derive(Default)]
pub struct HistoryPopupState {
    /// Window handle while the popup is open; `None` when closed.
    handle: Option<WindowHandle<view::HistoryPopupView>>,
    /// Watcher entity driving repaints on `NotificationState` changes.
    watcher: Option<Entity<HistoryPopupWatcher>>,
}

impl Global for HistoryPopupState {}

/// Tiny entity hosting the `state::watch()` subscription (same role as
/// `UpdatesPopupWatcher`); it has no state of its own.
pub struct HistoryPopupWatcher {}

fn pick_display(cx: &App) -> Option<DisplayId> {
    crate::monitor::pult_display(cx)
}

/// Layer-shell window options for the popup: TOP | RIGHT, overlay, never
/// exclusive, no keyboard interactivity (mouse-driven, like `updates_popup`).
fn window_options(display_id: Option<DisplayId>, height: f32) -> WindowOptions {
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

/// Open the popup (idempotent — no-op if already open). Also marks the history
/// read so the bell's unread dot clears the moment the inbox is viewed.
pub fn open(cx: &mut App) {
    AppState::notification(cx).dispatch(NotificationCommand::MarkAllRead);

    if cx.global::<HistoryPopupState>().handle.is_some() {
        return;
    }

    let display_id = pick_display(cx);
    match cx.open_window(window_options(display_id, POPUP_HEIGHT), |_, app_cx| {
        app_cx.new(|view_cx| view::HistoryPopupView::new(view_cx))
    }) {
        Ok(new_handle) => {
            cx.global_mut::<HistoryPopupState>().handle = Some(new_handle);
        }
        Err(err) => tracing::warn!("history_popup: failed to open popup: {err}"),
    }
}

/// Close the popup (clears state + destroys the window). Safe to call from
/// contexts that do NOT already hold `&mut Window` for this popup (bar widget
/// click, external toggle) — uses `handle.update`.
pub fn close(cx: &mut App) {
    if let Some(handle) = cx.global_mut::<HistoryPopupState>().handle.take() {
        let _ = handle.update(cx, |_, window: &mut Window, _| window.remove_window());
    }
}

/// Close the popup from inside a callback that already holds `&mut Window` for
/// this popup's window-id (the in-popup "✕" button). A blind `close(cx)` would
/// re-enter `handle.update` on the same id and silently fail — see HANDOFF.md
/// "СИСТЕМНЫЙ БАГ: window.remove_window()". Clear the tracked handle and call
/// `remove_window()` on the live reference directly.
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
/// Called from the bell widget's `on_click`, which holds `&mut Window` for the
/// BAR's window, not the popup's — so closing here correctly goes through
/// `close(cx)` (`handle.update`), not `close_this`.
pub fn toggle(_window: &mut Window, cx: &mut App) {
    let is_open = cx.global::<HistoryPopupState>().handle.is_some();
    if is_open {
        close(cx);
    } else {
        open(cx);
    }
}

/// Wire the history popup to the live notification service. Called once from
/// `main.rs` (after `notifications::init`).
pub fn init(cx: &mut App) {
    cx.set_global(HistoryPopupState::default());

    let signal = AppState::notification(cx).subscribe();

    let watcher = cx.new(|cx| {
        state::watch(
            cx,
            signal,
            |_this: &mut HistoryPopupWatcher,
             _state: chronos_services::NotificationState,
             cx: &mut Context<HistoryPopupWatcher>| {
                let handle = cx.global::<HistoryPopupState>().handle.clone();
                if let Some(handle) = handle {
                    let _ = handle.update(cx, |_, _window, view_cx| view_cx.notify());
                }
            },
        );
        HistoryPopupWatcher {}
    });

    cx.global_mut::<HistoryPopupState>().watcher = Some(watcher);
    tracing::info!("history_popup: subscribed to notification service");
}
