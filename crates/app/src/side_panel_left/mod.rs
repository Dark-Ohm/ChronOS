mod chat_view;
mod composer;
mod hover_strip;
pub mod sessions_list;
mod state;
pub mod tabs;
mod tool_card;

/// Detects RTL base direction by the first strong (directional) character.
pub fn is_rtl_text(text: &str) -> bool {
    for ch in text.chars() {
        match ch {
            '\u{05D0}'..='\u{05EA}' => return true,
            '\u{0600}'..='\u{06FF}' | '\u{0750}'..='\u{077F}' | '\u{08A0}'..='\u{08FF}' => return true,
            'A'..='Z' | 'a'..='z' => return false,
            _ => {}
        }
    }
    false
}

pub use state::{PanelState, SidePanelLeftState};
pub(crate) use tabs::chat::ChatTab;

use crate::frame::{self, FrameSide};
use chronos_services::hermes_acp::{
    AgentDescriptor, HermesClient, StreamingEvent, known_agents, load_shared_env,
};
use chronos_services::threads::{ThreadRecord, ThreadStore};
use chronos_services::{ModelInfo, SessionMode};
use gpui::{
    App, Bounds, DisplayId, Entity, Focusable, Global, Size, UTF16Selection, Window,
    WindowBackgroundAppearance, WindowBounds, WindowHandle, WindowKind, WindowOptions,
    layer_shell::*, point, prelude::*, px,
};
use gpui_component::Root;
use std::collections::HashMap;
use std::ops::Range;
use std::path::PathBuf;

pub struct LeftPanelResize;

// Forward decls for the two new view entities (Task 2). Defined in
// `rail_view.rs` and `workspace_view.rs`; declared here so the SoT
// `SidePanelLeftState_` can hold weak handles to them without a circular
// `mod` declaration in the child files.
mod rail_view;
mod workspace_view;
use rail_view::RailView;
use workspace_view::WorkspaceView;

/// Top air under the bar — live bar height (T200). Same contract as the
/// right panel's `panel_edge_gap()`; open-time geometry only.
fn panel_edge_gap() -> f32 {
    crate::state::bar_height_px()
}

/// T278 / Slice A1 — the lifecycle / UI source of truth.
///
/// Mirrors `side_panel_right::SidePanelRightState`'s shape (T276): rail
/// and content each have their own `WindowHandle<Root>`, and a weak
/// content-view handle lets `RailView` (a different window) reach the
/// content view for tab switches, dock toggles, and resize bookkeeping.
///
/// `ChatTab` (the legacy god-object) no longer owns a `WindowHandle`,
/// width, dock flag, exclusive zone, or resize state. It is the product-
/// state child of `WorkspaceView`; all window-level mutation lives here.
pub struct SidePanelLeftState_ {
    /// T278: the permanent 40px icon-rail surface. Owns the exclusive zone.
    pub(crate) rail_handle: Option<WindowHandle<Root>>,
    /// T278: the fixed-canvas content surface. Never resized after open —
    /// only the visible slice and input region change.
    pub(crate) content_handle: Option<WindowHandle<Root>>,
    /// Weak handle to the live `WorkspaceView` (lives in the `content`
    /// window). Needed by `RailView` (a different window) and by IPC
    /// handlers running in `App` context with no `Window` in scope.
    pub(crate) content_view: Option<gpui::WeakEntity<WorkspaceView>>,
    /// Weak handle to the live `ChatTab` product entity (owned by
    /// `WorkspaceView`). T279 round 2: the coordinator reducers
    /// (`select_session` / `create_thread` / `switch_project` /
    /// `remove_project_scope`) reach the chat column through THIS handle,
    /// not through `content_view` — `content_view` is already leased while
    /// `WorkspaceView::on_*_event` runs, and a second lease of the same
    /// entity is a `double_lease_panic` (`entity_map::lease`). `ChatTab`
    /// is a separate entity, so leasing it is safe. `None` in unit tests
    /// and whenever the surfaces are closed; reducers no-op then.
    pub(crate) chat: Option<gpui::WeakEntity<ChatTab>>,
    /// Currently selected left tab (Slice A catalog). Default = `Chat`
    /// (matches T220 behaviour where Super+A expands the chat column).
    pub active_tab: tabs::LeftTab,
    /// Current *logical* panel width (px), `RAIL_WIDTH..=MAX_PANEL_WIDTH`.
    /// T278: no surface is ever resized to this value directly — `rail`
    /// stays at `RAIL_WIDTH`, `content` stays at `CONTENT_CANVAS_WIDTH`;
    /// this number only drives the visible rectangle inside the content
    /// canvas and the rail's exclusive zone.
    pub panel_width: f32,
    /// Per-resizable-tab runtime width memory (Chat, Plan, Context Files).
    /// Reset on process restart; never persisted.
    pub remembered_widths: tabs::ResizableWidths,
    /// Transient active project canonical path (mirrors SQLite
    /// `workspace_project_state.active_thread_id` for the current session;
    /// SQLite remains the persistent source).
    pub active_project_path: Option<PathBuf>,
    /// Transient active session id mirror.
    pub active_session_id: Option<String>,
    /// Dock mode: when true, content is always visible (rail reserves
    /// `panel_width` instead of just `RAIL_WIDTH`). When false (default),
    /// only the rail shows until content is opened.
    pub dock_content: bool,
    /// True while a resize drag is active. Suppresses peek-close.
    pub resizing: bool,
    /// `true` when opened by hotkey/bar-click (`toggle`/`open_pinned`) —
    /// stays open until re-toggled. `false` when opened by hover (peek) —
    /// closes on mouse-leave debounce unless a pin request arrives.
    pub pinned: bool,
    /// Bumped on hover-enter (strip or panel). Leave schedules a close
    /// only if this value is still unchanged after the debounce window.
    pub peek_generation: u64,
    /// Last exclusive_zone value sent to the compositor (avoids redundant
    /// Wayland round-trips). Set on the rail surface only.
    pub last_exclusive_zone: Option<f32>,
}

impl Default for SidePanelLeftState_ {
    fn default() -> Self {
        Self {
            rail_handle: None,
            content_handle: None,
            content_view: None,
            chat: None,
            active_tab: tabs::LeftTab::Chat,
            panel_width: tabs::RAIL_WIDTH,
            remembered_widths: tabs::ResizableWidths::default(),
            active_project_path: None,
            active_session_id: None,
            dock_content: false,
            resizing: false,
            pinned: false,
            peek_generation: 0,
            last_exclusive_zone: None,
        }
    }
}

impl Global for SidePanelLeftState_ {}

impl SidePanelLeftState_ {
    /// Exclusive zone px: full panel when docked, rail-only when overlay.
    /// T278: this value is set on the **rail** surface only — the content
    /// canvas never reserves space itself (`exclusive_zone: Some(px(-1.))`
    /// opts it out of foreign reservations, including the top bar).
    pub fn exclusive_px(&self) -> f32 {
        if self.dock_content {
            self.panel_width
        } else {
            tabs::RAIL_WIDTH
        }
    }

    /// Clamp a candidate panel width into the hard drag range.
    pub fn resize(&mut self, new_width: f32) {
        self.panel_width = state::geometry::clamp_panel(new_width);
    }

    /// Expand or contract to the given target width.
    /// Called when content becomes visible (tab open / dock toggle) or
    /// when switching tabs with content already visible. Does NOT update
    /// `last_exclusive_zone` — the rail's render path recomputes it on
    /// the next paint and clears the cache itself when its own state
    /// changes (`ensure_content_width` mirrors the T276 pattern).
    pub fn ensure_content_width(&mut self, target: f32) {
        self.panel_width = state::geometry::clamp_panel(target);
        self.last_exclusive_zone = None;
    }
}

/// T278 pure lifecycle decision: `open_window` calls this directly once
/// `content` is confirmed open and `rail` has just been attempted. Kept
/// as a two-variant enum (not a bool) so a third state (e.g. a future
/// retry path) has somewhere to go without silently falling through an
/// `if`. Mirrors `side_panel_right::TwoSurfaceOpen`.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum TwoSurfaceOpen {
    /// `rail` opened too — commit both handles as one logical panel.
    CommitBoth,
    /// `rail` failed — `content` (already open) must be rolled back. Never
    /// leaves the state with one handle set and the other absent.
    RollbackContent,
}

/// Pure decision, no GPUI/Window side effects. See `side_panel_right` for
/// the same shape (T276).
pub(crate) fn two_surface_open_outcome(rail_opened: bool) -> TwoSurfaceOpen {
    if rail_opened {
        TwoSurfaceOpen::CommitBoth
    } else {
        TwoSurfaceOpen::RollbackContent
    }
}

fn display_height(display_id: Option<DisplayId>, cx: &App) -> f32 {
    // `display_id` is always the result of `pult_display_id_or_primary` —
    // the full fallback chain lives in `monitor.rs`. We just trust it.
    display_id
        .and_then(|id| cx.find_display(id))
        .map(|d| f32::from(d.bounds().size.height))
        .unwrap_or(1080.)
}

fn panel_height(display_id: Option<DisplayId>, cx: &App) -> f32 {
    // Wrap (T284 + T311 D3): the panel clears the bottom chrome too —
    // height is trimmed by the bottom-plate on top of the bar gap. Use
    // `wrap_inset_bottom`, not `wrap_inset` — the bottom edge keeps its
    // plate even when both rails are mapped, and is unaffected by rail
    // mapping.
    (display_height(display_id, cx) - panel_edge_gap() - frame::wrap_inset_bottom_cached()).max(100.)
}

