//! Desktop frame theme (T268 + T284) — a thin layer-shell strip that closes
//! the frame around the shell at the bottom bezel, plus the optional `wrap`
//! theme that turns the frame into a full perimeter card.
//!
//! ## Hide (default, T268)
//! The bottom strip over the `gaps_out` gap. Rules (from the task):
//! 1. **No exclusive zone** — the strip lives over the gap, it never pushes
//!    windows.
//! 2. **Half the gap** — default height 4px (= `gaps_out 8` / 2, same value
//!    as the side hover strips' `STRIP_WIDTH`). Configurable in
//!    `~/.config/chronos/frame.toml` with a sane floor, not hardcoded.
//! 3. **Corners are the deliverable** — three junction variants (`flush` /
//!    `break` / `rounded`), `break` picked by live corners.
//! 4. **Chrome from T267 tokens** — `bg.tertiary` surface + `border.subtle`
//!    top border. No fourth custom shade.
//!
//! The strip's span follows which side rails are mapped (T284 §4): with both
//! rails gone the strip is closed entirely — a floating bottom bar over the
//! wallpaper is exactly what the strip is not.
//!
//! ## Wrap (T284)
//! `style = "wrap"` in `frame.toml`: one fullscreen **matte** (Layer::Top,
//! paints only the chrome ring, hole = wallpaper) plus three invisible
//! exclusive strips L/R/B (thickness = `wrap.thickness`) that push
//! clients off the frame. The bar stays top-exclusive and reads as the top
//! edge of the frame; side rails/content inset by `wrap_inset()` on the
//! panel side.
//!
//! `frame::apply` never imports the panels — panel geometry is re-applied
//! through the `set_after_apply` hook registered in `main.rs` (one-way
//! dependency; otherwise `frame ↔ side_panel` cycle).
//!
//! Multi-monitor: everything is bound to the pult display only (same rule
//! as bar/panels).

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU8, Ordering};
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
/// Default `wrap.inner_radius` — the inner corners of the card, not half
/// the strip height (T284 spec §3).
const DEFAULT_INNER_RADIUS: f32 = 16.0;
const MIN_RADIUS: f32 = 0.0;
const MAX_RADIUS: f32 = 64.0;
/// Default `wrap.thickness` — the chrome ring width. Equal to the default
/// `inner_radius` so the corner rounding reads proportional to the frame;
/// the old figure (`bottom_strip.height`) was tuned for the thin Hide
/// strip and made the wrap frame read as a cheap 4px line (T303).
const DEFAULT_THICKNESS: f32 = 16.0;
const MIN_THICKNESS: f32 = 1.0;
const MAX_THICKNESS: f32 = 64.0;

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

/// Desktop frame style (T284). Deserialized from a **string** via
/// `deserialize_style` — never `#[derive(Deserialize)]` directly, otherwise
/// an unknown value fails the whole parse and `load()` silently replaces
/// the config with defaults (the T268 junction trap).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum FrameStyle {
    /// T268 path: bottom strip between the rails, closed when no rail is
    /// mapped.
    #[default]
    Hide,
    /// Perimeter card: matte + three exclusive edge strips, rails inside.
    Wrap,
}

impl FrameStyle {
    pub fn as_str(self) -> &'static str {
        match self {
            FrameStyle::Hide => "hide",
            FrameStyle::Wrap => "wrap",
        }
    }

    /// Unknown values → `Hide` + warn (never panic, never silently enable
    /// Wrap — spec §3).
    pub fn from_str_sanitized(s: &str) -> Self {
        match s {
            "wrap" => FrameStyle::Wrap,
            "hide" => FrameStyle::Hide,
            other => {
                tracing::warn!("frame: unknown style {other:?}, falling back to hide");
                FrameStyle::Hide
            }
        }
    }
}

fn deserialize_style<'de, D>(deserializer: D) -> Result<FrameStyle, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    Ok(FrameStyle::from_str_sanitized(&s))
}

/// `[wrap]` section — frame thickness and inner corner radius (T284 spec
/// §3; T303: thickness split off `bottom_strip.height`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct WrapConfig {
    pub thickness: f32,
    pub inner_radius: f32,
}

