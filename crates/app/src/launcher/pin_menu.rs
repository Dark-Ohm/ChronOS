//! Launcher → dock pin/unpin context menu.
//!
//! T275 Часть D: right-click a launcher result row to pin it to the dock (or
//! unpin it). Built on the same engine as the tray and dock menus —
//! `gpui-component::PopupMenu` inside an anchored popup whose root is a
//! `gpui_component::Root` (component widgets panic otherwise, T263). The menu
//! item is honest: already-pinned → "Unpin", not pinned → "Pin to dock".
//!
//! Pin/unpin writes `dock.toml`, refreshes the cached config and notifies the
//! dock so the icon appears/disappears immediately, without a shell restart.

use std::{rc::Rc, time::Duration};

use gpui::{
    AnyWindowHandle, App, Bounds, Context, DisplayId, DismissEvent, Entity, Focusable, Global,
    Pixels, Size, Subscription, Window, WindowBackgroundAppearance, WindowBounds, WindowHandle,
    WindowId, WindowKind, WindowOptions, div, layer_shell::*, point,
    popup::{
        PopupAnchor, PopupConstraintAdjustment, PopupGravity, PopupNotSupportedError, PopupOptions,
    },
    prelude::*, px,
};

use gpui_component::Root;
use gpui_component::menu::{PopupMenu, PopupMenuItem};

use crate::dock::config::{cached, update_cache, DockConfig};
use crate::dock::signal::notify_config_changed;
use crate::motion;

const MENU_WIDTH: f32 = 220.;
/// Single-item menu height (px).
const MENU_HEIGHT: f32 = 34.;

/// Global state for the launcher pin menu popup.
#[derive(Default)]
pub struct LauncherPinMenuState {
    /// Window handle while the menu is open; `None` when closed.
    handle: Option<WindowHandle<Root>>,
    /// The entry id that was right-clicked (for the pin/unpin action).
    entry_id: Option<String>,
    /// Weak handle to the live menu view (unused for repaint today — kept
    /// symmetric with `tray_menu`/`dock` for future menu items).
    view: Option<gpui::WeakEntity<LauncherPinMenuView>>,
    /// Generation guard for auto-close.
    close_generation: u64,
    /// Clears stale state when the compositor dismisses the xdg-popup.
    window_closed_subscription: Option<Subscription>,
    /// Transparent layer-surface that receives clicks outside the popup while
    /// the native popup intentionally has `grab: false` (T264).
    click_catcher: Option<AnyWindowHandle>,
}

impl Global for LauncherPinMenuState {}

impl LauncherPinMenuState {
    pub fn entry_id(&self) -> Option<&str> {
        self.entry_id.as_deref()
    }
}

pub struct LauncherPinMenuView {
    popup_menu: Option<Entity<PopupMenu>>,
    dismiss_subscription: Option<Subscription>,
    enter_t: f32,
}

impl LauncherPinMenuView {
    pub fn new(cx: &mut Context<Self>) -> Self {
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

impl Render for LauncherPinMenuView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.popup_menu.is_none() {
            let entry_id = cx
                .global::<LauncherPinMenuState>()
                .entry_id
                .clone()
                .unwrap_or_default();
            let menu = build_pin_menu(window, cx, entry_id);
            // Single close path: `PopupMenu` emits `DismissEvent` on confirm /
            // Escape / click-away — close the window there.
            self.dismiss_subscription = Some(cx.subscribe_in(
                &menu,
                window,
                |_this, _menu, _: &DismissEvent, window, cx| {
                    close_this(window, cx);
                },
            ));
            let handle = menu.read(cx).focus_handle(cx);
            handle.focus(window, cx);
            self.popup_menu = Some(menu);
        }
        let Some(menu) = self.popup_menu.clone() else {
            return div().into_any_element();
        };
        motion::apply_enter_menu(div().size_full().child(menu.clone()), self.enter_t)
            .into_any_element()
    }
}

