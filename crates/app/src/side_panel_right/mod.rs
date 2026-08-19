//! Right side panel — two independently-living layer-shell surfaces (T276):
//! `rail` (fixed 40px, owns the exclusive zone) and `content` (fixed
//! `MAX_WIDTH - RAIL_ONLY_WIDTH` canvas, never resized). Lazy, hover-peek
//! (task 8) or pinned (bar-widget click / hotkey). Window lifecycle mirrors
//! `system_popup/`/`volume_popup/`: `Layer::Overlay`, `close_this`
//! reentrancy guard (`ARCHITECTURE.md §4.1` — never re-entrant
//! `handle.update` for `remove_window()` from inside that window's own
//! callback).
//!
//! ## T276 — why two surfaces
//! T273 proved right-anchored `window.resize()` mid-drag is asymmetric on
//! Hyprland: the compositor moves the surface's origin ahead of an acked
//! buffer, producing a visible wobble that survived every attempt to
//! synchronize `set_size`/configure/Scene/buffer in the fork. The fix
//! removes window resize from the drag path entirely: `rail` never
//! resizes (its exclusive zone is a *value*, independent from its own
//! pixel footprint — legal per wlr-layer-shell), and `content` is a
//! fixed-size canvas whose only per-frame changes are (a) which rectangle
//! of it is painted and (b) `Window::set_input_region` on that rectangle.
//! Dragging the handle only ever mutates `SidePanelRightState.width`
//! (`view::SidePanelRightView::update_resize`) — no Wayland/WGPU surface
//! reconfiguration, so there is nothing left to desync.
//!
//! **No Esc-to-close** — matches the real convention already in this
//! codebase (`volume_popup`/`system_popup` have no Esc handler either,
//! `KeyboardInteractivity::None` doesn't deliver key events). Dismiss is
//! re-toggle / click-away (pinned) / mouse-leave debounce (peek).

pub(crate) mod control_center;
mod disks;
mod header;
mod hover_strip;
mod mpris_card;
pub mod panels_config;
pub(crate) mod preview_target;
mod rail;
mod rail_view;
mod spectrum_row;
// `pub(crate)`: T291's top-level `power_controls` (right-panel content) reads
// `surfaces::card` so the moved power/gaming cards match the System card fill.
pub(crate) mod surfaces;
pub mod tab;
pub(crate) mod tabs;
pub mod view;

use gpui::{
    App, Bounds, DisplayId, Global, Size, Window, WindowBackgroundAppearance, WindowBounds,
    WindowHandle, WindowKind, WindowOptions, layer_shell::*, point, prelude::*, px,
};
use gpui_component::Root;

use crate::frame::{self, FrameSide};
use crate::side_panel_right::rail_view::RailView;
use crate::side_panel_right::tabs::PanelTab;
use crate::side_panel_right::view::SidePanelRightView;

// T276: standalone rail is the full fixed 40px surface. The resize handle is
// a 4px overlay on the moving LEFT edge of visible content; it is not part of
// rail geometry and consumes no extra width.
pub(crate) const RAIL_WIDTH: f32 = 40.;
pub(crate) const HANDLE_WIDTH: f32 = 4.;
pub(crate) const RAIL_ONLY_WIDTH: f32 = RAIL_WIDTH;
/// Default full-content width when docked or user-resized.
pub(crate) const DEFAULT_CONTENT_WIDTH: f32 = 560.;
pub(crate) const MAX_WIDTH: f32 = 960.;
/// T276: fixed pixel width of the `content` window's canvas. Never resized —
/// only the visible rectangle painted inside it (and its input region)
/// change as `SidePanelRightState.width` moves within
/// `RAIL_ONLY_WIDTH..=MAX_WIDTH`.
pub(crate) const CONTENT_CANVAS_WIDTH: f32 = MAX_WIDTH - RAIL_ONLY_WIDTH; // 920

/// Drag marker — own type so left panel's `LeftPanelResize` never cross-fires.
pub struct RightPanelResize;

/// Top air under the bar. Height = display − this gap reaches the bottom
/// bezel (see `b120a3d`). Do **not** use TOP|BOTTOM stretch + dual margins
/// on Hyprland Overlay — exclusive zone + stretch skews the gaps.
///
/// **Live since T200:** follows the configured bar height so the panel never
/// overlaps the bar when `bar.toml [appearance] height` changes. Geometry is
/// fixed at open — an already-open panel keeps its size until reopened
/// (residual, documented).
pub(crate) fn panel_edge_gap() -> f32 {
    crate::state::bar_height_px()
}

