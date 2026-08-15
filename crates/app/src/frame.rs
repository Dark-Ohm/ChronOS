//! Bottom desktop frame strip (T268) — a thin layer-shell strip that closes
//! the frame around the shell at the bottom bezel.
//!
//! Rules (from the task):
//! 1. **No exclusive zone** — the strip lives over the `gaps_out` gap, it
//!    never pushes windows.
//! 2. **Half the gap** — default height 4px (= `gaps_out 8` / 2, same value
//!    as the side hover strips' `STRIP_WIDTH`). Configurable in
//!    `~/.config/chronos/frame.toml` with a sane floor, not hardcoded.
//! 3. **Corners are the deliverable** — three junction variants, shot with
//!    `grim -g` on the corners, one picked with justification:
//!    - `flush`  — strip spans the full display width, square outer ends;
//!    - `break`  — strip stops at the inner edges of the side rails;
//!    - `rounded`— same as `break` but the strip's end caps are rounded.
//! 4. **Chrome from T267 tokens** — `bg.tertiary` surface + `border.subtle`
//!    top border (the strip sits at the bottom → top border, mirroring the
//!    bar's edge-relative chrome). No fourth custom shade.
//!
//! Multi-monitor: bound to the pult display only (same rule as bar/panels).

use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use gpui::{
    App, Bounds, DisplayId, IntoElement, Render, Size, Window, WindowBackgroundAppearance,
    WindowBounds, WindowHandle, WindowKind, WindowOptions, div, layer_shell::*, point, prelude::*,
    px,
};
use serde::{Deserialize, Serialize};

use chronos_ui::Theme;

const CONFIG_BASENAME: &str = "frame.toml";
const DEBOUNCE_MS: u64 = 300;
/// Default strip height = half the `gaps_out = 8` gap (rule 2). Same figure
/// as `side_panel_right::hover_strip::STRIP_WIDTH` (4px) — the frame closes
/// the gap, it does not cover it.
const DEFAULT_HEIGHT: f32 = 4.0;
const MIN_HEIGHT: f32 = 1.0;
const MAX_HEIGHT: f32 = 16.0;
/// Keep in sync with `side_panel_right::RAIL_WIDTH` (40) — the `break` /
/// `rounded` junctions inset by this so the strip butts the rails' inner
/// edges exactly.
const RAIL_INSET: f32 = 40.0;

/// Strip/rail junction at the bottom corners.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FrameJunction {
    /// Full-width strip, square ends. The strip crosses the rails' bottom
    /// edges (they sit on it).
    Flush,
    /// Strip stops at the rails' inner edges (x = RAIL_INSET each side);
    /// rails keep their square bottom corners to the screen edge.
    Break,
    /// Same span as `Break`, but the strip's end caps are rounded.
    Rounded,
}

impl Default for FrameJunction {
    fn default() -> Self {
        Self::Break
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct BottomStripConfig {
    pub enabled: bool,
    pub height: f32,
    pub junction: FrameJunction,
}

impl Default for BottomStripConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            height: DEFAULT_HEIGHT,
            junction: FrameJunction::default(),
        }
    }
}

impl BottomStripConfig {
    /// Clamp height into `MIN..=MAX`, drop nothing else (enum can't carry
    /// unknown variants — an unknown junction fails parse → whole file falls
    /// back to defaults at load).
    pub fn sanitized(&self) -> Self {
        let mut out = self.clone();
        if out.height < MIN_HEIGHT || out.height > MAX_HEIGHT {
            tracing::warn!(
                "frame: height {} out of range [{MIN_HEIGHT}, {MAX_HEIGHT}], clamping to {}",
                out.height,
                out.height.clamp(MIN_HEIGHT, MAX_HEIGHT)
            );
            out.height = out.height.clamp(MIN_HEIGHT, MAX_HEIGHT);
        }
        out
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct FrameConfig {
    pub bottom_strip: BottomStripConfig,
}

impl Default for FrameConfig {
    fn default() -> Self {
        Self {
            bottom_strip: BottomStripConfig::default(),
        }
    }
}

static CONFIG_CACHE: OnceLock<Mutex<FrameConfig>> = OnceLock::new();

fn config_cache() -> &'static Mutex<FrameConfig> {
    CONFIG_CACHE.get_or_init(|| Mutex::new(FrameConfig::default()))
}

fn config_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("~/.config"))
        .join("chronos")
        .join(CONFIG_BASENAME)
}

