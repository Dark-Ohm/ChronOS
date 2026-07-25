//! System popup — brightness + power profile + gaming mode toggle.
//! Anchored to the bar system widget (LayerShell fallback on platforms
//! without `AnchoredPopup`).
//!
//! Opened by clicking the bar system widget. Window lifecycle mirrors
//! `volume_popup/`: anchored popup, no close-on-focus-loss (only explicit
//! toggle / ✕). In-popup close uses `close_this` (direct
//! `remove_window`) — never re-entrant `handle.update`.

pub mod gaming_mode;
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

use chronos_services::{BrightnessState, Service, UPowerData};

use crate::state::{self, AppState};
use crate::system_popup::gaming_mode::GamingModeState;
use crate::system_popup::view::SystemPopupView;

/// Popup width (px). Mockup 360.
pub(crate) const POPUP_WIDTH: f32 = 360.;
/// Fixed height — all three blocks (brightness, power, gaming) are always
/// shown, so height does not depend on data.
const BASE_HEIGHT: f32 = 274.;
/// Below the bar top edge — same budget as volume_popup / updates_popup.
const POPUP_MARGIN_TOP: f32 = 36.;
const POPUP_MARGIN_RIGHT: f32 = 8.;

pub(crate) fn estimate_popup_height() -> f32 {
    BASE_HEIGHT
}

/// Global state for the system popup.
#[derive(Default)]
pub struct SystemPopupState {
    handle: Option<WindowHandle<SystemPopupView>>,
    brightness_watcher: Option<Entity<SystemPopupBrightnessWatcher>>,
    upower_watcher: Option<Entity<SystemPopupUPowerWatcher>>,
}

impl Global for SystemPopupState {}

/// Hosts the `state::watch()` subscription for brightness.
pub struct SystemPopupBrightnessWatcher {}

/// Hosts the `state::watch()` subscription for UPower (power profile changes
/// from outside — e.g. `powerprofilesctl set` in another terminal).
pub struct SystemPopupUPowerWatcher {}

fn pick_display(cx: &App) -> Option<DisplayId> {
    crate::monitor::pult_display(cx)
}