impl Default for WrapConfig {
    fn default() -> Self {
        Self {
            thickness: DEFAULT_THICKNESS,
            inner_radius: DEFAULT_INNER_RADIUS,
        }
    }
}

impl WrapConfig {
    /// Clamp `thickness` into `MIN_THICKNESS..=MAX_THICKNESS` and
    /// `inner_radius` into `MIN_RADIUS..=MAX_RADIUS` (0 disables the corner
    /// rounding entirely).
    pub fn sanitized(&self) -> Self {
        let mut out = self.clone();
        if out.thickness < MIN_THICKNESS || out.thickness > MAX_THICKNESS {
            tracing::warn!(
                "frame: wrap.thickness {} out of range [{MIN_THICKNESS}, {MAX_THICKNESS}], clamping to {}",
                out.thickness,
                out.thickness.clamp(MIN_THICKNESS, MAX_THICKNESS)
            );
            out.thickness = out.thickness.clamp(MIN_THICKNESS, MAX_THICKNESS);
        }
        if out.inner_radius < MIN_RADIUS || out.inner_radius > MAX_RADIUS {
            tracing::warn!(
                "frame: wrap.inner_radius {} out of range [{MIN_RADIUS}, {MAX_RADIUS}], clamping to {}",
                out.inner_radius,
                out.inner_radius.clamp(MIN_RADIUS, MAX_RADIUS)
            );
            out.inner_radius = out.inner_radius.clamp(MIN_RADIUS, MAX_RADIUS);
        }
        out
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
    /// hide | wrap — missing key / unknown value → Hide (spec §3).
    #[serde(default, deserialize_with = "deserialize_style")]
    pub style: FrameStyle,
    pub bottom_strip: BottomStripConfig,
    pub wrap: WrapConfig,
}

impl Default for FrameConfig {
    fn default() -> Self {
        Self {
            style: FrameStyle::default(),
            bottom_strip: BottomStripConfig::default(),
            wrap: WrapConfig::default(),
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
    cfg.wrap = cfg.wrap.sanitized();
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

// ── Rail presence (T284 §4) ─────────────────────────────────────────────────
//
// The panels report whether their rail surface is mapped; the hide strip's
// existence and span, and the wrap inset, derive from this. Stored in an
// atomic so pure predicates (`hide_strip_wanted`, `hide_strip_insets`) can
// read it without an `App`.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameSide {
    Left,
    Right,
}

const RAIL_LEFT_BIT: u8 = 0b01;
const RAIL_RIGHT_BIT: u8 = 0b10;
static RAIL_MAPPED: AtomicU8 = AtomicU8::new(0);

fn rail_bit(side: FrameSide) -> u8 {
    match side {
        FrameSide::Left => RAIL_LEFT_BIT,
        FrameSide::Right => RAIL_RIGHT_BIT,
    }
}

pub fn rail_mapped(side: FrameSide) -> bool {
    RAIL_MAPPED.load(Ordering::Relaxed) & rail_bit(side) != 0
}

/// Panels call this on rail open/close (never from `init_hover_strip` — a
/// 4px hover strip is not a rail). Re-derives the hide strip and re-applies.
pub fn set_rail_mapped(side: FrameSide, mapped: bool, cx: &mut App) {
    let bit = rail_bit(side);
    if mapped {
        RAIL_MAPPED.fetch_or(bit, Ordering::Relaxed);
    } else {
        RAIL_MAPPED.fetch_and(!bit, Ordering::Relaxed);
    }
    apply(cx);
}

/// Hide-strip span insets per side (CSS order left, right): a mapped rail
/// pushes the strip inward by `RAIL_INSET`; a missing rail means the strip
/// reaches the screen edge.
pub fn hide_strip_insets(left_mapped: bool, right_mapped: bool) -> (f32, f32) {
    (
        if left_mapped { RAIL_INSET } else { 0.0 },
        if right_mapped { RAIL_INSET } else { 0.0 },
    )
}

/// The Hide strip exists only while at least one rail is mapped — with both
/// rails gone the bottom chrome would float over the wallpaper (T284 §4).
pub fn hide_strip_wanted(enabled: bool, left_mapped: bool, right_mapped: bool) -> bool {
    enabled && (left_mapped || right_mapped)
}

/// 0 in Hide; `wrap.thickness` in Wrap — the frame thickness rails and
/// content inset by, and the exclusive strips reserve (T284 §5, T303: no
/// longer `bottom_strip.height`, which is tuned for the thin Hide strip).
pub fn wrap_inset_for(cfg: &FrameConfig) -> f32 {
    match cfg.style {
        FrameStyle::Hide => 0.0,
        FrameStyle::Wrap => cfg.wrap.sanitized().thickness,
    }
}

pub fn wrap_inset() -> f32 {
    wrap_inset_for(&cached_config())
}

/// The matte's transparent hole (T284 spec §5.1): inset `h` on L/R/B, top at
/// the live bar height. `inner_radius` only rounds the hole's corners, it
/// does not change the rectangle bounds.
pub fn wrap_inner_rect(display_w: f32, display_h: f32, bar_h: f32, inset: f32) -> Bounds<f32> {
    Bounds::from_corners(
        point(inset, bar_h),
        point((display_w - inset).max(0.0), (display_h - inset).max(bar_h)),
    )
}

// ── Hide strip surface (T268) ───────────────────────────────────────────────

struct BottomStripView;

impl Render for BottomStripView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Click-through: the strip eats no input ever, not even hover.
        window.set_input_region(Some(&[]));

        let theme = Theme::global(cx);
        let strip = cached_config().bottom_strip;

        // Span/chrome per junction (rule 3): full width for `Flush`; inset to
        // the rails' inner boundary for `Break`/`Rounded`. The inset follows
        // which rails are actually mapped (T284 §4).
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
            let (left, right) = hide_strip_insets(
                rail_mapped(FrameSide::Left),
                rail_mapped(FrameSide::Right),
            );
            div()
                .id("bottom-frame-strip-shell")
                .size_full()
                .flex()
                .child(div().h_full().w(px(left)))
                .child(chrome)
                .child(div().h_full().w(px(right)))
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
fn open(cx: &mut App) -> bool {
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

// ── Wrap surfaces (T284 §5) ─────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WrapRole {
    /// Fullscreen chrome ring on Layer::Top (below the Overlay bar/panels).
    Matte,
    /// Invisible exclusive strip reserving the left edge (thickness = inset).
    ExclLeft,
    ExclRight,
    ExclBottom,
}

struct WrapSurfaceView {
    role: WrapRole,
}

impl Render for WrapSurfaceView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Click-through everywhere — the matte is pure chrome, the dummies
        // are invisible; input is taken by clients and exclusive zones, not
        // by the frame (spec §5.1/5.2).
        window.set_input_region(Some(&[]));
        if self.role != WrapRole::Matte {
            // The exclusive strips paint nothing — empty surface.
            return div().size_full().into_any_element();
        }

        let theme = Theme::global(cx);
        let cfg = cached_config();
        // Same sanitized source as `wrap_inset_for` — the matte paints what
        // the exclusive strips reserve, so the ring can never diverge from
        // the geometry clients are pushed by (T303 second drift, fixed).
        let inset = cfg.wrap.sanitized().thickness;
        let radius = cfg.wrap.inner_radius;

        // One uniform ring instead of strips + hand-drawn corner patches
        // (T303): the border shader offsets the inner edge by the border
        // width, so a full-size div with border `inset` and rounding
        // `radius + inset` paints a frame of constant thickness `inset`
        // whose inner contour is exactly the spec's rounded rect (inner
        // radius = `inner_radius`, outer = inner + thickness). No top
        // border: the matte sits below the bar on the same layer, but the
        // bar only covers y0-30 and the border would read as a line below
        // it — the bar itself is the top edge. No background — the hole
        // stays the wallpaper (spec §5.1; a transparent child would not
        // punch a fill).
        div()
            .id("frame-wrap-matte")
            .size_full()
            .border(px(inset))
            .border_t_0()
            .border_color(theme.bg.tertiary)
            .rounded(px(radius + inset))
            .into_any_element()
    }
}

#[derive(Default)]
struct WrapWindows {
    matte: Option<WindowHandle<WrapSurfaceView>>,
    left: Option<WindowHandle<WrapSurfaceView>>,
    right: Option<WindowHandle<WrapSurfaceView>>,
    bottom: Option<WindowHandle<WrapSurfaceView>>,
}

impl WrapWindows {
    fn slot(&mut self, role: WrapRole) -> &mut Option<WindowHandle<WrapSurfaceView>> {
        match role {
            WrapRole::Matte => &mut self.matte,
            WrapRole::ExclLeft => &mut self.left,
            WrapRole::ExclRight => &mut self.right,
            WrapRole::ExclBottom => &mut self.bottom,
        }
    }
}

static WRAP_WINDOWS: OnceLock<Mutex<WrapWindows>> = OnceLock::new();

fn wrap_windows() -> &'static Mutex<WrapWindows> {
    WRAP_WINDOWS.get_or_init(|| Mutex::new(WrapWindows::default()))
}

