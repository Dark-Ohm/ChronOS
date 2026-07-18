//! PTY session + VT100 grid view for the desktop terminal spike.

use std::io::{Read, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use alacritty_terminal::event::VoidListener;
use alacritty_terminal::grid::Dimensions;
use alacritty_terminal::index::{Column, Line};
use alacritty_terminal::term::cell::Flags;
use alacritty_terminal::term::{Config, Term};
use alacritty_terminal::vte::ansi::Processor;
use gpui::{
    App, Context, Focusable, FontWeight, InteractiveElement, KeyDownEvent, MouseButton, Render,
    SharedString, Window, div, prelude::*, px,
};
use portable_pty::{CommandBuilder, MasterPty, NativePtySystem, PtySize, PtySystem};

use chronos_ui::Theme;

/// Grid geometry for the spike (matches ~600×400 at ~7.5×16 cell).
const COLS: usize = 80;
const ROWS: usize = 24;
const CELL_H: f32 = 16.;
const FONT_SIZE: f32 = 13.;
/// How often the UI drains PTY bytes and repaints.
const POLL_MS: u64 = 16;

/// Terminal dimensions for `alacritty_terminal::Term`.
#[derive(Clone, Copy)]
struct TermSize {
    cols: usize,
    rows: usize,
}

impl Dimensions for TermSize {
    fn total_lines(&self) -> usize {
        self.rows
    }

    fn screen_lines(&self) -> usize {
        self.rows
    }

    fn columns(&self) -> usize {
        self.cols
    }
}

/// Owns the PTY child so Drop kills the shell when the view is dropped.
struct PtySession {
    /// Keep master open for the life of the session.
    _master: Box<dyn MasterPty + Send>,
    /// Keep child alive; dropping would SIGKILL the shell.
    _child: Box<dyn portable_pty::Child + Send + Sync>,
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
}

/// Desktop terminal view: grid text + keyboard → PTY.
pub struct DesktopTerminalView {
    focus: gpui::FocusHandle,
    term: Arc<Mutex<Term<VoidListener>>>,
    parser: Processor,
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
    rx: Receiver<Vec<u8>>,
    /// Kept so the child process is not killed while the view lives.
    _session: Arc<PtySession>,
    /// Cached line strings for render (rebuilt on PTY data).
    lines: Vec<SharedString>,
    cursor_col: usize,
    cursor_row: usize,
    show_cursor: bool,
    alive: Arc<AtomicBool>,
}

impl DesktopTerminalView {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let size = TermSize {
            cols: COLS,
            rows: ROWS,
        };
        let term = Term::new(Config::default(), &size, VoidListener);
        let term = Arc::new(Mutex::new(term));

        let (session, rx, alive) = match spawn_pty() {
            Ok(parts) => parts,
            Err(err) => {
                tracing::error!("desktop_terminal: PTY spawn failed: {err:#}");
                // Fall back to a dead session so the window still opens (visible error).
                return Self::dead(cx, format!("PTY error: {err:#}"));
            }
        };

        let writer = Arc::clone(&session.writer);
        let focus = cx.focus_handle();

        let mut view = Self {
            focus,
            term,
            parser: Processor::new(),
            writer,
            rx,
            _session: Arc::clone(&session),
            lines: vec![SharedString::from(""); ROWS],
            cursor_col: 0,
            cursor_row: 0,
            show_cursor: true,
            alive,
        };
        view.rebuild_lines();
        view.start_poll_loop(cx);
        view
    }

    /// Window still opens if PTY fails — shows the error string.
    fn dead(cx: &mut Context<Self>, msg: String) -> Self {
        let size = TermSize {
            cols: COLS,
            rows: ROWS,
        };
        let term = Term::new(Config::default(), &size, VoidListener);
        // Dummy channel that never receives.
        let (_tx, rx) = mpsc::channel();
        let writer = Arc::new(Mutex::new(
            Box::new(std::io::sink()) as Box<dyn Write + Send>
        ));
        let session = Arc::new(PtySession {
            _master: Box::new(DummyMaster),
            _child: Box::new(DummyChild),
            writer: Arc::clone(&writer),
        });
        Self {
            focus: cx.focus_handle(),
            term: Arc::new(Mutex::new(term)),
            parser: Processor::new(),
            writer,
            rx,
            _session: session,
            lines: vec![SharedString::from(msg)],
            cursor_col: 0,
            cursor_row: 0,
            show_cursor: false,
            alive: Arc::new(AtomicBool::new(false)),
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
                        if !view.alive.load(Ordering::Relaxed) {
                            return false;
                        }
                        if view.drain_bytes() {
                            cx.notify();
                        }
                        true
                    })
                    .unwrap_or(false);
                if !cont {
                    break;
                }
            }
        })
        .detach();
    }

    /// Drain pending PTY bytes into the VT parser. Returns true if UI should repaint.
    fn drain_bytes(&mut self) -> bool {
        let mut dirty = false;
        loop {
            match self.rx.try_recv() {
                Ok(buf) => {
                    if let Ok(mut term) = self.term.lock() {
                        self.parser.advance(&mut *term, &buf);
                        dirty = true;
                    }
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    self.alive.store(false, Ordering::Relaxed);
                    dirty = true;
                    break;
                }
            }
        }
        if dirty {
            self.rebuild_lines();
        }
        dirty
    }

    fn rebuild_lines(&mut self) {
        let Ok(term) = self.term.lock() else {
            return;
        };
        let grid = term.grid();
        let display_offset = grid.display_offset();
        let cols = grid.columns();
        let rows = grid.screen_lines();

        let mut lines = Vec::with_capacity(rows);
        let mut spike_seen = false;
        for row in 0..rows {
            let line = Line(-(display_offset as i32) + row as i32);
            let mut s = String::with_capacity(cols);
            for col in 0..cols {
                let cell = &grid[line][Column(col)];
                if cell.flags.contains(Flags::WIDE_CHAR_SPACER) {
                    continue;
                }
                s.push(cell.c);
            }
            // Keep trailing spaces for cursor alignment; trim only visual noise at end of empty lines.
            let trimmed = s.trim_end_matches(' ');
            if trimmed.contains("__CHRONOS_DT_SPIKE_OK__") {
                spike_seen = true;
            }
            if trimmed.is_empty() {
                lines.push(SharedString::from(" "));
            } else {
                lines.push(SharedString::from(trimmed.to_owned()));
            }
        }

        if spike_seen {
            // KEY acceptance criterion: shell I/O reached the VT grid (logged once-ish).
            tracing::info!(
                lines = ?lines.iter().map(|l| l.to_string()).filter(|l| !l.trim().is_empty()).collect::<Vec<_>>(),
                "desktop_terminal: SPIKE_OK visible in VT grid"
            );
        }

        let content = term.renderable_content();
        let cursor = content.cursor;
        let cursor_line = cursor.point.line.0 + display_offset as i32;
        self.cursor_row = if cursor_line >= 0 {
            cursor_line as usize
        } else {
            0
        };
        self.cursor_col = cursor.point.column.0;
        self.show_cursor = !matches!(
            cursor.shape,
            alacritty_terminal::vte::ansi::CursorShape::Hidden
        );
        self.lines = lines;
    }

    fn write_pty(&self, bytes: &[u8]) {
        let Ok(mut w) = self.writer.lock() else {
            return;
        };
        if let Err(err) = w.write_all(bytes).and_then(|_| w.flush()) {
            tracing::warn!("desktop_terminal: PTY write failed: {err}");
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
                if let Some(ch) = event.keystroke.key_char.as_ref() {
                    if !mods.alt && !mods.platform {
                        self.write_pty(ch.as_bytes());
                    }
                }
            }
        }
        // Immediate drain after input helps prompt feel snappy (still poll-driven for bulk out).
        if self.drain_bytes() {
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
            .child(
                div()
                    .flex_1()
                    .px(px(6.))
                    .py(px(4.))
                    .flex()
                    .flex_col()
                    .font_family("DejaVu Sans Mono")
                    .text_size(px(FONT_SIZE))
                    .text_color(fg)
                    .children(lines.into_iter().enumerate().map(move |(row, line)| {
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
                    })),
            )
    }
}

