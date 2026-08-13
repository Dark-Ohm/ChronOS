//! Volume popup — Sound UI: Volume + Microphone sliders, device menus,
//! footer dual mute. Anchored to the bar volume widget (LayerShell
//! fallback on platforms without `AnchoredPopup`).
//!
//! Opened by clicking the bar volume widget. Window lifecycle mirrors
//! `updates_popup/`: anchored popup, no close-on-focus-loss (only explicit
//! toggle / ✕). In-popup close uses `close_this` (direct
//! `remove_window`) — never re-entrant `handle.update`.

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

use chronos_services::{AudioState, Service};

use crate::state::{self, AppState};
use crate::volume_popup::view::VolumePopupView;

/// Popup width (px). Mockup 360.
pub(crate) const POPUP_WIDTH: f32 = 360.;
/// Base height without any device list expanded.
/// Tall enough for: header (~37) + Volume endpoint (~66) + divider (1) +
/// Microphone endpoint (~66) + footer dual-mute (~52) ≈ 222, rounded up
/// with slack so the footer is never clipped by the window bounds.
const BASE_HEIGHT: f32 = 290.;
/// Budget per device row when a picker is open.
const DEVICE_ROW_H: f32 = 28.;
/// Cap expanded list so the popup does not eat the whole screen.
const MAX_DEVICE_ROWS: usize = 8;
/// Below the bar top edge — same budget as updates_popup / tray_menu.
const POPUP_MARGIN_TOP: f32 = 36.;
const POPUP_MARGIN_RIGHT: f32 = 8.;

/// How tall the window should be for the current audio state + expanded picker.
pub(crate) fn estimate_popup_height(
    state: &AudioState,
    expanded: Option<view::EndpointKind>,
) -> f32 {
    let extra = match expanded {
        Some(view::EndpointKind::Sink) => {
            state.sink.available.len().min(MAX_DEVICE_ROWS) as f32 * DEVICE_ROW_H
        }
        Some(view::EndpointKind::Source) => {
            state.source.available.len().min(MAX_DEVICE_ROWS) as f32 * DEVICE_ROW_H
        }
        None => 0.,
    };
    BASE_HEIGHT + extra
}

/// Global state for the volume popup.
#[derive(Default)]
pub struct VolumePopupState {
    handle: Option<WindowHandle<VolumePopupView>>,
    watcher: Option<Entity<VolumePopupWatcher>>,
}

impl Global for VolumePopupState {}

/// Hosts the `state::watch()` subscription; no state of its own.
pub struct VolumePopupWatcher {}

fn pick_display(cx: &App) -> Option<DisplayId> {
    crate::monitor::pult_display(cx)
}