/// Per-surface options for the wrap matte and the three exclusive strips.
/// The matte is fullscreen on Layer::Top with NO exclusive zone (a fullscreen
/// surface with `exclusive != 0` reserves the whole screen — spec §5); the
/// dummies are Overlay strips whose exclusive zone pushes clients off the
/// frame.
fn wrap_window_options(role: WrapRole, display_id: Option<DisplayId>, cx: &App) -> WindowOptions {
    let (w, h) = display_id
        .and_then(|id| cx.find_display(id))
        .map(|d| {
            (
                f32::from(d.bounds().size.width),
                f32::from(d.bounds().size.height),
            )
        })
        .unwrap_or((1920., 1080.));
    let inset = wrap_inset();
    let (size, anchor, namespace, layer, exclusive_zone, exclusive_edge, margin) = match role {
        // The matte must cover the full screen, ring flush with every edge.
        // Hyprland places every layer surface inside the available area
        // (monitor minus all reservations, regardless of layer) — the matte
        // is always pushed up by the bar (30) and ExclBottom (inset), so a
        // BOTTOM-anchored matte flush-lands at y=-inset..(h-inset) and its
        // bottom border floats `inset` above the screen edge (measured
        // live: y=-16..1424 with inset 16). A negative bottom margin
        // counteracts the reservation exactly: bottom edge = available
        // bottom - margin = (1440-inset) - (-inset) = 1440, covering
        // y0-1440. The horizontal anchors are LEFT only: a LEFT|RIGHT
        // anchored full-width surface gets CENTERED (not clipped) when the
        // side panels' reservations shrink the available width — measured
        // live at x=-28 with the right panel open, ring off-screen left and
        // over the rail right. LEFT pins the left edge at 0 and the width
        // still covers the monitor. Layer stays Top: an Overlay matte
        // shares a layer with the rail/panels and its empty input region
        // swallows their clicks (measured live), while on Top it sits below
        // them. The bar covers the top border regardless (same layer,
        // opaque), so `border_t_0` below is belt-and-suspenders.
        WrapRole::Matte => (
            Size::new(px(w), px(h)),
            Anchor::LEFT | Anchor::BOTTOM,
            "frame_wrap_matte",
            Layer::Top,
            None,
            None,
            // Negative bottom AND left margins: the bar/ExclBottom and
            // ExclLeft reservations push a bottom/left-anchored matte to
            // y=-inset..(h-inset) / x=inset.. respectively (measured live:
            // y=-16..1424 and x=16.. with inset 16), so a negative margin
            // of -inset counteracts each exactly and the matte covers
            // x0-2560, y0-1440.
            Some((px(0.), px(0.), px(-inset), px(-inset))),
        ),
        WrapRole::ExclLeft => (
            Size::new(px(inset), px(h)),
            Anchor::TOP | Anchor::BOTTOM | Anchor::LEFT,
            "frame_wrap_excl_left",
            Layer::Overlay,
            Some(px(inset)),
            Some(Anchor::LEFT),
            None,
        ),
        WrapRole::ExclRight => (
            Size::new(px(inset), px(h)),
            Anchor::TOP | Anchor::BOTTOM | Anchor::RIGHT,
            "frame_wrap_excl_right",
            Layer::Overlay,
            Some(px(inset)),
            Some(Anchor::RIGHT),
            None,
        ),
        WrapRole::ExclBottom => (
            Size::new(px(w), px(inset)),
            Anchor::LEFT | Anchor::RIGHT | Anchor::BOTTOM,
            "frame_wrap_excl_bottom",
            Layer::Overlay,
            Some(px(inset)),
            Some(Anchor::BOTTOM),
            None,
        ),
    };
    WindowOptions {
        display_id,
        titlebar: None,
        window_bounds: Some(WindowBounds::Windowed(Bounds {
            origin: point(px(0.), px(0.)),
            size,
        })),
        app_id: Some("chronos-frame-wrap".to_string()),
        window_background: WindowBackgroundAppearance::Transparent,
        kind: WindowKind::LayerShell(LayerShellOptions {
            namespace: namespace.to_string(),
            layer,
            anchor,
            exclusive_zone,
            exclusive_edge,
            margin,
            keyboard_interactivity: KeyboardInteractivity::None,
            ..Default::default()
        }),
        ..Default::default()
    }
}

