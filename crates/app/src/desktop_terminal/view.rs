//! PTY session + VT100 grid view for the desktop terminal widget.
//!
//! The engine (PTY spawn, VT100 grid, snapshots, sizing) lives in
//! `chronos_services::terminal` (T177) — shared with the side-panel
//! Terminal tab. This file only owns the view, its poll loop and the grid
//! geometry.
//!
//! The actual shell is owned by the [`TerminalRegistry`] (a GPUI Global
//! set up in `main.rs`), keyed by widget id. This view only borrows a
//! shared `TerminalHandle` (`Arc<Mutex<Terminal>>`) — so closing the window
//! does NOT kill the PTY (T257): the registry keeps it alive for re-open /
//! drag (T259).
//!
//! T259 edit-mode UI (Super+Shift+E, `crate::edit_mode`): while active the
//! widget draws a management chrome — accent frame, title strip as a drag
//! handle, bottom-right resize handle, close ✕. **Fork constraint (agreed in
//! the design brainstorm, do NOT look for a "better" way):** layer-shell
//! surfaces have no runtime reposition API, so dragging can't move the window
//! live. The drag shows a grab-anchored ghost (the fork's `active_drag`
//! preview, clipped to this window's surface) and commits on mouse-up by
//! closing + reopening at the new anchor — the PTY survives because the
//! registry keys it by `widget_id`. Resize *can* go live (`window.resize()`)
//! and only persists on release.

use std::time::Duration;

use chronos_services::{TerminalHandle, terminal::TermSize};
use chronos_ui::{Theme, WindowRootExt};
use gpui::{
    App, Context, DragMoveEvent, Focusable, FontWeight, InteractiveElement, KeyDownEvent,
    MouseButton, MouseUpEvent, Pixels, Point, Render, SharedString, Size, Window, div, point,
    prelude::*, px,
};

use crate::desktop_terminal::{
    TerminalRegistryGlobal, TerminalWidgetSpec, close_one_in_window, load as load_specs,
    move_window, registry, save as save_specs,
};
use crate::edit_mode;

/// Grid geometry for the spike (matches ~600×400 at ~7.5×16 cell).
const COLS: usize = 80;
const ROWS: usize = 24;
const CELL_H: f32 = 16.;
const FONT_SIZE: f32 = 13.;
/// Cell width guess for the initial PTY size (reconciled by no one here —
/// the spike surface is fixed size, this stays static).
const CELL_W: f32 = 8.;
/// How often the UI drains PTY bytes and repaints.
const POLL_MS: u64 = 16;

/// T259: never let the user squash the widget below a usable terminal
/// viewport. ~40 columns × 12 rows of the 8×16 cell grid — a few lines and
/// columns are always visible (the PTY grid itself stays 80×24; the window
/// just clips it).
const MIN_WIDTH: f32 = 320.;
const MIN_HEIGHT: f32 = 192.;

/// T259: floor a resize width/height (no max — the user may stretch freely,
/// but never below [`MIN_WIDTH`]/[`MIN_HEIGHT`]).
fn clamp_size_w(w: f32) -> f32 {
    w.max(MIN_WIDTH)
}
fn clamp_size_h(h: f32) -> f32 {
    h.max(MIN_HEIGHT)
}

/// Drag marker for the widget **move** gesture — own type so it never
/// cross-fires with `WidgetResize` (both handles live in the same window).
struct WidgetDrag;
/// Drag marker for the bottom-right **resize** gesture.
struct WidgetResize;

/// T259 move-drag state, captured at drag start, consumed on release.
struct DragState {
    /// Cursor position in global screen coordinates (logical px) at drag
    /// start. Delta-from-start (not per-frame) keeps the teleport exact.
    start_cursor_global: Point<f32>,
    /// The widget spec as it was when the drag began (its anchor is what we
    /// mutate on commit).
    start_spec: TerminalWidgetSpec,
}

/// T259 resize-drag state. The start-anchor model (delta from start, never
/// from the current frame) is what keeps `window.resize()` — async on
/// Wayland — from accumulating error (same lesson as the right panel's T216).
struct ResizeState {
    start_cursor_local: Point<f32>,
    start_size: Size<f32>,
}

