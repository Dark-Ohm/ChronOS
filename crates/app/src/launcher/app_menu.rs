//! Launcher → app context menu (T265-D).
//!
//! Right-click a launcher cell/row opens ONE menu — not a second window next
//! to a pin menu. Items, all honest about current state:
//!
//! - **Launch** — `launch.rs`, same as Enter (records frecency, closes launcher).
//! - **Desktop Actions** — `entry.actions` (T265-A); section omitted when empty.
//! - **Add/Remove favorite** — `launcher.toml` `favorites.order` (T265-C).
//! - **Pin / Unpin dock** — `dock.toml`, exactly the previous pin-menu behavior.
//! - **Hide from list** — user-level `no_display` in `launcher.toml` `[hidden]`,
//!   NOT a `.desktop` edit on disk. Hidden stays in the service (T265-G).
//! - **Show in file manager** — `xdg-open` of the `.desktop` dir (exec-path dir
//!   fallback).
//! - **Properties** / **Launch as other user** — honest `disabled` with the
//!   reason in the label (no dialog in the kit / no pkexec backend — T246).
//!
//! Same engine as the tray/dock menus: `gpui-component::PopupMenu` in an
//! anchored popup whose root is a `gpui_component::Root`, `grab: false`
//! (T264), an Overlay click-catcher with a screen-space hole, and the live
//! compositor anchor query (`catcher_anchor_for`, урок `162798b4`/`180fe88`).

use std::{path::Path, process::{Command, Stdio}, rc::Rc, time::Duration};

use gpui::{
    AnyWindowHandle, App, Bounds, Context, DisplayId, DismissEvent, Entity, Focusable, Global,
    Pixels, Point, Size, Subscription, Window, WindowBackgroundAppearance, WindowBounds,
    WindowHandle, WindowId, WindowKind, WindowOptions, div, layer_shell::*, point,
    popup::{
        PopupAnchor, PopupConstraintAdjustment, PopupGravity, PopupNotSupportedError, PopupOptions,
    },
    prelude::*, px,
};

use chronos_services::applications::frecency;
use chronos_services::{strip_field_codes, AppEntry};
use gpui_component::Root;
use gpui_component::menu::{PopupMenu, PopupMenuItem};

use crate::dock::config::{update_cache, DockConfig};
use crate::dock::signal::notify_config_changed;
use crate::launcher::favorites::{desktop_dirs, desktop_file_path};
use crate::launcher::launch::launch;
use crate::launcher::launcher_config::{self, LauncherConfig};
use crate::motion;

/// Menu card minimum width (px).
const MENU_MIN_WIDTH: f32 = 240.;
/// Menu card maximum width (px) — enough for the long disabled labels.
const MENU_MAX_WIDTH: f32 = 340.;
/// Per-row height used to size the click-catcher hole (default PopupMenu size).
const ITEM_HEIGHT: f32 = 26.;
/// Separator height used for the same estimate.
const SEPARATOR_HEIGHT: f32 = 6.;
/// Menu container padding.
const MENU_PADDING: f32 = 8.;

/// Global state for the launcher app menu popup.
#[derive(Default)]
pub struct LauncherAppMenuState {
    /// Window handle while the menu is open; `None` when closed.
    handle: Option<WindowHandle<Root>>,
    /// The entry that was right-clicked (menu context).
    entry: Option<AppEntry>,
    /// Weak handle to the live menu view (unused for repaint today — kept
    /// symmetric with `tray_menu`/`dock` for future menu items).
    view: Option<gpui::WeakEntity<LauncherAppMenuView>>,
    /// Generation guard for auto-close.
    close_generation: u64,
    /// Clears stale state when the compositor dismisses the xdg-popup.
    window_closed_subscription: Option<Subscription>,
    /// Transparent layer-surface that receives clicks outside the popup while
    /// the native popup intentionally has `grab: false` (T264).
    click_catcher: Option<AnyWindowHandle>,
}

impl Global for LauncherAppMenuState {}