/// Open matte + three dummies as one set. Partial open is refused — if any
/// surface fails, everything already opened is rolled back.
fn open_wrap_windows(cx: &mut App) {
    {
        let slots = wrap_windows().lock().unwrap_or_else(|e| e.into_inner());
        if slots.matte.is_some() {
            return;
        }
    }
    let display_id = crate::monitor::pult_display_id_or_primary(cx);
    let mut opened: Vec<(WrapRole, WindowHandle<WrapSurfaceView>)> = Vec::new();
    for role in [
        WrapRole::Matte,
        WrapRole::ExclLeft,
        WrapRole::ExclRight,
        WrapRole::ExclBottom,
    ] {
        match cx.open_window(wrap_window_options(role, display_id, cx), |_, view_cx| {
            view_cx.new(|_| WrapSurfaceView { role })
        }) {
            Ok(handle) => opened.push((role, handle)),
            Err(err) => {
                tracing::warn!("frame: wrap surface {role:?} failed to open: {err}");
                for (failed_role, handle) in opened.drain(..) {
                    match handle.update(cx, |_, window: &mut Window, _| window.remove_window()) {
                        Ok(()) => tracing::info!("frame: rolled back wrap surface {failed_role:?}"),
                        Err(e) => tracing::warn!(
                            "frame: rollback could not close wrap surface {failed_role:?} ({e})"
                        ),
                    }
                }
                return;
            }
        }
    }
    let mut slots = wrap_windows().lock().unwrap_or_else(|e| e.into_inner());
    for (role, handle) in opened {
        *slots.slot(role) = Some(handle);
    }
}

