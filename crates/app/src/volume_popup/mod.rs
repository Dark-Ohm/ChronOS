//! Volume popup — Speakers + Microphone levels with fill-bars, steppers, and
//! an in-popup device picker (expand under the section title).
//!
//! Opened by clicking the bar volume widget. Window lifecycle mirrors
//! `updates_popup/` / `tray_menu/`: layer-shell Overlay, no exclusive keyboard,
//! no close-on-focus-loss (only explicit toggle / ✕). In-popup close uses
//! `close_this` (direct `remove_window`) — never re-entrant `handle.update`.

pub mod view;

use gpui::{
    App, Bounds, Context, DisplayId, Entity, Global, Size, Window, WindowBackgroundAppearance,
    WindowBounds, WindowHandle, WindowKind, WindowOptions, layer_shell::*, point, prelude::*, px,
};

use chronos_services::{AudioState, Service};

use crate::state::{self, AppState};
use crate::volume_popup::view::VolumePopupView;

/// Popup width (px). Spec ~300.
pub(crate) const POPUP_WIDTH: f32 = 300.;
/// Base height without any device list expanded.
const BASE_HEIGHT: f32 = 220.;
/// Budget per device row when a picker is open.
const DEVICE_ROW_H: f32 = 28.;
/// Cap expanded list so the popup does not eat the whole screen.
const MAX_DEVICE_ROWS: usize = 8;
/// Below the bar top edge — same budget as updates_popup / tray_menu.
const POPUP_MARGIN_TOP: f32 = 36.;
const POPUP_MARGIN_RIGHT: f32 = 8.;

/// How tall the window should be for the current audio state + expanded picker.
pub(crate) fn estimate_popup_height(state: &AudioState, expanded: Option<view::EndpointKind>) -> f32 {
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

fn window_options(display_id: Option<DisplayId>, height: f32) -> WindowOptions {
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

/// Open the popup (idempotent — no-op if already open).
pub fn open(cx: &mut App) {
    if cx.global::<VolumePopupState>().handle.is_some() {
        return;
    }

    let display_id = pick_display(cx);
    let height = estimate_popup_height(&AppState::audio(cx).get(), None);
    match cx.open_window(window_options(display_id, height), |_, app_cx| {
        app_cx.new(|view_cx| VolumePopupView::new(view_cx))
    }) {
        Ok(new_handle) => {
            cx.global_mut::<VolumePopupState>().handle = Some(new_handle);
        }
        Err(err) => tracing::warn!("volume_popup: failed to open popup: {err}"),
    }
}

/// Close from outside the popup (bar toggle). Uses `handle.update`.
pub fn close(cx: &mut App) {
    if let Some(handle) = cx.global_mut::<VolumePopupState>().handle.take() {
        let _ = handle.update(cx, |_, window: &mut Window, _| window.remove_window());
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
pub fn toggle(_window: &mut Window, cx: &mut App) {
    if cx.global::<VolumePopupState>().handle.is_some() {
        close(cx);
    } else {
        open(cx);
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
                    let _ = handle.update(cx, |view: &mut VolumePopupView, window: &mut Window, view_cx| {
                        let height = estimate_popup_height(&audio, view.expanded());
                        window.resize(Size::new(px(POPUP_WIDTH), px(height)));
                        view_cx.notify();
                    });
                }
            },
        );
        VolumePopupWatcher {}
    });

    cx.global_mut::<VolumePopupState>().watcher = Some(watcher);
    tracing::info!("volume_popup: subscribed to audio service");
}