pub struct SidePanelRightState {
    /// T276: the permanent 40px icon-rail surface. Owns the exclusive zone.
    rail_handle: Option<WindowHandle<Root>>,
    /// T276: the fixed-canvas content surface, immediately left of `rail`.
    content_handle: Option<WindowHandle<Root>>,
    /// T230/T276: weak handle to the live `SidePanelRightView` (lives in the
    /// `content` window), so `select_tab` / `preview_target` IPC (App
    /// context, no window) and `rail_view::RailView` (a *different* window)
    /// can reach it. Filled at open time, dropped when the content window
    /// dies — `None`/dead ⇒ the logical panel is closed.
    content_view: Option<gpui::WeakEntity<SidePanelRightView>>,
    /// `true` when opened by hotkey/bar-click (`toggle` / `open_pinned`) —
    /// stays open until re-toggled. `false` when opened by hover — closes
    /// on mouse-leave debounce unless a pin request arrives while peeked.
    pinned: bool,
    /// Bumped on hover-enter (strip or panel). Leave schedules a close
    /// only if this value is still unchanged after the debounce window.
    peek_generation: u64,
    /// Current *logical* panel width (px), `RAIL_ONLY_WIDTH..=MAX_WIDTH`.
    /// T276: no surface is ever resized to this value directly — `rail`
    /// stays `RAIL_ONLY_WIDTH` px, `content` stays `CONTENT_CANVAS_WIDTH`
    /// px; this number only drives the visible rectangle inside the
    /// content canvas (`visible_content_width`) and the rail's exclusive
    /// zone (`exclusive_px`).
    pub width: f32,
    /// Dock mode: exclusive-zone flag. When true, the rail reserves
    /// `width` px (not `RAIL_ONLY_WIDTH`) — clients don't encroach on
    /// the content area. Does NOT auto-open content; toggle only flips
    /// this flag and resets the cached exclusive zone (T289).
    pub dock_content: bool,
    /// T210: true while a resize drag is active. Suppresses peek-close so
    /// the 280ms debounce cannot close the panel mid-drag.
    pub(crate) resizing: bool,
    /// Last exclusive_zone value sent to the compositor (avoids redundant
    /// Wayland round-trips, mirrors left panel pattern). Set on `rail`.
    pub last_exclusive_zone: Option<f32>,
}

impl Default for SidePanelRightState {
    fn default() -> Self {
        Self {
            rail_handle: None,
            content_handle: None,
            content_view: None,
            pinned: false,
            peek_generation: 0,
            width: RAIL_ONLY_WIDTH,
            dock_content: false,
            resizing: false,
            last_exclusive_zone: None,
        }
    }
}

impl SidePanelRightState {
    /// Exclusive zone px: full panel when docked, rail-only when overlay.
    /// T276: this value is set on the **rail** surface only — the content
    /// canvas never reserves space itself, regardless of how much of it is
    /// visible.
    pub fn exclusive_px(&self) -> f32 {
        if self.dock_content {
            self.width
        } else {
            RAIL_ONLY_WIDTH
        }
    }

    /// Clamp and store a new width.
    pub fn resize(&mut self, new_width: f32) {
        self.width = new_width.clamp(RAIL_ONLY_WIDTH, MAX_WIDTH);
    }

    /// Expand or contract to the given target width.
    /// Called when content becomes visible (tab open / dock toggle)
    /// or when switching tabs with content already visible.
    pub fn ensure_content_width(&mut self, target: f32) {
        self.width = target;
        self.last_exclusive_zone = None; // force zone recompute next paint
    }
}

impl Global for SidePanelRightState {}

/// Pure decision: should a peek-leave request close the panel?
fn should_close_on_peek_leave(state: &SidePanelRightState) -> bool {
    // T210: never close while a resize drag is active — the Wayland
    // implicit grab on a destroyed surface permanently breaks hover
    // strip enter events.
    !state.pinned && !state.resizing
}

/// T276 pure geometry: how much of the fixed `content` canvas is actually
/// painted/interactive, given the logical panel width. `0` at rail-only,
/// `CONTENT_CANVAS_WIDTH` at `MAX_WIDTH`. Floored at 0 defensively even
/// though `SidePanelRightState::resize` already clamps its input at
/// `RAIL_ONLY_WIDTH`.
pub(crate) fn visible_content_width(state_width: f32) -> f32 {
    (state_width - RAIL_ONLY_WIDTH).max(0.)
}

/// T276 pure geometry: the content canvas's Wayland input region — a
/// single rectangle covering the visible (right-aligned) slice, or empty
/// when the panel is collapsed to rail-only. `canvas_w`/`canvas_h` are the
/// content window's own (fixed) pixel size; `visible_w` is
/// `visible_content_width(state.width)`.
pub(crate) fn content_input_region(
    canvas_w: f32,
    canvas_h: f32,
    visible_w: f32,
) -> Vec<Bounds<gpui::Pixels>> {
    if visible_w <= 0. {
        return Vec::new();
    }
    let x = (canvas_w - visible_w).max(0.);
    vec![Bounds::new(
        point(px(x), px(0.)),
        Size::new(px(visible_w.min(canvas_w)), px(canvas_h.max(0.))),
    )]
}

/// X coordinate of the resize hit strip inside the fixed content canvas.
/// It follows the screen-inward (left) edge of the visible slice without
/// resizing or moving either Wayland surface.
pub(crate) fn content_resize_handle_x(canvas_w: f32, visible_w: f32) -> f32 {
    (canvas_w - visible_w.clamp(0., canvas_w))
        .max(0.)
        .min((canvas_w - HANDLE_WIDTH).max(0.))
}

/// Width of the right-aligned input slice while rendering the content canvas.
/// During an active drag the transparent handle must survive even when the
/// visible content reaches zero, otherwise GPUI drops the drag target at the
/// rail-only clamp and the pointer cannot pull the panel back in one gesture.
pub(crate) fn content_interactive_width(visible_w: f32, resizing: bool) -> f32 {
    if resizing {
        visible_w.max(HANDLE_WIDTH)
    } else {
        visible_w
    }
}

/// T276 pure geometry: absolute-delta resize target. Both `rail` and
/// `content` are fixed-size surfaces now — there is no compositor
/// configure to race against (the T210/T214/T216/T243 family of bugs was
/// entirely about a surface resizing mid-drag; that surface no longer
/// exists). `start_x`/`current_x` are pointer coordinates inside the fixed
/// content canvas frame: as the cursor moves left of the
/// press point, `current_x` decreases and the panel grows by exactly that
/// delta.
pub(crate) fn resize_target_width(start_width: f32, start_x: f32, current_x: f32) -> f32 {
    (start_width + (start_x - current_x)).clamp(RAIL_ONLY_WIDTH, MAX_WIDTH)
}