/// Close all four wrap surfaces (idempotent).
fn close_wrap_windows(cx: &mut App) {
    let mut slots = wrap_windows().lock().unwrap_or_else(|e| e.into_inner());
    for role in [
        WrapRole::Matte,
        WrapRole::ExclLeft,
        WrapRole::ExclRight,
        WrapRole::ExclBottom,
    ] {
        if let Some(handle) = slots.slot(role).take() {
            match handle.update(cx, |_, window: &mut Window, _| window.remove_window()) {
                Ok(()) => tracing::info!("frame: closed wrap surface {role:?}"),
                Err(e) => tracing::warn!("frame: could not close wrap surface {role:?} ({e})"),
            }
        }
    }
}

// ── Apply / orchestration ───────────────────────────────────────────────────

const STYLE_ID_HIDE: u8 = 0;
const STYLE_ID_WRAP: u8 = 1;
/// Last applied style — a Hide↔Wrap transition is the only event that
/// re-triggers panel geometry via `after_apply`.
static LAST_STYLE: AtomicU8 = AtomicU8::new(STYLE_ID_HIDE);

/// Last applied sanitized wrap geometry. The matte's negative margin and the
/// exclusive strips' size/zone are baked in at surface-open time, so a
/// thickness/radius edit on a live Wrap shell must recreate the set
/// (T307) — the bar/Hide strip hot-reload live, the wrap surfaces did not.
static LAST_WRAP_GEOMETRY: OnceLock<Mutex<Option<WrapConfig>>> = OnceLock::new();

