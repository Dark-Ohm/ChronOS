//! PTY + VT100 terminal engine, shared by the desktop-terminal wallpaper
//! widget and the side-panel Terminal tab (T177).
//!
//! No GPUI here: this module only talks to `portable_pty` (spawn, resize,
//! I/O) and `alacritty_terminal` (VT100 grid + parsing). Each consumer owns
//! its own view, geometry and poll loop. Extracted from the
//! `desktop_terminal` spike (`crates/app/src/desktop_terminal/view.rs`) so
//! the tab and the wallpaper widget do not fork the PTY code.

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
use portable_pty::{CommandBuilder, MasterPty, NativePtySystem, PtySize, PtySystem};

pub mod registry;

pub mod kitty_theme;

/// Smallest grid alacritty accepts (it refuses a 0-column terminal).
pub const MIN_COLS: usize = 2;
pub const MIN_ROWS: usize = 1;

/// Terminal grid dimensions in cells.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TermSize {
    pub cols: usize,
    pub rows: usize,
}

impl TermSize {
    pub fn new(cols: usize, rows: usize) -> Self {
        Self { cols, rows }
    }

    /// Pixel-equivalent for the kernel pty (`MasterPty::resize`).
    pub fn to_pty_size(self, cell_w: f32, cell_h: f32) -> PtySize {
        PtySize {
            rows: u16::try_from(self.rows).unwrap_or(u16::MAX),
            cols: u16::try_from(self.cols).unwrap_or(u16::MAX),
            pixel_width: u16::try_from((self.cols as f32 * cell_w).round() as u32)
                .unwrap_or(u16::MAX),
            pixel_height: u16::try_from((self.rows as f32 * cell_h).round() as u32)
                .unwrap_or(u16::MAX),
        }
    }
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

/// Grid that fits `avail_w × avail_h` pixels at the given cell size.
///
/// Pure function so the column/row math is unit-testable without any UI.
/// Values are clamped to [`MIN_COLS`] / [`MIN_ROWS`] — the terminal must
/// never show less than a legal alacritty grid.
pub fn compute_grid(avail_w: f32, avail_h: f32, cell_w: f32, cell_h: f32) -> TermSize {
    let cols = if cell_w > 0.0 && avail_w > 0.0 {
        (avail_w / cell_w).floor() as usize
    } else {
        MIN_COLS
    };
    let rows = if cell_h > 0.0 && avail_h > 0.0 {
        (avail_h / cell_h).floor() as usize
    } else {
        MIN_ROWS
    };
    TermSize::new(cols.max(MIN_COLS), rows.max(MIN_ROWS))
}

/// Owns the PTY master + child so the shell lives exactly as long as this
/// session. Dropping it closes the slave fd → SIGHUP kills the shell.
pub struct PtySession {
    master: Box<dyn MasterPty + Send>,
    child: Box<dyn portable_pty::Child + Send + Sync>,
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
}

impl PtySession {
    pub fn writer(&self) -> Arc<Mutex<Box<dyn Write + Send>>> {
        Arc::clone(&self.writer)
    }

    /// Tell the kernel the grid changed (rows/cols + pixel size).
    pub fn resize(&self, size: PtySize) -> anyhow::Result<()> {
        self.master.resize(size)
    }

    /// Shell pid, when the platform exposes it (used for live pgrep checks).
    pub fn child_pid(&self) -> Option<u32> {
        self.child.process_id()
    }
}

/// Visible grid text + cursor, as plain strings (no GPUI types — the engine
/// stays UI-agnostic).
pub struct GridSnapshot {
    pub lines: Vec<String>,
    pub cursor_row: usize,
    pub cursor_col: usize,
    pub show_cursor: bool,
}

/// Full PTY + VT session. Created with [`Terminal::launch`], driven by the
/// consumer's poll loop via [`Terminal::drain`].
pub struct Terminal {
    term: Term<VoidListener>,
    parser: Processor,
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
    rx: Receiver<Vec<u8>>,
    /// Kept so the child process is not killed while the terminal lives.
    _session: Arc<PtySession>,
    /// Cleared on PTY EOF or read error — the honest "shell exited" signal.
    alive: Arc<AtomicBool>,
}

impl Terminal {
    /// Spawn `$SHELL` (or `/bin/sh`) on a fresh PTY and wrap it with a
    /// VT100 grid of `size` cells.
    pub fn launch(size: TermSize, cell_w: f32, cell_h: f32) -> anyhow::Result<Self> {
        let pty_system = NativePtySystem::default();
        let pair = pty_system.openpty(size.to_pty_size(cell_w, cell_h))?;

        // Prefer $SHELL; fall back to /bin/sh. No login shell (-l) so the
        // p10k/oh-my-zsh prompt noise does not drown the grid.
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".into());
        let mut cmd = CommandBuilder::new(&shell);
        cmd.env("TERM", "xterm-256color");
        cmd.env("COLORTERM", "truecolor");
        if let Some(home) = dirs::home_dir() {
            cmd.cwd(home);
        }

        let child = pair.slave.spawn_command(cmd)?;
        // Slave end is owned by the child after spawn; drop our handle.
        drop(pair.slave);

        let mut reader = pair.master.try_clone_reader()?;
        let writer = pair.master.take_writer()?;

        let (tx, rx) = mpsc::channel::<Vec<u8>>();
        let alive = Arc::new(AtomicBool::new(true));
        let alive_reader = Arc::clone(&alive);

        thread::Builder::new()
            .name("chronos-terminal-pty".into())
            .spawn(move || {
                let mut buf = [0u8; 8192];
                let mut first = true;
                loop {
                    match reader.read(&mut buf) {
                        Ok(0) => {
                            tracing::info!("terminal: PTY EOF");
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
                                    "terminal: first PTY chunk"
                                );
                            }
                            if tx.send(buf[..n].to_vec()).is_err() {
                                break;
                            }
                        }
                        Err(err) => {
                            tracing::warn!("terminal: PTY read error: {err}");
                            alive_reader.store(false, Ordering::Relaxed);
                            break;
                        }
                    }
                }
            })?;