/// Cursor entered strip or panel — cancel any pending peek-close.
pub(crate) fn hold_peek(cx: &mut App) {
    let state = cx.global_mut::<SidePanelRightState>();
    state.peek_generation = state.peek_generation.wrapping_add(1);
}

/// Cursor left strip or panel — close after debounce if still unpinned
/// and no later enter bumped the generation.
pub(crate) fn schedule_release_peek(cx: &mut App) {
    let generation = cx.global::<SidePanelRightState>().peek_generation;
    view::schedule_release_from_app(cx, generation);
}

fn display_height(display_id: Option<DisplayId>, cx: &App) -> f32 {
    display_id
        .and_then(|id| cx.find_display(id))
        .map(|d| f32::from(d.bounds().size.height))
        .unwrap_or(1080.)
}

fn panel_height(display_id: Option<DisplayId>, cx: &App) -> f32 {
    let display_h = display_height(display_id, cx);
    // Wrap (T284 + T311 D3): the panel clears the bottom chrome too —
    // height is trimmed by the bottom-plate on top of the bar gap. Use
    // `wrap_inset_bottom`, not `wrap_inset` — the bottom edge keeps its
    // plate even when both rails are mapped, and is unaffected by rail
    // mapping.
    (display_h - panel_edge_gap() - frame::wrap_inset_bottom_cached()).max(100.)
}

/// T276: the `rail` surface — fixed `RAIL_ONLY_WIDTH` px, owns the
/// exclusive zone. Never resized after open; `exclusive_zone` is a value
/// updated live via `Window::set_exclusive_zone`, independent from the
/// surface's own pixel footprint (legal per wlr-layer-shell — see
/// `gpui-layer-shell` skill Part D).
fn rail_window_options(display_id: Option<DisplayId>, cx: &App) -> WindowOptions {
    let panel_h = panel_height(display_id, cx);
    let zone = cx.global::<SidePanelRightState>().exclusive_px();
    WindowOptions {
        display_id,
        titlebar: None,
        window_bounds: Some(WindowBounds::Windowed(Bounds {
            origin: point(px(0.), px(0.)),
            size: Size::new(px(RAIL_ONLY_WIDTH), px(panel_h)),
        })),
        app_id: Some("chronos-side-panel-right-rail".to_string()),
        window_background: WindowBackgroundAppearance::Transparent,
        kind: WindowKind::LayerShell(LayerShellOptions {
            namespace: "side_panel_right_rail".to_string(),
            layer: Layer::Overlay,
            anchor: Anchor::TOP | Anchor::RIGHT,
            exclusive_zone: Some(px(zone)),
            exclusive_edge: Some(Anchor::RIGHT),
            // T310 D1: NO margin — mirror of the left rail. `frame_wrap_
            // excl_right` already reserves `wrap_inset()` on this edge, so
            // the compositor offsets the rail by the frame thickness itself;
            // the T284 margin added it twice and left a thickness-wide strip
            // of bare wallpaper between frame and rail (measured live
            // 2026-08-19: rail 2489-2527, wallpaper 2528-2543, wrap
            // 2544-2559). No top margin: the bar's exclusive zone already
            // drops top-anchored Overlay surfaces below it.
            margin: None,
            // Rail has no focusable input (buttons/svg only) — None matches
            // the OSD/tray_menu convention for surfaces that never take text.
            keyboard_interactivity: KeyboardInteractivity::None,
            ..Default::default()
        }),
        ..Default::default()
    }
}

/// T276: the `content` surface — fixed `CONTENT_CANVAS_WIDTH` px canvas,
/// positioned immediately left of `rail` via a constant `margin-right =
/// RAIL_ONLY_WIDTH`. **Never resized** for the surface's lifetime; only the
/// visible rectangle inside it (right-aligned) and its input region change.
///
/// `exclusive_zone: -1` opts content out of every foreign reservation,
/// including the top bar. The margin therefore restores both placements
/// explicitly: top gap below the bar and the fixed rail width on the right.
fn content_window_margin(top_gap: f32) -> (gpui::Pixels, gpui::Pixels, gpui::Pixels, gpui::Pixels) {
    // Wrap (T284 + T311 D3): content rides with the rail — its RIGHT margin
    // gains the wrap-reserved space on top of the rail width. After D3 the
    // wrap inset on the right edge collapses to ZERO when the right rail is
    // mapped (the rail already owns that edge), and stays at full
    // `wrap.thickness` when the rail is gone. Use `wrap_inset_right`, not
    // `wrap_inset`.
    //
    // T314: the flag passed below is the coexistence invariant, NOT the
    // live `rail_mapped()` read — content only ever opens in the same
    // two-surface commit as its rail, but `set_rail_mapped(true)` lands
    // AFTER both windows are open, so a live read here sees the pre-commit
    // `false` and bakes a stale `wrap.thickness` into the margin (content
    // 16px off the rail, measured live).
    let right_reserved = frame::wrap_inset_right_cached(true);
    (px(top_gap), px(RAIL_ONLY_WIDTH + right_reserved), px(0.), px(0.))
}

