//! Launcher module: desktop entry cache, fuzzy search, overlay view, launch.

pub mod cache;
pub mod entry;
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
/// against. Centering, float, pin, stay-focused and decorations are the
/// compositor's responsibility via these windowrules (shipped in
/// `docs/hyprland/chronos-launcher.lua`) — we explicitly do NOT try to
/// center via layer-shell margins anymore.
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
/// keyboard focus through the normal focus policy (filtered by our
/// `stay_focused=true` windowrule) and the overlay look comes from
/// `float + center + pin + dim_around` rules instead of the layer-shell
/// protocol. The per-frame focus re-assert in `view.rs` stays — it's cheap
/// and helpful for the rare toplevel that loses focus between open and
/// first paint.
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
    let handle = cx.global::<LauncherState>().handle.clone();
    let already_open = handle
        .as_ref()
        .map(|h| h.is_active(cx).unwrap_or(false))
        .unwrap_or(false);
    tracing::info!(already_open, "launcher::open check");
    if already_open {
        tracing::info!("launcher already open, returning");
        return;
    }

    let display_id = cx
        .primary_display()
        .map(|d| d.id())
        .or_else(|| cx.displays().into_iter().next().map(|d| d.id()));
    tracing::info!(?display_id, "launcher display_id");

    match cx.open_window(window_options(display_id), |_, cx| {
        cx.new(|cx| LauncherView::new(cx))
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

/// Toggle the launcher overlay open/closed.
pub fn toggle(cx: &mut App) {
    tracing::info!("launcher::toggle called");
    let handle = cx.global::<LauncherState>().handle.clone();
    let is_open = handle
        .as_ref()
        .map(|h| h.is_active(cx).unwrap_or(false))
        .unwrap_or(false);
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