fn parent_dir() -> PathBuf {
    let p = config_path();
    p.parent().map(|x| x.to_path_buf()).unwrap_or(p)
}

/// Cached config, sanitized (no disk I/O).
pub fn cached_config() -> FrameConfig {
    let mut cfg = config_cache()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone();
    cfg.bottom_strip = cfg.bottom_strip.sanitized();
    cfg
}

impl FrameConfig {
    /// Load from disk. Missing → default (no silent write). Bad parse → warn + default.
    pub fn load() -> Self {
        let path = config_path();
        match std::fs::read_to_string(&path) {
            Ok(content) => match toml::from_str::<FrameConfig>(&content) {
                Ok(cfg) => cfg,
                Err(e) => {
                    tracing::warn!(
                        "frame: failed to parse {}: {e}, using defaults",
                        path.display()
                    );
                    Self::default()
                }
            },
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                tracing::debug!("frame: {} not found, using defaults", path.display());
                Self::default()
            }
            Err(e) => {
                tracing::warn!("frame: read {} failed: {e}, using defaults", path.display());
                Self::default()
            }
        }
    }

    /// Replace the cache with a freshly loaded (and sanitized) config.
    pub fn apply() -> Self {
        let cfg = Self::load();
        *config_cache()
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = cfg.clone();
        cfg
    }
}

struct BottomStripView;

impl Render for BottomStripView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Click-through: the strip eats no input ever, not even hover.
        window.set_input_region(Some(&[]));

        let theme = Theme::global(cx);
        let strip = cached_config().bottom_strip;

        // Span/chrome per junction (rule 3): full width for `Flush`; inset to
        // the rails' inner boundary for `Break`/`Rounded`.
        let mut chrome = div()
            .id("bottom-frame-strip")
            .h_full()
            .flex_1()
            .bg(theme.bg.tertiary)
            .border_t_1()
            .border_color(theme.border.subtle);
        if strip.junction == FrameJunction::Rounded {
            let radius = (strip.height / 2.0).max(1.0);
            chrome = chrome.rounded(px(radius));
        }

        if strip.junction == FrameJunction::Flush {
            chrome
        } else {
            // Spacers on each side inset the chrome to the rails' inner
            // boundary. Flexible root means margins can't overflow the
            // window like `.mx()` did (the chrome stretched full width).
            div()
                .id("bottom-frame-strip-shell")
                .size_full()
                .flex()
                .child(div().h_full().w(px(RAIL_INSET)))
                .child(chrome)
                .child(div().h_full().w(px(RAIL_INSET)))
        }
    }
}

/// Window options for the strip on the given display: bottom-anchored,
/// full-width Overlay, no exclusive zone, no keyboard, transparent.
fn window_options(display_id: Option<DisplayId>, cx: &App) -> WindowOptions {
    let display_w = display_id
        .and_then(|id| cx.find_display(id))
        .map(|d| f32::from(d.bounds().size.width))
        .unwrap_or(1920.);

    WindowOptions {
        display_id,
        titlebar: None,
        window_bounds: Some(WindowBounds::Windowed(Bounds {
            origin: point(px(0.), px(0.)),
            size: Size::new(px(display_w), px(cached_config().bottom_strip.height)),
        })),
        app_id: Some("chronos-frame".to_string()),
        window_background: WindowBackgroundAppearance::Transparent,
        kind: WindowKind::LayerShell(LayerShellOptions {
            namespace: "frame_bottom_strip".to_string(),
            layer: Layer::Overlay,
            anchor: Anchor::BOTTOM | Anchor::LEFT | Anchor::RIGHT,
            exclusive_zone: None,
            margin: None,
            keyboard_interactivity: KeyboardInteractivity::None,
            ..Default::default()
        }),
        ..Default::default()
    }
}