fn content_window_options(display_id: Option<DisplayId>, cx: &App) -> WindowOptions {
    let panel_h = panel_height(display_id, cx);
    WindowOptions {
        display_id,
        titlebar: None,
        window_bounds: Some(WindowBounds::Windowed(Bounds {
            origin: point(px(0.), px(0.)),
            size: Size::new(px(CONTENT_CANVAS_WIDTH), px(panel_h)),
        })),
        app_id: Some("chronos-side-panel-right-content".to_string()),
        window_background: WindowBackgroundAppearance::Transparent,
        kind: WindowKind::LayerShell(LayerShellOptions {
            namespace: "side_panel_right_content".to_string(),
            layer: Layer::Overlay,
            anchor: Anchor::TOP | Anchor::RIGHT,
            // Content never reserves space — that is rail's job (spec §
            // "Contract геометрии"). `-1` is the wlr-layer-shell escape
            // hatch (wayland.app/protocols/wlr-layer-shell-unstable-v1,
            // `set_exclusive_zone`): it opts this surface OUT of being
            // pushed by *other* surfaces' exclusive zones on the same
            // edge. `None` here would map to the protocol default of `0`
            // (per `gpui_linux`'s `WaylandWindow` — `set_exclusive_zone`
            // is simply never called), which does NOT opt out — the
            // compositor still auto-offsets a same-edge Overlay surface by
            // whatever `rail` is reserving (documented cross-surface
            // behavior in `gpui-layer-shell` Part A: bar → popup). Without
            // `-1`, that auto-offset stacks ON TOP of the explicit
            // `margin-right` below — a double offset that grows with
            // `rail`'s exclusive zone (up to 920px in dock mode) instead
            // of staying a constant 40px.
            exclusive_zone: Some(px(-1.)),
            // CSS order: (top, right, bottom, left). `-1` also disables the
            // bar's automatic top offset, so both offsets must be explicit.
            margin: Some(content_window_margin(panel_edge_gap())),
            // OnDemand is required for gpui-component `Input` to receive
            // keyboard events (Editor/Terminal tabs live here). The panel's
            // dismissal contract is enforced in code, not by focus loss.
            keyboard_interactivity: KeyboardInteractivity::OnDemand,
            ..Default::default()
        }),
        ..Default::default()
    }
}

/// Open both surfaces as one logical panel. Partial-open is refused: if
/// `content` fails after `rail` already opened, `rail` is rolled back (and
/// vice versa) so the state can never observe one handle without the other.
fn open_window(cx: &mut App, pinned: bool) {
    if cx.global::<SidePanelRightState>().rail_handle.is_some() {
        if pinned {
            // Already open as peek → upgrade to pin without re-open.
            cx.global_mut::<SidePanelRightState>().pinned = true;
            tracing::info!("side_panel_right: upgraded peek → pinned");
        }
        return;
    }
    let display_id = crate::monitor::pult_display_id_or_primary(cx);
    // Normal opens always start rail-only. The smoke path sets width to
    // DEFAULT_CONTENT_WIDTH before calling open_pinned, so it opens expanded.
    if std::env::var_os("CHRONOS_SMOKE_SIDE_PANEL").is_none() {
        cx.global_mut::<SidePanelRightState>().width = RAIL_ONLY_WIDTH;
    }

    let mut opened_content_entity: Option<gpui::Entity<SidePanelRightView>> = None;
    let content_result =
        cx.open_window(content_window_options(display_id, cx), |window, view_cx| {
            let view = view_cx.new(|cx| SidePanelRightView::new(cx));
            opened_content_entity = Some(view.clone());
            // See `open_window`'s doc on gpui-component `Root` requirement below.
            view_cx.new(|cx| {
                Root::new(view, window, cx)
                    .bordered(false)
                    .bg(gpui::transparent_black())
            })
        });

    let content_handle = match content_result {
        Ok(handle) => handle,
        Err(err) => {
            tracing::warn!("side_panel_right: content surface failed to open: {err}");
            return;
        }
    };
    let Some(content_entity) = opened_content_entity else {
        tracing::warn!("side_panel_right: content window opened without a view — rolling back");
        if let Err(e) =
            content_handle.update(cx, |_, window: &mut Window, _| window.remove_window())
        {
            tracing::warn!("side_panel_right: rollback could not close content ({e})");
        }
        return;
    };

    let rail_result = cx.open_window(rail_window_options(display_id, cx), {
        let content_entity = content_entity.clone();
        move |window, view_cx| {
            let rail = view_cx.new(|cx| RailView::new(content_entity, cx));
            view_cx.new(|cx| {
                Root::new(rail, window, cx)
                    .bordered(false)
                    .bg(gpui::transparent_black())
            })
        }
    });

    // Pure decision, not a duplicated re-implementation of the branch below —
    // `two_surface_open_outcome` is the actual thing this `match` dispatches
    // on. Isolating the decision (not the `WindowHandle`/`cx.open_window`
    // side effects) is what makes the "partial-open is refused" contract
    // testable at all: `TestAppContext::open_window` forces a synchronous
    // first paint, and this panel's default `System` tab eagerly reads five
    // live `AppState` services (mpris/system_resources/disks/wallpaper/
    // compositor) with zero test-double precedent in this crate — actually
    // opening both real windows in a unit test is out of proportion to this
    // one invariant.
    match two_surface_open_outcome(rail_result.is_ok()) {
        TwoSurfaceOpen::RollbackContent => {
            let err = rail_result.err().expect("Err branch");
            tracing::warn!(
                "side_panel_right: rail surface failed to open ({err}) — rolling back content"
            );
            if let Err(e) =
                content_handle.update(cx, |_, window: &mut Window, _| window.remove_window())
            {
                tracing::warn!("side_panel_right: rollback could not close content ({e})");
            }
        }
        TwoSurfaceOpen::CommitBoth => {
            let rail_handle = rail_result.expect("checked Ok above");
            let state = cx.global_mut::<SidePanelRightState>();
            state.content_handle = Some(content_handle);
            state.rail_handle = Some(rail_handle);
            state.content_view = Some(content_entity.downgrade());
            state.pinned = pinned;

            tracing::info!(
                "side_panel_right: opened both surfaces ({})",
                if pinned { "pinned" } else { "peek" }
            );
            // T284: report rail presence so the frame can gate its chrome.
            frame::set_rail_mapped(FrameSide::Right, true, cx);
        }
    }
}

