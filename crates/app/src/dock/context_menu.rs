//! Dock context menu — right-click popup on a pinned icon.
//!
//! A gpui-component `PopupMenu` with a single "Unpin" item, hosted in the
//! popup window (T263 Часть 1.5). Window lifecycle follows the `tray_menu`
//! pattern: `Global` state, `close_this` reentrancy guard, anchored popup at
//! the dock icon with a fixed-corner `LayerShell` fallback.
//!
//! The window root MUST be a `gpui_component::Root` (component widgets panic
//! on `window.root()` otherwise) — `open()` wraps `DockMenuView` in `Root`.

use std::{rc::Rc, time::Duration};

use gpui::{
    AnyWindowHandle, App, AsyncApp, Bounds, Context, DisplayId, DismissEvent, Entity, Focusable,
    Global, Pixels, Size, Subscription, WeakEntity, Window, WindowBackgroundAppearance,
    WindowBounds, WindowHandle, WindowId, WindowKind, WindowOptions, div, layer_shell::*,
    point,
    popup::{
        PopupAnchor, PopupConstraintAdjustment, PopupGravity, PopupNotSupportedError, PopupOptions,
    },
    prelude::*,
    px,
};

use chronos_ui::{Theme, WindowRootExt};
use gpui_component::Root;
use gpui_component::menu::{PopupMenu, PopupMenuItem};

use crate::dock::config::DockConfig;
use crate::dock::signal::notify_config_changed;
use crate::motion;

/// Context menu width (px) — canon `min-width:230px`. The dock menu model
/// is a static single "Unpin" item with no submenu rows, so the T263
/// widest-reserve estimate is trivially the card width (reserve = 0) and
/// the surface stays card-sized; if the model ever grows submenus, mirror
/// `tray_menu::estimate_menu_width`. Clicks on the transparent surface
/// outside the card dismiss via the component's `on_mouse_down_out` —
/// client-side, no compositor grab (T264).
const MENU_WIDTH: f32 = 230.;
/// Top margin — bar height + small gap so popup sits below the bar.
const MENU_MARGIN_TOP: f32 = 36.;

/// Global state for the dock context menu popup.
#[derive(Default)]
pub struct DockMenuState {
    /// Window handle while the menu is open; `None` when closed. The window
    /// root is a `gpui_component::Root` (component requirement).
    handle: Option<WindowHandle<Root>>,
    /// Weak handle to the live `DockMenuView` (unused for repaint today —
    /// the menu is static; kept symmetric with `tray_menu` for future items).
    view: Option<WeakEntity<DockMenuView>>,
    /// The entry id that was right-clicked (for unpin action).
    entry_id: Option<String>,
    /// Generation guard for auto-close.
    close_generation: u64,
    /// Clears stale state when the compositor dismisses the xdg-popup.
    window_closed_subscription: Option<Subscription>,
    /// Transparent layer-surface that receives clicks outside the popup while
    /// the native popup intentionally has `grab: false` (T264).
    click_catcher: Option<AnyWindowHandle>,
}

impl Global for DockMenuState {}

impl DockMenuState {
    pub fn entry_id(&self) -> Option<&str> {
        self.entry_id.as_deref()
    }

    /// Test-only: stamp `entry_id` without opening a window (avoids Theme/Wayland).
    #[cfg(test)]
    pub fn set_entry_id_for_test(&mut self, id: Option<String>) {
        self.entry_id = id;
    }
}

pub struct DockMenuView {
    /// The `PopupMenu` entity with the "Unpin" item, built once.
    popup_menu: Option<Entity<PopupMenu>>,
    /// Dismiss subscription — `PopupMenu` emits `DismissEvent` on confirm /
    /// Escape / click-away; the single close path for the window.
    dismiss_subscription: Option<Subscription>,
    /// View-driven enter progress 0..=1 (anchored popups — `with_animation`
    /// is invisible on live Hyprland; see `motion::arm_enter_progress`).
    enter_t: f32,
}