/// T278: the `rail` surface — fixed `RAIL_WIDTH` px, owns the exclusive
/// zone. Never resized after open; `exclusive_zone` is a value updated
/// live via `Window::set_exclusive_zone`, independent from the surface's
/// own pixel footprint (legal per wlr-layer-shell — see `gpui-layer-shell`
/// skill Part D). `KeyboardInteractivity::None` because the rail has no
/// text inputs.
pub(crate) fn rail_window_options(display_id: Option<DisplayId>, cx: &App) -> WindowOptions {
    let panel_h = panel_height(display_id, cx);
    let zone = cx
        .try_global::<SidePanelLeftState_>()
        .map(|s| s.exclusive_px())
        .unwrap_or(tabs::RAIL_WIDTH);
    WindowOptions {
        display_id,
        titlebar: None,
        window_bounds: Some(WindowBounds::Windowed(Bounds {
            origin: point(px(0.), px(0.)),
            size: Size::new(px(tabs::RAIL_WIDTH), px(panel_h)),
        })),
        app_id: Some("chronos-side-panel-left-rail".to_string()),
        window_background: WindowBackgroundAppearance::Transparent,
        kind: WindowKind::LayerShell(LayerShellOptions {
            namespace: "side_panel_left_rail".to_string(),
            layer: Layer::Overlay,
            anchor: Anchor::TOP | Anchor::LEFT,
            exclusive_zone: Some(px(zone)),
            exclusive_edge: Some(Anchor::LEFT),
            // T310 D1: NO margin. `frame_wrap_excl_left` already holds an
            // `exclusive_zone` of `wrap_inset()` on this same edge, so the
            // compositor offsets the rail by the frame thickness on its own.
            // The T284 `margin.left = wrap_inset()` added that offset a
            // second time and parked the rail at 2 × thickness, leaving a
            // thickness-wide strip of bare wallpaper between the frame and
            // the rail (measured live 2026-08-19: wrap 0-15, wallpaper
            // 16-31, rail 32-70). Same stacking-reservation class as
            // T307/T308. No top margin either: the bar's exclusive zone
            // already drops top-anchored Overlay surfaces below it (a
            // second top offset would double the gap, gpui-layer-shell Part A).
            margin: None,
            keyboard_interactivity: KeyboardInteractivity::None,
            ..Default::default()
        }),
        ..Default::default()
    }
}

/// CSS-order: (top, right, bottom, left). `-1` (below) also disables the
/// bar's automatic top offset, so both offsets must be explicit.
fn content_window_margin(top_gap: f32) -> (gpui::Pixels, gpui::Pixels, gpui::Pixels, gpui::Pixels) {
    // Wrap (T284 + T311 D3): content rides with the rail — its LEFT margin
    // gains the wrap-reserved space on top of the rail width. After D3 the
    // wrap inset on the left edge collapses to ZERO when the left rail is
    // mapped (the rail already owns that edge), and stays at full
    // `wrap.left` when the rail is gone. Use `wrap_inset_left`, not
    // `wrap_inset` — the old constant reserved space twice when both the
    // rail and its own ExclLeft strip were open.
    //
    // T314: the flag passed below is the coexistence invariant, NOT the
    // live `rail_mapped()` read — content only ever opens in the same
    // two-surface commit as its rail, but `set_rail_mapped(true)` lands
    // AFTER both windows are open, so a live read here sees the pre-commit
    // `false` and bakes a stale `wrap.left` into the margin (content
    // at x=56 instead of x=40, measured live).
    let left_reserved = frame::wrap_inset_left_cached(true);
    (px(top_gap), px(0.), px(0.), px(tabs::RAIL_WIDTH + left_reserved))
}

/// T278: the `content` surface — fixed `CONTENT_CANVAS_WIDTH` px canvas,
/// positioned immediately right of `rail` via a constant `margin-left =
/// RAIL_WIDTH`. **Never resized** for the surface's lifetime; only the
/// visible rectangle inside it (left-aligned) and its input region change.
pub(crate) fn content_window_options(display_id: Option<DisplayId>, cx: &App) -> WindowOptions {
    let panel_h = panel_height(display_id, cx);
    WindowOptions {
        display_id,
        titlebar: None,
        window_bounds: Some(WindowBounds::Windowed(Bounds {
            origin: point(px(0.), px(0.)),
            size: Size::new(px(tabs::CONTENT_CANVAS_WIDTH), px(panel_h)),
        })),
        app_id: Some("chronos-side-panel-left-content".to_string()),
        window_background: WindowBackgroundAppearance::Transparent,
        kind: WindowKind::LayerShell(LayerShellOptions {
            namespace: "side_panel_left_content".to_string(),
            layer: Layer::Overlay,
            anchor: Anchor::TOP | Anchor::LEFT,
            // Content never reserves space — that is rail's job (spec §
            // "Contract геометрии"). `-1` is the wlr-layer-shell escape
            // hatch: opts this surface OUT of being pushed by *other*
            // surfaces' exclusive zones on the same edge. `None` would map
            // to the protocol default of `0`, which does NOT opt out and
            // the compositor would still auto-offset. See T276 / right
            // panel's `content_window_options` for the full rationale.
            exclusive_zone: Some(px(-1.)),
            margin: Some(content_window_margin(panel_edge_gap())),
            // OnDemand: Chat's composer + Sessions' rename/search live here.
            keyboard_interactivity: KeyboardInteractivity::OnDemand,
            ..Default::default()
        }),
        ..Default::default()
    }
}

fn open_window(cx: &mut App, pinned: bool) {
    if cx.global::<SidePanelLeftState_>().rail_handle.is_some() {
        if pinned {
            cx.global_mut::<SidePanelLeftState_>().pinned = true;
            tracing::info!("side_panel_left: upgraded peek → pinned");
        }
        return;
    }
    let display_id = crate::monitor::pult_display_id_or_primary(cx);

    // T278: open content first, then rail — exactly the T276 order.
    // Content failure is an early return; rail failure rolls content
    // back. `opened_workspace` is captured outside the closure so the
    // rail creation can reach it through a weak handle (mirrors the
    // T276 `opened_content_entity` pattern).
    let mut opened_workspace: Option<Entity<WorkspaceView>> = None;
    let mut opened_panel: Option<Entity<ChatTab>> = None;

    let content_result = cx.open_window(content_window_options(display_id, cx), |window, view_cx| {
        let panel = view_cx.new(|cx| ChatTab::new(window, cx));
        let workspace = view_cx.new(|cx| WorkspaceView::new(panel.clone(), cx));
        opened_panel = Some(panel);
        opened_workspace = Some(workspace.clone());
        view_cx.new(|cx| {
            Root::new(workspace, window, cx)
                .bordered(false)
                .bg(gpui::transparent_black())
        })
    });

    let content_handle = match content_result {
        Ok(handle) => handle,
        Err(err) => {
            tracing::warn!("side_panel_left: content surface failed to open: {err}");
            return;
        }
    };
    let Some(workspace_entity) = opened_workspace else {
        tracing::warn!("side_panel_left: content window opened without a workspace — rolling back");
        if let Err(e) = content_handle.update(cx, |_, window: &mut Window, _| window.remove_window())
        {
            tracing::warn!("side_panel_left: rollback could not close content ({e})");
        }
        return;
    };
    // T279 round 2: keep a weak handle to the chat product entity so the
    // coordinator reducers can reach it from `App` context. The entity
    // itself is kept alive by `workspace_entity` (`WorkspaceView.chat`).
    let chat_weak = opened_panel.as_ref().map(|p| p.downgrade());

    let rail_result = cx.open_window(rail_window_options(display_id, cx), |window, view_cx| {
        let rail = view_cx.new(|cx| RailView::new(workspace_entity.downgrade(), cx));
        view_cx.new(|cx| {
            Root::new(rail, window, cx)
                .bordered(false)
                .bg(gpui::transparent_black())
        })
    });

    match two_surface_open_outcome(rail_result.is_ok()) {
        TwoSurfaceOpen::RollbackContent => {
            let err = rail_result.err().expect("Err branch");
            tracing::warn!(
                "side_panel_left: rail surface failed to open ({err}) — rolling back content"
            );
            if let Err(e) =
                content_handle.update(cx, |_, window: &mut Window, _| window.remove_window())
            {
                tracing::warn!("side_panel_left: rollback could not close content ({e})");
            }
        }
        TwoSurfaceOpen::CommitBoth => {
            let rail_handle = rail_result.expect("checked Ok above");
            let state = cx.global_mut::<SidePanelLeftState_>();
            state.content_handle = Some(content_handle);
            state.rail_handle = Some(rail_handle);
            state.content_view = Some(workspace_entity.downgrade());
            state.chat = chat_weak;
            state.pinned = pinned;

            tracing::info!(
                "side_panel_left: opened both surfaces ({})",
                if pinned { "pinned" } else { "peek" }
            );
            // T284: report rail presence so the frame can gate its chrome.
            frame::set_rail_mapped(FrameSide::Left, true, cx);
        }
    }
}

pub fn open_pinned(cx: &mut App) {
    open_window(cx, true);
}