/// Build the pin/unpin menu. The label/action is chosen from the current dock
/// config so the item is always honest about the entry's pinned state.
fn build_pin_menu(window: &mut Window, cx: &mut App, entry_id: String) -> Entity<PopupMenu> {
    // Decide from the cached config (cheap, no disk I/O).
    let is_pinned = crate::dock::config::resolve_pinned(cx)
        .iter()
        .any(|p| p == &entry_id);

    PopupMenu::build(window, cx, |menu, _window, _cx| {
        if is_pinned {
            menu.min_w(px(MENU_WIDTH))
                .max_w(px(MENU_WIDTH))
                .item(PopupMenuItem::new("Unpin").on_click(move |_event, _window, cx: &mut App| {
                    let mut config = DockConfig::load();
                    config.unpin(&entry_id);
                    // Persist the explicit user exclusion so an unpinned
                    // mode/scene default does not instantly repaint the icon.
                    config.exclude(&entry_id);
                    if let Err(e) = config.save() {
                        tracing::error!("launcher pin-menu: failed to save config on unpin: {e}");
                    }
                    update_cache(config);
                    notify_config_changed(cx);
                    tracing::info!(entry_id, "launcher pin-menu: unpinned");
                }))
        } else {
            menu.min_w(px(MENU_WIDTH)).max_w(px(MENU_WIDTH)).item(
                PopupMenuItem::new("Pin to dock").on_click(move |_event, _window, cx: &mut App| {
                    let mut config = DockConfig::load();
                    config.pin(&entry_id);
                    if let Err(e) = config.save() {
                        tracing::error!("launcher pin-menu: failed to save config on pin: {e}");
                    }
                    update_cache(config);
                    notify_config_changed(cx);
                    tracing::info!(entry_id, "launcher pin-menu: pinned");
                }),
            )
        }
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
        Size::new(px(MENU_WIDTH), px(MENU_HEIGHT)),
        Rc::new(|window, cx| close_from_click_catcher(window, cx)),
    )
}

fn window_options(anchor_rect: Bounds<Pixels>, parent: AnyWindowHandle) -> WindowOptions {
    WindowOptions {
        titlebar: None,
        window_bounds: Some(WindowBounds::Windowed(Bounds {
            origin: point(px(0.), px(0.)),
            size: Size::new(px(MENU_WIDTH), px(MENU_HEIGHT)),
        })),
        app_id: Some("chronos-launcher-pin-menu".to_string()),
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
            // T264: no compositor grab — the component dismisses on click-away.
            grab: false,
        }),
        ..Default::default()
    }
}