pub struct LauncherAppMenuView {
    popup_menu: Option<Entity<PopupMenu>>,
    dismiss_subscription: Option<Subscription>,
    enter_t: f32,
}

impl LauncherAppMenuView {
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

impl Render for LauncherAppMenuView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.popup_menu.is_none() {
            let entry = cx
                .global::<LauncherAppMenuState>()
                .entry
                .clone()
                .unwrap_or_default();
            let menu = build_app_menu(window, cx, entry);
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

// --- Pure item-state helpers (unit-testable) ---

/// Favorite item label from current state.
fn favorite_label(is_favorite: bool) -> &'static str {
    if is_favorite {
        "Remove from favorites"
    } else {
        "Add to favorites"
    }
}

/// Pin item label from current dock state.
fn pin_label(is_pinned: bool) -> &'static str {
    if is_pinned { "Unpin" } else { "Pin to dock" }
}

/// Hide item label from current hidden state.
fn hide_label(is_hidden: bool) -> &'static str {
    if is_hidden { "Unhide" } else { "Hide from list" }
}

/// Toggle an id in `favorites.order`. Returns true if it is now a favorite.
fn toggle_favorite_in(config: &mut LauncherConfig, id: &str) -> bool {
    if let Some(pos) = config.favorites.order.iter().position(|x| x == id) {
        config.favorites.order.remove(pos);
        false
    } else {
        config.favorites.order.push(id.to_string());
        true
    }
}

/// Toggle an id in `hidden`. Returns true if it is now hidden (T265-D).
fn toggle_hidden_in(config: &mut LauncherConfig, id: &str) -> bool {
    if let Some(pos) = config.hidden.iter().position(|x| x == id) {
        config.hidden.remove(pos);
        false
    } else {
        config.hidden.push(id.to_string());
        true
    }
}

/// Number of "rows" (items + separators) the menu renders, for the catcher hole.
fn menu_row_count(entry: &AppEntry) -> usize {
    // 7 fixed items (Launch, favorite, pin, hide, show-in-FM, Properties,
    // Launch-as-other-user) + 2 always-present separators. Desktop Actions,
    // when present, add a leading separator + a header label + one flat item
    // per action (T297: actions are inline items, not a flyout submenu).
    let action_rows = if entry.actions.is_empty() {
        0
    } else {
        2 + entry.actions.len() // separator + "Desktop Actions" header + N items
    };
    9 + action_rows
}

/// Conservative menu height estimate for the click-catcher hole.
fn estimate_menu_height(entry: &AppEntry) -> f32 {
    menu_row_count(entry) as f32 * ITEM_HEIGHT + 2.0 * SEPARATOR_HEIGHT + MENU_PADDING
}

// --- Actions ---

/// Launch `exec` like Enter does: record frecency, spawn, close the launcher.
fn launch_entry_and_close(id: &str, exec: &str, cx: &mut App) {
    frecency::record_launch(id);
    if let Err(err) = launch(exec) {
        tracing::error!("launcher app-menu: failed to launch {id}: {err:#}");
    }
    crate::launcher::close(cx);
}

fn toggle_favorite(id: &str) {
    launcher_config::update(|c| {
        toggle_favorite_in(c, id);
    });
}

fn toggle_pin(id: &str, cx: &mut App) {
    let is_pinned = crate::dock::config::resolve_pinned(cx).iter().any(|p| p == id);
    let mut config = DockConfig::load();
    if is_pinned {
        config.unpin(id);
        // Persist the explicit user exclusion so an unpinned mode/scene default
        // does not instantly repaint the icon.
        config.exclude(id);
    } else {
        config.pin(id);
    }
    if let Err(e) = config.save() {
        tracing::error!("launcher app-menu: failed to save dock config on pin/unpin: {e}");
    }
    update_cache(config);
    notify_config_changed(cx);
}

fn toggle_hidden(id: &str) {
    launcher_config::update(|c| {
        toggle_hidden_in(c, id);
    });
}