        let writer = Arc::new(Mutex::new(writer));
        // Optional self-smoke: `CHRONOS_DT_PROBE=1` writes a known command
        // after the shell settles (used when ydotool is unavailable; the log
        // and VT grid must then show `__CHRONOS_DT_SPIKE_OK__`). Default off
        // so the session is a normal shell.
        if std::env::var_os("CHRONOS_DT_PROBE").is_some() {
            let w = Arc::clone(&writer);
            thread::Builder::new()
                .name("chronos-terminal-probe".into())
                .spawn(move || {
                    thread::sleep(Duration::from_millis(400));
                    if let Ok(mut guard) = w.lock() {
                        let probe = b"echo __CHRONOS_DT_SPIKE_OK__\r";
                        if let Err(err) = guard.write_all(probe).and_then(|_| guard.flush()) {
                            tracing::warn!("terminal: probe write failed: {err}");
                        } else {
                            tracing::info!("terminal: probe command written to PTY");
                        }
                    }
                })?;
        }

        let session = Arc::new(PtySession {
            master: pair.master,
            child,
            writer: Arc::clone(&writer),
        });

        tracing::info!(
            cols = size.cols,
            rows = size.rows,
            shell = %shell,
            "terminal: shell spawned on PTY"
        );
        Ok(Self {
            term: Term::new(Config::default(), &size, VoidListener),
            parser: Processor::new(),
            writer,
            rx,
            _session: session,
            alive,
        })
    }

    /// Adjust the grid. The kernel pty and the VT grid must agree or the
    /// shell's line wrapping goes stale. Kernel first: if the pty is already
    /// gone (shell exited), we fail before the VT grid reflows, so the two
    /// never diverge.
    pub fn resize(&mut self, size: TermSize, cell_w: f32, cell_h: f32) -> anyhow::Result<()> {
        self._session.resize(size.to_pty_size(cell_w, cell_h))?;
        self.term.resize(size);
        Ok(())
    }

    /// Pull all pending PTY bytes through the VT parser. Returns `true` when
    /// the screen changed and the consumer should repaint.
    pub fn drain(&mut self) -> bool {
        let mut dirty = false;
        loop {
            match self.rx.try_recv() {
                Ok(buf) => {
                    self.parser.advance(&mut self.term, &buf);
                    dirty = true;
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    self.alive.store(false, Ordering::Relaxed);
                    dirty = true;
                    break;
                }
            }
        }
        dirty
    }

    /// Send raw bytes to the shell (keyboard input, paste, …).
    pub fn write(&self, bytes: &[u8]) {
        let Ok(mut w) = self.writer.lock() else {
            return;
        };
        if let Err(err) = w.write_all(bytes).and_then(|_| w.flush()) {
            tracing::warn!("terminal: PTY write failed: {err}");
        }
    }

    /// The shell is still running (false after PTY EOF — shell exited).
    pub fn is_alive(&self) -> bool {
        self.alive.load(Ordering::Relaxed)
    }

    /// Shell pid when the platform exposes it.
    pub fn child_pid(&self) -> Option<u32> {
        self._session.child_pid()
    }

    /// Current visible grid as plain strings + cursor position.
    pub fn snapshot(&self) -> GridSnapshot {
        let lines = term_visible_lines(&self.term);
        let content = self.term.renderable_content();
        let cursor = content.cursor;
        let display_offset = self.term.grid().display_offset();
        let cursor_line = cursor.point.line.0 + display_offset as i32;
        let cursor_row = if cursor_line >= 0 {
            cursor_line as usize
        } else {
            0
        };
        GridSnapshot {
            lines,
            cursor_row,
            cursor_col: cursor.point.column.0,
            show_cursor: !matches!(
                cursor.shape,
                alacritty_terminal::vte::ansi::CursorShape::Hidden
            ),
        }
    }
}

/// Snapshot visible rows of a `Term` as plain strings (tests + diagnostics).
pub fn term_visible_lines(term: &Term<VoidListener>) -> Vec<String> {
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
        // Keep trailing spaces for cursor alignment; trim only visual noise
        // at the end of empty lines.
        lines.push(s.trim_end_matches(' ').to_owned());
    }
    lines
}

