//! Desktop-widget terminal: N layer-shell widgets, each backed by a PTY that
//! lives in a [`TerminalRegistry`] (a GPUI `Global`) independent of the
//! window — so closing a widget keeps its shell alive (T257).
//!
//! Layout/persistence lives in [`config`]; the registry in `chronos_services`.

mod config;
mod view;

use std::time::Duration;

use gpui::{
    App, Bounds, DisplayId, Size, WindowBackgroundAppearance, WindowBounds, WindowKind,
    WindowOptions, layer_shell::*, point, prelude::*, px,
};

use chronos_services::TerminalRegistry;

use crate::desktop_terminal::config::TerminalWidgetSpec;
use crate::desktop_terminal::view::DesktopTerminalView;

/// Spike fallback size/position when a spec omits them (won't happen once
/// T259 writes specs, kept so the default is sane and never zero).
const TERM_WIDTH: f32 = 600.;
const TERM_HEIGHT: f32 = 400.;
const MARGIN_TOP: f32 = 80.;
const MARGIN_LEFT: f32 = 48.;

/// GPUI global wrapper for [`TerminalRegistry`].
///
/// `TerminalRegistry` is defined in `chronos_services`, which must stay
/// GPUI-agnostic, so it cannot `impl gpui::Global` there (orphan rule). We
/// wrap it here. Stored as `Arc<Mutex<>>` so `View`s can hold a cheap clone
/// and reach the registry from any context. The `windows` map lets
/// `close_one` find the `WindowHandle` for a widget id without iterating
/// every window (which would need a typed downcast we don't have a clean API
/// for).
pub struct TerminalRegistryGlobal {
    pub registry: std::sync::Arc<std::sync::Mutex<TerminalRegistry>>,
    pub windows: std::sync::Mutex<std::collections::HashMap<String, gpui::WindowHandle<DesktopTerminalView>>>,
}

impl gpui::Global for TerminalRegistryGlobal {}

/// Access the registry global (panics if not registered — caller must ensure
/// `main.rs` registered it before any widget opens).
pub fn registry(cx: &App) -> &TerminalRegistryGlobal {
    cx.global::<TerminalRegistryGlobal>()
}

/// Pick the chrome display for the terminal window. Same fallback chain
/// as every other surface: configured uuid → largest display → primary.
/// First-time use will seed `~/.config/chronos/monitor.toml` via
/// `monitor::auto-designate` — that's the intended behaviour.
fn pick_display(cx: &App) -> Option<DisplayId> {
    crate::monitor::pult_display_id_or_primary(cx)
}

/// Build layer-shell window options for a widget spec. Size + anchor come from
/// the spec; when the spec leaves them at zero (defensive) we fall back to the
/// spike defaults so the window is always legal.
fn window_options(
    spec: &TerminalWidgetSpec,
    display_id: Option<DisplayId>,
) -> WindowOptions {
    let width = if spec.width > 0.0 { spec.width } else { TERM_WIDTH };
    let height = if spec.height > 0.0 {
        spec.height
    } else {
        TERM_HEIGHT
    };
    let anchor_x = if spec.anchor_x > 0.0 {
        spec.anchor_x
    } else {
        MARGIN_LEFT
    };
    let anchor_y = if spec.anchor_y > 0.0 {
        spec.anchor_y
    } else {
        MARGIN_TOP
    };

    WindowOptions {
        display_id,
        titlebar: None,
        window_bounds: Some(WindowBounds::Windowed(Bounds {
            origin: point(px(0.), px(0.)),
            size: Size::new(px(width), px(height)),
        })),
        app_id: Some("chronos-desktop-terminal".to_string()),
        window_background: WindowBackgroundAppearance::Transparent,
        kind: WindowKind::LayerShell(LayerShellOptions {
            namespace: "desktop-terminal".to_string(),
            layer: Layer::Background,
            anchor: Anchor::TOP | Anchor::LEFT,
            exclusive_zone: None,
            // CSS order: top, right, bottom, left.
            margin: Some((px(anchor_y), px(0.), px(0.), px(anchor_x))),
            keyboard_interactivity: KeyboardInteractivity::OnDemand,
            ..Default::default()
        }),
        ..Default::default()
    }
}

/// Open exactly one widget window for `spec`. The PTY is acquired from the
/// registry (idempotent per `spec.id`) so re-opening the same widget reuses
/// its live shell. Public so T259 can add/remove widgets.
pub fn open_one(spec: TerminalWidgetSpec, cx: &mut App) {
    let display_id = pick_display(cx);
    match cx.open_window(window_options(&spec, display_id), |_window, cx| {
        cx.new(|cx| DesktopTerminalView::new(spec.id.clone(), cx))
    }) {
        Ok(handle) => {
            // Track the handle so `close_one` can close it by id.
            registry(cx)
                .windows
                .lock()
                .expect("registry windows lock")
                .insert(spec.id.clone(), handle);
            tracing::info!(
                "desktop_terminal: opened widget {} ({}×{}, anchor x={} y={})",
                spec.id,
                spec.width,
                spec.height,
                spec.anchor_x,
                spec.anchor_y
            );
        }
        Err(err) => tracing::error!(
            "desktop_terminal: failed to open widget {}: {err}",
            spec.id
        ),
    }
}

/// Close a widget's *window* (the PTY is intentionally kept alive in the
/// registry — callers that want to kill the shell too must
/// `registry(cx).registry.lock().kill(id)`). Public for T259 (the window's
/// close / remove button). Uses the tracked `WindowHandle` and the
/// non-reentrant `remove_window` pattern shared by the other popups.
pub fn close_one(id: &str, cx: &mut App) {
    let handle = registry(cx)
        .windows
        .lock()
        .expect("registry windows lock")
        .remove(id);
    let Some(handle) = handle else {
        tracing::warn!("desktop_terminal: no open window for widget {id}");
        return;
    };
    // `remove_window` must run on the live window, never via re-entrant
    // `handle.update` (see HANDOFF.md "СИСТЕМНЫЙ БАГ: window.remove_window()").
    let _ = handle.update(cx, |_, window: &mut gpui::Window, _| window.remove_window());
    tracing::info!("desktop_terminal: closed window for widget {id} (PTY kept alive)");
}

/// Open the registry-backed widgets at startup. Reads the persisted spec list;
/// an empty list (no file) means zero windows — the old spike behaviour of
/// always opening one is gone.
pub fn init(cx: &mut App) {
    let specs = config::load();
    tracing::info!(
        "desktop_terminal: init — {} widget(s) from config",
        specs.len()
    );
    cx.spawn(async move |cx| {
        cx.background_executor()
            .timer(Duration::from_millis(200))
            .await;
        let _ = cx.update(|cx: &mut App| {
            for spec in specs {
                open_one(spec, cx);
            }
        });
    })
    .detach();
}
