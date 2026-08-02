//! Terminal tab — embedded PTY/VT100 session in the right panel (spec §4.1).
//!
//! Engine: `chronos_services::terminal`, shared with the desktop-terminal
//! wallpaper widget (T177). This view owns the lazy PTY spawn on first
//! activation, dynamic columns (the grid follows the panel width), keyboard
//! → PTY, and the honest Running / Failed / Exited states (§13).

use std::time::Duration;

use chronos_services::terminal::{TermSize, Terminal, compute_grid};
use chronos_ui::Theme;
use gpui::{
    App, Context, Focusable, FontWeight, InteractiveElement, IntoElement, KeyDownEvent, MouseButton,
    Render, SharedString, Window, div, prelude::*, px, svg,
};

use crate::side_panel_right::{HANDLE_WIDTH, RAIL_WIDTH, SidePanelRightState};

/// Tab font and cell geometry (same cell ratio as the desktop spike).
const FONT_SIZE: f32 = 13.;
const CELL_H: f32 = 16.;
/// Fallback advance width until the text system measurement lands.
const CELL_W_FALLBACK: f32 = FONT_SIZE * 0.6;
/// Grid area padding inside the tab.
const PAD_X: f32 = 10.;
const PAD_Y: f32 = 6.;
/// Header bar height.
const HEADER_H: f32 = 26.;
/// Poll interval for PTY drain.
const POLL_MS: u64 = 16;
/// Transient launch grid — the first render reconciles it to the real panel.
const LAUNCH_COLS: usize = 80;
const LAUNCH_ROWS: usize = 24;

/// Honest lifecycle (§13): a failed spawn and an exited shell are shown,
/// never a blank rectangle.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Status {
    Running,
    /// PTY did not start — the exact error is displayed.
    Failed(SharedString),
    /// Shell exited (PTY EOF) — the last screen stays, dimmed, with a hint.
    Exited,
}

pub struct TerminalTab {
    focus: gpui::FocusHandle,
    terminal: Option<Terminal>,
    status: Status,
    /// Measured advance width of one mono cell (text system).
    cell_w: f32,
    /// `true` once the mono advance was measured — the guard is a flag, not
    /// a value compare: the measured f32 equals `CELL_W_FALLBACK` exactly
    /// (13 × 0.6 in f32), so `cell_w == CELL_W_FALLBACK` would re-measure
    /// on every render.
    cell_measured: bool,
    /// Last applied grid — geometry is reconciled only on change.
    grid: TermSize,
    /// Cached grid text for render.
    lines: Vec<SharedString>,
    cursor_col: usize,
    cursor_row: usize,
    show_cursor: bool,
    /// Explicit body height in px, overriding the window-based calculation
    /// (T194b). `None` (default) — full right-panel body, sized against
    /// the window like before. `Some(h)` — a host embedding this tab in a
    /// constrained area (the Editor terminal drawer) supplies its own
    /// available height every render so the PTY grid matches the drawer's
    /// real box instead of the whole window.
    available_height_override: Option<f32>,
}