impl Focusable for DesktopTerminalView {
    fn focus_handle(&self, _cx: &App) -> gpui::FocusHandle {
        self.focus.clone()
    }
}

fn spawn_pty() -> anyhow::Result<(Arc<PtySession>, Receiver<Vec<u8>>, Arc<AtomicBool>)> {
    let pty_system = NativePtySystem::default();
    let pair = pty_system.openpty(PtySize {
        rows: ROWS as u16,
        cols: COLS as u16,
        pixel_width: (COLS as u16).saturating_mul(8),
        pixel_height: (ROWS as u16).saturating_mul(CELL_H as u16),
    })?;

    // Prefer $SHELL; fall back to /bin/sh. Spike deliberately avoids a full
    // login shell (no -l) so p10k/oh-my-zsh noise doesn't drown the smoke.
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".into());
    let mut cmd = CommandBuilder::new(&shell);
    cmd.env("TERM", "xterm-256color");
    cmd.env("COLORTERM", "truecolor");
    // Keep PATH from parent so basic commands resolve; clear fancy prompt hooks
    // that can dump multi-KB on every prompt (p10k instant prompt, etc.).
    cmd.env("ZDOTDIR", "/tmp/chronos-dt-empty-zdot");
    if let Some(home) = dirs::home_dir() {
        cmd.cwd(home);
    }

    // Empty ZDOTDIR so interactive zsh starts bare if SHELL is zsh.
    let _ = std::fs::create_dir_all("/tmp/chronos-dt-empty-zdot");

    let child = pair.slave.spawn_command(cmd)?;
    // Slave end is owned by the child after spawn; drop our handle.
    drop(pair.slave);

    let mut reader = pair.master.try_clone_reader()?;
    let writer = pair.master.take_writer()?;

    let (tx, rx) = mpsc::channel::<Vec<u8>>();
    let alive = Arc::new(AtomicBool::new(true));
    let alive_reader = Arc::clone(&alive);

    thread::Builder::new()
        .name("chronos-desktop-term-pty".into())
        .spawn(move || {
            let mut buf = [0u8; 8192];
            let mut first = true;
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => {
                        tracing::info!("desktop_terminal: PTY EOF");
                        alive_reader.store(false, Ordering::Relaxed);
                        break;
                    }
                    Ok(n) => {
                        if first {
                            first = false;
                            let preview = String::from_utf8_lossy(&buf[..n.min(240)]);
                            tracing::info!(
                                n,
                                preview = %preview.escape_debug(),
                                "desktop_terminal: first PTY chunk"
                            );
                        }
                        if tx.send(buf[..n].to_vec()).is_err() {
                            break;
                        }
                    }
                    Err(err) => {
                        tracing::warn!("desktop_terminal: PTY read error: {err}");
                        alive_reader.store(false, Ordering::Relaxed);
                        break;
                    }
                }
            }
        })?;

    let writer = Arc::new(Mutex::new(writer));
    // Optional self-smoke: `CHRONOS_DT_PROBE=1` writes a known command after
    // the shell settles. Used when ydotool is unavailable; log + VT grid must
    // show `__CHRONOS_DT_SPIKE_OK__`. Default off so the widget is a normal shell.
    if std::env::var_os("CHRONOS_DT_PROBE").is_some() {
        let w = Arc::clone(&writer);
        thread::Builder::new()
            .name("chronos-desktop-term-probe".into())
            .spawn(move || {
                thread::sleep(Duration::from_millis(400));
                if let Ok(mut guard) = w.lock() {
                    let probe = b"echo __CHRONOS_DT_SPIKE_OK__\r";
                    if let Err(err) = guard.write_all(probe).and_then(|_| guard.flush()) {
                        tracing::warn!("desktop_terminal: probe write failed: {err}");
                    } else {
                        tracing::info!("desktop_terminal: probe command written to PTY");
                    }
                }
            })?;
    }

    let session = Arc::new(PtySession {
        _master: pair.master,
        _child: child,
        writer: Arc::clone(&writer),
    });

    tracing::info!(
        cols = COLS,
        rows = ROWS,
        shell = %shell,
        "desktop_terminal: shell spawned on PTY"
    );
    Ok((session, rx, alive))
}