pub fn open_peek(cx: &mut App) {
    open_window(cx, false);
}

pub fn close(cx: &mut App) {
    let state = cx.global_mut::<SidePanelLeftState_>();
    let rail_handle = state.rail_handle.take();
    let content_handle = state.content_handle.take();
    // T278 architect round 2: the next `open_pinned`/`Super+A` must
    // come up rail-only (panel_width = 40, dock off). Without this
    // reset, a close→toggle cycle would restore the previous
    // expanded state — silently violating the rail-only summon
    // contract from T220. The reset runs BEFORE the early-return so
    // an idempotent close() (no surfaces open, e.g. from a stray IPC
    // double-fire) still snaps stale state to rail-only.
    state.content_view = None;
    state.chat = None;
    state.pinned = false;
    state.resizing = false;
    state.last_exclusive_zone = None;
    state.panel_width = tabs::RAIL_WIDTH;
    state.dock_content = false;
    // remembered_widths stay — they survive close so a later dock or
    // tab switch returns to the user's last drag width.
    if rail_handle.is_none() && content_handle.is_none() {
        return;
    }

    if let Some(handle) = rail_handle {
        // Clear exclusive zone before destroying the surface so the
        // compositor reclaims reserved space (T276 pattern).
        match handle.update(cx, |_, window: &mut Window, _| {
            window.set_exclusive_zone(px(0.));
            window.remove_window()
        }) {
            Ok(()) => tracing::info!("side_panel_left: rail closed"),
            Err(e) => tracing::warn!(
                "side_panel_left: rail close() could not reach the window ({e}) — possible ghost"
            ),
        }
    }
    if let Some(handle) = content_handle {
        match handle.update(cx, |_, window: &mut Window, _| window.remove_window()) {
            Ok(()) => tracing::info!("side_panel_left: content closed"),
            Err(e) => tracing::warn!(
                "side_panel_left: content close() could not reach the window ({e}) — possible ghost"
            ),
        }
    }
    // T284: rail no longer mapped — the frame re-derives its chrome.
    frame::set_rail_mapped(FrameSide::Left, false, cx);
}

/// T284: the frame style changed and the wrap inset (margin/height) can
/// only change at surface open time — recreate the open surfaces so the
/// geometry follows. Preserves the dock width; a closed panel just picks
/// up the new geometry on its next open. A chat that was open reconnects
/// exactly like a manual close/reopen (T285 cold-start gap).
pub fn apply_frame_inset(cx: &mut App) {
    let state = cx.global::<SidePanelLeftState_>();
    if state.rail_handle.is_none() {
        return;
    }
    let was_pinned = state.pinned;
    let width = state.panel_width;
    let docked = state.dock_content;
    close(cx);
    let s = cx.global_mut::<SidePanelLeftState_>();
    s.panel_width = width;
    s.dock_content = docked;
    if was_pinned {
        open_pinned(cx);
    }
}

/// Close both surfaces from inside a callback that already holds `&mut Window`
/// for one of the two panel surfaces. Must not re-enter `handle.update` on that
/// same window id (ghost-window guard, `ARCHITECTURE.md §4.1`) — the *other*
/// surface is closed via its own handle instead. Mirrors `side_panel_right`'s
/// `close_this` exactly.
pub(crate) fn close_this(window: &mut Window, cx: &mut App) {
    let this = window.window_handle();
    let state = cx.global::<SidePanelLeftState_>();
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
        let state = cx.global_mut::<SidePanelLeftState_>();
        state.rail_handle = None;
        state.content_handle = None;
        state.content_view = None;
        state.chat = None;
        state.pinned = false;
        state.resizing = false;
        // T278 architect round 2: close_this is the click-X path inside
        // panel.rs (`side-panel-left-close` button). Must mirror
        // `close()`'s rail-only reset so a click-X → re-open cycle
        // also returns to rail-only, not the saved expanded state.
        state.panel_width = tabs::RAIL_WIDTH;
        state.dock_content = false;
    }
    if is_rail {
        window.set_exclusive_zone(px(0.));
    }
    window.remove_window();
    if let Some(other) = other {
        let result = other.update(cx, |_, w: &mut Window, _| {
            if is_content {
                // `other` is rail in this branch — clear its zone too.
                w.set_exclusive_zone(px(0.));
            }
            w.remove_window();
        });
        if let Err(e) = result {
            tracing::warn!(
                "side_panel_left: close_this could not reach the other surface ({e}) — possible ghost"
            );
        }
    }
    tracing::info!(
        "side_panel_left: close_this ({})",
        if is_rail { "rail" } else { "content" }
    );
    // T284: rail no longer mapped — the frame re-derives its chrome.
    frame::set_rail_mapped(FrameSide::Left, false, cx);
}

/// Pure decision: should a peek-leave request close the panel?
/// T278: also blocks while a resize drag is active (mirrors right
/// panel T276 — a stale hover-leave must not close the surface the
/// cursor is currently dragging).
fn should_close_on_peek_leave(state: &SidePanelLeftState_) -> bool {
    !state.pinned && !state.resizing
}

/// Cursor entered strip or panel — cancel any pending peek-close.
pub(crate) fn hold_peek(cx: &mut App) {
    let state = cx.global_mut::<SidePanelLeftState_>();
    state.peek_generation = state.peek_generation.wrapping_add(1);
}

/// Cursor left strip or panel — close after debounce if still unpinned
/// and no later enter bumped the generation.
pub(crate) fn schedule_release_peek(cx: &mut App) {
    let generation = cx.global::<SidePanelLeftState_>().peek_generation;
    schedule_release_from_app(cx, generation);
}

/// Mouse left the strip and the panel. Closes only if not pinned and
/// not currently resizing (T276 peek guard).
pub fn close_peek_if_not_pinned(cx: &mut App) {
    if !should_close_on_peek_leave(cx.global::<SidePanelLeftState_>()) {
        return;
    }
    close(cx);
}

const PEEK_LEAVE_DEBOUNCE: std::time::Duration = std::time::Duration::from_millis(280);

pub(crate) fn schedule_release_from_app(cx: &mut gpui::App, generation: u64) {
    cx.spawn(async move |app_cx: &mut gpui::AsyncApp| {
        app_cx
            .background_executor()
            .timer(PEEK_LEAVE_DEBOUNCE)
            .await;
        app_cx.update(|app_cx| {
            if app_cx.global::<SidePanelLeftState_>().peek_generation != generation {
                return;
            }
            close_peek_if_not_pinned(app_cx);
        });
    })
    .detach();
}

/// Build the `WorkspaceSnapshot` the T281 reducer needs from the live SoT.
/// `open` is `rail_handle.is_some()` — the reducer never inspects a
/// `WindowHandle` itself.
fn workspace_snapshot(cx: &App) -> tabs::WorkspaceSnapshot {
    let state = cx.global::<SidePanelLeftState_>();
    tabs::WorkspaceSnapshot {
        open: state.rail_handle.is_some(),
        active_tab: state.active_tab,
        panel_width: state.panel_width,
        dock_content: state.dock_content,
        remembered_widths: state.remembered_widths,
    }
}

/// Toggle the pinned panel open/closed. Called from the IPC handler (no
/// `Window` in scope there — matches `launcher::toggle(cx)`'s shape).
///
/// T281 / Task 7: routed through `tabs::workspace_transition` so every
/// entry point (`toggle`, `select_tab`, `apply_dock_toggle`,
/// `expand_with_composer`, `compose_and_send`) shares the single reducer
/// boundary the plan asks for — not just IPC's Chat-forcing pair.
pub fn toggle(cx: &mut App) {
    let transition = tabs::workspace_transition(workspace_snapshot(cx), tabs::WorkspaceAction::Toggle);
    if transition.open_rail {
        open_pinned(cx);
    } else {
        close(cx);
    }
}

/// T278 architect round 4: dock-toggle reducer exposed as a free
/// function so it can be unit-tested without instantiating
/// `WorkspaceView` (which needs `ChatTab`, which spawns an async
/// ACP-connect that requires a live Tokio runtime — unconstructable
/// in `TestAppContext`). `WorkspaceView::on_dock_toggle` is a thin
/// wrapper around this; rail clicks and tests share the same source
/// of truth (`tabs::dock_transition`).
///
/// Mutates `SidePanelLeftState_` in place: applies the pure transition,
/// invalidates `last_exclusive_zone` when the computed `exclusive_px`
/// changes (so the rail re-pushes on the next paint).
///
/// T281 / Task 7: routed through `tabs::workspace_transition` (`ToggleDock`
/// arm), which itself composes `tabs::dock_transition` — same numbers,
/// single reducer boundary.
pub fn apply_dock_toggle(cx: &mut App) {
    let transition = tabs::workspace_transition(workspace_snapshot(cx), tabs::WorkspaceAction::ToggleDock);
    let (next_width, next_dock) = (transition.panel_width, transition.dock_content);
    let state = cx.global_mut::<SidePanelLeftState_>();
    let was_docked = state.dock_content;
    let was_width = state.panel_width;
    state.panel_width = next_width;
    state.dock_content = next_dock;
    let new_zone = state.exclusive_px();
    if state.last_exclusive_zone != Some(new_zone) {
        state.last_exclusive_zone = None;
    }
    tracing::info!(
        was_docked,
        was_width,
        now_dock = state.dock_content,
        now_width = state.panel_width,
        exclusive_px = new_zone,
        "side_panel_left: dock toggle"
    );
}