/// `xdg-open` the directory of the `.desktop` file, falling back to the
/// executable's directory. No `.unwrap()` — failures are logged, never panic.
fn show_in_file_manager(id: &str, exec: &str) {
    let dirs = desktop_dirs();
    let target = desktop_file_path(id, &dirs)
        .and_then(|path| path.parent().map(|d| d.to_path_buf()))
        .or_else(|| {
            let clean = strip_field_codes(exec);
            let first = clean.split_whitespace().next().unwrap_or_default();
            Path::new(first).parent().map(|d| d.to_path_buf())
        });
    match target {
        Some(dir) => {
            let result = Command::new("xdg-open")
                .arg(&dir)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn();
            if let Err(e) = result {
                tracing::error!("launcher app-menu: xdg-open {dir:?} failed: {e}");
            }
        }
        None => tracing::warn!("launcher app-menu: no file-manager path for {id}"),
    }
}

/// Build the full app context menu. Item labels/actions are chosen from current
/// state so they are always honest.
fn build_app_menu(window: &mut Window, cx: &mut App, entry: AppEntry) -> Entity<PopupMenu> {
    let is_pinned = crate::dock::config::resolve_pinned(cx).iter().any(|p| p == &entry.id);
    let config = launcher_config::get();
    let is_favorite = config.favorites.order.iter().any(|id| id == &entry.id);
    let is_hidden = config.hidden.iter().any(|id| id == &entry.id);

    PopupMenu::build(window, cx, |menu, _window, _cx| {
        let mut menu = menu.min_w(px(MENU_MIN_WIDTH)).max_w(px(MENU_MAX_WIDTH));

        // Launch.
        {
            let id = entry.id.clone();
            let exec = entry.exec.clone();
            menu = menu.item(PopupMenuItem::new("Launch").on_click(move |_e, _w, cx: &mut App| {
                launch_entry_and_close(&id, &exec, cx);
            }));
        }

        // Desktop Actions — flat list (T297). The native `submenu()` flyout
        // opens right/down OUTSIDE the fixed-size popup surface, and Wayland
        // delivers no pointer events to content past the buffer edge — the
        // child submenu was clipped and physically unclickable. Flattening
        // keeps every action inside the single sized window (actions are 1-3
        // items; nesting is not worth the extra surface).
        if !entry.actions.is_empty() {
            menu = menu.separator();
            menu = menu.item(PopupMenuItem::label("Desktop Actions"));
            let entry_id = entry.id.clone();
            for action in &entry.actions {
                let exec = action.exec.clone();
                let id = entry_id.clone();
                let name = action.name.clone();
                menu = menu.item(
                    PopupMenuItem::new(name).on_click(move |_e, _w, cx: &mut App| {
                        launch_entry_and_close(&id, &exec, cx);
                    }),
                );
            }
        }

        menu = menu.separator();

        // Add / Remove favorite.
        {
            let id = entry.id.clone();
            let label = favorite_label(is_favorite);
            menu = menu.item(PopupMenuItem::new(label).on_click(move |_e, _w, _cx: &mut App| {
                toggle_favorite(&id);
            }));
        }

        // Pin / Unpin.
        {
            let id = entry.id.clone();
            let label = pin_label(is_pinned);
            menu = menu.item(PopupMenuItem::new(label).on_click(move |_e, _w, cx: &mut App| {
                toggle_pin(&id, cx);
            }));
        }

        // Hide / Unhide.
        {
            let id = entry.id.clone();
            let label = hide_label(is_hidden);
            menu = menu.item(PopupMenuItem::new(label).on_click(move |_e, _w, _cx: &mut App| {
                toggle_hidden(&id);
            }));
        }

        menu = menu.separator();

        // Show in file manager.
        {
            let id = entry.id.clone();
            let exec = entry.exec.clone();
            menu = menu.item(
                PopupMenuItem::new("Show in file manager").on_click(move |_e, _w, _cx: &mut App| {
                    show_in_file_manager(&id, &exec);
                }),
            );
        }

        // Honest disabled items (no backend yet — T246: no control without a
        // backend, so the reason rides in the label).
        menu = menu.item(PopupMenuItem::new("Properties — no dialog in kit yet").disabled(true));
        menu = menu.item(
            PopupMenuItem::new("Launch as other user — no pkexec backend").disabled(true),
        );

        menu
    })
}