/// T259 drag preview: the semi-transparent frame the fork renders under the
/// cursor while the move drag is in flight (its `active_drag.view`). Sized
/// like the widget, grab-anchored, clipped to this window's surface — the
/// honest visual a fork with no runtime repositioning can produce. The ghost
/// is a plain static rect: the real teleport happens on mouse-up.
struct TerminalDragGhost {
    width: f32,
    height: f32,
    accent: gpui::Hsla,
}

impl Render for TerminalDragGhost {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .w(px(self.width))
            .h(px(self.height))
            .rounded(px(8.))
            .border_2()
            .border_color(self.accent)
            .bg(self.accent.opacity(0.10))
    }
}

/// Desktop terminal view: grid text + keyboard → PTY.
pub struct DesktopTerminalView {
    /// Stable widget identity; key into the PTY registry.
    widget_id: String,
    focus: gpui::FocusHandle,
    /// Shared handle to the registry-owned PTY. `None` only on spawn failure.
    terminal: Option<TerminalHandle>,
    /// Honest error when the PTY could not be spawned (§13).
    error: Option<SharedString>,
    /// Cached line strings for render (rebuilt on PTY data).
    lines: Vec<SharedString>,
    cursor_col: usize,
    cursor_row: usize,
    show_cursor: bool,
    /// T259: set while a move drag is in flight (drag-start → mouse-up).
    drag_state: Option<DragState>,
    /// T259: set while a resize drag is in flight (mouse-down → mouse-up).
    resize_state: Option<ResizeState>,
}

impl DesktopTerminalView {
    /// Create the view for `widget_id`, acquiring its PTY from the registry
    /// (idempotent — re-opening the same id reuses the live shell). On a
    /// test build the registry-backed spawn is bypassed (returns an error)
    /// so unit tests never raise a real shell.
    pub fn new(widget_id: String, cx: &mut Context<Self>) -> Self {
        let mut view = Self {
            widget_id,
            focus: cx.focus_handle(),
            terminal: None,
            error: None,
            lines: vec![SharedString::from(""); ROWS],
            cursor_col: 0,
            cursor_row: 0,
            show_cursor: false,
            drag_state: None,
            resize_state: None,
        };
        view.launch(cx);
        view
    }

    /// Widget identity — used by `close_one` to match the right window and by
    /// `open_one` to keep the `WindowHandle` map in sync.
    pub fn widget_id(&self) -> &str {
        &self.widget_id
    }

    /// Acquire the shared PTY from the registry. `cfg(test)` skips the real
    /// spawn so unit tests never raise a shell (the spike's live smoke covers
    /// the real path).
    fn launch(&mut self, cx: &mut Context<Self>) {
        match spawn_terminal(self.widget_id.clone(), cx) {
            Ok(handle) => {
                self.terminal = Some(handle);
                self.show_cursor = true;
                self.refresh();
                self.start_poll_loop(cx);
            }
            Err(msg) => {
                self.error = Some(msg);
            }
        }
    }