/// T279 / Task 4 — rail-tab-select reducer. A free function on `&mut App`
/// (mirrors `apply_dock_toggle`) so a unit test exercises the full path
/// without instantiating `WorkspaceView` — which needs `ChatTab`, whose
/// `new()` spawns an async ACP connect requiring a live Tokio runtime
/// (unconstructable in `TestAppContext`). The rail view delegates to
/// this; `tab_select_transition` is the pure decision core.
///
/// Branch #2 (collapse) remembers the width being collapsed away, so a
/// later re-open returns to the user's last-drag width. `resizable_opt`
/// gates the remember: fixed-width tabs have no runtime width memory.
///
/// T281 / Task 7: routed through `tabs::workspace_transition` (`SelectTab`
/// arm), which itself composes `tabs::tab_select_transition` — same
/// numbers, single reducer boundary.
pub fn select_tab(tab: tabs::LeftTab, cx: &mut App) {
    let transition = tabs::workspace_transition(workspace_snapshot(cx), tabs::WorkspaceAction::SelectTab(tab));
    let (next_tab, next_width, next_dock) =
        (transition.active_tab, transition.panel_width, transition.dock_content);
    let state = cx.global_mut::<SidePanelLeftState_>();
    let was_tab = state.active_tab;
    let was_width = state.panel_width;
    let collapsing =
        next_tab == was_tab && !state.dock_content && next_width == tabs::RAIL_WIDTH;
    if collapsing {
        if let Some(resizable) = workspace_view::resizable_active(was_tab) {
            state.remembered_widths.set(resizable, was_width);
        }
    }
    state.active_tab = next_tab;
    if next_width > tabs::RAIL_WIDTH {
        state.ensure_content_width(next_width);
    } else {
        state.panel_width = tabs::RAIL_WIDTH;
        state.last_exclusive_zone = None;
    }
    state.dock_content = next_dock;
    let new_zone = state.exclusive_px();
    if state.last_exclusive_zone != Some(new_zone) {
        state.last_exclusive_zone = None;
    }
    tracing::info!(
        was_tab = was_tab.label(),
        now_tab = state.active_tab.label(),
        now_width = state.panel_width,
        now_dock = state.dock_content,
        "side_panel_left: rail tab select"
    );
}

/// Direct handle to the live chat column for the coordinator reducers.
/// Reached through `SidePanelLeftState_.chat` (registered by `open_window`
/// on `CommitBoth`, reset by `close`/`close_this`), NOT `content_view` —
/// `content_view` is already leased while `WorkspaceView::on_*_event` runs,
/// and a second lease of the same entity is a `double_lease_panic`.
/// `None` in unit tests and whenever the surfaces are closed; reducers
/// no-op in that case.
fn chat_handle(cx: &App) -> Option<Entity<ChatTab>> {
    cx.global::<SidePanelLeftState_>()
        .chat
        .as_ref()
        .and_then(|w| w.upgrade())
}

/// T279 / Task 4 — session-select coordinator reducer. Free function on
/// `&mut App` (T278 lesson) so a unit test calls it by name and asserts
/// the SoT. Records the id, switches to Chat, and loads the transcript
/// into the live `ChatTab` (`load_thread_by_id` → `load_thread` → legacy
/// `select_session`: outgoing cache, cached replay, ACP `load_session`).
pub fn select_session(thread_id: String, cx: &mut App) {
    cx.global_mut::<SidePanelLeftState_>().active_session_id = Some(thread_id.clone());
    select_tab(tabs::LeftTab::Chat, cx);
    if let Some(chat) = chat_handle(cx) {
        chat.update(cx, |chat, cx| chat.load_thread_by_id(&thread_id, cx));
    }
}

/// T279 / Task 4 — "+ New" reducer. Opens Chat and mints a fresh thread
/// in the live `ChatTab` (`create_new_session`), mirroring the
/// inline-sidebar "＋" path. Free function on `&mut App` so the Sessions
/// tab reaches it through the coordinator.
pub fn create_thread(cx: &mut App) {
    select_tab(tabs::LeftTab::Chat, cx);
    if let Some(chat) = chat_handle(cx) {
        chat.update(cx, |chat, cx| chat.create_new_session(cx));
    }
}

/// T279 / Task 4 — project-switch coordinator reducer. Free function on
/// `&mut App` (T278 lesson) so a unit test calls it by name and asserts
/// the SoT. Clears the old session id, sets the new project path, and
/// clears the outgoing chat column via `ChatTab::clear_for_project`
/// (reached through `SidePanelLeftState_.chat`).
///
/// T280: after clearing, `ChatTab::restore_project_thread` below loads
/// the store's `active_thread(project_path)` and restores it; the
/// coordinator re-derives the Sessions list from the new project on next
/// render.
pub fn switch_project(new_project_path: std::path::PathBuf, cx: &mut App) {
    {
        let state = cx.global_mut::<SidePanelLeftState_>();
        state.active_session_id = None;
        state.active_project_path = Some(new_project_path.clone());
        let new_zone = state.exclusive_px();
        if state.last_exclusive_zone != Some(new_zone) {
            state.last_exclusive_zone = None;
        }
        tracing::info!(
            now_project = format!("{:?}", state.active_project_path),
            now_session = format!("{:?}", state.active_session_id),
            "side_panel_left: project switched (session cleared)"
        );
    }
    if let Some(chat) = chat_handle(cx) {
        chat.update(cx, |chat, cx| {
            chat.clear_for_project(&new_project_path, cx);
            // T280: after clearing, restore the new project's persisted
            // active thread (valid → load; stale → empty Chat).
            chat.restore_project_thread(&new_project_path, cx);
        });
    }
}

/// T279 / Task 4 — project-removal reducer. When the removed path is the
/// active project, clears the active project + session scope AND the chat
/// column (`ChatTab::clear_for_project`) so the removed project's chat
/// cannot leak onto the screen. Returns `true` when the scope was cleared.
/// Free function on `&mut App` (T278 lesson).
pub fn remove_project_scope(path: std::path::PathBuf, cx: &mut App) -> bool {
    let clears = {
        let state = cx.global::<SidePanelLeftState_>();
        state.active_project_path.as_deref() == Some(path.as_path())
    };
    if !clears {
        return false;
    }
    {
        let state = cx.global_mut::<SidePanelLeftState_>();
        state.active_project_path = None;
        state.active_session_id = None;
        state.last_exclusive_zone = None;
        tracing::info!(
            removed_project = format!("{path:?}"),
            "side_panel_left: active project removed (scope cleared)"
        );
    }
    if let Some(chat) = chat_handle(cx) {
        chat.update(cx, |chat, cx| chat.clear_for_project(&path, cx));
    }
    true
}

/// T281 gate 8 — restore the session scope on startup.
///
/// Reads the persisted active project from `ProjectsConfig.active` (the last
/// project the user selected — `project_switcher::set_active` writes it on
/// every project switch) and mirrors it onto the workspace SoT. When the live
/// `ChatTab` already exists it also loads that project's last valid thread;
/// the store validates id + project_path and archived=0, so a stale /
/// archived / deleted / cross-project active id yields empty Chat (never
/// another project's leak). `ChatTab::new` performs the same restore when the
/// panel is first opened after a restart, so this is safe to call even before
/// the chat entity exists (it then only seeds the SoT path).
pub fn restore_active_project_on_startup(cx: &mut App) {
    let Some(active) = crate::project_switcher::cached().active.clone() else {
        return;
    };
    let path = PathBuf::from(active);
    {
        let state = cx.global_mut::<SidePanelLeftState_>();
        // Seed the scope only if nothing is already selected — never clobber a
        // live in-flight session (e.g. an IPC-driven open at startup).
        if state.active_project_path.is_none() {
            state.active_project_path = Some(path.clone());
        }
    }
    if let Some(chat) = chat_handle(cx) {
        chat.update(cx, |chat, cx| chat.restore_project_thread(path.as_path(), cx));
    }
}

/// T226 tooling: open the left agent panel pinned, dock the chat column
/// (full panel width, not overlay) and focus the composer so typed input
/// lands in the message box. `App` context — IPC handler has no `Window`,
/// so it reaches the workspace through the weak handle.
///
/// T278: dock + width live on `SidePanelLeftState_` (SoT). Width is set
/// via `ensure_content_width` so the cache invalidation hooks fire; the
/// workspace then mirrors SoT into the legacy child on its next render.
/// Composer focus is queued for the next render — the IPC path has no
/// `&mut Window`, so we let `WorkspaceView::render` consume the flag.
pub fn expand_with_composer(cx: &mut App) {
    open_pinned(cx);
    let Some(workspace) = cx
        .global::<SidePanelLeftState_>()
        .content_view
        .as_ref()
        .and_then(|w| w.upgrade())
    else {
        tracing::warn!("side_panel_left: expand_with_composer has no workspace");
        return;
    };
    // T281 / Task 7: route through the single reducer so this always lands
    // on Chat+dock regardless of which tab was active before the call —
    // see `set_panel_width`'s doc comment for the bug this closes.
    let transition = tabs::workspace_transition(workspace_snapshot(cx), tabs::WorkspaceAction::ExpandComposer);
    workspace.update(cx, |view, cx| {
        view.set_panel_width(transition.panel_width, transition.dock_content, transition.active_tab, cx);
        view.request_focus_composer(cx);
    });
}