fn last_wrap_geometry() -> &'static Mutex<Option<WrapConfig>> {
    LAST_WRAP_GEOMETRY.get_or_init(|| Mutex::new(None))
}

/// Pure decision: does `current` wrap geometry differ from the last applied
/// one? `None` (first apply) counts as a change. Split out of `apply_wrap`
/// so the recreate rule is unit-testable without an `App`.
fn wrap_geometry_changed(last: Option<&WrapConfig>, current: &WrapConfig) -> bool {
    last != Some(current)
}

type AfterApplyHook = Box<dyn Fn(&mut App) + Send + Sync>;

static AFTER_APPLY: OnceLock<Mutex<Option<AfterApplyHook>>> = OnceLock::new();

fn after_apply_slot() -> &'static Mutex<Option<AfterApplyHook>> {
    AFTER_APPLY.get_or_init(|| Mutex::new(None))
}

/// Register the panel geometry hook (called from `main.rs` after the side
/// panel modules are in scope). The frame never imports the panels — the
/// hook keeps the module dependency one-way (no `frame ↔ side_panel` cycle).
pub fn set_after_apply(hook: impl Fn(&mut App) + Send + Sync + 'static) {
    *after_apply_slot().lock().unwrap_or_else(|e| e.into_inner()) = Some(Box::new(hook));
}

fn run_after_apply(cx: &mut App) {
    let slot = after_apply_slot().lock().unwrap_or_else(|e| e.into_inner());
    if let Some(hook) = slot.as_ref() {
        hook(cx);
    }
}

