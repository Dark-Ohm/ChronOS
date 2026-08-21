//! Calendar popup — kit `Calendar` (from `gpui-component/time`) shown when
//! the bar clock widget is clicked. Anchored to the bar clock, LayerShell
//! fallback when `AnchoredPopup` is not supported.
//!
//! Lifecycle mirrors `volume_popup/`: idempotent open, dismissal ours
//! (click-away via the shared click-catcher, re-toggle), `close_this`
//! reentrancy guard around `window.remove_window()`.

pub mod view;

use std::rc::Rc;

use gpui::{
    AnyWindowHandle, App, Bounds, DisplayId, Global, Pixels, Size, Subscription, Window,
    WindowBackgroundAppearance, WindowBounds, WindowHandle, WindowId, WindowKind, WindowOptions,
    layer_shell::*,
    point,
    popup::{
        PopupAnchor, PopupConstraintAdjustment, PopupGravity, PopupNotSupportedError, PopupOptions,
    },
    prelude::*,
    px,
};

use gpui_component::Root;

use crate::calendar_popup::view::CalendarPopupView;

/// Popup width (px) — a bit narrower than the updates popup (420) so the
/// calendar card reads as a compact drop-down, not a settings panel.
const POPUP_WIDTH: f32 = 300.0;
/// Below the bar top edge — same budget as updates_popup / volume_popup.
const POPUP_MARGIN_TOP: f32 = 36.;
const POPUP_MARGIN_RIGHT: f32 = 8.;

/// Measured from the kit calendar's rendered geometry:
/// header (prev/next + month/year) ~36, weekday row ~28,
/// 6 weeks * ~34 (day buttons ~34 tall incl. gap) ~204,
/// padding ~24 (p_3 = 12*2) → ~292, rounded up with slack so the
/// footer clip trap (~30px slack per T295 spec) never triggers.
const POPUP_HEIGHT: f32 = 320.;

/// Global state for the calendar popup.
#[derive(Default)]
pub struct CalendarPopupState {
    handle: Option<WindowHandle<Root>>,
    /// Transparent layer-surface that receives clicks outside the popup while
    /// the native popup intentionally has `grab: false` (T264).
    click_catcher: Option<AnyWindowHandle>,
    /// Clears stale state when the compositor closes the popup externally
    /// (bypassing our explicit close paths).
    window_closed_subscription: Option<Subscription>,
}

impl Global for CalendarPopupState {}

fn pick_display(cx: &App) -> Option<DisplayId> {
    crate::monitor::pult_display(cx)
}

