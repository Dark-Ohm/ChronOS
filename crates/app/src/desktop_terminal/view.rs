//! PTY session + VT100 grid view for the desktop terminal spike.
//!
//! The engine (PTY spawn, VT100 grid, snapshots, sizing) lives in
//! `chronos_services::terminal` (T177) — shared with the side-panel
//! Terminal tab. This file only owns the view, its poll loop and the fixed
//! spike geometry (background surface, not resizable).

use std::time::Duration;

use chronos_services::terminal::{TermSize, Terminal};
use chronos_ui::Theme;
use gpui::{
    App, Context, Focusable, FontWeight, InteractiveElement, KeyDownEvent, MouseButton, Render,
    SharedString, Window, div, prelude::*, px,
};

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

/// Desktop terminal view: grid text + keyboard → PTY.
pub struct DesktopTerminalView {
    focus: gpui::FocusHandle,
    terminal: Option<Terminal>,
    /// Honest error when the PTY could not be spawned (§13).
    error: Option<SharedString>,
    /// Cached line strings for render (rebuilt on PTY data).
    lines: Vec<SharedString>,
    cursor_col: usize,
    cursor_row: usize,
    show_cursor: bool,
}

impl DesktopTerminalView {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let mut view = Self {
            focus: cx.focus_handle(),
            terminal: None,
            error: None,
            lines: vec![SharedString::from(""); ROWS],
            cursor_col: 0,
            cursor_row: 0,
            show_cursor: false,
        };
        view.launch(cx);
        view
    }

    /// Spawn the shared engine. `cfg(test)` skips the launch so unit tests
    /// never raise a real shell (the spike's live smoke covers the real path).
    fn launch(&mut self, cx: &mut Context<Self>) {
        match spawn_terminal() {
            Ok(term) => {
                self.terminal = Some(term);
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
                        let Some(term) = view.terminal.as_mut() else {
                            return false;
                        };
                        let dirty = term.drain();
                        let alive = term.is_alive();
                        // `term` borrow ends here.
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
        let snap = term.snapshot();
        self.lines = snap.lines.into_iter().map(SharedString::from).collect();
        self.cursor_row = snap.cursor_row;
        self.cursor_col = snap.cursor_col;
        self.show_cursor = snap.show_cursor;
    }

    fn write_pty(&self, bytes: &[u8]) {
        if let Some(term) = self.terminal.as_ref() {
            term.write(bytes);
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
            && term.drain()
        {
            self.refresh();
            cx.notify();
        }
    }
}

impl Render for DesktopTerminalView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = Theme::global(cx);
        let bg = theme.bg.primary;
        let fg = theme.text.primary;
        let border = theme.border.default;
        let muted = theme.text.muted;
        let cursor_bg = theme.accent.primary;

        let lines = self.lines.clone();
        let cursor_row = self.cursor_row;
        let cursor_col = self.cursor_col;
        let show_cursor = self.show_cursor;
        let focus = self.focus.clone();
        let error = self.error.clone();

        div()
            .track_focus(&self.focus)
            .id("desktop-terminal")
            .size_full()
            .flex()
            .flex_col()
            .bg(bg)
            .border_1()
            .border_color(border)
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
                    .h(px(22.))
                    .px(px(8.))
                    .flex()
                    .items_center()
                    .bg(theme.bg.elevated)
                    .child(
                        div()
                            .text_color(muted)
                            .text_size(px(11.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child("desktop-terminal (spike)"),
                    ),
            )
            .child({
                let mut grid = div()
                    .flex_1()
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
    }
}

impl Focusable for DesktopTerminalView {
    fn focus_handle(&self, _cx: &App) -> gpui::FocusHandle {
        self.focus.clone()
    }
}

/// Spawn the shared engine. `cfg(test)` returns an error so unit tests never
/// raise a real shell — the laziness contract is verified live (pgrep), not
/// in tests (T177 report).
fn spawn_terminal() -> Result<Terminal, SharedString> {
    #[cfg(test)]
    {
        // Keep the geometry constants referenced in test builds.
        let _ = (TermSize::new(COLS, ROWS), CELL_W, CELL_H);
        Err(SharedString::from("PTY disabled in unit tests"))
    }
    #[cfg(not(test))]
    {
        Terminal::launch(TermSize::new(COLS, ROWS), CELL_W, CELL_H)
            .map_err(|e| SharedString::from(format!("PTY error: {e:#}")))
    }
}