fn apply_hide(cx: &mut App, cfg: &FrameConfig) {
    // Wrap surfaces never coexist with the hide strip.
    close_wrap_windows(cx);

    let wanted = hide_strip_wanted(
        cfg.bottom_strip.enabled,
        rail_mapped(FrameSide::Left),
        rail_mapped(FrameSide::Right),
    );
    let Some(handle) = *frame_window().lock().unwrap_or_else(|e| e.into_inner()) else {
        if wanted {
            open(cx);
        }
        return;
    };
    if !wanted {
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

fn apply_wrap(cx: &mut App, cfg: &FrameConfig) {
    // Hide strip never coexists with the matte.
    if frame_window().lock().unwrap_or_else(|e| e.into_inner()).is_some() {
        close(cx);
    }

    // The wrap geometry (matte negative margin + strips' size/exclusive
    // zone) is set at surface-open time. On a live thickness/radius edit the
    // windows must be recreated; otherwise `open_wrap_windows` is idempotent
    // and leaves the existing set alone.
    let geometry_changed = {
        let mut slot = last_wrap_geometry().lock().unwrap_or_else(|e| e.into_inner());
        let changed = wrap_geometry_changed(slot.as_ref(), &cfg.wrap);
        *slot = Some(cfg.wrap.clone());
        changed
    };
    if geometry_changed {
        close_wrap_windows(cx);
    }
    open_wrap_windows(cx);
}

/// Live-apply the cached config (style, strip, wrap geometry). Idempotent.
/// Called on every `frame.toml` change (300 ms debounce), on rail
/// open/close, and once at startup.
pub fn apply(cx: &mut App) {
    let cfg = cached_config();
    let style_id = match cfg.style {
        FrameStyle::Hide => STYLE_ID_HIDE,
        FrameStyle::Wrap => STYLE_ID_WRAP,
    };
    let changed = LAST_STYLE.swap(style_id, Ordering::Relaxed) != style_id;

    match cfg.style {
        FrameStyle::Hide => apply_hide(cx, &cfg),
        FrameStyle::Wrap => apply_wrap(cx, &cfg),
    }

    // A style transition changes the panel geometry (margin/height), which
    // is only writable at surface open time — panels recreate themselves
    // through the hook.
    if changed {
        run_after_apply(cx);
    }
}

/// Appearance control target: RMW-write only the `style` key so unknown
/// keys/sections in `frame.toml` survive (never dump `FrameConfig` — that
/// would wipe height/radius/foreign keys, T284 spec §3).
pub fn write_style(style: FrameStyle) -> Result<(), String> {
    write_style_at(&config_path(), style)
}

fn write_style_at(path: &Path, style: FrameStyle) -> Result<(), String> {
    let content = std::fs::read_to_string(path).unwrap_or_default();
    let mut doc: toml::Value = content
        .parse()
        .map_err(|e: toml::de::Error| format!("frame: failed to parse {}: {e}", path.display()))?;
    let table = doc
        .as_table_mut()
        .ok_or_else(|| format!("frame: {} is not a TOML table", path.display()))?;
    // Insert, not `doc["style"] = ..` — toml 0.8's IndexMut panics on a
    // missing key instead of inserting.
    table.insert("style".to_string(), toml::Value::String(style.as_str().to_string()));
    let body = toml::to_string_pretty(&doc)
        .map_err(|e| format!("frame: failed to serialize style: {e}"))?;
    std::fs::write(path, body).map_err(|e| format!("frame: failed to write {}: {e}", path.display()))
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

/// Opens the frame once at startup. Called from `main.rs`. Deferred ~40 ms
/// so Wayland has enumerated displays (frame must land on the pult, like
/// bar/panels). `apply` decides the surface set from the loaded style — no
/// strip is opened until a rail maps, and no matte is opened in Hide.
pub fn init(cx: &mut App) {
    FrameConfig::apply();
    spawn_watcher(cx);
    cx.spawn(async move |cx| {
        cx.background_executor()
            .timer(Duration::from_millis(40))
            .await;
        let _ = cx.update(|cx| {
            apply(cx);
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
        assert_eq!(cfg.style, FrameStyle::Hide);
        assert_eq!(cfg.wrap.thickness, DEFAULT_THICKNESS);
        assert_eq!(cfg.wrap.inner_radius, DEFAULT_INNER_RADIUS);
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

    // ── T284: style / wrap config ──────────────────────────────────────

    #[test]
    fn missing_style_is_hide() {
        let cfg: FrameConfig = toml::from_str("[bottom_strip]\nenabled=true\n").unwrap();
        assert_eq!(cfg.style, FrameStyle::Hide);
    }

    #[test]
    fn unknown_style_falls_back_to_hide() {
        let cfg: FrameConfig = toml::from_str("style = \"diagonal\"\n").unwrap();
        assert_eq!(cfg.style, FrameStyle::Hide);
    }

    #[test]
    fn wrap_style_parses() {
        let cfg: FrameConfig = toml::from_str("style = \"wrap\"\n").unwrap();
        assert_eq!(cfg.style, FrameStyle::Wrap);
    }

    #[test]
    fn wrap_inset_zero_in_hide_thickness_in_wrap() {
        let hide = FrameConfig {
            style: FrameStyle::Hide,
            ..FrameConfig::default()
        };
        let wrap = FrameConfig {
            style: FrameStyle::Wrap,
            wrap: WrapConfig {
                thickness: 4.0,
                ..Default::default()
            },
            ..FrameConfig::default()
        };
        assert_eq!(wrap_inset_for(&hide), 0.0);
        assert_eq!(wrap_inset_for(&wrap), 4.0);
        // No [wrap] section → default thickness drives the inset.
        let wrap_default = FrameConfig {
            style: FrameStyle::Wrap,
            ..FrameConfig::default()
        };
        assert_eq!(wrap_inset_for(&wrap_default), DEFAULT_THICKNESS);
    }

    #[test]
    fn hide_strip_wanted_false_when_no_rails() {
        assert!(!hide_strip_wanted(true, false, false));
        assert!(hide_strip_wanted(true, true, false));
        assert!(!hide_strip_wanted(false, true, true));
    }

    #[test]
    fn hide_strip_insets_one_rail() {
        assert_eq!(hide_strip_insets(true, false), (RAIL_INSET, 0.0));
        assert_eq!(hide_strip_insets(false, true), (0.0, RAIL_INSET));
        assert_eq!(hide_strip_insets(true, true), (RAIL_INSET, RAIL_INSET));
    }

    #[test]
    fn wrap_radius_clamped() {
        let mut cfg = WrapConfig::default();
        cfg.inner_radius = 99.0;
        assert_eq!(cfg.sanitized().inner_radius, MAX_RADIUS);
        cfg.inner_radius = -1.0;
        assert_eq!(cfg.sanitized().inner_radius, MIN_RADIUS);
        cfg.inner_radius = 16.0;
        assert_eq!(cfg.sanitized().inner_radius, 16.0);
    }

    #[test]
    fn wrap_thickness_clamped() {
        let mut cfg = WrapConfig::default();
        assert_eq!(cfg.thickness, DEFAULT_THICKNESS);
        cfg.thickness = 99.0;
        assert_eq!(cfg.sanitized().thickness, MAX_THICKNESS);
        cfg.thickness = -2.0;
        assert_eq!(cfg.sanitized().thickness, MIN_THICKNESS);
        cfg.thickness = 16.0;
        assert_eq!(cfg.sanitized().thickness, 16.0);
    }

    #[test]
    fn wrap_inner_rect_matches_spec() {
        let inner = wrap_inner_rect(2560.0, 1440.0, 32.0, 4.0);
        assert_eq!(inner.origin.x, 4.0);
        assert_eq!(inner.origin.y, 32.0);
        assert_eq!(inner.size.width, 2552.0);
        assert_eq!(inner.size.height, 1404.0);
    }

    #[test]
    fn wrap_geometry_changed_only_on_actual_edit() {
        let base = WrapConfig {
            thickness: 16.0,
            inner_radius: 16.0,
        };
        let same = WrapConfig {
            thickness: 16.0,
            inner_radius: 16.0,
        };
        let thicker = WrapConfig {
            thickness: 24.0,
            inner_radius: 16.0,
        };
        let rounder = WrapConfig {
            thickness: 16.0,
            inner_radius: 32.0,
        };

        // First apply (no recorded geometry yet) is a change.
        assert!(wrap_geometry_changed(None, &base));
        // Same geometry — no recreate.
        assert!(!wrap_geometry_changed(Some(&base), &same));
        // Thickness edit — recreate.
        assert!(wrap_geometry_changed(Some(&base), &thicker));
        // Radius edit — recreate (matte's ring radius repaints on recreate).
        assert!(wrap_geometry_changed(Some(&base), &rounder));
    }

    #[test]
    fn write_style_preserves_unknown_keys() {
        let dir = std::env::temp_dir().join(format!("chronos-frame-write-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("frame.toml");
        std::fs::write(
            &path,
            "[wrap]\ninner_radius = 24.0\n\n[bottom_strip]\nenabled = false\nheight = 8.0\njunction = \"flush\"\n",
        )
        .unwrap();
        write_style_at(&path, FrameStyle::Wrap).unwrap();
        let doc: toml::Value = std::fs::read_to_string(&path).unwrap().parse().unwrap();
        assert_eq!(doc["style"], toml::Value::String("wrap".into()));
        assert_eq!(doc["wrap"]["inner_radius"], toml::Value::Float(24.0));
        assert_eq!(doc["bottom_strip"]["enabled"], toml::Value::Boolean(false));
        assert_eq!(doc["bottom_strip"]["height"], toml::Value::Float(8.0));
        assert_eq!(doc["bottom_strip"]["junction"], toml::Value::String("flush".into()));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn write_style_overwrites_existing_style_key() {
        let dir = std::env::temp_dir().join(format!("chronos-frame-write-test2-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("frame.toml");
        std::fs::write(&path, "style = \"wrap\"\n").unwrap();
        write_style_at(&path, FrameStyle::Hide).unwrap();
        let doc: toml::Value = std::fs::read_to_string(&path).unwrap().parse().unwrap();
        assert_eq!(doc["style"], toml::Value::String("hide".into()));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