/// T241 tooling: open the left panel, write `text` into the composer, and
/// send it to the agent — all in one IPC command. Bypasses Wayland seat focus
/// entirely (same class of tool as `preview-target`).
pub fn compose_and_send(text: String, cx: &mut App) {
    open_pinned(cx);
    let Some(workspace) = cx
        .global::<SidePanelLeftState_>()
        .content_view
        .as_ref()
        .and_then(|w| w.upgrade())
    else {
        tracing::warn!("side_panel_left: compose_and_send has no workspace");
        return;
    };
    // T281 / Task 7: same reducer as `expand_with_composer` — always lands
    // on Chat+dock so the text actually appears where it's written.
    let transition = tabs::workspace_transition(workspace_snapshot(cx), tabs::WorkspaceAction::ComposeAndSend);
    workspace.update(cx, |view, cx| {
        view.set_panel_width(transition.panel_width, transition.dock_content, transition.active_tab, cx);
        // Send the message via the legacy child. This reaches the same
        // `Window` through the parent entity — `send_composer` is
        // identical to the UI button path. T286: the composer text lives
        // in the kit `InputState`, whose setters need a `Window`, so the
        // write and the send both happen inside the window lease.
        let content_handle = cx.global::<SidePanelLeftState_>().content_handle.clone();
        if let Some(handle) = content_handle {
            let _ = handle.update(cx, |_root, window, cx| {
                view.chat.update(cx, |child, cx| {
                    child.composer_input.update(cx, |s, cx| s.set_value(text.clone(), window, cx));
                    child.send_composer(window, cx);
                });
            });
        }
        view.request_focus_composer(cx);
    });
}