/// Layer-shell window options — TOP | RIGHT, overlay, never exclusive, no
/// keyboard interactivity. Used as fallback when `AnchoredPopup` isn't
/// supported on this platform.
fn fallback_window_options(display_id: Option<DisplayId>) -> WindowOptions {
    WindowOptions {
        display_id,
        titlebar: None,
        window_bounds: Some(WindowBounds::Windowed(Bounds {
            origin: point(px(0.), px(0.)),
            size: Size::new(px(POPUP_WIDTH), px(estimate_popup_height())),
        })),
        app_id: Some("chronos-system-popup".to_string()),
        window_background: WindowBackgroundAppearance::Transparent,
        kind: WindowKind::LayerShell(LayerShellOptions {
            namespace: "system-popup".to_string(),
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

/// Anchored popup — positioned relative to the system widget's bounds,
/// extending down-and-left from the icon's bottom-right corner.
fn window_options(
    anchor_rect: Bounds<Pixels>,
    parent: AnyWindowHandle,
) -> WindowOptions {
    WindowOptions {
        titlebar: None,
        window_bounds: Some(WindowBounds::Windowed(Bounds {
            origin: point(px(0.), px(0.)),
            size: Size::new(px(POPUP_WIDTH), px(estimate_popup_height())),
        })),
        app_id: Some("chronos-system-popup".to_string()),
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

/// Open the popup (idempotent — no-op if already open). Falls back to a
/// fixed-corner LayerShell popup when `AnchoredPopup` isn't supported.
pub fn open(cx: &mut App, anchor_rect: Bounds<Pixels>, parent: AnyWindowHandle) {
    if cx.global::<SystemPopupState>().handle.is_some() {
        return;
    }

    AppState::brightness(cx).dispatch(chronos_services::BrightnessCommand::Refresh);

    let result = cx.open_window(window_options(anchor_rect, parent), |_, app_cx| {
        app_cx.new(|view_cx| SystemPopupView::new(view_cx))
    });

    let result = match result {
        Err(err) => {
            if err.downcast_ref::<PopupNotSupportedError>().is_some() {
                tracing::warn!(
                    "system_popup: AnchoredPopup not supported on this platform, falling back to fixed-corner LayerShell"
                );
                let display_id = pick_display(cx);
                cx.open_window(fallback_window_options(display_id), |_, app_cx| {
                    app_cx.new(|view_cx| SystemPopupView::new(view_cx))
                })
            } else {
                Err(err)
            }
        }
        ok => ok,
    };

    match result {
        Ok(new_handle) => {
            cx.global_mut::<SystemPopupState>().handle = Some(new_handle);
        }
        Err(err) => tracing::warn!("system_popup: failed to open popup: {err}"),
    }
}

/// Close from outside the popup (bar toggle). Uses `handle.update`.
pub fn close(cx: &mut App) {
    if let Some(handle) = cx.global_mut::<SystemPopupState>().handle.take() {
        if let Err(e) = handle.update(cx, |_, window: &mut Window, _| window.remove_window()) {
            tracing::warn!("system_popup: close remove_window failed (already dead?): {e}");
        }
    }
}

/// Close from inside a callback that already holds `&mut Window` for this
/// popup (✕ button). Must not re-enter `handle.update` on the same id.
pub(crate) fn close_this(window: &mut Window, cx: &mut App) {
    let this = window.window_handle();
    let tracked = cx
        .global::<SystemPopupState>()
        .handle
        .as_ref()
        .map(|h| **h == this)
        .unwrap_or(false);
    if tracked {
        cx.global_mut::<SystemPopupState>().handle.take();
    }
    window.remove_window();
}

/// Bar-icon toggle. Caller's window is the bar, not the popup → use `close`.
pub fn toggle(
    anchor_rect: Bounds<Pixels>,
    parent: AnyWindowHandle,
    _window: &mut Window,
    cx: &mut App,
) {
    if cx.global::<SystemPopupState>().handle.is_some() {
        close(cx);
    } else {
        open(cx, anchor_rect, parent);
    }
}

/// Wire the system popup to the live brightness + upower services. Called
/// once from `main.rs`.
pub fn init(cx: &mut App) {
    cx.set_global(SystemPopupState::default());
    GamingModeState::init(cx);

    let brightness_signal = AppState::brightness(cx).subscribe();
    let brightness_watcher = cx.new(|cx| {
        state::watch(
            cx,
            brightness_signal,
            |_this: &mut SystemPopupBrightnessWatcher,
             _brightness: BrightnessState,
             cx: &mut Context<SystemPopupBrightnessWatcher>| {
                let handle = cx.global::<SystemPopupState>().handle.clone();
                if let Some(handle) = handle {
                    let _ = handle.update(cx, |view: &mut SystemPopupView, _window, view_cx| {
                        view_cx.notify();
                    });
                }
            },
        );
        SystemPopupBrightnessWatcher {}
    });
    cx.global_mut::<SystemPopupState>().brightness_watcher = Some(brightness_watcher);

    let upower_signal = AppState::upower(cx).subscribe();
    let upower_watcher = cx.new(|cx| {
        state::watch(
            cx,
            upower_signal,
            |_this: &mut SystemPopupUPowerWatcher,
             _upower: UPowerData,
             cx: &mut Context<SystemPopupUPowerWatcher>| {
                let handle = cx.global::<SystemPopupState>().handle.clone();
                if let Some(handle) = handle {
                    let _ = handle.update(cx, |view: &mut SystemPopupView, _window, view_cx| {
                        view_cx.notify();
                    });
                }
            },
        );
        SystemPopupUPowerWatcher {}
    });
    cx.global_mut::<SystemPopupState>().upower_watcher = Some(upower_watcher);

    tracing::info!("system_popup: subscribed to brightness + upower services");
}