impl DockMenuView {
    pub fn new(cx: &mut Context<Self>) -> Self {
        // Menu enter follows the reference `ctx-in` curve (`cubic-bezier(.2,.8,.2,1)`,
        // `.12s`) — the popups' EaseOutBack overshoot would feel out of place here.
        motion::arm_enter_progress_with(
            cx,
            Duration::from_millis(motion::MENU_ENTER_MS),
            motion::ease_menu_enter,
            |view, t| {
                view.enter_t = t;
            },
        );
        Self {
            popup_menu: None,
            dismiss_subscription: None,
            enter_t: 0.0,
        }
    }
}

impl Render for DockMenuView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.popup_menu.is_none() {
            let menu = build_unpin_menu(window, cx);
            // Single close path: `PopupMenu` emits `DismissEvent` on confirm /
            // Escape / click-away — close the window there (the Unpin handler
            // does NOT close, so no double `remove_window`).
            self.dismiss_subscription = Some(cx.subscribe_in(
                &menu,
                window,
                |_this, _menu, _: &DismissEvent, window, cx| {
                    close_this(window, cx);
                },
            ));
            // Focus so Enter/Space can trigger the item (canon `navIdx`).
            let handle = menu.read(cx).focus_handle(cx);
            handle.focus(window, cx);
            self.popup_menu = Some(menu);
        }
        let Some(menu) = self.popup_menu.clone() else {
            return div().into_any_element();
        };
        // Host: full-window transparent surface, enter fade. PopupMenu draws
        // its own card (popover style) inside. Route the window root through
        // `window_font` so the menu text inherits `theme.font_ui` (T227).
        let theme = *Theme::global(cx);
        motion::apply_enter_menu(
            div().size_full().window_font(&theme).child(menu.clone()),
            self.enter_t,
        )
        .into_any_element()
    }
}

/// Build the single-item "Unpin" menu. The action mirrors the previous
/// hand-rolled row: read the entry from global state, clear it, unpin from
/// the dock config, persist + notify, then close the window.
fn build_unpin_menu(window: &mut Window, cx: &mut App) -> Entity<PopupMenu> {
    PopupMenu::build(window, cx, |menu, _window, _cx| {
        menu.min_w(px(MENU_WIDTH)).max_w(px(MENU_WIDTH)).item(
            PopupMenuItem::new("Unpin").on_click(move |_event, _window, cx: &mut App| {
                // Read entry_id from global before clearing.
                let id = cx
                    .global::<DockMenuState>()
                    .entry_id
                    .clone()
                    .unwrap_or_default();

                // Clear global state.
                {
                    let state = cx.global_mut::<DockMenuState>();
                    state.entry_id = None;
                    state.close_generation = state.close_generation.wrapping_add(1);
                }

                // Unpin: remove from config, save, rebuild dock.
                let mut config = DockConfig::load();
                config.unpin(&id);
                // Gamer/scene composition can reintroduce an entry that is
                // absent from `pinned`; persist the explicit user exclusion
                // so Unpin actually removes the visible icon in every mode.
                config.exclude(&id);
                if let Err(e) = config.save() {
                    tracing::error!("dock: failed to save config after unpin: {e}");
                }

                // Update the cached config.
                crate::dock::config::update_cache(config);

                // Notify dock views to rebuild.
                notify_config_changed(cx);

                // The window closes via the `DismissEvent` subscription
                // (PopupMenu emits it right after this handler).
            }),
        )
    })
}

fn pick_display(cx: &App) -> Option<DisplayId> {
    crate::monitor::pult_display_id_or_primary(cx)
}

fn open_click_catcher(
    cx: &mut App,
    anchor_rect: Bounds<Pixels>,
) -> anyhow::Result<AnyWindowHandle> {
    crate::popup_click_catcher::open_for_popup(
        cx,
        anchor_rect,
        Size::new(px(MENU_WIDTH), px(62.)),
        Rc::new(|window, cx| close_from_click_catcher(window, cx)),
    )
}