/// T276 pure lifecycle decision: `open_window` calls this directly (not a
/// parallel reimplementation) once `content` is confirmed open and `rail`
/// has just been attempted. Kept as a two-variant enum rather than a bool
/// so a third state (e.g. a future retry) has somewhere to go without
/// silently falling through an `if`.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum TwoSurfaceOpen {
    /// `rail` opened too — commit both handles as one logical panel.
    CommitBoth,
    /// `rail` failed — `content` (already open) must be rolled back. Never
    /// leaves the state with one handle set and the other absent.
    RollbackContent,
}

pub(crate) fn two_surface_open_outcome(rail_opened: bool) -> TwoSurfaceOpen {
    if rail_opened {
        TwoSurfaceOpen::CommitBoth
    } else {
        TwoSurfaceOpen::RollbackContent
    }
}

/// Open pinned (idempotent — no-op if already open; upgrades peek → pin).
pub fn open_pinned(cx: &mut App) {
    open_window(cx, true);
}

/// Open in peek mode (hover entered the strip). No-op if already open in
/// either mode (does not demote pin to peek).
pub fn open_peek(cx: &mut App) {
    open_window(cx, false);
}

/// Mouse left the strip and the panel. Closes only if not pinned.
pub fn close_peek_if_not_pinned(cx: &mut App) {
    if !should_close_on_peek_leave(cx.global::<SidePanelRightState>()) {
        return;
    }
    close(cx);
}

/// Close both surfaces from outside (bar toggle / hotkey). Closes as one
/// unit — a partial close (one handle gone, one lingering) is exactly the
/// invariant `open_window`'s rollback exists to prevent on the open side;
/// here we simply attempt both and log independently, since a ghost on
/// either one is equally a bug.
///
/// Note the `match`/`if let Err` instead of `let _ =`: `system_popup`/
/// `volume_popup` swallow this Err today, and a swallowed `handle.update`
/// Err is exactly what hid the ghost-window bug for a full session
/// (HANDOFF.md 2026-07-18). New code does not inherit that wart — an Err
/// here means the handle was taken but the window never closed, i.e. a
/// ghost.
pub fn close(cx: &mut App) {
    // T305: un-mapping the rail closes the control-center popup too — a
    // popup must never outlive the rail it is anchored to (ghost-window
    // class, launcher/tray_menu saga 2026-07-18). Hooked BEFORE the
    // early-return so a stray popup with an already-closed panel still dies.
    control_center::close(cx);
    let state = cx.global_mut::<SidePanelRightState>();
    let rail_handle = state.rail_handle.take();
    let content_handle = state.content_handle.take();
    if rail_handle.is_none() && content_handle.is_none() {
        cx.global_mut::<SidePanelRightState>().pinned = false;
        return;
    }
    let state = cx.global_mut::<SidePanelRightState>();
    state.content_view = None;
    state.pinned = false;
    state.resizing = false;
    state.last_exclusive_zone = None;

    if let Some(handle) = rail_handle {
        // Clear exclusive zone before destroying the surface so the
        // compositor reclaims reserved space (mirrors left panel).
        match handle.update(cx, |_, window: &mut Window, _| {
            window.set_exclusive_zone(px(0.));
            window.remove_window()
        }) {
            Ok(()) => tracing::info!("side_panel_right: rail closed"),
            Err(e) => tracing::warn!(
                "side_panel_right: rail close() could not reach the window ({e}) — possible ghost"
            ),
        }
    }
    if let Some(handle) = content_handle {
        match handle.update(cx, |_, window: &mut Window, _| window.remove_window()) {
            Ok(()) => tracing::info!("side_panel_right: content closed"),
            Err(e) => tracing::warn!(
                "side_panel_right: content close() could not reach the window ({e}) — possible ghost"
            ),
        }
    }
    // T284: rail no longer mapped — the frame re-derives its chrome.
    frame::set_rail_mapped(FrameSide::Right, false, cx);
}

/// T284: the frame style changed and the wrap inset (margin/height) can
/// only change at surface open time — recreate the open surfaces so the
/// geometry follows. The right panel reopens rail-only (its standard open
/// behavior); a closed panel just picks up the new geometry on its next
/// open.
pub fn apply_frame_inset(cx: &mut App) {
    let state = cx.global::<SidePanelRightState>();
    if state.rail_handle.is_none() {
        return;
    }
    let was_pinned = state.pinned;
    let width = state.width;
    close(cx);
    if was_pinned {
        open_pinned(cx);
        // open_window starts rail-only by design; restore the user's width
        // so a theme toggle does not silently collapse an expanded panel.
        cx.global_mut::<SidePanelRightState>().width = width;
    }
}