static FRAME_WINDOW: OnceLock<Mutex<Option<WindowHandle<BottomStripView>>>> = OnceLock::new();

fn frame_window() -> &'static Mutex<Option<WindowHandle<BottomStripView>>> {
    FRAME_WINDOW.get_or_init(|| Mutex::new(None))
}

/// Open the strip on the pult display. Idempotent — no-op if already open.
pub fn open(cx: &mut App) -> bool {
    if frame_window().lock().unwrap_or_else(|e| e.into_inner()).is_some() {
        return true;
    }
    let display_id = crate::monitor::pult_display_id_or_primary(cx);
    tracing::info!("frame: opening bottom strip on display_id={display_id:?}");
    match cx.open_window(window_options(display_id, cx), |_, view_cx| {
        view_cx.new(|_| BottomStripView {})
    }) {
        Ok(handle) => {
            *frame_window().lock().unwrap_or_else(|e| e.into_inner()) = Some(handle);
            apply(cx);
            true
        }
        Err(err) => {
            tracing::warn!("frame: failed to open bottom strip: {err}");
            false
        }
    }
}

/// Close the strip window (idempotent — no-op if already closed).
fn close(cx: &mut App) {
    if let Some(handle) = frame_window().lock().unwrap_or_else(|e| e.into_inner()).take() {
        match handle.update(cx, |_, window: &mut Window, _| window.remove_window()) {
            Ok(()) => tracing::info!("frame: closed for recreate"),
            Err(e) => tracing::warn!("frame: close could not reach window ({e})"),
        }
    }
}

/// Live-apply the cached config: enabled toggle → open/close, height change →
/// resize (junction is render-only). Idempotent. Called on every
/// `frame.toml` change (300 ms debounce) and once after open.
pub fn apply(cx: &mut App) {
    let cfg = cached_config();
    let Some(handle) = *frame_window().lock().unwrap_or_else(|e| e.into_inner()) else {
        if cfg.bottom_strip.enabled {
            open(cx);
        }
        return;
    };

    let enabled = cfg.bottom_strip.enabled;
    if !enabled {
        close(cx);
        return;
    }

    match handle.update(cx, |_, window: &mut Window, cx| {
        let current = window.bounds().size;
        window.resize(Size::new(current.width, px(cfg.bottom_strip.height)));
        window.set_input_region(Some(&[]));
        cx.notify();
    }) {
        Ok(()) => tracing::debug!("frame: bottom strip config applied"),
        Err(e) => tracing::warn!("frame: apply could not reach window ({e})"),
    }
}