impl TerminalTab {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let mut this = Self {
            focus: cx.focus_handle(),
            terminal: None,
            status: Status::Failed(SharedString::from("")),
            cell_w: CELL_W_FALLBACK,
            cell_measured: false,
            grid: TermSize::new(LAUNCH_COLS, LAUNCH_ROWS),
            lines: Vec::new(),
            cursor_col: 0,
            cursor_row: 0,
            show_cursor: false,
            available_height_override: None,
        };
        this.launch(cx);
        this
    }

    /// Set (or clear) the explicit body height override — see the field
    /// doc. Cheap: just a compare-and-store, no I/O. The caller's own
    /// render loop drives the cadence (T194b: the drawer host calls this
    /// once before rendering the terminal child each frame).
    pub fn set_available_height(&mut self, height: Option<f32>) {
        self.available_height_override = height;
    }

    /// Lazily spawn the shell: the tab view is created on first activation
    /// (`TabContent::create`), so no tab open → no PTY → no process.
    fn launch(&mut self, cx: &mut Context<Self>) {
        match spawn_terminal() {
            Ok(term) => {
                tracing::info!(
                    pid = term.child_pid(),
                    "side_panel_right terminal: shell spawned (lazy — tab opened)"
                );
                self.terminal = Some(term);
                self.status = Status::Running;
                self.refresh_from_terminal();
                self.start_poll_loop(cx);
            }
            Err(msg) => {
                self.terminal = None;
                self.status = Status::Failed(msg);
            }
        }
    }

    /// Re-spawn the shell after it exited (the Exited state's restart chip).
    fn restart(&mut self, cx: &mut Context<Self>) {
        self.launch(cx);
        cx.notify();
    }

    fn start_poll_loop(&self, cx: &mut Context<Self>) {
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(POLL_MS))
                    .await;
                let cont = this
                    .update(cx, |view, cx| {
                        let Some(term) = view.terminal.as_mut() else {
                            return false;
                        };
                        let dirty = term.drain();
                        let alive = term.is_alive();
                        // `term` borrow ends here.
                        if dirty {
                            view.refresh_from_terminal();
                        }
                        if !alive && matches!(view.status, Status::Running) {
                            view.status = Status::Exited;
                            view.show_cursor = false;
                            tracing::info!("side_panel_right terminal: shell exited (PTY EOF)");
                        }
                        if dirty || matches!(view.status, Status::Exited) {
                            cx.notify();
                        }
                        // Stop polling once the shell exited — the last
                        // frame stays on screen under the Exited banner.
                        !matches!(view.status, Status::Exited)
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
    fn refresh_from_terminal(&mut self) {
        let Some(term) = self.terminal.as_ref() else {
            return;
        };
        let snap = term.snapshot();
        self.lines = snap.lines.into_iter().map(SharedString::from).collect();
        self.cursor_row = snap.cursor_row;
        self.cursor_col = snap.cursor_col;
        self.show_cursor = snap.show_cursor && matches!(self.status, Status::Running);
    }

    /// Measure the mono cell width once via the text system, then reconcile
    /// the grid to the current panel size. Runs from `render` — idempotent
    /// when nothing changed (the reconcile is a cheap compare).
    fn reconcile_geometry(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.cell_measured {
            self.measure_cell_width(window, cx);
            self.cell_measured = true;
        }
        let panel_width = cx.global::<SidePanelRightState>().width;
        let avail_w = (panel_width - RAIL_WIDTH - HANDLE_WIDTH - 2.0 * PAD_X).max(0.0);
        // T194b: a host embedding this tab in a constrained box (the Editor
        // terminal drawer) supplies its own total box height in place of
        // the window height. The header row is still rendered inside that
        // box either way, so the same `- HEADER_H` subtraction applies to
        // both the standalone-tab and embedded-drawer cases.
        let total_h = self
            .available_height_override
            .unwrap_or_else(|| window.bounds().size.height.as_f32());
        let avail_h = (total_h - HEADER_H - 2.0 * PAD_Y).max(0.0);
        let target = compute_grid(avail_w, avail_h, self.cell_w, CELL_H);
        if target == self.grid {
            return;
        }
        self.grid = target;
        if let Some(term) = self.terminal.as_mut()
            && let Err(err) = term.resize(target, self.cell_w, CELL_H)
        {
            tracing::warn!("side_panel_right terminal: resize failed: {err}");
        }
        self.refresh_from_terminal();
        tracing::info!(
            cols = target.cols,
            rows = target.rows,
            cell_w = self.cell_w,
            "side_panel_right terminal: grid reconciled"
        );
    }

    fn measure_cell_width(&mut self, window: &mut Window, cx: &Context<Self>) {
        let theme = *Theme::global(cx);
        // Shape one mono glyph through the public text-system API and read
        // its advance width (the fork keeps `TextSystem::font_id` private,
        // so `shape_line` is the public measurement route).
        let ts = window.text_system();
        let line = ts.shape_line(
            SharedString::from("m"),
            px(FONT_SIZE),
            &[gpui::TextRun {
                len: 1,
                font: gpui::font(theme.font_mono),
                color: gpui::hsla(0., 0., 0., 1.),
                background_color: None,
                underline: None,
                strikethrough: None,
            }],
            None,
        );
        self.cell_w = line.width().as_f32();
        tracing::info!(
            cell_w = self.cell_w,
            eighty_cols_px = self.cell_w * 80.0,
            font = theme.font_mono,
            "side_panel_right terminal: measured mono cell advance"
        );
    }

    fn write_pty(&self, bytes: &[u8]) {
        if let Some(term) = self.terminal.as_ref() {
            term.write(bytes);
        }
    }

    fn handle_key(&mut self, event: &KeyDownEvent, _window: &mut Window, cx: &mut Context<Self>) {
        if !matches!(self.status, Status::Running) {
            return;
        }
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
        // Immediate drain after input helps the prompt feel snappy (bulk
        // output still arrives on the poll loop).
        if let Some(term) = self.terminal.as_mut()
            && term.drain()
        {
            self.refresh_from_terminal();
            cx.notify();
        }
    }
}

impl Render for TerminalTab {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = *Theme::global(cx);
        self.reconcile_geometry(window, cx);

        let fg = theme.text.primary;
        let muted = theme.text.muted;
        let cursor_bg = theme.accent.primary;

        // --- Header: title + shell + input hint --------------------------
        let header = div()
            .id("terminal-header")
            .h(px(HEADER_H))
            .px(px(10.))
            .flex()
            .items_center()
            .justify_between()
            .bg(theme.bg.elevated)
            .border_b_1()
            .border_color(theme.border.subtle)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .child(
                        svg()
                            .path("icons/rail-terminal.svg")
                            .size(px(12.))
                            .text_color(theme.accent.primary),
                    )
                    .child(
                        div()
                            .text_size(px(11.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(theme.text.primary)
                            .child("Terminal"),
                    )
                    .child(
                        div()
                            .text_size(px(10.5))
                            .text_color(muted)
                            .child(shell_name()),
                    ),
            )
            .child(
                div()
                    .text_size(px(10.5))
                    .text_color(muted)
                    .child("click to type"),
            );

        // --- Body ---------------------------------------------------------
        let lines = self.lines.clone();
        let cursor_row = self.cursor_row;
        let cursor_col = self.cursor_col;
        let show_cursor = self.show_cursor;

        let grid = div()
            .id("terminal-grid")
            .flex_1()
            .min_h(px(0.))
            .px(px(PAD_X))
            .py(px(PAD_Y))
            .flex()
            .flex_col()
            .font_family(theme.font_mono)
            .text_size(px(FONT_SIZE))
            .text_color(fg)
            .children(lines.into_iter().enumerate().map(move |(row, line)| {
                let is_cursor_line = show_cursor && row == cursor_row;
                let line_for_cursor = line.clone();
                div()
                    .id(SharedString::from(format!("term-row-{row}")))
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
                            let after: String = chars.iter().skip(cursor_col + 1).collect();
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
            }));

        let body = match &self.status {
            Status::Running => grid.into_any_element(),
            Status::Exited => {
                // Last screen stays visible but dimmed; the banner tells the
                // truth instead of a black rectangle (§13).
                let restart_click = cx.listener(|this, _e, _w, cx| {
                    this.restart(cx);
                });
                div()
                    .flex_1()
                    .min_h(px(0.))
                    .flex()
                    .flex_col()
                    .child(grid.opacity(0.45))
                    .child(
                        div()
                            .id("terminal-exited-banner")
                            .h(px(30.))
                            .px(px(10.))
                            .flex()
                            .items_center()
                            .gap(px(10.))
                            .bg(theme.bg.elevated)
                            .border_t_1()
                            .border_color(theme.border.subtle)
                            .child(
                                div()
                                    .flex_1()
                                    .text_size(px(11.))
                                    .text_color(theme.text.muted)
                                    .child("Shell exited — process finished"),
                            )
                            .child(
                                div()
                                    .id("terminal-restart")
                                    .px(px(10.))
                                    .py(px(3.))
                                    .rounded_md()
                                    .cursor_pointer()
                                    .hover(|s| s.bg(theme.interactive.hover))
                                    .on_click(restart_click)
                                    .child(
                                        div()
                                            .text_size(px(11.))
                                            .text_color(theme.accent.primary)
                                            .child("restart"),
                                    ),
                            ),
                    )
                    .into_any_element()
            }
            Status::Failed(msg) => {
                let msg = msg.clone();
                div()
                    .id("terminal-failed")
                    .flex_1()
                    .min_h(px(0.))
                    .flex()
                    .flex_col()
                    .items_center()
                    .justify_center()
                    .gap(px(10.))
                    .px(px(16.))
                    .child(
                        svg()
                            .path("icons/rail-terminal.svg")
                            .size(px(32.))
                            .text_color(muted.opacity(0.55)),
                    )
                    .child(
                        div()
                            .text_size(px(12.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(theme.text.primary)
                            .child("Terminal is unavailable"),
                    )
                    .child(
                        div()
                            .text_size(px(11.))
                            .text_color(theme.status.error)
                            .text_center()
                            .child(msg),
                    )
                    .into_any_element()
            }
        };

        div()
            .track_focus(&self.focus)
            .id("terminal-tab-root")
            .size_full()
            .flex()
            .flex_col()
            .overflow_hidden()
            .on_mouse_down(MouseButton::Left, {
                let focus = self.focus.clone();
                move |_ev, window, cx| {
                    focus.focus(window, cx);
                }
            })
            .on_key_down(cx.listener(|this, event, window, cx| {
                this.handle_key(event, window, cx);
            }))
            .child(header)
            .child(body)
    }
}

impl Focusable for TerminalTab {
    fn focus_handle(&self, _cx: &App) -> gpui::FocusHandle {
        self.focus.clone()
    }
}

/// The shell the engine launches ($SHELL, or /bin/sh fallback) — header text.
fn shell_name() -> String {
    std::env::var("SHELL")
        .ok()
        .and_then(|s| {
            s.rsplit('/').next().map(|base| base.to_string())
        })
        .unwrap_or_else(|| "sh".to_string())
}

/// Spawn the shared engine. `cfg(test)` returns an error so unit tests never
/// raise a real shell — tab laziness is verified live via pgrep (T177
/// report), not in tests.
fn spawn_terminal() -> Result<Terminal, SharedString> {
    #[cfg(test)]
    {
        // Keep the geometry constants referenced in test builds.
        let _ = (TermSize::new(LAUNCH_COLS, LAUNCH_ROWS), CELL_W_FALLBACK, CELL_H);
        Err(SharedString::from("PTY disabled in unit tests"))
    }
    #[cfg(not(test))]
    {
        Terminal::launch(TermSize::new(LAUNCH_COLS, LAUNCH_ROWS), CELL_W_FALLBACK, CELL_H)
            .map_err(|e| SharedString::from(format!("PTY error: {e:#}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::TestAppContext;

    fn install_globals(cx: &mut TestAppContext) {
        cx.update(|cx| {
            cx.set_global(Theme::default());
            cx.set_global(SidePanelRightState::default());
        });
    }

    #[gpui::test]
    fn terminal_tab_never_raises_shell_in_tests(cx: &mut TestAppContext) {
        install_globals(cx);
        let view = cx.new(TerminalTab::new);
        cx.update_entity(&view, |this, _cx| {
            assert!(
                matches!(this.status, Status::Failed(_)),
                "unit tests must land in the honest Failed state, got {:?}",
                this.status
            );
            assert!(this.terminal.is_none(), "no PTY may be spawned in tests");
        });
    }

    #[gpui::test]
    fn restart_from_failed_keeps_honest_state(cx: &mut TestAppContext) {
        install_globals(cx);
        let view = cx.new(TerminalTab::new);
        cx.update_entity(&view, |this, cx| {
            this.restart(cx);
            // Still Failed in tests — restart must not silently succeed
            // without a shell.
            assert!(matches!(this.status, Status::Failed(_)));
            assert!(this.terminal.is_none());
        });
    }

    #[test]
    fn fallback_cell_width_is_reasonable() {
        // 13 px mono ≈ 7.8 px per cell (DejaVu Sans Mono advance is 1229/2048
        // of the em square). The measured value replaces this at first render.
        assert!(
            (7.5..8.1).contains(&CELL_W_FALLBACK),
            "fallback cell width {CELL_W_FALLBACK} out of expected band"
        );
    }

    #[test]
    fn shell_name_never_empty() {
        assert!(!shell_name().is_empty());
    }
}