    fn start_poll_loop(&self, cx: &mut Context<Self>) {
        // Same shape as `osd::schedule_hide` / `state::watch`.
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(POLL_MS))
                    .await;
                let cont = this
                    .update(cx, |view, cx| {
                        let Some(term) = view.terminal.as_ref() else {
                            return false;
                        };
                        let mut guard = term.lock().expect("pty lock");
                        let dirty = guard.drain();
                        let alive = guard.is_alive();
                        // `guard` borrow ends here.
                        drop(guard);
                        if dirty {
                            view.refresh();
                            cx.notify();
                        }
                        // Stop polling once the shell exited (PTY EOF) —
                        // the last frame stays on screen.
                        alive
                    })
                    .unwrap_or(false);
                if !cont {
                    break;
                }
            }
        })
        .detach();
    }

    /// Pull the latest grid snapshot into the render cache.
    fn refresh(&mut self) {
        let Some(term) = self.terminal.as_ref() else {
            return;
        };
        let guard = term.lock().expect("pty lock");
        let snap = guard.snapshot();
        self.lines = snap.lines.into_iter().map(SharedString::from).collect();
        self.cursor_row = snap.cursor_row;
        self.cursor_col = snap.cursor_col;
        self.show_cursor = snap.show_cursor;
    }

    fn write_pty(&self, bytes: &[u8]) {
        if let Some(term) = self.terminal.as_ref() {
            term.lock().expect("pty lock").write(bytes);
        }
    }

    fn handle_key(&mut self, event: &KeyDownEvent, _window: &mut Window, cx: &mut Context<Self>) {
        let key = event.keystroke.key.as_str();
        let mods = &event.keystroke.modifiers;

        if mods.control {
            match key {
                "c" => self.write_pty(&[0x03]),
                "d" => self.write_pty(&[0x04]),
                "z" => self.write_pty(&[0x1a]),
                "l" => self.write_pty(&[0x0c]),
                _ => {}
            }
            cx.notify();
            return;
        }

        match key {
            "enter" => self.write_pty(b"\r"),
            "backspace" => self.write_pty(&[0x7f]),
            "tab" => self.write_pty(b"\t"),
            "escape" => self.write_pty(b"\x1b"),
            "up" => self.write_pty(b"\x1b[A"),
            "down" => self.write_pty(b"\x1b[B"),
            "right" => self.write_pty(b"\x1b[C"),
            "left" => self.write_pty(b"\x1b[D"),
            "home" => self.write_pty(b"\x1b[H"),
            "end" => self.write_pty(b"\x1b[F"),
            "pageup" => self.write_pty(b"\x1b[5~"),
            "pagedown" => self.write_pty(b"\x1b[6~"),
            _ => {
                if let Some(ch) = event.keystroke.key_char.as_ref()
                    && !mods.alt
                    && !mods.platform
                {
                    self.write_pty(ch.as_bytes());
                }
            }
        }
        // Immediate drain after input helps prompt feel snappy (still poll-driven for bulk out).
        if let Some(term) = self.terminal.as_mut()
            && term.lock().expect("pty lock").drain()
        {
            self.refresh();
            cx.notify();
        }
    }

    // ── T259 edit-mode gestures ───────────────────────────────────────────

    /// Drag-start hook (fires when the title-strip drag crosses the fork's
    /// `DRAG_THRESHOLD`): record the grab cursor (global) and the widget's
    /// spec as it exists in the config right now, so the release can compute
    /// the delta and commit. `cursor_global` is `window.origin + grab offset`
    /// — computed by the caller from the drag constructor's offset.
    fn begin_drag(&mut self, cursor_global: Point<Pixels>, _cx: &mut Context<Self>) {
        let Some(spec) = load_specs().into_iter().find(|s| s.id == self.widget_id) else {
            tracing::warn!(
                "desktop_terminal: drag start for unknown widget {} (spec missing from config)",
                self.widget_id
            );
            return;
        };
        self.drag_state = Some(DragState {
            start_cursor_global: point(cursor_global.x.as_f32(), cursor_global.y.as_f32()),
            start_spec: spec,
        });
    }

    /// Drag commit (mouse-up, hovered over the strip OR released anywhere
    /// else — `on_mouse_up_out`): compute the new anchor from the final
    /// cursor, persist it, then teleport the window via
    /// [`move_window`] (close + reopen, PTY survives by id).
    fn finalize_drag(&mut self, position: Point<Pixels>, window: &mut Window, cx: &mut Context<Self>) {
        let Some(state) = self.drag_state.take() else {
            return;
        };
        let cursor_global = point(
            window.bounds().origin.x.as_f32() + position.x.as_f32(),
            window.bounds().origin.y.as_f32() + position.y.as_f32(),
        );
        let mut spec = state.start_spec;
        let new_x = (spec.anchor_x + cursor_global.x - state.start_cursor_global.x).max(0.0);
        let new_y = (spec.anchor_y + cursor_global.y - state.start_cursor_global.y).max(0.0);
        // Sub-threshold "drag" (a click that never really moved) → no-op;
        // skipping the close/reopen churn also avoids an unnecessary PTY
        // re-attach.
        if (new_x - spec.anchor_x).abs() < 2.0 && (new_y - spec.anchor_y).abs() < 2.0 {
            return;
        }
        spec.anchor_x = new_x;
        spec.anchor_y = new_y;

        let mut specs = load_specs();
        if let Some(slot) = specs.iter_mut().find(|s| s.id == spec.id) {
            *slot = spec.clone();
        }
        if let Err(err) = save_specs(&specs) {
            tracing::warn!("desktop_terminal: drag-save failed: {err}");
        }
        move_window(&spec, window, cx);
    }

    /// Resize-drag start (mouse-down on the corner handle): capture the
    /// window-local cursor and the current surface size as the anchor.
    fn begin_resize(&mut self, position: Point<Pixels>, window: &mut Window, _cx: &mut Context<Self>) {
        self.resize_state = Some(ResizeState {
            start_cursor_local: point(position.x.as_f32(), position.y.as_f32()),
            start_size: Size::new(
                window.bounds().size.width.as_f32(),
                window.bounds().size.height.as_f32(),
            ),
        });
    }

    /// Resize-drag move: recompute the target size from the start anchor
    /// (never from the current frame — `window.resize` is async on Wayland),
    /// clamp to the minimums, and resize the surface live.
    fn update_resize(&mut self, position: Point<Pixels>, window: &mut Window, cx: &mut Context<Self>) {
        let Some(state) = self.resize_state.as_ref() else {
            return;
        };
        let new_w = clamp_size_w(
            state.start_size.width + position.x.as_f32() - state.start_cursor_local.x,
        );
        let new_h = clamp_size_h(
            state.start_size.height + position.y.as_f32() - state.start_cursor_local.y,
        );
        window.resize(Size::new(px(new_w), px(new_h)));
        cx.notify();
    }

    /// Resize commit (mouse-up): the surface is already at its new size
    /// (resized live during the drag) — persist it so a restart restores it.
    /// No close/reopen needed: size is runtime-settable in this fork.
    fn finalize_resize(&mut self, window: &mut Window, _cx: &mut Context<Self>) {
        if self.resize_state.take().is_none() {
            return;
        }
        let w = clamp_size_w(window.bounds().size.width.as_f32());
        let h = clamp_size_h(window.bounds().size.height.as_f32());
        let mut specs = load_specs();
        let Some(slot) = specs.iter_mut().find(|s| s.id == self.widget_id) else {
            return;
        };
        // No real change (plain click on the corner) → skip the disk write.
        if (slot.width - w).abs() < 0.5 && (slot.height - h).abs() < 0.5 {
            return;
        }
        slot.width = w;
        slot.height = h;
        if let Err(err) = save_specs(&specs) {
            tracing::warn!("desktop_terminal: resize-save failed: {err}");
        }
        tracing::info!(
            "desktop_terminal: widget {} resized to {}×{}",
            self.widget_id,
            w,
            h
        );
    }
}