fn pick_display(cx: &App) -> Option<DisplayId> {
    crate::monitor::pult_display_id_or_primary(cx)
}

/// Convert `event_position` (window-local point of the right-click) into an
/// output-local 1×1 `Bounds` for the click-catcher's pass-through hole.
///
/// Wayland's `xdg_shell` never reports where the compositor placed a
/// toplevel (no protocol event for it) — the launcher opens `center = true`
/// via a Hyprland windowrule (`packaging/hyprland/40-windowrules-chronos.lua`), so its
/// client-side `window.bounds().origin` is frozen at the geometry we
/// *requested* (`(0, 0)`, see `launcher/mod.rs::window_options`), not where
/// it actually ended up. Adding that frozen origin to `event_position` (the
/// T275-01 attempt) was a no-op in practice — the click-catcher's hole
/// landed at the request-time position (screen top-left) regardless of
/// where the launcher actually rendered (screen center), so hover over the
/// visible menu never got a pointer cursor and clicks never reached it.
///
/// The only source of truth is a live compositor query: ask Hyprland where
/// `chronos-launcher` actually is (`Clients::get`, global layout space),
/// then subtract the pult display's own origin to land in that display's
/// local space — the same frame `popup_click_catcher::open_for_popup` uses
/// for its input regions. Falls back to `event_position` alone (pre-fix
/// behavior — wrong for a centered window, but no worse) if Hyprland is
/// unreachable, e.g. under Niri; this menu is Hyprland-primary like the
/// rest of the shell.
fn catcher_anchor_for(cx: &App, event_position: Point<Pixels>) -> Bounds<Pixels> {
    let origin = launcher_output_local_origin(cx).unwrap_or_default();
    Bounds::new(origin + event_position, Size::new(px(1.), px(1.)))
}

fn launcher_output_local_origin(cx: &App) -> Option<Point<Pixels>> {
    let display = crate::monitor::pult_display_info(cx)?;
    let (wx, wy) = chronos_services::compositor::hyprland::window_position("chronos-launcher")?;
    let display_origin = display.bounds().origin;
    Some(point(
        px(wx as f32) - display_origin.x,
        px(wy as f32) - display_origin.y,
    ))
}

fn open_click_catcher(
    cx: &mut App,
    anchor_rect: Bounds<Pixels>,
    popup_size: Size<Pixels>,
) -> anyhow::Result<AnyWindowHandle> {
    crate::popup_click_catcher::open_for_popup(
        cx,
        anchor_rect,
        popup_size,
        Rc::new(|window, cx| close_from_click_catcher(window, cx)),
    )
}

