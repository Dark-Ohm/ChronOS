//! System popup — brightness + power profile + gaming mode toggle.
//!
//! Opened by clicking the bar's system widget (⚙). Window lifecycle mirrors
//! `volume_popup/` / `updates_popup/` / `tray_menu/`: layer-shell Overlay,
//! TOP|RIGHT, no exclusive keyboard, **no close-on-focus-loss** (only
//! explicit toggle / ✕ / Esc). In-popup close uses `close_this` (direct
//! `remove_window`) — never re-entrant `handle.update` (ghost-window saga,
//! HANDOFF.md "СИСТЕМНЫЙ БАГ: window.remove_window()").
//!
//! Visual spec: `design/System Popup.dc.html` + `design.md` §6.

pub mod gaming_mode;
pub mod view;

use gpui::{
    App, Bounds, Context, DisplayId, Entity, Global, Size, Window, WindowBackgroundAppearance,
    WindowBounds, WindowHandle, WindowKind, WindowOptions, layer_shell::*, point, prelude::*, px,
};

use chronos_services::{BrightnessState, Service, UPowerData};

use crate::state::{self, AppState};
use crate::system_popup::gaming_mode::GamingModeState;
use crate::system_popup::view::SystemPopupView;

/// Popup width (px). Matches volume_popup / design mockup.
pub(crate) const POPUP_WIDTH: f32 = 300.;
/// Fixed height — the popup always shows all three blocks (brightness,
/// power profile, gaming mode), so the height does not depend on data.
/// Budget: header 48 + divider 1 + brightness 80 + divider 1 + power 70 +
/// divider 1 + gaming 80 = 281. Rounded up for padding slack.
const POPUP_HEIGHT: f32 = 284.;
/// Below the bar top edge — same budget as volume_popup / updates_popup.
const POPUP_MARGIN_TOP: f32 = 36.;
const POPUP_MARGIN_RIGHT: f32 = 8.;

/// Global state for the system popup.
#[derive(Default)]
pub struct SystemPopupState {
    handle: Option<WindowHandle<SystemPopupView>>,
    /// Watches brightness + upower signals; repaints the popup on change.
    brightness_watcher: Option<Entity<SystemPopupBrightnessWatcher>>,
    upower_watcher: Option<Entity<SystemPopupUPowerWatcher>>,
}

impl Global for SystemPopupState {}

/// Hosts the `state::watch()` subscription for brightness; no state of its own.
pub struct SystemPopupBrightnessWatcher {}

/// Hosts the `state::watch()` subscription for UPower (power profile changes
/// from outside — e.g. `powerprofilesctl set` in another terminal).
pub struct SystemPopupUPowerWatcher {}

fn pick_display(cx: &App) -> Option<DisplayId> {
    crate::monitor::pult_display(cx)
}

fn window_options(display_id: Option<DisplayId>) -> WindowOptions {
    WindowOptions {
        display_id,
        titlebar: None,
        window_bounds: Some(WindowBounds::Windowed(Bounds {
            origin: point(px(0.), px(0.)),
            size: Size::new(px(POPUP_WIDTH), px(POPUP_HEIGHT)),
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

/// Open the popup (idempotent — no-op if already open). Triggers a brightness
/// refresh so the slider reflects the live monitor state, not a stale init
/// value.
///
/// `display_id` — the pult (chrome) display from `crate::monitor::pult_display`,
/// so the popup opens on the same monitor as the bar. Falls back to
/// `pick_display` only if the caller passes `None`.
pub fn open(display_id: Option<DisplayId>, cx: &mut App) {
    if cx.global::<SystemPopupState>().handle.is_some() {
        return;
    }

    // Refresh brightness on open — DDC latency is 100-300ms, so we don't poll
    // on a frame cadence; this is the moment to re-read.
    AppState::brightness(cx).dispatch(chronos_services::BrightnessCommand::Refresh);

    let display_id = display_id.or_else(|| pick_display(cx));
    match cx.open_window(window_options(display_id), |_, app_cx| {
        app_cx.new(|view_cx| SystemPopupView::new(view_cx))
    }) {
        Ok(new_handle) => {
            cx.global_mut::<SystemPopupState>().handle = Some(new_handle);
        }
        Err(err) => tracing::warn!("system_popup: failed to open popup: {err}"),
    }
}

/// Close from outside the popup (bar toggle). Uses `handle.update`.
pub fn close(cx: &mut App) {
    if let Some(handle) = cx.global_mut::<SystemPopupState>().handle.take() {
        let _ = handle.update(cx, |_, window: &mut Window, _| window.remove_window());
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
/// The popup opens on the same display as the clicked bar window, so on a
/// multi-monitor setup it follows the cursor instead of always landing on the
/// first display in `cx.displays()` (which is what `pick_display` alone does
/// when `cx.primary_display()` returns None — the current state on Hyprland
/// 0.55.4+ with the Lua config layer).
pub fn toggle(_window: &mut Window, cx: &mut App) {
    if cx.global::<SystemPopupState>().handle.is_some() {
        close(cx);
    } else {
        let display = crate::monitor::pult_display(cx);
        open(display, cx);
    }
}

/// Wire the system popup to the live brightness + upower services. Called
/// once from `main.rs`.
pub fn init(cx: &mut App) {
    cx.set_global(SystemPopupState::default());
    GamingModeState::init(cx);

    // Brightness watcher — repaint on brightness change (after a step, the
    // service re-reads and emits; the popup slider follows).
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

    // UPower watcher — repaint when the power profile changes from outside
    // (e.g. `powerprofilesctl set performance` in another terminal, or the
    // battery widget's cycle-on-click).
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
