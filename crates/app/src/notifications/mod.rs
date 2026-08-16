//! Notifications module: a layer-shell popup surface that renders the live
//! stack of active notifications from our `org.freedesktop.Notifications`
//! daemon.
//!
//! T124: each toast card is an independent visual unit — no outer list
//! border, each card has its own border + rounded corners + bottom progress
//! bar. Cards stack top-to-bottom, newest last (topmost is most recent).
//!
//! Architecture (mirrors `bar`/`launcher`):
//!   * `NotificationPopupState` — a GPUI global holding the latest
//!     `NotificationState` snapshot plus the open window handle.
//!   * `NotificationWatcher` — a tiny entity whose only job is to host the
//!     `state::watch()` subscription (watch needs an entity/Context, not a
//!     bare `&mut App`). It mutates the global on every snapshot.
//!   * `init()` wires the daemon's reactive stream into the global and opens
//!     /closes the layer-shell surface as notifications appear/disappear.
//!   * `view.rs` renders the card stack, reading straight from the global and
//!     re-painting on every global change (we `notify()` the view from
//!     `sync_window` since `cx.notify()` only targets entities, not globals).
//!     A 100 ms tick loop drives smooth progress bar decay.
//!
//! Sizing: the popup uses a *fixed* window height (`POPUP_HEIGHT`), NOT a
//! pixel estimate of the content. GPUI text metrics are not measured here,
//! and any per-line / per-card arithmetic drifted from the real render
//! height across bugs #9 → #11 → #12 (a long body or a stack of cards
//! silently clipped at the bottom, or — worse — pushed a real control out of
//! the clickable area). Instead the surface is a hard cap and the inner card
//! stack is clipped to `LIST_MAX_H`, so it can never grow the window taller
//! no matter how tall rows actually render. This mirrors the `updates_popup`
//! fix (commit `67f7d10`). T124 raised the cap to 480 px for taller cards.

pub mod history_list;
pub mod view;

use gpui::{
    App, Bounds, Context, DisplayId, Entity, Global, Size, WindowBackgroundAppearance,
    WindowBounds, WindowHandle, WindowKind, WindowOptions, layer_shell::*, point, prelude::*, px,
};

use crate::notifications::view::NotificationsView;
use crate::state::{self, AppState};

use chronos_services::{NotificationState, Service};

/// Width of the notification popup column (px). Matches the mockup's
/// 340 px so the stack sits comfortably in the top-right corner without
/// dominating the screen (mockup: `width: 340px`).
const POPUP_WIDTH: f32 = 340.;

/// Hard pixel clip on the card stack container (`view.rs`:
/// `.max_h(px(LIST_MAX_H)).overflow_hidden()`). The window itself is a fixed
/// cap of exactly this height, so the stack can never push the surface
/// taller regardless of how many notifications are queued or how tall each
/// card renders. A flood of notifications is silently clipped at the bottom
/// (they already expire on a timer, so losing old ones off-screen is
/// acceptable — unlike a privileged action button, which MUST stay visible).
///
/// Raised from 360 → 480 for T124 to accommodate taller toast cards (icon
/// row + summary + body + actions + progress bar ~150 px per card, 3 cards
/// ~450 px with gaps). Still a hard cap — older toasts clip off the bottom
/// and expire.
pub(crate) const LIST_MAX_H: f32 = 480.;

/// Window height while open. A single fixed cap — the popup is only ever
/// opened when there is at least one notification, and its size does not
/// depend on content (see the module docs above). Mirrors
/// `updates_popup::MAX_POPUP_H`.
const POPUP_HEIGHT: f32 = LIST_MAX_H;

/// Hard clip on a single card's body text (px) — ~5 lines at the body line
/// height. A long body is truncated by clip rather than allowed to overflow
/// the card and collide with the next one.
pub(crate) const BODY_MAX_H: f32 = 90.;

/// Global state for the notifications popup: the last snapshot we rendered,
/// the open window handle, and the watcher entity that drives updates.
#[derive(Default)]
pub struct NotificationPopupState {
    handle: Option<WindowHandle<NotificationsView>>,
    watcher: Option<Entity<NotificationWatcher>>,
    current: NotificationState,
}

impl Global for NotificationPopupState {}

/// Tiny entity that hosts the `state::watch()` subscription. It has no state
/// of its own — `watch` needs an entity/Context to spawn its update loop.
pub struct NotificationWatcher {}

/// Resolve the display to anchor the popup on (primary, else first).
fn pick_display(cx: &App) -> Option<DisplayId> {
    crate::monitor::pult_display(cx)
}