fn window_options(
    anchor_rect: Bounds<Pixels>,
    parent: AnyWindowHandle,
    height: f32,
) -> WindowOptions {
    WindowOptions {
        titlebar: None,
        window_bounds: Some(WindowBounds::Windowed(Bounds {
            origin: point(px(0.), px(0.)),
            size: Size::new(px(MENU_MAX_WIDTH), px(height)),
        })),
        app_id: Some("chronos-launcher-app-menu".to_string()),
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

fn fallback_window_options(display_id: Option<DisplayId>, height: f32) -> WindowOptions {
    WindowOptions {
        display_id,
        titlebar: None,
        window_bounds: Some(WindowBounds::Windowed(Bounds {
            origin: point(px(0.), px(0.)),
            size: Size::new(px(MENU_MAX_WIDTH), px(height)),
        })),
        app_id: Some("chronos-launcher-app-menu".to_string()),
        window_background: WindowBackgroundAppearance::Transparent,
        kind: WindowKind::LayerShell(LayerShellOptions {
            namespace: "launcher-app-menu".to_string(),
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

/// Open the app context menu for `entry`, anchored at the right-clicked cell.
///
/// `anchor_rect` is surface-local (window-local) — it positions the
/// `AnchoredPopup` (the Wayland positioner expects parent-surface coords).
/// `event_position` is the same click, also window-local; `open()` converts
/// it to output-local itself (via [`catcher_anchor_for`]) for the click-
/// catcher's pass-through hole.
pub fn open(
    cx: &mut App,
    anchor_rect: Bounds<Pixels>,
    event_position: Point<Pixels>,
    parent: AnyWindowHandle,
    entry: AppEntry,
) {
    let height = estimate_menu_height(&entry);
    let catcher_anchor = catcher_anchor_for(cx, event_position);
    let requested_entry = entry.id.clone();
    let open_entry = cx
        .global::<LauncherAppMenuState>()
        .entry
        .as_ref()
        .map(|e| e.id.as_str());
    match open_action(open_entry, &requested_entry) {
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
        let state = cx.global_mut::<LauncherAppMenuState>();
        state.entry = Some(entry);
        state.close_generation = state.close_generation.wrapping_add(1);
        state.close_generation
    };

    let handle = cx.global::<LauncherAppMenuState>().handle.clone();
    match handle {
        Some(existing) => {
            let _ = existing.update(cx, |_, _window, view_cx| {
                view_cx.notify();
            });
        }
        None => {
            let click_catcher = open_click_catcher(
                cx,
                catcher_anchor,
                Size::new(px(MENU_MAX_WIDTH), px(height)),
            )
            .ok();
            let mut opened_view: Option<gpui::WeakEntity<LauncherAppMenuView>> = None;
            let mut open = |cx: &mut App, options: WindowOptions| {
                cx.open_window(options, |window, view_cx| {
                    let view = view_cx.new(|view_cx| LauncherAppMenuView::new(view_cx));
                    opened_view = Some(view.downgrade());
                    view_cx.new(|view_cx| {
                        Root::new(view, window, view_cx)
                            .bordered(false)
                            .bg(gpui::transparent_black())
                    })
                })
            };
            let result = match open(cx, window_options(anchor_rect, parent, height)) {
                Err(err) => {
                    if let Some(catcher) = click_catcher {
                        let _ = catcher.update(cx, |_, window, _| window.remove_window());
                    }
                    if err.downcast_ref::<PopupNotSupportedError>().is_some() {
                        tracing::warn!(
                            "launcher app-menu: AnchoredPopup not supported, falling back to layer-shell"
                        );
                        let display_id = pick_display(cx);
                        open(cx, fallback_window_options(display_id, height))
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
                            .global::<LauncherAppMenuState>()
                            .handle
                            .as_ref()
                            .map(|handle| handle.window_id());
                        if closed_id != window_id || !window_closed_is_tracked(tracked, closed_id) {
                            return;
                        }
                        let catcher = {
                            let state = cx.global_mut::<LauncherAppMenuState>();
                            state.handle = None;
                            state.view = None;
                            state.entry = None;
                            state.close_generation = state.close_generation.wrapping_add(1);
                            state.window_closed_subscription = None;
                            state.click_catcher.take()
                        };
                        if let Some(catcher) = catcher {
                            let _ = catcher.update(cx, |_, window, _| window.remove_window());
                        }
                    });
                    let state = cx.global_mut::<LauncherAppMenuState>();
                    state.handle = Some(new_handle);
                    state.view = opened_view;
                    state.window_closed_subscription = Some(window_closed_subscription);
                    state.click_catcher = click_catcher;
                }
                Err(err) => tracing::warn!("launcher app-menu: failed to open: {err}"),
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
        .global::<LauncherAppMenuState>()
        .handle
        .as_ref()
        .map(|h| **h == this)
        .unwrap_or(false);
    let catcher = {
        let state = cx.global_mut::<LauncherAppMenuState>();
        if tracked {
            state.handle.take();
            state.view = None;
            state.window_closed_subscription = None;
        }
        state.click_catcher.take()
    };
    let state = cx.global_mut::<LauncherAppMenuState>();
    state.entry = None;
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
        let state = cx.global_mut::<LauncherAppMenuState>();
        state.entry = None;
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
        let state = cx.global_mut::<LauncherAppMenuState>();
        state.entry = None;
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
            if app_cx.global::<LauncherAppMenuState>().close_generation != generation {
                return;
            }
            close(app_cx);
        });
    })
    .detach();
}

/// Register the launcher app-menu global. Called from `launcher::init`.
pub fn init(cx: &mut App) {
    cx.set_global(LauncherAppMenuState::default());
}

#[cfg(test)]
mod tests {
    use super::*;
    use chronos_services::DesktopAction;

    fn entry_with_actions() -> AppEntry {
        AppEntry {
            actions: vec![
                DesktopAction {
                    id: "NewWorkspace".into(),
                    name: "New Workspace".into(),
                    exec: "/usr/bin/zed --new".into(),
                },
                DesktopAction {
                    id: "OpenFolder".into(),
                    name: "Open Folder".into(),
                    exec: "/usr/bin/zed --folder".into(),
                },
            ],
            ..AppEntry::fixture("zed", "Zed")
        }
    }

    #[test]
    fn pin_vs_unpin_labels() {
        assert_eq!(pin_label(false), "Pin to dock");
        assert_eq!(pin_label(true), "Unpin");
    }

    #[test]
    fn favorite_and_hide_labels_flip() {
        assert_eq!(favorite_label(false), "Add to favorites");
        assert_eq!(favorite_label(true), "Remove from favorites");
        assert_eq!(hide_label(false), "Hide from list");
        assert_eq!(hide_label(true), "Unhide");
    }

    #[test]
    fn hide_writes_id_into_hidden_and_toggles() {
        let mut config = LauncherConfig::default();
        assert!(toggle_hidden_in(&mut config, "firefox"));
        assert_eq!(config.hidden, vec!["firefox"]);
        assert!(!toggle_hidden_in(&mut config, "firefox"), "second toggle unhides");
        assert!(config.hidden.is_empty());
    }

    #[test]
    fn favorite_toggle_writes_order() {
        let mut config = LauncherConfig::default();
        assert!(toggle_favorite_in(&mut config, "kitty"));
        assert_eq!(config.favorites.order, vec!["kitty"]);
        assert!(!toggle_favorite_in(&mut config, "kitty"));
        assert!(config.favorites.order.is_empty());
    }

    #[test]
    fn desktop_action_id_maps_to_exec() {
        // The mapping the menu relies on: each Desktop Action's id pairs with
        // its own stripped exec (T265-A), so clicking an action launches it.
        let entry = entry_with_actions();
        let exec_of = |id: &str| {
            entry
                .actions
                .iter()
                .find(|a| a.id == id)
                .map(|a| a.exec.as_str())
        };
        assert_eq!(exec_of("NewWorkspace"), Some("/usr/bin/zed --new"));
        assert_eq!(exec_of("OpenFolder"), Some("/usr/bin/zed --folder"));
        assert_eq!(exec_of("Missing"), None);
    }

    #[test]
    fn menu_row_count_and_height_grow_with_actions() {
        let no_actions = AppEntry::fixture("x", "X");
        assert_eq!(menu_row_count(&no_actions), 9);
        assert!(estimate_menu_height(&no_actions) > 0.0);
        let with_actions = entry_with_actions();
        assert!(
            estimate_menu_height(&with_actions) > estimate_menu_height(&no_actions),
            "Desktop Actions must add height"
        );
        // Two actions → separator + header + 2 flat items = 4 extra rows.
        assert_eq!(menu_row_count(&with_actions), 13);
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