impl Render for DesktopTerminalView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = Theme::global(cx);
        let editing = edit_mode::is_active(cx);
        let bg = theme.bg.primary;
        let fg = theme.text.primary;
        let border = theme.border.default;
        let muted = theme.text.muted;
        let cursor_bg = theme.accent.primary;
        // Edit-mode frame: accent border so the affordance is unmistakable.
        let frame_border = if editing {
            theme.accent.primary
        } else {
            border
        };

        let lines = self.lines.clone();
        let cursor_row = self.cursor_row;
        let cursor_col = self.cursor_col;
        let show_cursor = self.show_cursor;
        let focus = self.focus.clone();
        let error = self.error.clone();
        let id = self.widget_id.clone();

        // ── edit-mode handlers (built before the element tree — Rust 2024
        // RPIT capture rules, same as the side panels) ─────────────────────
        let close_handler = cx.listener(|this, _e: &gpui::ClickEvent, window, cx| {
            close_one_in_window(&this.widget_id, window, cx);
        });
        let drag_finalize_in = cx.listener(|this, ev: &MouseUpEvent, window, cx| {
            this.finalize_drag(ev.position, window, cx);
        });
        let drag_finalize_out = cx.listener(|this, ev: &MouseUpEvent, window, cx| {
            this.finalize_drag(ev.position, window, cx);
        });
        let resize_mouse_down = cx.listener(|this, ev: &gpui::MouseDownEvent, window, cx| {
            this.begin_resize(ev.position, window, cx);
        });
        let resize_drag_move = cx.listener(
            |this, ev: &DragMoveEvent<WidgetResize>, window, cx| {
                this.update_resize(ev.event.position, window, cx);
            },
        );
        let resize_finalize_in = cx.listener(|this, _ev: &MouseUpEvent, window, cx| {
            this.finalize_resize(window, cx);
        });
        let resize_finalize_out = cx.listener(|this, _ev: &MouseUpEvent, window, cx| {
            this.finalize_resize(window, cx);
        });

        // Drag ghost payload — fixed at drag start so the preview mirrors the
        // widget's current size. `window.bounds()` is read here (render),
        // not in the drag constructor, because the constructor only receives
        // the grab offset.
        let this_for_drag = cx.entity();
        let ghost_size = (
            window.bounds().size.width.as_f32(),
            window.bounds().size.height.as_f32(),
        );
        let ghost_accent = theme.accent.primary;
        let title_id = format!("dt-title-{id}");
        let close_id = format!("dt-close-{id}");
        let resize_id = format!("dt-resize-{id}");

        div()
            .track_focus(&self.focus)
            .id("desktop-terminal")
            .window_font(theme)
            .size_full()
            // `relative` so the absolute resize handle can sit on a corner.
            .relative()
            .flex()
            .flex_col()
            // T266: the terminal widget plate follows surface alpha; the
            // title strip stays opaque (its own fill, unchanged).
            .bg(theme.surface_color(bg))
            .border_1()
            .border_color(frame_border)
            .rounded(theme.radius)
            .overflow_hidden()
            .on_mouse_down(MouseButton::Left, {
                let focus = focus.clone();
                move |_ev, window, cx| {
                    focus.focus(window, cx);
                }
            })
            .on_key_down(cx.listener(|this, event, window, cx| {
                this.handle_key(event, window, cx);
            }))
            .child(
                div()
                    .id(title_id)
                    .h(px(22.))
                    .px(px(8.))
                    .flex()
                    .items_center()
                    .gap(px(4.))
                    .bg(theme.bg.elevated)
                    // T259: in edit mode the whole title strip is the drag
                    // handle. The ghost (fork `active_drag` preview) follows
                    // the cursor grab-anchored; the window itself only
                    // teleports on mouse-up (fork has no runtime reposition).
                    .when(editing, |el| {
                        let this = this_for_drag.clone();
                        el.cursor_move()
                            .on_drag(
                                WidgetDrag,
                                move |_v: &WidgetDrag, offset: Point<Pixels>, window, cx| {
                                    this.update(cx, |this, cx| {
                                        this.begin_drag(window.bounds().origin + offset, cx);
                                    });
                                    cx.new(|_| TerminalDragGhost {
                                        width: ghost_size.0,
                                        height: ghost_size.1,
                                        accent: ghost_accent,
                                    })
                                },
                            )
                            .on_mouse_up(MouseButton::Left, drag_finalize_in)
                            .on_mouse_up_out(MouseButton::Left, drag_finalize_out)
                    })
                    .child(
                        div()
                            .flex_1()
                            .min_w(px(0.))
                            .text_color(muted)
                            .text_size(px(11.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(format!("desktop-terminal · {id}")),
                    )
                    // T259: close ✕ (edit mode only) — kills the PTY, drops
                    // the spec from `desktop_terminal.toml`, closes the
                    // window. Distinct from the T256 header finding: that is
                    // the right panel's fake close, unrelated.
                    .when(editing, |el| {
                        el.child(
                            div()
                                .id(close_id)
                                .w(px(16.))
                                .h(px(16.))
                                .rounded(px(4.))
                                .flex()
                                .items_center()
                                .justify_center()
                                .text_size(px(10.5))
                                .text_color(muted)
                                .hover(|s| {
                                    s.bg(theme.status.error)
                                        .text_color(theme.text.primary)
                                })
                                .on_click(close_handler)
                                .child("✕"),
                        )
                    }),
            )
            .child({
                let mut grid = div()
                    .flex_1()
                    .min_h(px(0.))
                    .px(px(6.))
                    .py(px(4.))
                    .flex()
                    .flex_col()
                    .font_family(theme.font_mono)
                    .text_size(px(FONT_SIZE))
                    .text_color(fg);
                if let Some(err) = error {
                    grid = grid.child(
                        div()
                            .px(px(10.))
                            .py(px(16.))
                            .text_size(px(12.))
                            .text_color(theme.status.error)
                            .child(err),
                    );
                } else {
                    grid = grid.children(
                        lines.into_iter().enumerate().map(move |(row, line)| {
                            let is_cursor_line = show_cursor && row == cursor_row;
                            let line_for_cursor = line.clone();
                            div()
                                .id(SharedString::from(format!("dt-row-{row}")))
                                .h(px(CELL_H))
                                .flex()
                                .items_center()
                                .whitespace_nowrap()
                                .overflow_hidden()
                                .when(is_cursor_line, {
                                    let cursor_col = cursor_col;
                                    let cursor_bg = cursor_bg;
                                    let fg = fg;
                                    move |el| {
                                        // Split line so the cursor cell can be highlighted.
                                        let chars: Vec<char> = line_for_cursor.chars().collect();
                                        let before: String = chars.iter().take(cursor_col).collect();
                                        let at = chars.get(cursor_col).copied().unwrap_or(' ');
                                        let after: String =
                                            chars.iter().skip(cursor_col + 1).collect();
                                        el.child(
                                            div()
                                                .flex()
                                                .flex_row()
                                                .child(before)
                                                .child(
                                                    div()
                                                        .bg(cursor_bg)
                                                        .text_color(fg)
                                                        .child(at.to_string()),
                                                )
                                                .child(after),
                                        )
                                    }
                                })
                                .when(!is_cursor_line, |el| el.child(line))
                        }),
                    );
                }
                grid
            })
            // T259: bottom-right resize handle (edit mode only) — last child
            // so it paints and hit-tests on top of the terminal grid. Live
            // `window.resize()` during the drag (the one runtime geometry
            // op this fork supports), config persisted on release.
            .when(editing, |el| {
                el.child(
                    div()
                        .id(resize_id)
                        .absolute()
                        .right(px(0.))
                        .bottom(px(0.))
                        .w(px(16.))
                        .h(px(16.))
                        .cursor_nwse_resize()
                        .on_mouse_down(MouseButton::Left, resize_mouse_down)
                        .on_drag(WidgetResize, |_, _, _, cx| cx.new(|_| gpui::EmptyView))
                        .on_drag_move(resize_drag_move)
                        .on_mouse_up(MouseButton::Left, resize_finalize_in)
                        .on_mouse_up_out(MouseButton::Left, resize_finalize_out)
                        .child(
                            // Subtle corner bracket so the invisible hit area
                            // reads as a resize grip.
                            div()
                                .absolute()
                                .right(px(3.))
                                .bottom(px(3.))
                                .w(px(10.))
                                .h(px(10.))
                                .border_t_2()
                                .border_l_2()
                                .border_color(theme.accent.primary.opacity(0.6)),
                        ),
                )
            })
    }
}