/// Layer-shell window options for the notifications popup.
///
/// TOP | RIGHT anchored, `Layer::Overlay`, **never** exclusive (exclusive
/// zones are forbidden for popups — they must not reserve compositor space),
/// and `KeyboardInteractivity::None` (the popup is purely mouse/click driven;
/// the keyboard has no business here). Height is the fixed `POPUP_HEIGHT`
/// cap — content is clipped to fit, never allowed to resize the surface.
///
/// Margins: top 12 px (screen top ~42 px minus 30 px bar exclusive zone),
/// right 16 px (mockup: `top: 42px; right: 16px`).
fn window_options(display_id: Option<DisplayId>) -> WindowOptions {
    WindowOptions {
        display_id,
        titlebar: None,
        window_bounds: Some(WindowBounds::Windowed(Bounds {
            origin: point(px(0.), px(0.)),
            size: Size::new(px(POPUP_WIDTH), px(POPUP_HEIGHT)),
        })),
        app_id: Some("chronos-notifications".to_string()),
        window_background: WindowBackgroundAppearance::Transparent,
        kind: WindowKind::LayerShell(LayerShellOptions {
            namespace: "notifications".to_string(),
            layer: Layer::Overlay,
            anchor: Anchor::TOP | Anchor::RIGHT,
            exclusive_zone: None,
            margin: Some((px(12.), px(16.), px(0.), px(0.))),
            keyboard_interactivity: KeyboardInteractivity::None,
            ..Default::default()
        }),
        ..Default::default()
    }
}

/// Reconcile the window's open/closed state with the current snapshot:
/// no notifications → close; any notifications → ensure open (and repaint).
fn sync_window(cx: &mut App) {
    let is_empty = cx
        .global::<NotificationPopupState>()
        .current
        .notifications
        .is_empty();

    if is_empty {
        if let Some(handle) = cx.global_mut::<NotificationPopupState>().handle.take() {
            let _ = handle.update(cx, |_, window: &mut gpui::Window, _| window.remove_window());
        }
        return;
    }

    // Non-empty: ensure a window exists.
    let handle = cx.global::<NotificationPopupState>().handle.clone();
    match handle {
        Some(existing) => {
            // Window already open. Repaint with the new snapshot (the surface
            // is a fixed cap — the inner view may have updated progress).
            let _ = existing.update(cx, |_, _window, view_cx| {
                view_cx.notify();
            });
        }
        None => {
            let display_id = pick_display(cx);
            match cx.open_window(window_options(display_id), |_, app_cx| {
                app_cx.new(|view_cx| NotificationsView::new(view_cx))
            }) {
                Ok(new_handle) => {
                    cx.global_mut::<NotificationPopupState>().handle = Some(new_handle);
                }
                Err(err) => tracing::warn!("Failed to open notifications window: {}", err),
            }
        }
    }
}

/// Register the notifications global and subscribe it to the live
/// `NotificationState` stream from the daemon.
///
/// Called once at startup from `main.rs` (the orchestrator wires this in).
pub fn init(cx: &mut App) {
    cx.set_global(NotificationPopupState::default());

    let signal = AppState::notification(cx).subscribe();

    // Host the watch loop on a tiny entity (watch needs a `Context<C>`).
    let watcher = cx.new(|cx| {
        state::watch(
            cx,
            signal,
            |_this: &mut NotificationWatcher,
             state: NotificationState,
             cx: &mut Context<NotificationWatcher>| {
                cx.global_mut::<NotificationPopupState>().current = state;
                sync_window(cx);
            },
        );
        NotificationWatcher {}
    });

    cx.global_mut::<NotificationPopupState>().watcher = Some(watcher);
}

/// Push an internal (non-FDO) notification. Used for system events like
/// display hotplug that don't go through the D-Bus daemon.
pub fn push_internal(cx: &mut App, summary: &str, body: &str) {
    use chronos_services::notification::types::{Notification, Urgency};

    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;

    {
        let state = cx.global_mut::<NotificationPopupState>();
        let id = state.current.next_id;
        state.current.next_id += 1;
        let note = Notification {
            id,
            app_name: "ChronOS".into(),
            app_icon: String::new(),
            summary: summary.into(),
            body: body.into(),
            urgency: Urgency::Normal,
            actions: vec![],
            expire_at: Some(now_ms + 10_000),
        };
        state.current.notifications.push(note.clone());
        state.current.push_history(note);
        state.current.unread += 1;
        state.current.recompute_flags();
    }
    sync_window(cx);
}