// --- Dead-session stubs (only used when PTY spawn fails) --------------------

struct DummyMaster;

impl MasterPty for DummyMaster {
    fn resize(&self, _size: PtySize) -> anyhow::Result<()> {
        Ok(())
    }
    fn get_size(&self) -> anyhow::Result<PtySize> {
        Ok(PtySize {
            rows: ROWS as u16,
            cols: COLS as u16,
            pixel_width: 0,
            pixel_height: 0,
        })
    }
    fn try_clone_reader(&self) -> anyhow::Result<Box<dyn Read + Send>> {
        Ok(Box::new(std::io::empty()))
    }
    fn take_writer(&self) -> anyhow::Result<Box<dyn Write + Send>> {
        Ok(Box::new(std::io::sink()))
    }
    #[cfg(unix)]
    fn process_group_leader(&self) -> Option<i32> {
        None
    }
    #[cfg(unix)]
    fn as_raw_fd(&self) -> Option<std::os::fd::RawFd> {
        None
    }
    #[cfg(unix)]
    fn tty_name(&self) -> Option<std::path::PathBuf> {
        None
    }
}

#[derive(Debug)]
struct DummyChild;

impl portable_pty::Child for DummyChild {
    fn try_wait(&mut self) -> std::io::Result<Option<portable_pty::ExitStatus>> {
        Ok(None)
    }
    fn wait(&mut self) -> std::io::Result<portable_pty::ExitStatus> {
        Ok(portable_pty::ExitStatus::with_exit_code(0))
    }
    fn process_id(&self) -> Option<u32> {
        None
    }
    #[cfg(windows)]
    fn as_raw_handle(&self) -> Option<std::os::windows::io::RawHandle> {
        None
    }
}