impl Focusable for DesktopTerminalView {
    fn focus_handle(&self, _cx: &App) -> gpui::FocusHandle {
        self.focus.clone()
    }
}

/// Acquire the shared engine for `widget_id` from the PTY registry. `cfg(test)`
/// returns an error so unit tests never raise a real shell — the laziness
/// contract is verified live (pgrep), not in tests (T177 report).
fn spawn_terminal(widget_id: String, cx: &App) -> Result<TerminalHandle, SharedString> {
    #[cfg(test)]
    {
        // Keep the geometry constants referenced in test builds.
        let _ = (TermSize::new(COLS, ROWS), CELL_W, CELL_H);
        let _ = cx; // registry is unavailable in unit tests; we never use it.
        Err(SharedString::from("PTY disabled in unit tests"))
    }
    #[cfg(not(test))]
    {
        let global: &TerminalRegistryGlobal = registry(cx);
        let mut reg = global.registry.lock().expect("registry lock");
        reg.get_or_spawn(
            &widget_id,
            TermSize::new(COLS, ROWS),
            CELL_W,
            CELL_H,
        )
        .map_err(|e| SharedString::from(format!("PTY error: {e:#}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resize_floors_at_minimum() {
        assert_eq!(clamp_size_w(0.0), MIN_WIDTH, "zero width → min");
        assert_eq!(clamp_size_w(-50.0), MIN_WIDTH, "negative width → min");
        assert_eq!(clamp_size_w(MIN_WIDTH), MIN_WIDTH, "at min stays");
        assert_eq!(clamp_size_w(900.0), 900.0, "no ceiling");

        assert_eq!(clamp_size_h(0.0), MIN_HEIGHT, "zero height → min");
        assert_eq!(clamp_size_h(-10.0), MIN_HEIGHT, "negative height → min");
        assert_eq!(clamp_size_h(MIN_HEIGHT), MIN_HEIGHT);
        assert_eq!(clamp_size_h(1000.0), 1000.0);
    }

    #[test]
    fn min_constants_leave_room_for_a_few_terminal_rows() {
        // MIN_HEIGHT must clear the 22px title strip and still fit several
        // 16px grid rows (T259 §2: "a few lines/columns").
        let grid_room = MIN_HEIGHT - 22.0;
        assert!(
            grid_room / CELL_H >= 10.0,
            "min height must fit ≥10 grid rows (got {})",
            grid_room / CELL_H
        );
        assert!(MIN_WIDTH / CELL_W >= 40.0, "min width must fit ≥40 columns");
    }
}