/// Layer-shell window options — TOP | RIGHT, overlay, never exclusive, no
/// keyboard interactivity. Used as fallback when `AnchoredPopup` isn't
/// supported on this platform (mirrors `updates_popup`).
fn fallback_window_options(display_id: Option<DisplayId>, height: f32) -> WindowOptions {
    WindowOptions {
        display_id,
        titlebar: None,
        window_bounds: Some(WindowBounds::Windowed(Bounds {
            origin: point(px(0.), px(0.)),
            size: Size::new(px(POPUP_WIDTH), px(height)),
        })),
        app_id: Some("chronos-volume-popup".to_string()),
        window_background: WindowBackgroundAppearance::Transparent,
        kind: WindowKind::LayerShell(LayerShellOptions {
            namespace: "volume-popup".to_string(),
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

/// Anchored popup — positioned relative to the volume widget's bounds,
/// extending down-and-left from the icon's bottom-right corner.
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
        app_id: Some("chronos-volume-popup".to_string()),
        window_background: WindowBackgroundAppearance::Transparent,
        kind: WindowKind::AnchoredPopup(PopupOptions {
            parent,
            anchor_rect,
            anchor: PopupAnchor::BottomRight,
            gravity: PopupGravity::BottomLeft,
            constraint_adjustment: PopupConstraintAdjustment::SLIDE_X
                | PopupConstraintAdjustment::FLIP_X,
            offset: point(px(0.), px(4.)),
            // T264 A2: Hyprland 0.56.x can retain this xdg-popup's seat grab
            // after client-side destruction and then drop compositor-wide
            // pointer input — the whole session loses mouse until relogin.
            // Dismissal is ours (click-away / Escape / re-toggle), so no bar
            // popup asks the compositor for a grab.
            grab: false,
        }),
        ..Default::default()
    }
}

/// Open the popup (idempotent — no-op if already open). Falls back to a
/// fixed-corner LayerShell popup when `AnchoredPopup` isn't supported.
pub fn open(cx: &mut App, anchor_rect: Bounds<Pixels>, parent: AnyWindowHandle) {
    if cx.global::<VolumePopupState>().handle.is_some() {
        return;
    }

    let height = estimate_popup_height(&AppState::audio(cx).get(), None);

    let result = cx.open_window(window_options(anchor_rect, parent, height), |_, app_cx| {
        app_cx.new(|view_cx| VolumePopupView::new(view_cx))
    });

    let result = match result {
        Err(err) => {
            if err.downcast_ref::<PopupNotSupportedError>().is_some() {
                tracing::warn!(
                    "volume_popup: AnchoredPopup not supported on this platform, falling back to fixed-corner LayerShell"
                );
                let display_id = pick_display(cx);
                cx.open_window(fallback_window_options(display_id, height), |_, app_cx| {
                    app_cx.new(|view_cx| VolumePopupView::new(view_cx))
                })
            } else {
                Err(err)
            }
        }
        ok => ok,
    };

    match result {
        Ok(new_handle) => {
            cx.global_mut::<VolumePopupState>().handle = Some(new_handle);
        }
        Err(err) => tracing::warn!("volume_popup: failed to open popup: {err}"),
    }
}

/// Close from outside the popup (bar toggle). Uses `handle.update`.
pub fn close(cx: &mut App) {
    if let Some(handle) = cx.global_mut::<VolumePopupState>().handle.take() {
        if let Err(e) = handle.update(cx, |_, window: &mut Window, _| window.remove_window()) {
            tracing::warn!("volume_popup: close remove_window failed (already dead?): {e}");
        }
    }
}

/// Close from inside a callback that already holds `&mut Window` for this
/// popup (✕ button). Must not re-enter `handle.update` on the same id.
pub(crate) fn close_this(window: &mut Window, cx: &mut App) {
    let this = window.window_handle();
    let tracked = cx
        .global::<VolumePopupState>()
        .handle
        .as_ref()
        .map(|h| **h == this)
        .unwrap_or(false);
    if tracked {
        cx.global_mut::<VolumePopupState>().handle.take();
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
    if cx.global::<VolumePopupState>().handle.is_some() {
        close(cx);
    } else {
        open(cx, anchor_rect, parent);
    }
}

/// Resize the open popup to fit expanded device list (if any).
pub(crate) fn resize_to_fit(window: &mut Window, expanded: Option<view::EndpointKind>, cx: &App) {
    let height = estimate_popup_height(&AppState::audio(cx).get(), expanded);
    window.resize(Size::new(px(POPUP_WIDTH), px(height)));
}

/// Wire the volume popup to the live audio service. Called once from `main.rs`.
pub fn init(cx: &mut App) {
    cx.set_global(VolumePopupState::default());

    let signal = AppState::audio(cx).subscribe();

    let watcher = cx.new(|cx| {
        state::watch(
            cx,
            signal,
            |_this: &mut VolumePopupWatcher,
             audio: AudioState,
             cx: &mut Context<VolumePopupWatcher>| {
                let handle = cx.global::<VolumePopupState>().handle.clone();
                if let Some(handle) = handle {
                    let _ = handle.update(
                        cx,
                        |view: &mut VolumePopupView, window: &mut Window, view_cx| {
                            let height = estimate_popup_height(&audio, view.expanded());
                            window.resize(Size::new(px(POPUP_WIDTH), px(height)));
                            view_cx.notify();
                        },
                    );
                }
            },
        );
        VolumePopupWatcher {}
    });

    cx.global_mut::<VolumePopupState>().watcher = Some(watcher);
    tracing::info!("volume_popup: subscribed to audio service");
}