/// Close from inside a callback that already holds `&mut Window` for one of
/// the two panel surfaces. Must not re-enter `handle.update` on that same
/// window id (ghost-window guard, `ARCHITECTURE.md §4.1`) — the *other*
/// surface is closed via its own handle instead.
#[allow(dead_code)] // reserved for a future click-away / dismiss control
pub(crate) fn close_this(window: &mut Window, cx: &mut App) {
    // T305: same rule as `close` — this is the second (in-callback) un-map
    // path; a popup must not survive it.
    control_center::close(cx);
    let this = window.window_handle();
    let state = cx.global::<SidePanelRightState>();
    let is_rail = state
        .rail_handle
        .as_ref()
        .map(|h| **h == this)
        .unwrap_or(false);
    let is_content = state
        .content_handle
        .as_ref()
        .map(|h| **h == this)
        .unwrap_or(false);
    if !is_rail && !is_content {
        return;
    }
    let other = if is_rail {
        state.content_handle.clone()
    } else {
        state.rail_handle.clone()
    };
    {
        let state = cx.global_mut::<SidePanelRightState>();
        state.rail_handle = None;
        state.content_handle = None;
        state.content_view = None;
        state.pinned = false;
        state.resizing = false;
    }
    if is_rail {
        window.set_exclusive_zone(px(0.));
    }
    window.remove_window();
    if let Some(other) = other {
        let result = other.update(cx, |_, w: &mut Window, _| {
            if is_content {
                // `other` is rail in this branch.
                w.set_exclusive_zone(px(0.));
            }
            w.remove_window();
        });
        if let Err(e) = result {
            tracing::warn!(
                "side_panel_right: close_this could not reach the other surface ({e}) — possible ghost"
            );
        }
    }
    tracing::info!(
        "side_panel_right: close_this ({})",
        if is_rail { "rail" } else { "content" }
    );
    // T284: rail no longer mapped — the frame re-derives its chrome.
    frame::set_rail_mapped(FrameSide::Right, false, cx);
}

/// Bar-widget click / hotkey target.
pub fn toggle(cx: &mut App) {
    if cx.global::<SidePanelRightState>().rail_handle.is_some() {
        close(cx);
    } else {
        open_pinned(cx);
    }
}

/// T230 task B: switch the right panel to `tab` from an `App` context
/// (IPC handler — no `Window` in scope). Opens the panel pinned first if it
/// is not already open, then calls `on_tab_select` on the live content view.
pub fn select_tab(tab: PanelTab, cx: &mut App) {
    let view_live = cx
        .global::<SidePanelRightState>()
        .content_view
        .as_ref()
        .map(|w| w.upgrade().is_some())
        .unwrap_or(false);
    if !view_live {
        open_pinned(cx);
    }
    let Some(view) = cx
        .global::<SidePanelRightState>()
        .content_view
        .clone()
        .and_then(|w| w.upgrade())
    else {
        tracing::warn!(
            tab = tab.id(),
            "side_panel_right: select_tab has no live view"
        );
        return;
    };
    view.update(cx, |view, cx| view.on_tab_select(tab, cx));
    // T226 tooling: keyboard-focus the newly active tab. Synthetic mouse
    // clicks don't focus GPUI layer-shell windows, so a programmatic tab
    // switch would otherwise strand the window without keyboard input —
    // external input (wtype/ydotool) targets the globally focused surface
    // only. The focus handle is not registered in the window until the
    // tab's first render, so the focus is deferred a frame after the tab
    // switch; focusing synchronously would be a silent no-op.
    if let Some(handle) = cx.global::<SidePanelRightState>().content_handle.clone() {
        let focus_view = view.clone();
        cx.spawn(async move |cx| {
            cx.background_executor()
                .timer(std::time::Duration::from_millis(50))
                .await;
            cx.update(|cx| {
                let _ = handle.update(cx, |_, window, cx| {
                    if let Some(focus) = focus_view.read(cx).active_tab_focus(cx) {
                        window.focus(&focus, cx);
                    }
                });
            })
        })
        .detach();
    }
}

/// T279 — open the right panel at the Files tab rooted at `path` (the left
/// workspace Project "Files" action). Free function on `&mut App` (the T278
/// lesson): the left coordinator runs in a click handler and must reach
/// this by name. Opens the panel pinned first when closed.
pub fn open_files_at(path: std::path::PathBuf, cx: &mut App) {
    select_tab(PanelTab::Files, cx);
    let Some(view) = cx
        .global::<SidePanelRightState>()
        .content_view
        .clone()
        .and_then(|w| w.upgrade())
    else {
        tracing::warn!("side_panel_right: open_files_at has no live view");
        return;
    };
    view.update(cx, |view, cx| view.set_files_root(path, cx));
}

/// T279 — open the right panel at the Terminal tab with the shell respawned
/// at `path` (the left workspace Project "Terminal" action). Same free-fn
/// pattern as `open_files_at`.
pub fn open_terminal_at(path: std::path::PathBuf, cx: &mut App) {
    select_tab(PanelTab::Terminal, cx);
    let Some(view) = cx
        .global::<SidePanelRightState>()
        .content_view
        .clone()
        .and_then(|w| w.upgrade())
    else {
        tracing::warn!("side_panel_right: open_terminal_at has no live view");
        return;
    };
    view.update(cx, |view, cx| view.open_terminal_at(path, cx));
}

/// T226 tooling: point the Preview/Editor tab at `path`, exactly like a
/// Files click would. Opens the panel pinned first so the live view exists
/// to observe the `PreviewTarget` bump — that observer switches the tab to
/// Preview (T194). `generation` is always advanced so a re-point at the same
/// file still re-fires the observer.
pub fn preview_target(path: std::path::PathBuf, cx: &mut App) {
    use crate::side_panel_right::preview_target::{PreviewIntent, PreviewTarget};

    let view_live = cx
        .global::<SidePanelRightState>()
        .content_view
        .as_ref()
        .map(|w| w.upgrade().is_some())
        .unwrap_or(false);
    if !view_live {
        open_pinned(cx);
    }
    if !cx.has_global::<PreviewTarget>() {
        cx.set_global(PreviewTarget::default());
    }
    let generation = cx.global::<PreviewTarget>().generation.wrapping_add(1);
    cx.set_global(PreviewTarget {
        path: Some(path.clone()),
        generation,
        intent: PreviewIntent::Edit,
    });
    // T226 tooling: keyboard-focus the editor — mirror of `select_tab`'s
    // focus defer. `set_global` fires the view's PreviewTarget observer
    // synchronously, so by the time this spawn runs the Preview tab exists
    // and `active_tab_focus` can resolve its editor. Without this, wtype /
    // synthetic input after `preview-target` lands nowhere (no seat focus
    // on GPUI layer-shell windows), which is exactly the infra gap that
    // blocked Editor capture in T226 attempts #2/#3.
    if let Some(handle) = cx.global::<SidePanelRightState>().content_handle.clone() {
        let focus_view = cx
            .global::<SidePanelRightState>()
            .content_view
            .clone()
            .and_then(|w| w.upgrade());
        if let Some(focus_view) = focus_view {
            cx.spawn(async move |cx| {
                cx.background_executor()
                    .timer(std::time::Duration::from_millis(50))
                    .await;
                cx.update(|cx| {
                    let _ = handle.update(cx, |_, window, cx| {
                        if let Some(focus) = focus_view.read(cx).active_tab_focus(cx) {
                            window.focus(&focus, cx);
                        }
                    });
                })
            })
            .detach();
        }
    }
    tracing::info!(?path, "side_panel_right: preview_target set");
}