impl portable_pty::ChildKiller for DummyChild {
    fn kill(&mut self) -> std::io::Result<()> {
        Ok(())
    }
    fn clone_killer(&self) -> Box<dyn portable_pty::ChildKiller + Send + Sync> {
        Box::new(DummyChild)
    }
}

/// Snapshot visible rows of a `Term` as plain strings (test + diagnostics).
fn term_visible_lines(term: &Term<VoidListener>) -> Vec<String> {
    let grid = term.grid();
    let display_offset = grid.display_offset();
    let cols = grid.columns();
    let rows = grid.screen_lines();
    let mut lines = Vec::with_capacity(rows);
    for row in 0..rows {
        let line = Line(-(display_offset as i32) + row as i32);
        let mut s = String::with_capacity(cols);
        for col in 0..cols {
            let cell = &grid[line][Column(col)];
            if cell.flags.contains(Flags::WIDE_CHAR_SPACER) {
                continue;
            }
            s.push(cell.c);
        }
        lines.push(s.trim_end_matches(' ').to_owned());
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use alacritty_terminal::event::VoidListener;
    use alacritty_terminal::term::{Config, Term};
    use alacritty_terminal::vte::ansi::Processor;

    #[test]
    fn vt_parser_renders_echo_output() {
        let size = TermSize {
            cols: 40,
            rows: 10,
        };
        let mut term = Term::new(Config::default(), &size, VoidListener);
        let mut parser: Processor = Processor::new();
        // Simulate shell printing a probe line + newline.
        let bytes = b"__CHRONOS_DT_SPIKE_OK__\r\n$ ";
        parser.advance(&mut term, bytes);
        let lines = term_visible_lines(&term);
        let joined = lines.join("\n");
        assert!(
            joined.contains("__CHRONOS_DT_SPIKE_OK__"),
            "expected spike marker in grid, got:\n{joined}"
        );
    }

    #[test]
    fn icon_not_applicable_term_size_minima() {
        // alacritty refuses 0×0; our spike constants must stay legal.
        assert!(COLS >= 2);
        assert!(ROWS >= 1);
    }
}