fn fallback_window_options(display_id: Option<DisplayId>) -> WindowOptions {
    WindowOptions {
        display_id,
        titlebar: None,
        window_bounds: Some(WindowBounds::Windowed(Bounds {
            origin: point(px(0.), px(0.)),
            size: Size::new(px(MENU_WIDTH), px(MENU_HEIGHT)),
        })),
        app_id: Some("chronos-launcher-pin-menu".to_string()),
        window_background: WindowBackgroundAppearance::Transparent,
        kind: WindowKind::LayerShell(LayerShellOptions {
            namespace: "launcher-pin-menu".to_string(),
            layer: Layer::Overlay,
            anchor: Anchor::TOP,
            exclusive_zone: None,
            margin: Some((px(36.), px(0.), px(0.), px(0.))),
            keyboard_interactivity: KeyboardInteractivity::OnDemand,
            ..Default::default()
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

/// Open the pin/unpin menu for `entry_id`, anchored at the right-clicked row.
///
/// `anchor_rect` is surface-local — it positions the `AnchoredPopup` (the
/// Wayland positioner expects window coords). `catcher_anchor` is the same
/// point in output-local coords — it positions the pass-through hole of the
/// transparent click-catcher. For the launcher (a centered Normal window)
/// the two differ by `window.bounds().origin`; reusing one anchor for both
/// put the menu at one place and the click hole at another (T275).
pub fn open(
    cx: &mut App,
    anchor_rect: Bounds<Pixels>,
    catcher_anchor: Bounds<Pixels>,
    parent: AnyWindowHandle,
    entry_id: String,
) {
    let open_entry = cx.global::<LauncherPinMenuState>().entry_id.clone();
    match open_action(open_entry.as_deref(), &entry_id) {
        OpenAction::Close => {
            close(cx);
            return;
        }
        OpenAction::OpenFresh => {
            if open_entry.is_some() {
                close(cx);
            }
        }
    }

    let generation = {
        let state = cx.global_mut::<LauncherPinMenuState>();
        state.entry_id = Some(entry_id);
        state.close_generation = state.close_generation.wrapping_add(1);
        state.close_generation
    };

    let handle = cx.global::<LauncherPinMenuState>().handle.clone();
    match handle {
        Some(existing) => {
            let _ = existing.update(cx, |_, _window, view_cx| {
                view_cx.notify();
            });
        }
        None => {
            let click_catcher = open_click_catcher(cx, catcher_anchor).ok();
            let mut opened_view: Option<gpui::WeakEntity<LauncherPinMenuView>> = None;
            let mut open = |cx: &mut App, options: WindowOptions| {
                cx.open_window(options, |window, view_cx| {
                    let view = view_cx.new(|view_cx| LauncherPinMenuView::new(view_cx));
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
                            "launcher pin-menu: AnchoredPopup not supported, falling back to layer-shell"
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
                            .global::<LauncherPinMenuState>()
                            .handle
                            .as_ref()
                            .map(|handle| handle.window_id());
                        if closed_id != window_id || !window_closed_is_tracked(tracked, closed_id) {
                            return;
                        }
                        let catcher = {
                            let state = cx.global_mut::<LauncherPinMenuState>();
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
                    let state = cx.global_mut::<LauncherPinMenuState>();
                    state.handle = Some(new_handle);
                    state.view = opened_view;
                    state.window_closed_subscription = Some(window_closed_subscription);
                    state.click_catcher = click_catcher;
                }
                Err(err) => tracing::warn!("launcher pin-menu: failed to open: {err}"),
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
        .global::<LauncherPinMenuState>()
        .handle
        .as_ref()
        .map(|h| **h == this)
        .unwrap_or(false);
    let catcher = {
        let state = cx.global_mut::<LauncherPinMenuState>();
        if tracked {
            state.handle.take();
            state.view = None;
            state.window_closed_subscription = None;
        }
        state.click_catcher.take()
    };
    let state = cx.global_mut::<LauncherPinMenuState>();
    state.entry_id = None;
    state.close_generation = state.close_generation.wrapping_add(1);
    state.window_closed_subscription = None;
    if let Some(catcher) = catcher {
        let _ = catcher.update(cx, |_, catcher_window, _| catcher_window.remove_window());
    }
    window.remove_window();
}

/// Close from the transparent click-catcher's own callback.
pub(crate) fn close_from_click_catcher(window: &mut Window, cx: &mut App) {
    let this = window.window_handle();
    let (popup, catcher) = {
        let state = cx.global_mut::<LauncherPinMenuState>();
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

/// Close both surfaces (clears state + destroys window).
pub fn close(cx: &mut App) {
    let (popup, catcher) = {
        let state = cx.global_mut::<LauncherPinMenuState>();
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

/// Auto-close after 5 seconds.
fn schedule_autoclose(cx: &mut App, generation: u64) {
    cx.spawn(async move |app_cx: &mut gpui::AsyncApp| {
        app_cx
            .background_executor()
            .timer(Duration::from_secs(5))
            .await;
        app_cx.update(|app_cx| {
            if app_cx.global::<LauncherPinMenuState>().close_generation != generation {
                return;
            }
            close(app_cx);
        });
    })
    .detach();
}

/// Register the launcher pin-menu global. Called from `launcher::init`.
pub fn init(cx: &mut App) {
    cx.set_global(LauncherPinMenuState::default());
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{Bounds, Context, DismissEvent, Entity, Modifiers, MouseButton, WindowId};

    struct EmptyView;

    impl Render for EmptyView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl gpui::IntoElement {
            div()
        }
    }

    #[test]
    fn different_entry_requires_a_fresh_anchor() {
        assert_eq!(
            open_action(Some("firefox"), "terminal"),
            OpenAction::OpenFresh
        );
    }

    #[test]
    fn same_entry_toggles_closed() {
        assert_eq!(open_action(Some("kitty"), "kitty"), OpenAction::Close);
    }

    #[test]
    fn compositor_close_matches_only_the_tracked_window() {
        let tracked = WindowId::from(11);
        assert!(window_closed_is_tracked(Some(tracked), WindowId::from(11)));
        assert!(!window_closed_is_tracked(Some(tracked), WindowId::from(12)));
        assert!(!window_closed_is_tracked(None, WindowId::from(11)));
    }
}