/// Layer-shell options for the context menu: centered horizontally,
/// anchored TOP, positioned just below the bar. Fixed-corner fallback when
/// `AnchoredPopup` isn't supported on this platform. `OnDemand` keyboard so
/// the component's Escape/arrow handling works on the fallback path too.
fn fallback_window_options(display_id: Option<DisplayId>) -> WindowOptions {
    WindowOptions {
        display_id,
        titlebar: None,
        window_bounds: Some(WindowBounds::Windowed(Bounds {
            origin: point(px(0.), px(0.)), size: Size::new(px(MENU_WIDTH), px(34.)),
        })),
        app_id: Some("chronos-dock-menu".to_string()),
        window_background: WindowBackgroundAppearance::Transparent,
        kind: WindowKind::LayerShell(LayerShellOptions {
            namespace: "dock-menu".to_string(),
            layer: Layer::Overlay,
            anchor: Anchor::TOP,
            exclusive_zone: None,
            margin: Some((px(MENU_MARGIN_TOP), px(0.), px(0.), px(0.))),
            keyboard_interactivity: KeyboardInteractivity::OnDemand,
            ..Default::default()
        }),
        ..Default::default()
    }
}

/// Anchored popup — positioned at the dock icon's bounds. The dock sits in
/// the left cluster of the top bar, so the menu opens below-right of the
/// icon (canon `positionRoot` at the click point).
fn window_options(
    anchor_rect: Bounds<Pixels>,
    parent: AnyWindowHandle,
) -> WindowOptions {
    WindowOptions {
        titlebar: None,
        window_bounds: Some(WindowBounds::Windowed(Bounds {
            origin: point(px(0.), px(0.)), size: Size::new(px(MENU_WIDTH), px(34.)),
        })),
        app_id: Some("chronos-dock-menu".to_string()),
        window_background: WindowBackgroundAppearance::Transparent,
        kind: WindowKind::AnchoredPopup(PopupOptions {
            parent,
            anchor_rect,
            anchor: PopupAnchor::BottomLeft,
            gravity: PopupGravity::BottomRight,
            constraint_adjustment: PopupConstraintAdjustment::SLIDE_X
                | PopupConstraintAdjustment::SLIDE_Y
                | PopupConstraintAdjustment::FLIP_X
                | PopupConstraintAdjustment::FLIP_Y,
            offset: point(px(0.), px(4.)),
            // T264: Hyprland 0.56.x can retain this xdg-popup's seat grab
            // after client-side destruction and then drop compositor-wide
            // pointer input. PopupMenu still dismisses on click-away/Escape,
            // so this safety-critical menu must not request a compositor grab.
            grab: false,
        }),
        ..Default::default()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum OpenAction {
    Close,
    OpenFresh,
}

fn open_action(open_entry: Option<&str>, requested_entry: &str) -> OpenAction {
    match open_entry {
        Some(open) if open == requested_entry => OpenAction::Close,
        _ => OpenAction::OpenFresh,
    }
}

fn window_closed_is_tracked(tracked: Option<WindowId>, closed: WindowId) -> bool {
    tracked == Some(closed)
}

/// Open the context menu for `entry_id` at the dock icon's bounds. If already
/// open for the same entry, close it (toggle). If open for a different entry,
/// switch. Anchored popup with fixed-corner LayerShell fallback.
pub fn open(
    cx: &mut App,
    anchor_rect: Bounds<Pixels>,
    parent: AnyWindowHandle,
    entry_id: String,
) {
    let open_entry = cx
        .global::<DockMenuState>()
        .entry_id
        .clone();
    match open_action(open_entry.as_deref(), &entry_id) {
        OpenAction::Close => {
            close(cx);
            return;
        }
        OpenAction::OpenFresh => {
            // An xdg-popup's anchor is creation-time state. Switching dock
            // icons must remap the popup or it stays at the previous icon.
            if open_entry.is_some() {
                close(cx);
            }
        }
    }

    let generation = {
        let state = cx.global_mut::<DockMenuState>();
        state.entry_id = Some(entry_id);
        state.close_generation = state.close_generation.wrapping_add(1);
        state.close_generation
    };

    let handle = cx.global::<DockMenuState>().handle.clone();
    match handle {
        Some(existing) => {
            let _ = existing.update(cx, |_, _window, view_cx| {
                view_cx.notify();
            });
        }
        None => {
            // Window root must be a component `Root` (see module docs).
            let click_catcher = open_click_catcher(cx, anchor_rect).ok();
            let mut opened_view: Option<WeakEntity<DockMenuView>> = None;
            let mut open = |cx: &mut App, options: WindowOptions| {
                cx.open_window(options, |window, view_cx| {
                    let view = view_cx.new(|view_cx| DockMenuView::new(view_cx));
                    opened_view = Some(view.downgrade());
                    view_cx.new(|view_cx| {
                        Root::new(view, window, view_cx)
                            .bordered(false)
                            .bg(gpui::transparent_black())
                    })
                })
            };
            let result = match open(cx, window_options(anchor_rect, parent)) {
                Err(err) => {
                    if let Some(catcher) = click_catcher {
                        let _ = catcher.update(cx, |_, window, _| window.remove_window());
                    }
                    if err.downcast_ref::<PopupNotSupportedError>().is_some() {
                        tracing::warn!(
                            "dock context menu: AnchoredPopup not supported on this platform, falling back to fixed-corner LayerShell"
                        );
                        let display_id = pick_display(cx);
                        open(cx, fallback_window_options(display_id))
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
                            .global::<DockMenuState>()
                            .handle
                            .as_ref()
                            .map(|handle| handle.window_id());
                        if closed_id != window_id || !window_closed_is_tracked(tracked, closed_id) {
                            return;
                        }
                        let catcher = {
                            let state = cx.global_mut::<DockMenuState>();
                            state.handle = None;
                            state.view = None;
                            state.entry_id = None;
                            state.close_generation = state.close_generation.wrapping_add(1);
                            state.window_closed_subscription = None;
                            state.click_catcher.take()
                        };
                        if let Some(catcher) = catcher {
                            let _ = catcher.update(cx, |_, window, _| window.remove_window());
                        }
                    });
                    let state = cx.global_mut::<DockMenuState>();
                    state.handle = Some(new_handle);
                    state.view = opened_view;
                    state.window_closed_subscription = Some(window_closed_subscription);
                    state.click_catcher = click_catcher;
                }
                Err(err) => tracing::warn!("dock context menu: failed to open: {err}"),
            }
        }
    }

    schedule_autoclose(cx, generation);
}

/// Close from inside a callback that already holds `&mut Window` for this
/// popup (the `DismissEvent` subscription). Must not re-enter `handle.update`
/// on the same id — direct `remove_window` on the live reference.
pub(crate) fn close_this(window: &mut Window, cx: &mut App) {
    let this = window.window_handle();
    let tracked = cx
        .global::<DockMenuState>()
        .handle
        .as_ref()
        .map(|h| **h == this)
        .unwrap_or(false);
    let catcher = {
        let state = cx.global_mut::<DockMenuState>();
        if tracked {
            state.handle.take();
            state.view = None;
            state.window_closed_subscription = None;
        }
        state.click_catcher.take()
    };
    let state = cx.global_mut::<DockMenuState>();
    state.entry_id = None;
    state.close_generation = state.close_generation.wrapping_add(1);
    state.window_closed_subscription = None;
    if let Some(catcher) = catcher {
        let _ = catcher.update(cx, |_, catcher_window, _| catcher_window.remove_window());
    }
    window.remove_window();
}

/// Close both surfaces from the transparent click-catcher's own callback.
pub(crate) fn close_from_click_catcher(window: &mut Window, cx: &mut App) {
    let this = window.window_handle();
    let (popup, catcher) = {
        let state = cx.global_mut::<DockMenuState>();
        state.entry_id = None;
        state.close_generation = state.close_generation.wrapping_add(1);
        state.window_closed_subscription = None;
        state.view = None;
        (state.handle.take(), state.click_catcher.take())
    };
    if let Some(popup) = popup {
        let _ = popup.update(cx, |_, popup_window, _| popup_window.remove_window());
    }
    if catcher == Some(this) {
        window.remove_window();
    } else if let Some(catcher) = catcher {
        let _ = catcher.update(cx, |_, catcher_window, _| catcher_window.remove_window());
    }
}

/// Close the context menu (clears state + destroys window).
pub fn close(cx: &mut App) {        let (popup, catcher) = {
            let state = cx.global_mut::<DockMenuState>();
            state.entry_id = None;
            state.close_generation = state.close_generation.wrapping_add(1);
            state.window_closed_subscription = None;
            state.view = None;
            (state.handle.take(), state.click_catcher.take())
        };
        if let Some(handle) = popup {
            let _ = handle.update(cx, |_, window: &mut gpui::Window, _| window.remove_window());
        }
        if let Some(handle) = catcher {
            let _ = handle.update(cx, |_, window, _| window.remove_window());
        }

}

/// Auto-close after 5 seconds (shorter than tray_menu — small menu).
fn schedule_autoclose(cx: &mut App, generation: u64) {
    cx.spawn(async move |app_cx: &mut AsyncApp| {
        app_cx
            .background_executor()
            .timer(Duration::from_secs(5))
            .await;
        app_cx.update(|app_cx| {
            if app_cx.global::<DockMenuState>().close_generation != generation {
                return;
            }
            close(app_cx);
        });
    })
    .detach();
}

#[cfg(test)]
mod tests {
    use super::{OpenAction, open_action, window_closed_is_tracked, window_options};
    use std::{cell::Cell, rc::Rc};

    use gpui::{
        AppContext, Bounds, Context, DismissEvent, Entity, Focusable, Modifiers, MouseButton,
        ParentElement, Render, Styled, Subscription, TestAppContext, Window, WindowId, WindowKind,
        div, point, px,
    };
    use gpui_component::menu::{PopupMenu, PopupMenuItem};

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

    struct DismissHarness {
        menu: Entity<PopupMenu>,
        dismissed: Rc<Cell<bool>>,
        _subscription: Subscription,
    }

    impl DismissHarness {
        fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
            let menu = PopupMenu::build(window, cx, |menu, _, _| {
                menu.item(PopupMenuItem::new("Unpin"))
            });
            let dismissed = Rc::new(Cell::new(false));
            let dismissed_on_event = dismissed.clone();
            let subscription = cx.subscribe_in(
                &menu,
                window,
                move |_this, _menu, _: &DismissEvent, _window, _cx| {
                    dismissed_on_event.set(true);
                },
            );
            menu.focus_handle(cx).focus(window, cx);
            Self {
                menu,
                dismissed,
                _subscription: subscription,
            }
        }
    }

    impl Render for DismissHarness {
        fn render(
            &mut self,
            _window: &mut Window,
            _cx: &mut Context<Self>,
        ) -> impl gpui::IntoElement {
            div().size_full().child(self.menu.clone())
        }
    }

    #[test]
    fn different_entry_requires_a_fresh_anchor() {
        assert_eq!(
            open_action(Some("firefox"), "terminal"),
            OpenAction::OpenFresh
        );
    }

    #[gpui::test]
    fn anchored_popup_does_not_request_a_compositor_grab(cx: &mut TestAppContext) {
        let parent = cx.update(|cx| {
            cx.open_window(Default::default(), |_window, cx| cx.new(|_| EmptyView))
                .expect("test parent window")
        });
        let options = window_options(Bounds::default(), parent.into());
        let WindowKind::AnchoredPopup(options) = options.kind else {
            panic!("dock context menu must remain an anchored popup");
        };

        assert!(!options.grab, "T264 forbids compositor popup grabs");
    }

    #[gpui::test]
    fn escape_emits_the_dismiss_event_used_by_the_close_path(cx: &mut TestAppContext) {
        cx.update(gpui_component::init);
        let (view, cx) = cx.add_window_view(DismissHarness::new);

        cx.simulate_keystrokes("escape");

        assert!(view.read_with(cx, |view, _| view.dismissed.get()));
    }

    #[gpui::test]
    fn click_outside_the_card_emits_the_dismiss_event_used_by_the_close_path(
        cx: &mut TestAppContext,
    ) {
        cx.update(gpui_component::init);
        let (view, cx) = cx.add_window_view(DismissHarness::new);

        cx.simulate_mouse_down(
            point(px(700.), px(500.)),
            MouseButton::Left,
            Modifiers::default(),
        );

        assert!(view.read_with(cx, |view, _| view.dismissed.get()));
    }
    #[test]
    fn compositor_close_matches_only_the_tracked_window() {
        let tracked = WindowId::from(11);
        assert!(window_closed_is_tracked(Some(tracked), WindowId::from(11)));
        assert!(!window_closed_is_tracked(Some(tracked), WindowId::from(12)));
        assert!(!window_closed_is_tracked(None, WindowId::from(11)));
    }
}
