//! Desktop-widget terminal: N layer-shell widgets, each backed by a PTY that
//! lives in a [`TerminalRegistry`] (a GPUI `Global`) independent of the
//! window — so closing a widget keeps its shell alive (T257).
//!
//! Layout/persistence lives in [`config`]; the registry in `chronos_services`.
//!
//! T259 added the edit-mode management surface (drag/resize/add/remove):
//! `move_window` / `close_one_in_window` are the window-side close helpers
//! (direct `window.remove_window()`, never a re-entrant `handle.update` —
//! HANDOFF «СИСТЕМНЫЙ БАГ» rule), `add_widget` backs the System-tab button.

mod config;
mod view;

use std::time::Duration;

use gpui::{
    App, Bounds, DisplayId, Size, Window, WindowBackgroundAppearance, WindowBounds, WindowKind,
    WindowOptions, layer_shell::*, point, prelude::*, px,
};

use chronos_services::TerminalRegistry;

use crate::desktop_terminal::view::DesktopTerminalView;

/// Re-export for sibling modules (view, side_panel System tab): the config
/// API stays in `config`, but callers should not need the module path.
pub(crate) use config::{TerminalWidgetSpec, load, make_spec, save};

/// Spike fallback size/position when a spec omits them (won't happen once
/// T259 writes specs, kept so the default is sane and never zero).
const TERM_WIDTH: f32 = 600.;
const TERM_HEIGHT: f32 = 400.;
const MARGIN_TOP: f32 = 80.;
const MARGIN_LEFT: f32 = 48.;

/// Diagonal step applied to each newly added widget so repeated clicks don't
/// stack them on top of each other (T259 §4).
const ADD_STEP: f32 = 40.;

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
    // T257 code review: the previous `let _ = handle.update(...)` swallowed
    // the Result and logged "closed" unconditionally — the exact pattern
    // that caused the ghost-window saga in launcher/tray_menu (a reentrant
    // `Err("window not found")` going silently missing). Log the real
    // outcome instead.
    match handle.update(cx, |_, window: &mut gpui::Window, _| window.remove_window()) {
        Ok(()) => tracing::info!("desktop_terminal: closed window for widget {id} (PTY kept alive)"),
        Err(err) => tracing::error!("desktop_terminal: failed to close window for widget {id}: {err}"),
    }
}

/// Remove a widget **entirely** — kill its PTY session (registry), drop its
/// spec from `desktop_terminal.toml`, then close the window.
///
/// **Window-callback path only** (T259 ✕ button): the caller already holds
/// the live `window: &mut Window`, so we close via that reference directly —
/// per the HANDOFF «СИСТЕМНЫЙ БАГ» rule a re-entrant `handle.update(...)`
/// here would silently no-op and leave a ghost surface. The PTY kill is what
/// makes this different from [`close_one`]: the shell really dies (`ps` finds
/// nothing), it is not merely hidden.
pub fn close_one_in_window(id: &str, window: &mut Window, cx: &mut App) {
    // 1. Kill the shell — the registry session, not just the window.
    registry(cx).registry.lock().expect("registry lock").kill(id);
    // 2. Drop the spec so the widget does not resurrect on the next start.
    let mut specs = config::load();
    let before = specs.len();
    specs.retain(|s| s.id != id);
    if specs.len() != before {
        if let Err(err) = config::save(&specs) {
            tracing::warn!("desktop_terminal: remove-save failed: {err}");
        }
    }
    // 3. Forget the tracked handle and close the surface directly.
    registry(cx)
        .windows
        .lock()
        .expect("registry windows lock")
        .remove(id);
    window.remove_window();
    tracing::info!("desktop_terminal: removed widget {id} (PTY killed)");
}

/// Teleport a widget window to `spec`'s (new) anchor — the drag commit
/// (T259 §1). **Window-callback path only**: same HANDOFF rule as
/// [`close_one_in_window`] — direct `window.remove_window()`, then reopen
/// with the new spec. The PTY is untouched: the registry keys sessions by
/// `spec.id`, so re-opening the same id re-attaches to the same live shell.
pub fn move_window(spec: &TerminalWidgetSpec, window: &mut Window, cx: &mut App) {
    registry(cx)
        .windows
        .lock()
        .expect("registry windows lock")
        .remove(&spec.id);
    window.remove_window();
    open_one(spec.clone(), cx);
    tracing::info!(
        "desktop_terminal: moved widget {} → anchor ({}, {})",
        spec.id,
        spec.anchor_x,
        spec.anchor_y
    );
}

/// T259 «+ Add terminal»: create a widget spec (fresh id, default size,
/// anchor offset from the last one), persist it, and open it. Backs the
/// System-tab button; public so the button only calls this one entry point.
pub fn add_widget(cx: &mut App) {
    let mut specs = config::load();
    let (anchor_x, anchor_y) = next_anchor(&specs);
    let spec = config::make_spec(anchor_x, anchor_y, TERM_WIDTH, TERM_HEIGHT);
    specs.push(spec.clone());
    if let Err(err) = config::save(&specs) {
        tracing::warn!("desktop_terminal: add-save failed: {err}");
    }
    open_one(spec, cx);
}

/// Anchor for the next widget: the last spec's anchor shifted diagonally by
/// [`ADD_STEP`], so a burst of «+ Add terminal» clicks never stacks windows
/// exactly on top of each other. No specs yet → the spike's default spot.
pub(crate) fn next_anchor(specs: &[TerminalWidgetSpec]) -> (f32, f32) {
    match specs.last() {
        Some(last) => (last.anchor_x + ADD_STEP, last.anchor_y + ADD_STEP),
        None => (MARGIN_LEFT, MARGIN_TOP),
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn next_anchor_defaults_when_no_specs() {
        assert_eq!(next_anchor(&[]), (MARGIN_LEFT, MARGIN_TOP));
    }

    #[test]
    fn next_anchor_steps_diagonally_from_last() {
        let specs = vec![TerminalWidgetSpec {
            id: "w1".into(),
            anchor_x: 48.0,
            anchor_y: 80.0,
            width: 600.0,
            height: 400.0,
        }];
        assert_eq!(next_anchor(&specs), (88.0, 120.0));
    }

    #[test]
    fn next_anchor_chains_from_newest_spec() {
        let specs = vec![
            TerminalWidgetSpec {
                id: "w1".into(),
                anchor_x: 48.0,
                anchor_y: 80.0,
                width: 600.0,
                height: 400.0,
            },
            TerminalWidgetSpec {
                id: "w2".into(),
                anchor_x: 120.0,
                anchor_y: 200.0,
                width: 600.0,
                height: 400.0,
            },
        ];
        assert_eq!(next_anchor(&specs), (160.0, 240.0));
    }
}