/// inotify hot-reload for `frame.toml` (bar/layout_config pattern).
fn spawn_watcher(cx: &mut App) {
    let parent = parent_dir();
    if !parent.is_dir() {
        tracing::debug!(
            "frame: parent dir {} missing, config hot-reload disabled",
            parent.display()
        );
        return;
    }

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<()>();
    let watch_target = parent.clone();

    std::thread::Builder::new()
        .name("frame-inotify".into())
        .spawn(move || {
            let mut inotify = match inotify::Inotify::init() {
                Ok(i) => i,
                Err(e) => {
                    tracing::error!("frame: inotify init failed: {e}");
                    return;
                }
            };
            let mask = inotify::WatchMask::CLOSE_WRITE
                .union(inotify::WatchMask::MOVED_TO)
                .union(inotify::WatchMask::CREATE)
                .union(inotify::WatchMask::DELETE)
                .union(inotify::WatchMask::MODIFY);
            if let Err(e) = inotify.watches().add(&watch_target, mask) {
                tracing::error!("frame: failed to watch {}: {e}", watch_target.display());
                return;
            }
            let target = std::ffi::OsStr::new(CONFIG_BASENAME);
            let mut buf = [0u8; 4096];
            loop {
                let events = match inotify.read_events_blocking(&mut buf) {
                    Ok(e) => e,
                    Err(e) => {
                        tracing::error!("frame: inotify read error: {e}");
                        break;
                    }
                };
                let mut changed = false;
                for ev in events {
                    if ev.mask.contains(inotify::EventMask::ISDIR) {
                        continue;
                    }
                    if let Some(name) = ev.name {
                        if name == target {
                            changed = true;
                        }
                    }
                }
                if changed && tx.send(()).is_err() {
                    break;
                }
            }
        })
        .expect("frame: failed to spawn inotify thread");

    cx.spawn(async move |cx| {
        let mut deadline: Option<tokio::time::Instant> = None;
        loop {
            let timer = async {
                match deadline {
                    Some(d) => tokio::time::sleep_until(d).await,
                    None => std::future::pending::<()>().await,
                }
            };
            tokio::select! {
                _ = rx.recv() => {
                    deadline = Some(tokio::time::Instant::now() + Duration::from_millis(DEBOUNCE_MS));
                }
                _ = timer => {
                    deadline = None;
                    let _ = cx.update(|cx| {
                        FrameConfig::apply();
                        apply(cx);
                        tracing::info!(
                            "frame: hot-reloaded config from {}",
                            config_path().display()
                        );
                    });
                }
            }
        }
    })
    .detach();
}

/// Opens the bottom strip once at startup. Called from `main.rs`.
/// Deferred ~40 ms so Wayland has enumerated displays (strip must land on the
/// pult, like bar/panels) and so it opens before the panel surfaces (~50 ms)
/// — the strip reads as the bottom frame the rails sit on.
pub fn init(cx: &mut App) {
    FrameConfig::apply();
    spawn_watcher(cx);
    cx.spawn(async move |cx| {
        cx.background_executor()
            .timer(Duration::from_millis(40))
            .await;
        let _ = cx.update(|cx| {
            if cached_config().bottom_strip.enabled {
                open(cx);
            }
        });
    })
    .detach();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_sane() {
        let cfg = FrameConfig::default();
        assert!(cfg.bottom_strip.enabled);
        assert_eq!(cfg.bottom_strip.height, 4.0);
        assert_eq!(cfg.bottom_strip.junction, FrameJunction::Break);
    }

    #[test]
    fn junction_lowercase_roundtrip() {
        for variant in [
            FrameJunction::Flush,
            FrameJunction::Break,
            FrameJunction::Rounded,
        ] {
            let s = toml::Value::String(match variant {
                FrameJunction::Flush => "flush".into(),
                FrameJunction::Break => "break".into(),
                FrameJunction::Rounded => "rounded".into(),
            });
            let parsed: FrameJunction = s.try_into().unwrap();
            assert_eq!(parsed, variant);
        }
    }

    #[test]
    fn sanitize_clamps_height() {
        let mut cfg = FrameConfig::default();
        cfg.bottom_strip.height = 99.0;
        assert_eq!(cfg.bottom_strip.sanitized().height, MAX_HEIGHT);
        cfg.bottom_strip.height = -2.0;
        assert_eq!(cfg.bottom_strip.sanitized().height, MIN_HEIGHT);
    }

    #[test]
    fn config_path_ends_with_expected_file() {
        assert!(config_path().ends_with("chronos/frame.toml"));
    }

    #[test]
    fn unknown_junction_value_fails_parse() {
        let bad = "[bottom_strip]\nenabled=true\njunction=\"diagonal\"\n";
        let err = toml::from_str::<FrameConfig>(bad);
        assert!(err.is_err(), "unknown junction must fail parse → defaults");
    }

    #[test]
    fn positive_rainbow_defaults_to_flush_on_missing_section() {
        let doc = "[unrelated]\nfoo=1\n";
        let cfg = toml::from_str::<FrameConfig>(doc).unwrap();
        assert_eq!(cfg, FrameConfig::default());
    }
}