//! Notifications module: a layer-shell popup surface that renders the live
//! stack of active notifications from our `org.freedesktop.Notifications`
//! daemon.
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

pub mod view;

use gpui::{
    App, Bounds, Context, DisplayId, Entity, Global, Size, WindowBackgroundAppearance,
    WindowBounds, WindowHandle, WindowKind, WindowOptions, layer_shell::*, point, prelude::*, px,
};

use crate::notifications::view::NotificationsView;
use crate::state::{self, AppState};

use chronos_services::{NotificationState, Service};

/// Width of the notification popup column (px). Narrow so it hugs the
/// top-right corner without dominating the screen.
const POPUP_WIDTH: f32 = 360.;

/// Per-card vertical geometry estimates (px). Used to rubber-band the
/// surface height to its real content. The numbers are deliberately simple:
/// a layer-shell surface does NOT auto-size to its children, so we must size
/// the window ourselves. The surface ALSO enables internal vertical scroll
/// past `MAX_POPUP_HEIGHT`, so small estimation errors never clip content —
/// a short estimate just trades a little wasted space for scroll-room, and a
/// long estimate caps at the max and scrolls.
const CARD_PAD_Y: f32 = 12.;
const HEADER_H: f32 = 16.;
const TITLE_H: f32 = 18.;
const BODY_LINE_H: f32 = 18.;
const ACTION_H: f32 = 26.;
const CARD_INNER_GAP: f32 = 4.;
const STACK_GAP: f32 = 8.;
/// Approx glyphs that fit on one body line at `POPUP_WIDTH` minus card
/// padding (336px / ~7.6px per glyph ≈ 44).
const BODY_CHARS_PER_LINE: f32 = 44.;
/// Floor so a single tiny notification never collapses the surface to nothing.
const MIN_POPUP_HEIGHT: f32 = 48.;

/// Cap the surface height to a fraction of the display so a flood of
/// notifications (or one with a huge body) can't cover the whole screen.
/// Past the cap the inner stack scrolls instead of growing the window.
fn max_popup_height(cx: &App) -> f32 {
    let display_h = cx
        .primary_display()
        .or_else(|| cx.displays().into_iter().next())
        .map(|d| f32::from(d.bounds().size.height))
        .unwrap_or(1080.);
    (display_h * 0.4).clamp(160., 560.)
}

/// Estimate the content height (px) for the current notification stack.
fn estimate_content_height(state: &NotificationState) -> f32 {
    if state.notifications.is_empty() {
        return MIN_POPUP_HEIGHT;
    }
    let mut total = 0.;
    let mut first = true;
    for n in &state.notifications {
        if !first {
            total += STACK_GAP;
        }
        first = false;

        let body_lines = if n.body.is_empty() {
            0.
        } else {
            ((n.body.chars().count() as f32) / BODY_CHARS_PER_LINE).ceil().max(1.)
        };
        let mut card =
            HEADER_H + CARD_INNER_GAP + TITLE_H + CARD_INNER_GAP + BODY_LINE_H * body_lines;
        if !n.actions.is_empty() {
            card += CARD_INNER_GAP + ACTION_H;
        }
        card += CARD_PAD_Y * 2.;
        total += card;
    }
    total.max(MIN_POPUP_HEIGHT)
}

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
    cx.primary_display()
        .map(|d| d.id())
        .or_else(|| cx.displays().into_iter().next().map(|d| d.id()))
}

/// Layer-shell window options for the notifications popup.
///
/// TOP | RIGHT anchored, `Layer::Overlay`, **never** exclusive (exclusive
/// zones are forbidden for popups — they must not reserve compositor space),
/// and `KeyboardInteractivity::None` (the popup is purely mouse/click driven;
/// the keyboard has no business here).
fn window_options(display_id: Option<DisplayId>, state: &NotificationState) -> WindowOptions {
    let height = estimate_content_height(state).min(max_popup_height_owned());
    WindowOptions {
        display_id,
        titlebar: None,
        window_bounds: Some(WindowBounds::Windowed(Bounds {
            origin: point(px(0.), px(0.)),
            size: Size::new(px(POPUP_WIDTH), px(height)),
        })),
        app_id: Some("chronos-notifications".to_string()),
        window_background: WindowBackgroundAppearance::Transparent,
        kind: WindowKind::LayerShell(LayerShellOptions {
            namespace: "notifications".to_string(),
            layer: Layer::Overlay,
            anchor: Anchor::TOP | Anchor::RIGHT,
            exclusive_zone: None,
            margin: Some((px(12.), px(12.), px(12.), px(12.))),
            keyboard_interactivity: KeyboardInteractivity::None,
            ..Default::default()
        }),
        ..Default::default()
    }
}

/// `max_popup_height` needs an `&App`; this is the no-context variant used at
/// window-creation time before we have one (falls back to the 1080p clamp).
fn max_popup_height_owned() -> f32 {
    560_f32.clamp(160., 560.)
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
            // Window already open — repaint with the new snapshot AND resize
            // the layer-shell surface to fit the new content (a long body or a
            // second notification makes the old fixed height clip).
            let height = {
                let state = &cx.global::<NotificationPopupState>().current;
                estimate_content_height(state).min(max_popup_height(cx))
            };
            let _ = existing.update(cx, |_, window: &mut gpui::Window, _| {
                window.resize(Size::new(px(POPUP_WIDTH), px(height)));
            });
            let _ = existing.update(cx, |_, _window, view_cx| {
                view_cx.notify();
            });
        }
        None => {
            let display_id = pick_display(cx);
            let state = &cx.global::<NotificationPopupState>().current;
            match cx.open_window(window_options(display_id, state), |_, app_cx| {
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