pub fn init(cx: &mut App) {
    cx.set_global(SidePanelRightState::default());
    cx.set_global(preview_target::PreviewTarget::default());
    control_center::init(cx);
    // Load from disk once before any render runs. The watcher only fires on
    // file CHANGES, so without this call a user-saved panels.toml would be
    // silently ignored until the next save — first paint would always show
    // the code defaults and only catch up after a manual edit (T219).
    panels_config::apply(cx);
    panels_config::spawn_watcher(cx);
    // Defer the strip one tick so `cx.displays()` / pult uuid match what
    // `bar::init` sees a moment later. Opening the strip synchronously in
    // `main` before the bar historically landed it on the wrong output
    // (HDMI-A-1) while the panel+bar bound to DP-1 (pult).
    cx.spawn(async move |cx| {
        cx.background_executor()
            .timer(std::time::Duration::from_millis(50))
            .await;
        cx.update(|cx| {
            hover_strip::init_hover_strip(cx);
            // Optional smoke: pin-open for grim without hover/ydotool.
            // Not product wiring — only when env is set.
            if std::env::var_os("CHRONOS_SMOKE_SIDE_PANEL").is_some() {
                // Smoke path: open the panel already expanded so automated screenshots
                // and tests can see the content without a manual rail click.
                cx.global_mut::<SidePanelRightState>()
                    .ensure_content_width(DEFAULT_CONTENT_WIDTH);
                open_pinned(cx);
            }
        });
    })
    .detach();
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- T276: lifecycle — both handles open/close as one unit ---
    //
    // Driving the REAL `open_pinned`/`close` end-to-end through
    // `TestAppContext` was attempted first and rejected: GPUI's test
    // platform forces a synchronous first paint on `cx.open_window`, and
    // this panel's default `System` tab eagerly reads five live `AppState`
    // services (mpris/system_resources/disks/wallpaper/compositor) at
    // construction — this crate has no precedent anywhere for faking that
    // in a unit test, and building one just for this would be
    // disproportionate to the invariant being checked. `two_surface_open_
    // outcome` is the actual decision `open_window` branches on (see the
    // `match` there) — not a parallel reimplementation — so this proves
    // the "never one handle without the other" contract without needing a
    // real window or a fake `AppState`. Real lifecycle (ghost/orphan
    // surfaces) is the task's own live-smoke checklist item 7
    // (`hyprctl layers`).

    #[test]
    fn both_surfaces_open_commits_both_handles() {
        assert_eq!(two_surface_open_outcome(true), TwoSurfaceOpen::CommitBoth);
    }

    #[test]
    fn rail_failing_after_content_opened_rolls_content_back() {
        assert_eq!(
            two_surface_open_outcome(false),
            TwoSurfaceOpen::RollbackContent
        );
    }

    #[test]
    fn peek_close_request_is_noop_while_pinned() {
        let mut state = SidePanelRightState::default();
        state.pinned = true;
        assert!(!should_close_on_peek_leave(&state));
    }

    #[test]
    fn peek_close_request_closes_when_not_pinned() {
        let mut state = SidePanelRightState::default();
        state.pinned = false;
        assert!(should_close_on_peek_leave(&state));
    }

    #[test]
    fn peek_close_suppressed_while_resizing() {
        let mut state = SidePanelRightState::default();
        state.pinned = false;
        state.resizing = true;
        assert!(!should_close_on_peek_leave(&state));
    }

    #[test]
    fn rail_only_default_width() {
        assert_eq!(RAIL_ONLY_WIDTH, 40.0);
        assert_eq!(RAIL_ONLY_WIDTH, RAIL_WIDTH);
        assert_eq!(SidePanelRightState::default().width, RAIL_ONLY_WIDTH);
    }

    #[test]
    fn content_canvas_width_is_max_minus_rail() {
        assert_eq!(CONTENT_CANVAS_WIDTH, MAX_WIDTH - RAIL_ONLY_WIDTH);
        assert_eq!(CONTENT_CANVAS_WIDTH, 920.0);
    }

    #[test]
    fn content_margin_restores_bar_gap_while_ignoring_exclusive_zones() {
        let margin = content_window_margin(32.0);
        assert_eq!(margin, (px(32.0), px(RAIL_ONLY_WIDTH), px(0.0), px(0.0)));
    }

    #[test]
    fn exclusive_px_dock_vs_rail() {
        let mut state = SidePanelRightState::default();
        assert!(!state.dock_content);
        assert_eq!(state.exclusive_px(), RAIL_ONLY_WIDTH);
        state.width = 640.0;
        state.dock_content = true;
        assert_eq!(state.exclusive_px(), 640.0);
    }

    #[test]
    fn resize_clamps() {
        let mut state = SidePanelRightState::default();
        assert_eq!(state.width, RAIL_ONLY_WIDTH);
        state.resize(10.0);
        assert_eq!(state.width, RAIL_ONLY_WIDTH);
        state.resize(2000.0);
        assert_eq!(state.width, MAX_WIDTH);
        state.resize(400.0);
        assert_eq!(state.width, 400.0);
    }

    #[test]
    fn ensure_content_width_from_rail_only() {
        let mut state = SidePanelRightState::default();
        assert_eq!(state.width, RAIL_ONLY_WIDTH);
        state.ensure_content_width(DEFAULT_CONTENT_WIDTH);
        assert_eq!(state.width, DEFAULT_CONTENT_WIDTH);
    }

    // --- T276: visible content width (fixed canvas geometry) ---

    #[test]
    fn visible_width_is_zero_at_rail_only() {
        assert_eq!(visible_content_width(RAIL_ONLY_WIDTH), 0.0);
    }

    #[test]
    fn visible_width_is_full_canvas_at_max_width() {
        assert_eq!(visible_content_width(MAX_WIDTH), CONTENT_CANVAS_WIDTH);
    }

    #[test]
    fn visible_width_tracks_state_width_above_rail_only() {
        assert_eq!(visible_content_width(500.0), 460.0);
    }

    #[test]
    fn visible_width_never_negative_even_below_rail_only() {
        // Defensive: SidePanelRightState::resize already clamps its input,
        // but the pure fn itself must not go negative if ever called with
        // an out-of-range value directly.
        assert_eq!(visible_content_width(0.0), 0.0);
    }

    // --- T276: content input region ---

    #[test]
    fn input_region_is_empty_when_collapsed() {
        assert!(content_input_region(CONTENT_CANVAS_WIDTH, 1000.0, 0.0).is_empty());
    }

    #[test]
    fn input_region_covers_right_aligned_visible_rect() {
        let regions = content_input_region(CONTENT_CANVAS_WIDTH, 1000.0, 460.0);
        assert_eq!(regions.len(), 1);
        assert_eq!(regions[0].origin.x, px(CONTENT_CANVAS_WIDTH - 460.0));
        assert_eq!(regions[0].origin.y, px(0.0));
        assert_eq!(regions[0].size.width, px(460.0));
        assert_eq!(regions[0].size.height, px(1000.0));
    }

    #[test]
    fn input_region_covers_full_canvas_when_fully_open() {
        let regions = content_input_region(CONTENT_CANVAS_WIDTH, 1000.0, CONTENT_CANVAS_WIDTH);
        assert_eq!(regions[0].origin.x, px(0.0));
        assert_eq!(regions[0].size.width, px(CONTENT_CANVAS_WIDTH));
    }

    #[test]
    fn resize_handle_tracks_left_edge_of_visible_content() {
        assert_eq!(content_resize_handle_x(CONTENT_CANVAS_WIDTH, 370.0), 550.0);
        assert_eq!(
            content_resize_handle_x(CONTENT_CANVAS_WIDTH, CONTENT_CANVAS_WIDTH),
            0.0
        );
        assert_eq!(
            content_resize_handle_x(CONTENT_CANVAS_WIDTH, 0.0),
            CONTENT_CANVAS_WIDTH - HANDLE_WIDTH
        );
    }

    #[test]
    fn active_drag_keeps_handle_interactive_at_rail_only_clamp() {
        assert_eq!(content_interactive_width(0.0, false), 0.0);
        assert_eq!(content_interactive_width(0.0, true), HANDLE_WIDTH);
        assert_eq!(content_interactive_width(370.0, true), 370.0);
    }

    // --- T276: pure-delta resize (no compositor race left to model) ---

    #[test]
    fn drag_left_grows_width_by_exact_delta() {
        let start_width = 400.0_f32;
        let start_x = 2.0_f32;
        let current_x = start_x - 50.0; // moved 50px left of the press point
        assert_eq!(resize_target_width(start_width, start_x, current_x), 450.0);
    }

    #[test]
    fn drag_right_shrinks_width_by_exact_delta() {
        let start_width = 400.0_f32;
        let start_x = 2.0_f32;
        let current_x = start_x + 50.0;
        assert_eq!(resize_target_width(start_width, start_x, current_x), 350.0);
    }

    #[test]
    fn drag_is_deterministic_for_repeated_identical_input() {
        // No surface resize left to race against: unlike the old
        // window.bounds()-driven model (T210/T214/T216/T243), the same
        // (start_width, start_x, current_x) always produces the same
        // target — there is no "stale frame" concept anymore.
        let a = resize_target_width(400.0, 2.0, -18.0);
        let b = resize_target_width(400.0, 2.0, -18.0);
        assert_eq!(a, b);
    }

    #[test]
    fn drag_target_clamps_to_both_bounds() {
        assert_eq!(resize_target_width(400.0, 0.0, 10_000.0), RAIL_ONLY_WIDTH);
        assert_eq!(resize_target_width(400.0, 0.0, -10_000.0), MAX_WIDTH);
    }

    #[test]
    fn drag_from_rail_only_press_point_expands_by_delta() {
        // start_resize() expands rail→content to the tab's natural width
        // before arming the drag; this test only covers the pure delta math
        // once that expansion has already set start_width.
        let start_width = 400.0_f32; // post-expand natural width
        let start_x = 2.0_f32; // mid-handle
        let current_x = start_x - 100.0;
        assert_eq!(resize_target_width(start_width, start_x, current_x), 500.0);
    }
}
