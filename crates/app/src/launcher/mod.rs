//! Launcher module: fuzzy search, overlay view, launch.

pub mod launch;
pub mod search;
pub mod view;

use gpui::{
    App, Bounds, DisplayId, Global, Size, Window, WindowBackgroundAppearance, WindowBounds,
    WindowHandle, WindowKind, WindowOptions, point, prelude::*, px,
};

use crate::launcher::view::LauncherView;

const LAUNCHER_WIDTH: f32 = 600.;
const LAUNCHER_HEIGHT: f32 = 400.;

/// Tracks the open launcher window so `toggle` can open/close it.
#[derive(Default)]
struct LauncherState {
    handle: Option<WindowHandle<LauncherView>>,
}

impl Global for LauncherState {}

/// Window options for the launcher as a plain XDG toplevel.
///
/// `app_id: "chronos-launcher"` is the surface's `xdg_toplevel.set_app_id`; on
/// Hyprland it becomes the window's `initialClass`, which is what
/// `hl.window_rule({ match = { class = "chronos-launcher" }, ... })` matches
/// against. Centering, float and decorations are the compositor's responsibility
/// via these windowrules (shipped in `docs/hyprland/chronos-launcher.lua`) —
/// we explicitly do NOT try to center via layer-shell margins anymore.
///
/// Why not a layer shell anymore — blood-earned context:
///   * `KeyboardInteractivity::Exclusive` on `Layer::Overlay` wedges the
///     input stack on Hyprland/Niri: the exclusive layer-surface never ack's
///     keyboard focus, the compositor waits indefinitely, the session dies.
///     Verified in a prior session, recorded in DECISIONS.log 2026-07-11.
///   * `OnDemand` opens the window but never grants keyboard focus on its
///     own (layer-shell surfaces don't participate in `xdg_activation_v1`),
///     so the user has to click before they can type in the search field —
///     see MEMORY.md "Launcher keyboard interactivity — Exclusive vs
///     OnDemand".
///   * `xdg_activation_v1` for layer-shell surfaces is explicitly rejected
///     by GPUI's own backend comment.
/// An ordinary XDG toplevel sidesteps all three: the compositor grants
/// keyboard focus through the normal focus policy and the overlay look comes
/// from `float + center + dim_around` rules instead of the layer-shell
/// protocol. Focus-lost-close is handled by `observe_window_activation` in
/// `open()` — the launcher closes when it loses focus (click away, workspace
/// switch), matching rofi/fuzzel UX. No `stay_focused` or `pin` windowrules;
/// no per-frame focus re-assert.
fn window_options(display_id: Option<DisplayId>) -> WindowOptions {
    WindowOptions {
        display_id,
        titlebar: None,
        window_bounds: Some(WindowBounds::Windowed(Bounds {
            origin: point(px(0.), px(0.)),
            size: Size::new(px(LAUNCHER_WIDTH), px(LAUNCHER_HEIGHT)),
        })),
        app_id: Some("chronos-launcher".to_string()),
        window_background: WindowBackgroundAppearance::Transparent,
        kind: WindowKind::Normal,
        ..Default::default()
    }
}

/// Open the launcher overlay. No-op if it is already open.
pub fn open(cx: &mut App) {
    tracing::info!("launcher::open called");
    // "Open" == we hold a window handle. Every close path (Esc, Enter,
    // focus-lost, toggle) goes through `close()`, which takes the handle, so
    // `handle.is_some()` is authoritative. `is_active` (== focused) is the
    // WRONG predicate here: an open-but-unfocused launcher would read as
    // "closed", and a second open() would orphan the first window — an
    // unclosable ghost that eats the session (seen live 2026-07-17).
    let already_open = cx.global::<LauncherState>().handle.is_some();
    tracing::info!(already_open, "launcher::open check");
    if already_open {
        tracing::info!("launcher already open, returning");
        return;
    }

    let display_id = crate::monitor::pult_display(cx);
    tracing::info!(?display_id, "launcher display_id");

    match cx.open_window(window_options(display_id), |window, cx| {
        let entity = cx.new(|cx| LauncherView::new(cx));
        entity.update(cx, |_view, cx| {
            // Dismiss is explicit only: Esc (LauncherView::handle_key), clicking
            // a result (launches then closes), or re-toggling the hotkey. We
            // deliberately do NOT close on focus loss: with `follow_mouse=1` in
            // Hyprland, moving the cursor onto any other surface deactivates the
            // launcher, which made it vanish the instant the pointer left it
            // (seen live 2026-07-17; the earlier 300ms debounce only delayed
            // that, it didn't fix it). The observer only re-focuses the text
            // input when focus returns, so typing keeps working after a hover.
            cx.observe_window_activation(window, move |view, window, cx| {
                if window.is_window_active() {
                    view.focus_input(window, cx);
                }
            })
            // Dropping the Subscription cancels the observer immediately —
            // it must outlive this scope for the refocus to keep firing.
            .detach();
        });
        entity
    }) {
        Ok(handle) => {
            tracing::info!("launcher window created successfully");
            // Focus the window immediately so keyboard input works without clicking
            let _ = handle.update(cx, |view, window, cx| {
                view.focus_input(window, cx);
            });
            cx.global_mut::<LauncherState>().handle = Some(handle);
        }
        Err(err) => {
            tracing::error!(%err, "Failed to open launcher window");
        }
    }
}

/// Close the launcher overlay if it is open.
pub fn close(cx: &mut App) {
    tracing::info!("launcher::close called");
    if let Some(handle) = cx.global_mut::<LauncherState>().handle.take() {
        tracing::info!("launcher handle taken, removing window");
        let _ = handle.update(cx, |_, window: &mut Window, _| window.remove_window());
    } else {
        tracing::info!("launcher::close: no handle to close");
    }
}

/// Close the launcher, but only touch the tracked handle if `window` IS the
/// tracked window. Wayland delivers activation events asynchronously: a
/// deactivation queued for an already-removed window can arrive after toggle
/// has opened a fresh one, and a blind global `close()` then steals the fresh
/// window's handle, leaving an unclosable ghost (seen live 2026-07-17).
///
/// When called from inside a window callback (activation observer, key press,
/// click handler), we already have `&mut Window` for this window — calling
/// `handle.update` on it would be a reentrant call that fails silently.
/// Instead: clear the handle directly and call `remove_window()` on the live
/// reference.
pub(crate) fn close_this(window: &mut Window, cx: &mut App) {
    let this = window.window_handle();
    let tracked = cx
        .global::<LauncherState>()
        .handle
        .as_ref()
        .map(|h| **h == this)
        .unwrap_or(false);
    if tracked {
        tracing::info!("launcher::close_this: clearing handle and removing window");
        cx.global_mut::<LauncherState>().handle.take(); // clear handle BEFORE remove
        window.remove_window(); // direct, no reentrant handle.update
    } else {
        tracing::info!("launcher::close_this: untracked window, removing self only");
        window.remove_window();
    }
}

/// Toggle the launcher overlay open/closed.
pub fn toggle(cx: &mut App) {
    tracing::info!("launcher::toggle called");
    // Same predicate as open(): holding a handle == open (see comment there).
    let is_open = cx.global::<LauncherState>().handle.is_some();
    tracing::info!(is_open, "launcher::toggle state");
    if is_open {
        close(cx);
    } else {
        open(cx);
    }
}

/// Register the launcher's global state. Called once at startup from `main.rs`.
pub fn init(cx: &mut App) {
    tracing::info!("launcher::init called");
    cx.set_global(LauncherState::default());
}