// --- Dead-session stubs (no real pty; used by engine tests) ---------------

struct DummyMaster;

impl MasterPty for DummyMaster {
    fn resize(&self, _size: PtySize) -> anyhow::Result<()> {
        Ok(())
    }
    fn get_size(&self) -> anyhow::Result<PtySize> {
        Ok(PtySize {
            rows: 24,
            cols: 80,
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

#[cfg(test)]
mod tests {
    use super::*;
    use alacritty_terminal::term::{Config, Term};

    #[test]
    fn compute_grid_fits_exact_cells() {
        // 640 px at 8 px cells → 80 cols; 400 px at 16 px cells → 25 rows.
        let g = compute_grid(640.0, 400.0, 8.0, 16.0);
        assert_eq!(g, TermSize::new(80, 25));
    }

    #[test]
    fn compute_grid_floors_partial_cells() {
        // 505 px at 8 px cells → 63 cols (63.125 floors to 63).
        let g = compute_grid(505.0, 400.0, 8.0, 16.0);
        assert_eq!(g.cols, 63);
    }

    #[test]
    fn compute_grid_clamps_to_legal_minimums() {
        let g = compute_grid(0.0, 0.0, 0.0, 0.0);
        assert_eq!(g, TermSize::new(MIN_COLS, MIN_ROWS));
        let g = compute_grid(-50.0, -50.0, 8.0, 16.0);
        assert_eq!(g, TermSize::new(MIN_COLS, MIN_ROWS));
    }

    #[test]
    fn compute_grid_ignores_zero_cell_size() {
        // Zero cell width must not divide-by-zero.
        let g = compute_grid(640.0, 400.0, 0.0, 0.0);
        assert_eq!(g, TermSize::new(MIN_COLS, MIN_ROWS));
    }

    #[test]
    fn vt_parser_renders_echo_output() {
        let size = TermSize::new(40, 10);
        let mut term = Term::new(Config::default(), &size, VoidListener);
        let mut parser: Processor = Processor::new();
        // Simulate a shell printing a probe line + newline.
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
    fn resize_reflows_grid_to_new_cells() {
        let size = TermSize::new(80, 24);
        let mut term = Term::new(Config::default(), &size, VoidListener);
        let mut parser: Processor = Processor::new();
        // Fill 30 lines so the screen has more than the new 10-row height.
        for i in 0..30 {
            parser.advance(&mut term, format!("line number {i}\r\n").as_bytes());
        }
        // Shrink to 40×10 — the grid must reflow, keep the bottom lines.
        term.resize(TermSize::new(40, 10));
        let lines = term_visible_lines(&term);
        assert_eq!(lines.len(), 10, "resized grid must expose exactly 10 rows");
        for line in &lines {
            assert!(
                line.chars().count() <= 40,
                "reflowed line longer than 40 cols: {line:?}"
            );
        }
        let joined = lines.join("\n");
        assert!(
            joined.contains("line number 29"),
            "shrink must keep the most recent output, got:\n{joined}"
        );
    }

    #[test]
    fn dummy_session_roundtrips_resize_and_write() {
        // The stubs keep a session constructible without a real pty — the
        // same DummyMaster/DummyChild the spike used for its dead-session
        // path (T177 reuses them, not a second set).
        let writer: Arc<Mutex<Box<dyn Write + Send>>> =
            Arc::new(Mutex::new(Box::new(Vec::new())));
        let session = PtySession {
            master: Box::new(DummyMaster),
            child: Box::new(DummyChild),
            writer: Arc::clone(&writer),
        };
        session
            .resize(PtySize {
                rows: 10,
                cols: 40,
                pixel_width: 320,
                pixel_height: 160,
            })
            .expect("dummy resize must succeed");
        session
            .writer()
            .lock()
            .expect("writer lock")
            .write_all(b"ls")
            .expect("dummy write must succeed");
    }

    /// Real-spawn smoke: forks a shell, so it runs only when explicitly
    /// requested via `CHRONOS_TEST_LAUNCH=1` (skips silently otherwise).
    /// The default `cargo test` run must stay hermetic.
    #[test]
    fn launch_spawns_shell_when_env_allows() {
        if std::env::var_os("CHRONOS_TEST_LAUNCH").is_none() {
            return;
        }
        let mut term =
            Terminal::launch(TermSize::new(40, 10), 8.0, 16.0).expect("shell must launch");
        assert!(term.is_alive());
        // Give the shell a moment to print its prompt, then drain + snapshot.
        std::thread::sleep(Duration::from_millis(400));
        term.drain();
        let snap = term.snapshot();
        assert!(
            snap.lines.iter().any(|l| !l.is_empty()),
            "expected a visible prompt, got: {:?}",
            snap.lines
        );
        // Resize round-trip on a live pty.
        term.resize(TermSize::new(60, 20), 8.0, 16.0).expect("live resize");
        assert_eq!(term.snapshot().lines.len(), 20);
    }
}