pub fn init(cx: &mut App) {
    cx.set_global(SidePanelLeftState_::default());
    // T281 gate 8 — restore the persisted active project scope on startup so a
    // restart reopens where the user left off (the live ChatTab restores the
    // project's last valid session itself, in `ChatTab::new`).
    restore_active_project_on_startup(cx);
    // Defer the strip one tick so `cx.displays()` / pult uuid match what
    // `bar::init` sees a moment later. Opening the strip synchronously in
    // `main` before the bar historically landed it on the wrong output
    // (HDMI-A-1) while the panel+bar bound to DP-1 (pult).
    cx.spawn(async move |cx| {
        cx.background_executor()
            .timer(std::time::Duration::from_millis(50))
            .await;
        cx.update(|cx| {
            // Hover-peek disabled by design decision (2026-07-23) — see
            // T278 / design spec §4. The hover-strip module stays
            // dormant (its init function is never called). The
            // `peek_generation` machinery is still wired and used by the
            // rail/content `on_hover` guards.
            // Optional smoke: pin-open for grim without hover/ydotool.
            // Not product wiring — only when env is set.
            if std::env::var_os("CHRONOS_SMOKE_SIDE_PANEL_LEFT").is_some() {
                open_pinned(cx);
            }
        });
    })
    .detach();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project_switcher::{ProjectEntry, ProjectsConfig};

    #[test]
    fn state_starts_as_peek() {
        let state = state::SidePanelLeftState::new();
        assert_eq!(state.state, PanelState::Peek);
    }

    #[test]
    fn state_default_width_opens_rail_only() {
        // T220: a summon opens rail-only (strip + handle), NOT the chat column.
        let state = state::SidePanelLeftState::new();
        assert_eq!(state.width, sessions_list::SIDEBAR_MIN_WIDTH);
        assert!(state.width <= sessions_list::SIDEBAR_MIN_WIDTH + f32::EPSILON);
        assert!(!state.dock_chat);
    }

    #[test]
    fn state_min_width_is_sidebar_plus_handle() {
        let state = state::SidePanelLeftState::new();
        assert_eq!(state.min_width, sessions_list::SIDEBAR_MIN_WIDTH);
    }

    #[test]
    fn rails_and_handles_match_right_panel() {
        // T276: the standalone right rail owns the full collapsed footprint;
        // the untouched left panel still splits the same 40px into rail+handle.
        assert_eq!(
            crate::side_panel_right::RAIL_ONLY_WIDTH,
            sessions_list::SIDEBAR_COLLAPSED_WIDTH + sessions_list::SIDEBAR_HANDLE_WIDTH
        );
        // T220: summon width must equal the right panel's rail-only width.
        assert_eq!(
            state::SidePanelLeftState::rail_only_width(),
            crate::side_panel_right::RAIL_ONLY_WIDTH
        );
    }

    #[test]
    fn panel_top_corners_follow_the_same_bar_junction_rule() {
        // T217: both panels resolve their top-corner radius through the single
        // `state::panel_corner_radius` (mirrors T204's single-constant rule),
        // so a left and a right corner at the same screen x can never drift.
        let display_w = 2560.0;
        crate::state::set_bar_geometry(16.0, 384.0, 2176.0); // fraction:0.7 centered

        // Free edges (beyond the bar) rhyme with the bar.
        assert_eq!(crate::state::panel_corner_radius(0.0), 16.0); // left panel TL
        assert_eq!(crate::state::panel_corner_radius(display_w), 16.0); // right panel TR
        // Right panel rail-only strip sits right of the bar → rounded.
        assert_eq!(crate::state::panel_corner_radius(display_w - 40.0), 16.0);
        // Left panel rail-only strip sits left of the bar → rounded.
        assert_eq!(crate::state::panel_corner_radius(40.0), 16.0);

        // Under the bar → square (butt, no seam) for either panel.
        assert_eq!(crate::state::panel_corner_radius(2000.0), 0.0);
        assert_eq!(crate::state::panel_corner_radius(1000.0), 0.0);

        // Full-width bar → every corner square.
        crate::state::set_bar_geometry(16.0, 0.0, display_w);
        assert_eq!(crate::state::panel_corner_radius(0.0), 0.0);
        assert_eq!(crate::state::panel_corner_radius(display_w), 0.0);

        // Restore process-wide default for other tests.
        crate::state::set_bar_geometry(0.0, 0.0, f32::INFINITY);
    }

    #[test]
    fn toggle_collapse_recalculates_min_width() {
        let mut state = state::SidePanelLeftState::new();
        assert!(state.sessions_collapsed);
        assert_eq!(state.width, sessions_list::SIDEBAR_MIN_WIDTH);
        // Expand sessions: min must fit 200 + handle
        state.sessions_collapsed = false;
        state.recalc_min_width();
        assert_eq!(
            state.min_width,
            sessions_list::SIDEBAR_EXPANDED_WIDTH + sessions_list::SIDEBAR_HANDLE_WIDTH
        );
        assert!(state.width >= state.min_width);
    }

    #[test]
    fn clamp_width_below_min_after_recalc() {
        let mut state = state::SidePanelLeftState::new();
        state.resize(10.0);
        assert_eq!(state.width, sessions_list::SIDEBAR_MIN_WIDTH);
    }

    #[test]
    fn exclusive_px_dock_vs_overlay() {
        let mut state = state::SidePanelLeftState::new();
        assert!(!state.dock_chat);
        // Bar strip includes handle so tiles don't sit under the grab edge.
        assert_eq!(state.exclusive_px(), sessions_list::SIDEBAR_MIN_WIDTH);
        state.sessions_collapsed = false;
        assert_eq!(
            state.exclusive_px(),
            sessions_list::SIDEBAR_EXPANDED_WIDTH + sessions_list::SIDEBAR_HANDLE_WIDTH
        );
        // T220: dock on at rail-only width — exclusive zone == width == rail-only.
        state.dock_chat = true;
        assert_eq!(state.exclusive_px(), sessions_list::SIDEBAR_MIN_WIDTH);
        // Dock on at expanded width — exclusive zone follows the width.
        state.width = 400.0;
        assert_eq!(state.exclusive_px(), 400.0);
    }

    #[test]
    fn ensure_chat_width_expands_from_sidebar_only() {
        let mut state = state::SidePanelLeftState::new();
        state.width = sessions_list::SIDEBAR_MIN_WIDTH;
        state.ensure_chat_width();
        assert!(state.width > sessions_list::SIDEBAR_MIN_WIDTH);
        assert_eq!(state.width, state::SidePanelLeftState::DEFAULT_CHAT_WIDTH);
        // Remembered width is now set so a later summon→expand returns it.
        assert_eq!(state.remembered_chat_width, Some(state.width));
    }

    #[test]
    fn ensure_chat_width_restores_remembered_width() {
        // T220 req #1: expand to N, collapse, next expand returns N not 352.
        let mut state = state::SidePanelLeftState::new();
        let n = 500.0;
        state.width = n;
        state.remembered_chat_width = Some(n);
        // Collapse back to rail-only (simulating close).
        state.width = sessions_list::SIDEBAR_MIN_WIDTH;
        // Re-expand: must return the remembered N, not DEFAULT_CHAT_WIDTH.
        state.ensure_chat_width();
        assert_eq!(state.width, n);
    }

    #[test]
    fn resize_remembers_expanded_width() {
        // T220 req #1: a manual drag/resize sets the remembered width.
        let mut state = state::SidePanelLeftState::new();
        let n = 600.0;
        state.resize(n);
        assert_eq!(state.remembered_chat_width, Some(n));
        // Collapse and re-expand via ensure_chat_width → returns N.
        state.width = sessions_list::SIDEBAR_MIN_WIDTH;
        state.ensure_chat_width();
        assert_eq!(state.width, n);
    }

    // ── T278 / Slice A1 — two-surface lifecycle contracts ──

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
        let mut state = SidePanelLeftState_::default();
        state.pinned = true;
        assert!(!should_close_on_peek_leave(&state));
    }

    #[test]
    fn peek_close_request_closes_when_not_pinned() {
        let mut state = SidePanelLeftState_::default();
        state.pinned = false;
        assert!(should_close_on_peek_leave(&state));
    }

    #[test]
    fn peek_close_suppressed_while_resizing() {
        // T278: same suppression rule as T276 / right panel — a resize
        // drag must not be terminated by a stale hover-leave from the
        // rail or content canvas.
        let mut state = SidePanelLeftState_::default();
        state.pinned = false;
        state.resizing = true;
        assert!(!should_close_on_peek_leave(&state));
    }

    #[test]
    fn sot_default_matches_left_rail_only() {
        let state = SidePanelLeftState_::default();
        assert_eq!(state.rail_handle, None);
        assert_eq!(state.content_handle, None);
        assert_eq!(state.content_view, None);
        assert_eq!(state.panel_width, tabs::RAIL_WIDTH);
        assert_eq!(state.active_tab, tabs::LeftTab::Chat);
        assert!(!state.dock_content);
        assert!(!state.resizing);
        assert!(!state.pinned);
        assert_eq!(state.peek_generation, 0);
        assert_eq!(state.last_exclusive_zone, None);
        // Default ResizableWidths slots match spec §7.
        assert_eq!(state.remembered_widths.chat, 560.0);
        assert_eq!(state.remembered_widths.plan, 480.0);
        assert_eq!(state.remembered_widths.context_files, 560.0);
    }

    #[test]
    fn sot_exclusive_px_dock_vs_overlay() {
        // Mirrors the right-panel T276 contract.
        let mut state = SidePanelLeftState_::default();
        assert_eq!(state.exclusive_px(), tabs::RAIL_WIDTH);
        state.dock_content = true;
        assert_eq!(state.exclusive_px(), state.panel_width);
        state.panel_width = 600.0;
        assert_eq!(state.exclusive_px(), 600.0);
    }

    #[test]
    fn sot_resize_clamps_into_drag_range() {
        let mut state = SidePanelLeftState_::default();
        state.resize(0.0); // below RAIL_WIDTH
        assert_eq!(state.panel_width, tabs::RAIL_WIDTH);
        state.resize(2000.0); // above MAX_PANEL_WIDTH
        assert_eq!(state.panel_width, tabs::MAX_PANEL_WIDTH);
        state.resize(500.0); // in range
        assert_eq!(state.panel_width, 500.0);
    }

    #[test]
    fn sot_ensure_content_width_invalidates_zone_cache() {
        // T278 mirror of T276: any explicit width change must clear the
        // rail's cached exclusive_zone so the next paint re-pushes.
        let mut state = SidePanelLeftState_::default();
        state.last_exclusive_zone = Some(40.0);
        state.ensure_content_width(500.0);
        assert_eq!(state.panel_width, 500.0);
        assert_eq!(state.last_exclusive_zone, None);
    }

    #[test]
    fn left_rail_width_matches_right_rail_only_width() {
        // Spec §3: both rails own the full collapsed footprint — 40 px
        // end-to-end (the legacy split into 36+4 stays inside the
        // legacy per-instance state for backward compatibility with the
        // A1 bridge but is no longer the surface width).
        assert_eq!(
            tabs::RAIL_WIDTH,
            crate::side_panel_right::RAIL_ONLY_WIDTH
        );
    }

    #[test]
    fn left_content_canvas_width_is_max_minus_rail() {
        assert_eq!(tabs::CONTENT_CANVAS_WIDTH, 920.0);
        assert_eq!(
            tabs::CONTENT_CANVAS_WIDTH,
            tabs::MAX_PANEL_WIDTH - tabs::RAIL_WIDTH
        );
    }

    #[test]
    fn window_options_have_no_resize_calls() {
        // T278 spec §"Запрещено": `window.resize()` is forbidden across
        // `side_panel_left`. Skip comment lines (`//`/`*`) and string
        // literals so the test does not match its own error message.
        fn scan_for_resize(src: &str, file_label: &str) {
            for (i, line) in src.lines().enumerate() {
                let trimmed = line.trim_start();
                if trimmed.starts_with("//") || trimmed.starts_with("/*")
                    || trimmed.starts_with('*') || trimmed.starts_with("//!")
                {
                    continue;
                }
                // Strip inline string literals — ` "window.resize() ..." `
                // would otherwise match. We only flag the bare call site.
                let mut without_strings = String::with_capacity(line.len());
                for (idx, part) in line.split('"').enumerate() {
                    if idx % 2 == 0 {
                        without_strings.push_str(part);
                    }
                }
                assert!(
                    !without_strings.contains("window.resize("),
                    "{file_label} line {} contains a live `window.resize(` \
                     call — forbidden by the T278 contract. Drag must only \
                     mutate SidePanelLeftState_.panel_width and re-issue \
                     set_input_region on the next paint. Line: {line}",
                    i + 1,
                );
            }
        }
        scan_for_resize(include_str!("mod.rs"), "side_panel_left::mod.rs");
        scan_for_resize(
            include_str!("workspace_view.rs"),
            "side_panel_left::workspace_view.rs",
        );
        scan_for_resize(
            include_str!("rail_view.rs"),
            "side_panel_left::rail_view.rs",
        );
    }

    #[gpui::test]
    async fn window_options_match_spec(cx: &mut gpui::TestAppContext) {
        // Direct test of the WindowOptions builders. Runs against
        // GPUI's TestAppContext which provides a real `App`; the
        // display fallback (`unwrap_or(1080.)`) lets us skip any
        // monitor/AppState wiring — we just need the global to exist
        // so `try_global` inside the options builders resolves.
        cx.update(|cx| {
            crate::side_panel_left::init(cx);
        });
        let opts = cx.update(|cx| rail_window_options(None, cx));
        match opts.kind {
            gpui::WindowKind::LayerShell(ls) => {
                assert_eq!(ls.namespace, "side_panel_left_rail");
                assert!(ls.anchor.contains(gpui::layer_shell::Anchor::TOP));
                assert!(ls.anchor.contains(gpui::layer_shell::Anchor::LEFT));
                assert_eq!(ls.layer, gpui::layer_shell::Layer::Overlay);
                assert_eq!(
                    ls.keyboard_interactivity,
                    gpui::layer_shell::KeyboardInteractivity::None
                );
                assert_eq!(ls.exclusive_edge, Some(gpui::layer_shell::Anchor::LEFT));
            }
            _ => panic!("rail must be a LayerShell window"),
        }
        assert_eq!(opts.app_id.as_deref(), Some("chronos-side-panel-left-rail"));
        let rail_w = match opts.window_bounds.expect("rail window_bounds") {
            gpui::WindowBounds::Windowed(b) => b.size.width.as_f32(),
            _ => panic!("rail must be a Windowed window"),
        };
        assert_eq!(rail_w, tabs::RAIL_WIDTH);

        let opts = cx.update(|cx| content_window_options(None, cx));
        match opts.kind {
            gpui::WindowKind::LayerShell(ls) => {
                assert_eq!(ls.namespace, "side_panel_left_content");
                assert!(ls.anchor.contains(gpui::layer_shell::Anchor::TOP));
                assert!(ls.anchor.contains(gpui::layer_shell::Anchor::LEFT));
                assert_eq!(ls.layer, gpui::layer_shell::Layer::Overlay);
                assert_eq!(
                    ls.keyboard_interactivity,
                    gpui::layer_shell::KeyboardInteractivity::OnDemand
                );
                assert_eq!(
                    ls.exclusive_zone,
                    Some(gpui::px(-1.0)),
                    "content opts out of foreign exclusive zones"
                );
            }
            _ => panic!("content must be a LayerShell window"),
        }
        assert_eq!(
            opts.app_id.as_deref(),
            Some("chronos-side-panel-left-content")
        );
        let content_w = match opts.window_bounds.expect("content window_bounds") {
            gpui::WindowBounds::Windowed(b) => b.size.width.as_f32(),
            _ => panic!("content must be a Windowed window"),
        };
        assert_eq!(content_w, tabs::CONTENT_CANVAS_WIDTH);
    }

    // ── T278 / Slice A1 — architect round 2 regression ──
    //
    // The original close() and close_this() did NOT reset panel_width or
    // dock_content, so a `Super+A → close → Super+A` cycle opened at the
    // last-expanded state instead of rail-only. Tests pin the contract:
    // after close (either path), panel_width == RAIL_WIDTH and
    // dock_content == false, regardless of how the previous session ended.

    #[gpui::test]
    async fn reopen_after_dock_resets_to_rail_only(cx: &mut gpui::TestAppContext) {
        cx.update(|cx| {
            crate::side_panel_left::init(cx);
        });
        // Simulate the user having expanded and docked the panel.
        cx.update(|cx| {
            let state = cx.global_mut::<SidePanelLeftState_>();
            state.ensure_content_width(560.0);
            state.dock_content = true;
            state.pinned = true;
        });
        cx.update(|cx| super::close(cx));
        cx.update(|cx| {
            let state = cx.global::<SidePanelLeftState_>();
            assert_eq!(
                state.panel_width, tabs::RAIL_WIDTH,
                "close() must reset panel_width to RAIL_WIDTH so the \
                 next summon opens rail-only, not at the last-expanded N"
            );
            assert!(
                !state.dock_content,
                "close() must reset dock_content so the next summon \
                 comes up in overlay mode (dock off), not docked"
            );
            assert!(!state.pinned, "close() must also clear pinned");
            assert!(!state.resizing);
            assert_eq!(state.last_exclusive_zone, None);
            assert_eq!(state.rail_handle, None);
            assert_eq!(state.content_handle, None);
        });
    }

    #[gpui::test]
    async fn close_this_path_also_resets_to_rail_only(_cx: &mut gpui::TestAppContext) {
        // `close_this` is the click-X path (`side-panel-left-close`
        // button inside the legacy panel render). It runs from inside a
        // callback that already holds a `&mut Window`, so the test
        // can't drive it end-to-end without a real Wayland surface.
        // We instead read the source for the reset call — same contract
        // the live path enforces, just statically anchored so a future
        // regression (e.g. someone deleting the reset during a refactor)
        // surfaces here.
        let src = include_str!("mod.rs");
        let close_this_idx = src
            .find("pub(crate) fn close_this")
            .expect("close_this must exist in mod.rs");
        let close_block = &src[close_this_idx..];
        // The reset calls sit inside the inner block before
        // `window.remove_window()`.
        assert!(
            close_block.contains("state.panel_width = tabs::RAIL_WIDTH"),
            "close_this must reset panel_width to RAIL_WIDTH (architect round 2)"
        );
        assert!(
            close_block.contains("state.dock_content = false"),
            "close_this must reset dock_content to false (architect round 2)"
        );
    }

    /// T278 architect round 2: the legacy child must mirror the VISIBLE
    /// slice width, not the logical panel_width. At panel_width = 40
    /// (rail-only) visible_w = 0 — the legacy child is omitted from the
    /// render tree (no painting past visible slice, no opaque band). At
    /// any non-rail width, the mirrored width equals the visible slice
    /// so the legacy sidebar (40 px) fits exactly inside the slice.
    #[test]
    fn painted_slice_width_matches_visible_w() {
        use state::geometry;
        // Rail-only: panel_w = 40, visible_w = 0 → render nothing.
        let panel_w = tabs::RAIL_WIDTH;
        let visible_w = geometry::visible_content_width(panel_w);
        assert_eq!(visible_w, 0.0);
        // The mirror clamps to SIDEBAR_MIN_WIDTH so the legacy render
        // never collapses to zero width (which would panic its
        // sidebar layout). 0 → 40, but visible_w == 0 is what gates
        // the `when(visible_w > 0.0, ...)` branch.
        let mirrored = visible_w.max(crate::side_panel_left::sessions_list::SIDEBAR_MIN_WIDTH);
        assert_eq!(mirrored, crate::side_panel_left::sessions_list::SIDEBAR_MIN_WIDTH);
        assert!(
            visible_w <= 0.0,
            "rail-only must yield visible_w == 0 so the legacy child \
             is omitted from the render tree"
        );
        // Expanded: panel_w = 560, visible_w = 520 → child mirrors 520.
        let panel_w = 560.0;
        let visible_w = geometry::visible_content_width(panel_w);
        assert_eq!(visible_w, 520.0);
        let mirrored = visible_w.max(crate::side_panel_left::sessions_list::SIDEBAR_MIN_WIDTH);
        assert_eq!(mirrored, 520.0);
        // Full canvas: panel_w = 960, visible_w = 920 → child mirrors 920.
        let panel_w = tabs::MAX_PANEL_WIDTH;
        let visible_w = geometry::visible_content_width(panel_w);
        assert_eq!(visible_w, tabs::CONTENT_CANVAS_WIDTH);
        let mirrored = visible_w.max(crate::side_panel_left::sessions_list::SIDEBAR_MIN_WIDTH);
        assert_eq!(mirrored, tabs::CONTENT_CANVAS_WIDTH);
    }

    /// T278 architect round 3: the dock reducer is the pure helper
    /// `tabs::dock_transition` — exercised directly here so a future
    /// regression in the reducer (the round 2 "always preserve"
    /// deadlock) cannot land without a test failure. The integration
    /// path through `WorkspaceView::on_dock_toggle` is covered by the
    /// production code (mod.rs / rail_view.rs); this test pins the
    /// pure transition.
    #[test]
    fn dock_transition_from_rail_only_expands_to_preferred_width() {
        // Rail-only + dock on → expand to active tab's remembered width
        // (Chat default 560). Without this branch, dock=true at width=40
        // deadlocks: content invisible, every active-tab click is a
        // dock-wins no-op, only close+reopen resets.
        let remembered = tabs::ResizableWidths::default();
        let (next_w, next_dock) = tabs::dock_transition(
            tabs::RAIL_WIDTH,
            false,
            tabs::LeftTab::Chat,
            &remembered,
        );
        assert!(next_dock, "dock must be on after rail-only → toggle");
        assert_eq!(next_w, remembered.chat, "must expand to Chat remembered");
    }

    #[test]
    fn dock_transition_from_rail_only_uses_fixed_width_for_fixed_tabs() {
        // Spec §7: Sessions is fixed at 400. Rail-only + dock on with
        // Sessions active must open Sessions at 400, not the Chat 560.
        let remembered = tabs::ResizableWidths::default();
        let (next_w, next_dock) = tabs::dock_transition(
            tabs::RAIL_WIDTH,
            false,
            tabs::LeftTab::Sessions,
            &remembered,
        );
        assert!(next_dock);
        assert_eq!(next_w, tabs::LeftTab::Sessions.preferred_panel_width());
    }

    #[test]
    fn dock_transition_from_overlay_preserves_width_on_dock_on() {
        // Expanded (visible_w > 0) + dock on → keep width, flip flag.
        // Panel was already visible; the dock flag just widens the
        // rail's exclusive zone.
        let remembered = tabs::ResizableWidths::default();
        let (next_w, next_dock) = tabs::dock_transition(
            560.0,
            false,
            tabs::LeftTab::Chat,
            &remembered,
        );
        assert!(next_dock);
        assert_eq!(next_w, 560.0, "overlay → dock on must not resize");
    }

    #[test]
    fn dock_transition_from_docked_preserves_width_on_dock_off() {
        // Docked + dock off → keep width, flip flag. The visible slice
        // stays open at the user's drag width; the rail's exclusive
        // zone narrows back to RAIL_WIDTH.
        let remembered = tabs::ResizableWidths::default();
        let (next_w, next_dock) = tabs::dock_transition(
            612.0,
            true,
            tabs::LeftTab::Chat,
            &remembered,
        );
        assert!(!next_dock);
        assert_eq!(next_w, 612.0, "docked → undock must not resize");
    }

    #[test]
    fn dock_transition_uses_remembered_width_for_resizable_tab() {
        // Chat user previously dragged to 700; rail-only → dock on
        // must restore 700, not the 560 default.
        let mut remembered = tabs::ResizableWidths::default();
        remembered.chat = 700.0;
        let (next_w, next_dock) = tabs::dock_transition(
            tabs::RAIL_WIDTH,
            false,
            tabs::LeftTab::Chat,
            &remembered,
        );
        assert!(next_dock);
        assert_eq!(next_w, 700.0);
    }

    #[test]
    fn dock_transition_does_not_leak_into_dock_off_cases() {
        // Sanity: dock off (any branch) never expands. Even from rail-only
        // the user toggling dock off goes back to rail-only with
        // panel_width preserved at 40.
        let remembered = tabs::ResizableWidths::default();
        let (next_w, next_dock) = tabs::dock_transition(
            tabs::RAIL_WIDTH,
            true,
            tabs::LeftTab::Chat,
            &remembered,
        );
        assert!(!next_dock);
        assert_eq!(
            next_w, tabs::RAIL_WIDTH,
            "dock off from rail-only stays at RAIL_WIDTH"
        );
    }

    /// T278 architect round 2: dock-toggle icon convention is action-
    /// oriented. `⊞` enables dock (shown when currently undocked); `⊟`
    /// disables dock (shown when currently docked). Pure enum so we can
    /// test it without rendering.
    #[test]
    fn dock_toggle_icon_convention_is_action_oriented() {
        fn icon_for(dock: bool) -> &'static str {
            if dock { "⊟" } else { "⊞" }
        }
        assert_eq!(icon_for(false), "⊞", "undocked shows the enable icon");
        assert_eq!(icon_for(true), "⊟", "docked shows the disable icon");
    }

    /// T278 architect round 4 integration: the production reducer
    /// `apply_dock_toggle(cx)` is invoked on a real `App` (in
    /// `TestAppContext`) and the SoT must match what the pure helper
    /// `tabs::dock_transition` produced for the same inputs. The
    /// round-3 `on_dock_toggle_uses_pure_helper` test was a tautology
    /// (it assigned and read back without ever calling the reducer);
    /// this one actually exercises the production code path.
    ///
    /// Three branches (spec §4.1):
    /// - rail-only + dock on → expand to `width_for_open(active, remembered)`.
    /// - overlay    + dock on → preserve width, flip flag.
    /// - docked     + dock off → preserve width, flip flag.
    ///
    /// `apply_dock_toggle` is a free function on `&mut App` (not an
    /// entity method) precisely so the test harness can drive it
    /// without instantiating `WorkspaceView` — which requires a live
    /// `ChatTab`, which spawns an async ACP-connect that
    /// requires a Tokio runtime (unconstructable in `TestAppContext`).
    /// `WorkspaceView::on_dock_toggle` is now a thin dispatcher.
    #[gpui::test]
    async fn apply_dock_toggle_matches_helper_in_real_app(
        cx: &mut gpui::TestAppContext,
    ) {
        cx.update(|cx| crate::side_panel_left::init(cx));

        let run_branch = |cx: &mut gpui::TestAppContext,
                          panel_w: f32,
                          dock: bool,
                          tab: tabs::LeftTab,
                          remembered: tabs::ResizableWidths| {
            cx.update(|cx| {
                let state = cx.global_mut::<SidePanelLeftState_>();
                state.panel_width = panel_w;
                state.dock_content = dock;
                state.active_tab = tab;
                state.remembered_widths = remembered;
            });
            let expected = tabs::dock_transition(
                panel_w,
                dock,
                tab,
                &cx.update(|cx| {
                    cx.global::<SidePanelLeftState_>().remembered_widths.clone()
                }),
            );
            cx.update(|cx| apply_dock_toggle(cx));
            let actual = cx.update(|cx| {
                let state = cx.global::<SidePanelLeftState_>();
                (state.panel_width, state.dock_content)
            });
            assert_eq!(
                actual, expected,
                "apply_dock_toggle must match dock_transition \
                 for (panel_w={panel_w}, dock={dock}, tab={tab:?})"
            );
        };

        // Branch 1: rail-only + dock on → expand (Chat default 560).
        run_branch(
            cx,
            tabs::RAIL_WIDTH,
            false,
            tabs::LeftTab::Chat,
            tabs::ResizableWidths::default(),
        );

        // Branch 2: overlay + dock on → preserve width.
        run_branch(
            cx,
            612.0,
            false,
            tabs::LeftTab::Chat,
            tabs::ResizableWidths::default(),
        );

        // Branch 3: docked + dock off → preserve width.
        run_branch(
            cx,
            612.0,
            true,
            tabs::LeftTab::Chat,
            tabs::ResizableWidths::default(),
        );

        // Branch 4: rail-only + dock on with a remembered Chat width
        // — proves the production path reads `state.remembered_widths`
        // and not just the static default.
        let mut remembered = tabs::ResizableWidths::default();
        remembered.chat = 700.0;
        run_branch(cx, tabs::RAIL_WIDTH, false, tabs::LeftTab::Chat, remembered);

        // Branch 5: rail-only + dock on with a fixed-width tab
        // (Sessions = 400) — proves the production path uses
        // `active_tab.preferred_panel_width()` for non-resizable tabs.
        run_branch(
            cx,
            tabs::RAIL_WIDTH,
            false,
            tabs::LeftTab::Sessions,
            tabs::ResizableWidths::default(),
        );
    }

    // ── T279 round 2 / Task 4 — coordinator reducers on `&mut App` ──
    //
    // The Chat path is unreachable here (`SoT.chat == None` without a live
    // `WorkspaceView` — `ChatTab::new` spawns an async ACP connect needing
    // a Tokio runtime, unconstructable in `TestAppContext`); the reducers
    // must no-op, not panic. That the chat column reads the SAME state is
    // proven by construction: `select_session` writes `active_session_id`
    // and loads the same id via `load_thread_by_id`; `switch_project`
    // writes the path and clears via the same path in `clear_for_project`.

    /// `select_session` records the id on the SoT and switches the
    /// workspace to Chat.
    #[gpui::test]
    async fn select_session_records_id_and_opens_chat(cx: &mut gpui::TestAppContext) {
        // T279 round 2 review: start with Sessions as the active tab so
        // the assertion below proves the reducer switched to Chat —
        // with the default `active_tab = Chat` the assert would be a
        // tautology (same class as the T278 `on_dock_toggle` theater).
        cx.update(|cx| crate::side_panel_left::init(cx));
        cx.update(|cx| {
            cx.global_mut::<SidePanelLeftState_>().active_tab = tabs::LeftTab::Sessions;
        });
        cx.update(|cx| select_session("thread-42".to_string(), cx));
        cx.update(|cx| {
            let state = cx.global::<SidePanelLeftState_>();
            assert_eq!(
                state.active_session_id.as_deref(),
                Some("thread-42"),
                "select_session must record the id on the SoT"
            );
            assert_eq!(
                state.active_tab,
                tabs::LeftTab::Chat,
                "select_session must switch the workspace to Chat"
            );
        });
    }

    /// `switch_project` sets the new project path and clears the stale
    /// session id (the old project's thread must not leak into the new
    /// scope).
    #[gpui::test]
    async fn switch_project_sets_path_and_clears_session(cx: &mut gpui::TestAppContext) {
        cx.update(|cx| crate::side_panel_left::init(cx));
        // Precondition: a stale session from the previous project.
        cx.update(|cx| {
            cx.global_mut::<SidePanelLeftState_>().active_session_id =
                Some("old-thread".to_string());
        });
        cx.update(|cx| switch_project(std::path::PathBuf::from("/home/neo/new-proj"), cx));
        cx.update(|cx| {
            let state = cx.global::<SidePanelLeftState_>();
            assert_eq!(
                state.active_project_path.as_deref(),
                Some(std::path::Path::new("/home/neo/new-proj")),
                "switch_project must set the new project path"
            );
            assert_eq!(
                state.active_session_id, None,
                "switch_project must clear the old session id"
            );
        });
    }

    /// `remove_project_scope` clears only when the removed path IS the
    /// active project; a foreign path leaves the scope untouched.
    #[gpui::test]
    async fn remove_project_scope_clears_only_the_active_project(
        cx: &mut gpui::TestAppContext,
    ) {
        cx.update(|cx| crate::side_panel_left::init(cx));

        // Removing the ACTIVE project clears path + session and reports true.
        let cleared = cx.update(|cx| {
            cx.global_mut::<SidePanelLeftState_>().active_project_path =
                Some(std::path::PathBuf::from("/home/neo/active"));
            cx.global_mut::<SidePanelLeftState_>().active_session_id =
                Some("leak".to_string());
            remove_project_scope(std::path::PathBuf::from("/home/neo/active"), cx)
        });
        assert!(cleared, "removing the active project must clear scope");
        cx.update(|cx| {
            let state = cx.global::<SidePanelLeftState_>();
            assert_eq!(state.active_project_path, None);
            assert_eq!(state.active_session_id, None);
        });

        // Removing a FOREIGN project is a no-op and reports false.
        let cleared = cx.update(|cx| {
            cx.global_mut::<SidePanelLeftState_>().active_project_path =
                Some(std::path::PathBuf::from("/home/neo/keep"));
            cx.global_mut::<SidePanelLeftState_>().active_session_id =
                Some("keep-thread".to_string());
            remove_project_scope(std::path::PathBuf::from("/home/neo/other"), cx)
        });
        assert!(!cleared, "removing a foreign project must not clear scope");
        cx.update(|cx| {
            let state = cx.global::<SidePanelLeftState_>();
            assert_eq!(
                state.active_project_path.as_deref(),
                Some(std::path::Path::new("/home/neo/keep"))
            );
            assert_eq!(state.active_session_id.as_deref(), Some("keep-thread"));
        });
    }

    // ── T281 gate 8 — restore session on startup ──

    /// Restart must seed the persisted active project onto the workspace SoT,
    /// so the left panel reopens in the project the user last worked in.
    #[gpui::test]
    async fn restore_on_startup_seeds_active_project_path(cx: &mut gpui::TestAppContext) {
        cx.update(|cx| crate::side_panel_left::init(cx));
        // Seed the persisted active project without touching projects.toml.
        crate::project_switcher::set_cached_for_test(ProjectsConfig {
            active: Some("/home/neo/restart-proj".into()),
            projects: vec![ProjectEntry {
                name: "restart-proj".into(),
                path: "/home/neo/restart-proj".into(),
            }],
        });
        cx.update(|cx| {
            cx.global_mut::<SidePanelLeftState_>().active_project_path = None;
        });
        cx.update(|cx| crate::side_panel_left::restore_active_project_on_startup(cx));
        cx.update(|cx| {
            let state = cx.global::<SidePanelLeftState_>();
            assert_eq!(
                state.active_project_path.as_deref(),
                Some(std::path::Path::new("/home/neo/restart-proj")),
                "restart must restore the persisted active project scope"
            );
        });
    }

    /// With no persisted active project, startup restore must leave the scope
    /// untouched (empty Chat, no phantom project).
    #[gpui::test]
    async fn restore_on_startup_noop_without_active_project(cx: &mut gpui::TestAppContext) {
        // Seed the empty config BEFORE init — the cache is a process-wide
        // OnceLock shared across tests in this binary.
        crate::project_switcher::set_cached_for_test(ProjectsConfig::default());
        cx.update(|cx| crate::side_panel_left::init(cx));
        cx.update(|cx| {
            assert_eq!(
                cx.global::<SidePanelLeftState_>().active_project_path,
                None,
                "no active project → no scope seeded on startup"
            );
        });
    }
}