/// Layer-shell window options — TOP | RIGHT, overlay, never exclusive, no
/// keyboard interactivity (mouse-driven). Used as fallback when
/// `AnchoredPopup` isn't supported on this platform.
fn fallback_window_options(display_id: Option<DisplayId>) -> WindowOptions {
    WindowOptions {
        display_id,
        titlebar: None,
        window_bounds: Some(WindowBounds::Windowed(Bounds {
            origin: point(px(0.), px(0.)),
            size: Size::new(px(POPUP_WIDTH), px(POPUP_HEIGHT)),
        })),
        app_id: Some("chronos-calendar-popup".to_string()),
        window_background: WindowBackgroundAppearance::Transparent,
        kind: WindowKind::LayerShell(LayerShellOptions {
            namespace: "calendar-popup".to_string(),
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

/// Anchored popup window options — positioned relative to the clock
/// widget's bounds, extending down-and-left from the icon's bottom-right
/// corner.
fn window_options(anchor_rect: Bounds<Pixels>, parent: AnyWindowHandle) -> WindowOptions {
    WindowOptions {
        titlebar: None,
        window_bounds: Some(WindowBounds::Windowed(Bounds {
            origin: point(px(0.), px(0.)),
            size: Size::new(px(POPUP_WIDTH), px(POPUP_HEIGHT)),
        })),
        app_id: Some("chronos-calendar-popup".to_string()),
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

fn open_click_catcher(
    cx: &mut App,
    anchor_rect: Bounds<Pixels>,
) -> anyhow::Result<AnyWindowHandle> {
    crate::popup_click_catcher::open_for_popup(
        cx,
        anchor_rect,
        Size::new(px(POPUP_WIDTH), px(POPUP_HEIGHT)),
        Rc::new(|window, cx| close_from_click_catcher(window, cx)),
    )
}

fn window_closed_is_tracked(tracked: Option<WindowId>, closed: WindowId) -> bool {
    tracked == Some(closed)
}

/// Open the popup (idempotent — no-op if already open).
pub fn open(cx: &mut App, anchor_rect: Bounds<Pixels>, parent: AnyWindowHandle) {
    if cx.global::<CalendarPopupState>().handle.is_some() {
        return;
    }

    // Singleton: Calendar and Sound share the bar's top-right corner. Only
    // one may be open — two full-output click-catchers stacked would fight.
    crate::volume_popup::close(cx);

    let mut click_catcher = open_click_catcher(cx, anchor_rect).ok();

    let result = cx.open_window(window_options(anchor_rect, parent), |window, app_cx| {
        let view = app_cx.new(|view_cx| CalendarPopupView::new(window, view_cx));
        app_cx.new(|view_cx| {
            Root::new(view, window, view_cx)
                .bordered(false)
                .bg(gpui::transparent_black())
        })
    });

    let result = match result {
        Err(err) => {
            // The LayerShell fallback sits at a fixed corner, not at
            // `anchor_rect`, so the anchored catcher hole would be wrong —
            // drop it (mirrors tray_menu's fallback: no click-catcher there).
            if let Some(catcher) = click_catcher.take() {
                if let Err(e) = catcher.update(cx, |_, window, _| window.remove_window()) {
                    tracing::warn!("calendar_popup: failed to drop click-catcher on fallback: {e}");
                }
            }
            if err.downcast_ref::<PopupNotSupportedError>().is_some() {
                tracing::warn!(
                    "calendar_popup: AnchoredPopup not supported, falling back to LayerShell"
                );
                let display_id = pick_display(cx);
                cx.open_window(fallback_window_options(display_id), |window, app_cx| {
                    let view = app_cx.new(|view_cx| CalendarPopupView::new(window, view_cx));
                    app_cx.new(|view_cx| {
                        Root::new(view, window, view_cx)
                            .bordered(false)
                            .bg(gpui::transparent_black())
                    })
                })
            } else {
                Err(err)
            }
        }
        ok => ok,
    };

    match result {
        Ok(new_handle) => {
            let window_id = new_handle.window_id();
            let window_closed_subscription = cx.on_window_closed(move |cx, closed_id| {
                let tracked = cx
                    .global::<CalendarPopupState>()
                    .handle
                    .as_ref()
                    .map(|handle| handle.window_id());
                if closed_id != window_id || !window_closed_is_tracked(tracked, closed_id) {
                    return;
                }
                let catcher = {
                    let state = cx.global_mut::<CalendarPopupState>();
                    state.handle = None;
                    state.window_closed_subscription = None;
                    state.click_catcher.take()
                };
                if let Some(catcher) = catcher {
                    if let Err(e) = catcher.update(cx, |_, window, _| window.remove_window()) {
                        tracing::warn!(
                            "calendar_popup: failed to remove click-catcher on window close: {e}"
                        );
                    }
                }
            });
            let state = cx.global_mut::<CalendarPopupState>();
            state.handle = Some(new_handle);
            state.window_closed_subscription = Some(window_closed_subscription);
            state.click_catcher = click_catcher;
        }
        Err(err) => tracing::warn!("calendar_popup: failed to open popup: {err}"),
    }
}

/// Close the popup (clears state + destroys the window). Safe to call from
/// contexts that do NOT already hold `&mut Window` for this popup (bar
/// widget click, external toggle) — uses `handle.update`. Removes both the
/// popup and the click-catcher.
pub fn close(cx: &mut App) {
    let (handle, catcher) = {
        let state = cx.global_mut::<CalendarPopupState>();
        state.window_closed_subscription = None;
        (state.handle.take(), state.click_catcher.take())
    };
    if let Some(handle) = handle {
        if let Err(e) = handle.update(cx, |_, window: &mut Window, _| window.remove_window()) {
            tracing::warn!("calendar_popup: close remove_window failed (already dead?): {e}");
        }
    }
    if let Some(catcher) = catcher {
        if let Err(e) = catcher.update(cx, |_, window, _| window.remove_window()) {
            tracing::warn!("calendar_popup: close click-catcher remove_window failed (already dead?): {e}");
        }
    }
}

/// Close the popup from inside a callback that already holds `&mut Window`
/// for this popup's window-id (the in-popup "✕" button). A blind
/// `close(cx)` would re-enter `handle.update` on the same id, which
/// silently fails while the callback is running and leaves a ghost popup.
pub(crate) fn close_this(window: &mut Window, cx: &mut App) {
    let this = window.window_handle();
    let tracked = cx
        .global::<CalendarPopupState>()
        .handle
        .as_ref()
        .map(|h| **h == this)
        .unwrap_or(false);
    let catcher = {
        let state = cx.global_mut::<CalendarPopupState>();
        if tracked {
            state.handle.take();
            state.window_closed_subscription = None;
        }
        state.click_catcher.take()
    };
    if let Some(catcher) = catcher {
        if let Err(e) = catcher.update(cx, |_, catcher_window, _| catcher_window.remove_window()) {
            tracing::warn!("calendar_popup: close_this failed to remove click-catcher: {e}");
        }
    }
    window.remove_window();
}

/// Close both surfaces from the transparent click-catcher's own callback.
fn close_from_click_catcher(window: &mut Window, cx: &mut App) {
    let this = window.window_handle();
    let (popup, catcher) = {
        let state = cx.global_mut::<CalendarPopupState>();
        state.window_closed_subscription = None;
        (state.handle.take(), state.click_catcher.take())
    };
    if let Some(popup) = popup {
        if let Err(e) = popup.update(cx, |_, popup_window, _| popup_window.remove_window()) {
            tracing::warn!("calendar_popup: click-catcher failed to remove popup: {e}");
        }
    }
    if catcher == Some(this) {
        window.remove_window();
    } else if let Some(catcher) = catcher {
        if let Err(e) = catcher.update(cx, |_, catcher_window, _| catcher_window.remove_window()) {
            tracing::warn!("calendar_popup: click-catcher failed to remove catcher: {e}");
        }
    }
}

/// Bar-icon toggle. Caller's window is the bar, not the popup → use `close`.
pub fn toggle(
    anchor_rect: Bounds<Pixels>,
    parent: AnyWindowHandle,
    _window: &mut Window,
    cx: &mut App,
) {
    if cx.global::<CalendarPopupState>().handle.is_some() {
        close(cx);
    } else {
        open(cx, anchor_rect, parent);
    }
}

/// Wire the calendar popup. Called once from `main.rs`.
pub fn init(cx: &mut App) {
    cx.set_global(CalendarPopupState::default());
    tracing::info!("calendar_popup: initialized");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn popup_dimensions_reasonable() {
        assert!(POPUP_WIDTH > 200.0 && POPUP_WIDTH < 420.0);
        assert!(POPUP_HEIGHT > 250.0 && POPUP_HEIGHT < 400.0);
    }

    #[test]
    fn margins_positive() {
        assert!(POPUP_MARGIN_TOP > 0.0);
        assert!(POPUP_MARGIN_RIGHT >= 0.0);
    }

    #[gpui::test]
    fn anchored_popup_does_not_request_a_compositor_grab(cx: &mut gpui::TestAppContext) {
        use gpui::{AppContext, Context, Render, Window, WindowKind, div};

        struct EmptyView;

        impl Render for EmptyView {
            fn render(
                &mut self,
                _window: &mut Window,
                _cx: &mut Context<Self>,
            ) -> impl gpui::IntoElement {
                div()
            }
        }

        let parent = cx.update(|cx| {
            cx.open_window(Default::default(), |_window, cx| cx.new(|_| EmptyView))
                .expect("test parent window")
        });
        let options = super::window_options(Bounds::default(), parent.into());
        let WindowKind::AnchoredPopup(options) = options.kind else {
            panic!("calendar popup must remain an anchored popup");
        };

        assert!(!options.grab, "T264 forbids compositor popup grabs");
    }
}
