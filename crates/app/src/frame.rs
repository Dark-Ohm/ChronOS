//! Desktop frame theme — the shell's two frame modes (T312):
//!
//! ## Normal (default)
//! No shell. Chrome sits flush against the screen edges: the bar at `y=0`
//! full width, the side rails at `x=0..39` / `x=2520..2559`, no plate, no
//! insets, no bottom strip. Wallpaper starts immediately past the rails.
//! This is the "every pixel for windows" mode.
//!
//! ## Wrapped
//! `style = "wrapped"` in `frame.toml`: one unified shell with an aperture.
//! A fullscreen **matte** (Layer::Top, paints only the chrome ring, hole =
//! wallpaper) plus three invisible exclusive strips L/R/B (thickness =
//! per-edge `wrap.left/right/bottom`) that push clients off the frame. The
//! bar stays top-exclusive and reads as the top edge of the frame; the side
//! rails are the frame's own edges. The bottom plate is the matte's bottom
//! border (`wrap.bottom`) — there is no separate bottom-strip surface.
//!
//! `style = "hide"` / `"wrap"` stay accepted as aliases of `normal` /
//! `wrapped` so an existing `frame.toml` never collapses to defaults.
//!
//! `frame::apply` never imports the panels — panel geometry is re-applied
//! through the `set_after_apply` hook registered in `main.rs` (one-way
//! dependency; otherwise `frame ↔ side_panel` cycle).
//!
//! Multi-monitor: everything is bound to the pult display only (same rule
//! as bar/panels).

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
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
/// Legacy default for `[bottom_strip] height` (T268). The bottom strip is no
/// longer rendered — the wrapped bottom edge is `wrap.bottom` and `normal`
/// has no bottom chrome (T312). The constant stays only so an existing
/// `frame.toml` with a `[bottom_strip]` section still parses without failing.
const DEFAULT_HEIGHT: f32 = 4.0;
const MIN_HEIGHT: f32 = 1.0;
const MAX_HEIGHT: f32 = 16.0;
/// Single source of truth for the rail width (40). The side panels re-export
/// this constant (they no longer define their own copy) and the frame uses it
/// for the matte's under-rail corner fill. Owned here so the frame never
/// imports the panels — the dependency stays one-way (panels → frame).
pub(crate) const RAIL_WIDTH: f32 = 40.0;
/// Default `wrap.inner_radius` — the inner corners of the card, not half
/// the strip height (T284 spec §3). T315 artboard: 10px = 25% of rail
/// width, visible but not dominant.
const DEFAULT_INNER_RADIUS: f32 = 10.0;
const MIN_RADIUS: f32 = 0.0;
const MAX_RADIUS: f32 = 64.0;
/// Default `wrap.left`/`wrap.right` — the chrome ring width on edges
/// WITHOUT a rail. Equal to the default `inner_radius` so the corner
/// rounding reads proportional to the frame; the old figure
/// (`bottom_strip.height`) was tuned for the thin Hide strip and made the
/// wrap frame read as a cheap 4px line (T303).
const DEFAULT_THICKNESS: f32 = 16.0;
const MIN_THICKNESS: f32 = 1.0;
const MAX_THICKNESS: f32 = 64.0;
/// Default `wrap.bottom` — the bottom plate that survives on wrap shells
/// regardless of rail mapping. T311 D3: lower than the lateral edges because
/// the bottom edge never hosts a rail and a tall strip there is pure dead
/// screen estate. T315 artboard: 12px = 40% of bar height (30), 30% of rail
/// width (40) — subordinate but not a rendering artifact (was 6, inherited
/// from Hide mode).
///
/// T318 эррата (владелец, 2026-08-19): 12 читалось всё ещё тонковато рядом
/// с рельсом в 40 и баром в 30. Привязан к `DEFAULT_THICKNESS` — низ и
/// боковое кольцо теперь одна величина, а не два независимых числа
/// (старая претензия из диагноза T315, пункт 5). Крутится живьём:
/// `wrap.bottom` в `~/.config/chronos/frame.toml`, hot-reload.
const DEFAULT_BOTTOM_THICKNESS: f32 = DEFAULT_THICKNESS;
/// Default `wrap.top` — 0 means "follow the live bar height"; a non-zero
/// value is an explicit override of the shell's top edge.
const DEFAULT_TOP: f32 = 0.0;

/// Bottom-corner junction for the old Hide strip (T268). Legacy in T312:
/// the strip is no longer rendered — the wrapped bottom edge is `wrap.bottom`
/// and `normal` has no bottom chrome. The enum and its variants stay only so
/// an existing `frame.toml` with `bottom_strip.junction` still parses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FrameJunction {
    /// Full-width strip, square ends. The strip crosses the rails' bottom
    /// edges (they sit on it).
    Flush,
    /// Strip stops at the rails' inner edges (x = RAIL_WIDTH each side);
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

/// Desktop frame style (T284, renamed in T312). Deserialized from a
/// **string** via `deserialize_style` — never `#[derive(Deserialize)]`
/// directly, otherwise an unknown value fails the whole parse and `load()`
/// silently replaces the config with defaults (the T268 junction trap).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum FrameStyle {
    /// No shell — chrome at the screen edges, no plate, no insets (T312).
    #[default]
    Normal,
    /// Perimeter card: matte + three exclusive edge strips, rails inside.
    Wrapped,
}

impl FrameStyle {
    pub fn as_str(self) -> &'static str {
        match self {
            FrameStyle::Normal => "normal",
            FrameStyle::Wrapped => "wrapped",
        }
    }

    /// Unknown values → `Normal` + warn (never panic, never silently enable
    /// Wrapped — spec §3). Case-insensitive; `hide`/`wrap` stay as aliases
    /// of `normal`/`wrapped` so an existing `frame.toml` never collapses to
    /// defaults (T268).
    pub fn from_str_sanitized(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "normal" | "hide" => FrameStyle::Normal,
            "wrapped" | "wrap" => FrameStyle::Wrapped,
            other => {
                tracing::warn!("frame: unknown style {other:?}, falling back to normal");
                FrameStyle::Normal
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

/// `[wrap]` section — per-edge thickness, inner corner radius, and the top
/// edge. T319: each edge is an explicit value (`top`/`left`/`right`/`bottom`).
/// T311 D3 semantics is preserved: a side covered by a rail reads 0 from the
/// per-edge helpers (the rail paints it), a rail-free side reads its
/// configured value, bottom always reads `bottom`, top follows the bar unless
/// `top` overrides it. Reading code uses the per-edge helpers
/// (`wrap_inset_left/right/bottom()`, `shell_top_gap()`); the raw fields stay
/// as config inputs only.
///
/// Legacy names: `thickness` (uniform side width) and `bottom_thickness` are
/// still accepted on deserialize as aliases (`thickness` → left + right,
/// `bottom_thickness` → bottom) so an existing `frame.toml` never collapses
/// to defaults (the T268 trap).
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct WrapConfig {
    /// Inner corner radius — painted by the matte (and by the bar at the
    /// top). `inner_radius = 0` disables rounding entirely.
    pub inner_radius: f32,
    /// Top edge thickness. `0` = follow the live bar height; non-zero =
    /// explicit override (bar height ignored for the shell's top edge).
    pub top: f32,
    /// Left edge thickness — applies only when the left rail is NOT mapped
    /// (a mapped rail IS the edge, T314; the config value is ignored).
    pub left: f32,
    /// Right edge thickness — mirror of `left`.
    pub right: f32,
    /// Bottom plate thickness — the rail-free edge that always carries the
    /// plate.
    pub bottom: f32,
}

impl Default for WrapConfig {
    fn default() -> Self {
        Self {
            inner_radius: DEFAULT_INNER_RADIUS,
            top: DEFAULT_TOP,
            left: DEFAULT_THICKNESS,
            right: DEFAULT_THICKNESS,
            bottom: DEFAULT_BOTTOM_THICKNESS,
        }
    }
}

/// Deserialization with legacy aliases: `thickness` feeds `left`/`right`
/// (uniform side width), `bottom_thickness` feeds `bottom`. Explicit new
/// keys win over the alias; missing keys fall back to defaults. Kept manual
/// because serde's `alias` maps one-to-one and cannot fan one key into two
/// fields.
///
/// Per-edge values are parsed leniently: a type error on ONE key (e.g.
/// `left = "много"`) maps to `None` so that key falls back to its default
/// while the rest of the section survives (T319 verification §3). A
/// structural error at the table level (e.g. `wrap = "not a table"`) still
/// propagates and drops the whole config, matching the junction behavior.
fn lenient_f32<'de, D>(deserializer: D) -> Result<Option<f32>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Ok(Option::<f32>::deserialize(deserializer).unwrap_or(None))
}

impl<'de> Deserialize<'de> for WrapConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Raw {
            #[serde(default, deserialize_with = "lenient_f32")]
            inner_radius: Option<f32>,
            #[serde(default, deserialize_with = "lenient_f32")]
            top: Option<f32>,
            #[serde(default, deserialize_with = "lenient_f32")]
            left: Option<f32>,
            #[serde(default, deserialize_with = "lenient_f32")]
            right: Option<f32>,
            #[serde(default, deserialize_with = "lenient_f32")]
            bottom: Option<f32>,
            #[serde(default, deserialize_with = "lenient_f32")]
            thickness: Option<f32>,
            #[serde(default, deserialize_with = "lenient_f32")]
            bottom_thickness: Option<f32>,
        }
        let raw = Raw::deserialize(deserializer)?;
        let d = WrapConfig::default();
        Ok(WrapConfig {
            inner_radius: raw.inner_radius.unwrap_or(d.inner_radius),
            top: raw.top.unwrap_or(d.top),
            left: raw.left.or(raw.thickness).unwrap_or(d.left),
            right: raw.right.or(raw.thickness).unwrap_or(d.right),
            bottom: raw.bottom.or(raw.bottom_thickness).unwrap_or(d.bottom),
        })
    }
}

impl WrapConfig {
    /// Clamp `top` (0 stays 0 = follow-bar sentinel), `left`/`right`/`bottom`
    /// into `MIN_THICKNESS..=MAX_THICKNESS`, and `inner_radius` into
    /// `MIN_RADIUS..=MAX_RADIUS` (0 disables rounding entirely).
    pub fn sanitized(&self) -> Self {
        let mut out = self.clone();
        out.top = clamp_top(out.top);
        out.left = clamp_thickness(out.left, "wrap.left");
        out.right = clamp_thickness(out.right, "wrap.right");
        out.bottom = clamp_thickness(out.bottom, "wrap.bottom");
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

/// `top` accepts 0 as the "follow the bar" sentinel and otherwise clamps
/// like any other edge thickness.
fn clamp_top(v: f32) -> f32 {
    if v == 0.0 {
        0.0
    } else {
        clamp_thickness(v, "wrap.top")
    }
}

fn clamp_thickness(v: f32, label: &'static str) -> f32 {
    if v < MIN_THICKNESS || v > MAX_THICKNESS {
        let clamped = v.clamp(MIN_THICKNESS, MAX_THICKNESS);
        tracing::warn!(
            "frame: {label} {v} out of range [{MIN_THICKNESS}, {MAX_THICKNESS}], clamping to {clamped}"
        );
        clamped
    } else {
        v
    }
}

/// Legacy `[bottom_strip]` section (T268). Kept so an existing `frame.toml`
/// still parses, but its fields are no longer read (T312): the wrapped bottom
/// edge is `wrap.bottom` and `normal` has no bottom chrome.
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
    /// normal | wrapped — missing key / unknown value → Normal (spec §3).
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
// The panels report whether their rail surface is mapped; the wrap inset and
// the exclusive strips' zones derive from this. Stored in an atomic so pure
// predicates can read it without an `App`.

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
/// 4px hover strip is not a rail). Re-applies the frame — the wrap strips'
/// exclusive zones follow rail mapping.
pub fn set_rail_mapped(side: FrameSide, mapped: bool, cx: &mut App) {
    let bit = rail_bit(side);
    if mapped {
        RAIL_MAPPED.fetch_or(bit, Ordering::Relaxed);
    } else {
        RAIL_MAPPED.fetch_and(!bit, Ordering::Relaxed);
    }
    apply(cx);
}

/// 0 in Normal; the rail-free edge width in Wrapped — the frame thickness
/// rails and content inset by, and the exclusive strips reserve (T284 §5,
/// T303: no longer `bottom_strip.height`).
///
/// Kept as the "rail-free edge default" — `wrap_inset_left` /
/// `wrap_inset_right` for side correctness, this for the legacy callers
/// that only need a non-sided value (e.g. tests). Deprecated for new
/// call sites — use a per-edge helper instead. Returns `wrap.left` as the
/// representative rail-free value.
pub fn wrap_inset_for(cfg: &FrameConfig) -> f32 {
    match cfg.style {
        FrameStyle::Normal => 0.0,
        FrameStyle::Wrapped => cfg.wrap.sanitized().left,
    }
}

/// `wrap_inset()` reads `wrap_inset_for(cached_config())` — the rail-free
/// default. Use per-edge helpers when the side matters.
pub fn wrap_inset() -> f32 {
    wrap_inset_for(&cached_config())
}

// ── Per-edge insets (T311 D3) ─────────────────────────────────────────────────
//
// The wrap frame is no longer a uniform ring. Top follows the bar height
// unless `wrap.top` overrides it; bottom carries `wrap.bottom` (the only
// edge with no rail counterpart); left/right collapse to 0 on the side
// that hosts a rail and stay at `wrap.left`/`wrap.right` on the rail-free
// side. The pure helpers take an explicit `*_rail_mapped` flag so unit
// tests do not need to mutate the global `RAIL_MAPPED` atomic.
//
// The four corners of the aperture ("what windows + wallpaper see") each
// land on a different surface (T311 D4):
//
// - upper-left, upper-right — painted by the side rails (`rounded_tl` /
//   `rounded_tr` at the rail root, T217). Out of frame's scope per brief.
// - lower-left, lower-right — painted by the matte itself via the single
//   `.rounded(px(radius))` on the matte div, with `border_b(inset_bottom)`
//   carrying the bottom plate. Same `wrap.inner_radius` constant drives
//   both — there is no second magic number for the lower corners.
//
// The bar's lower edge is the canonical top edge of the aperture; if its
// `appearance.radius` ever desynchronises from `wrap.inner_radius`, the
// upper corners read as a flat seam (T311 D4 — see report for the open
// follow-up: bar.rs is out of scope for this ticket).

/// The shell's effective top gap: `wrap.top` when it overrides (>0), else
/// the live bar height. In Normal there is no wrap frame, but the aperture
/// ring still needs a top extent, so this returns the bar height there too.
/// Used by the matte's aperture ring (the panels clear the bar separately
/// via `bar_height_px`).
pub fn shell_top_gap(cfg: &FrameConfig) -> f32 {
    let bar_h = crate::state::bar_height_px();
    match cfg.style {
        FrameStyle::Normal => bar_h,
        FrameStyle::Wrapped => {
            let top = cfg.wrap.sanitized().top;
            if top > 0.0 {
                top
            } else {
                bar_h
            }
        }
    }
}

/// Bottom edge — the rail-free edge that always carries the plate.
pub fn wrap_inset_bottom(cfg: &FrameConfig) -> f32 {
    match cfg.style {
        FrameStyle::Normal => 0.0,
        FrameStyle::Wrapped => cfg.wrap.sanitized().bottom,
    }
}

/// Bottom edge, reading from the cache (convenience for callers with an
/// `&App` that already knows `bottom_thickness` is rail-independent).
pub fn wrap_inset_bottom_cached() -> f32 {
    wrap_inset_bottom(&cached_config())
}

/// Left edge — 0 when left rail is mapped (the rail paints that edge),
/// otherwise `wrap.left`.
pub fn wrap_inset_left(cfg: &FrameConfig, left_rail_mapped: bool) -> f32 {
    match cfg.style {
        FrameStyle::Normal => 0.0,
        FrameStyle::Wrapped if left_rail_mapped => 0.0,
        FrameStyle::Wrapped => cfg.wrap.sanitized().left,
    }
}

/// Left edge, reading from the live cache.
pub fn wrap_inset_left_cached(left_rail_mapped: bool) -> f32 {
    wrap_inset_left(&cached_config(), left_rail_mapped)
}

/// Right edge — mirror of `wrap_inset_left`.
pub fn wrap_inset_right(cfg: &FrameConfig, right_rail_mapped: bool) -> f32 {
    match cfg.style {
        FrameStyle::Normal => 0.0,
        FrameStyle::Wrapped if right_rail_mapped => 0.0,
        FrameStyle::Wrapped => cfg.wrap.sanitized().right,
    }
}

/// Right edge, reading from the live cache.
pub fn wrap_inset_right_cached(right_rail_mapped: bool) -> f32 {
    wrap_inset_right(&cached_config(), right_rail_mapped)
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

/// T318 эррата: кольцо апертуры — обёртка вокруг скруглённого окна.
///
/// Вырез образуют четыре независимые поверхности (бар, два рельса, планка),
/// и «скруглить дыру» ни одна из них в одиночку не может. Две попытки до
/// этого были неверными по форме:
///
/// 1. `rounded_tr/br` на самом рельсе — срезает материал у кромки: рельс
///    превращается в плашку со скруглёнными краями, в углу видны обои.
///    Кривизна вывернута наизнанку.
/// 2. Четыре квадрата с одним скруглённым углом — даёт ВЫПУКЛУЮ четверть
///    круга, торчащую в вырез. Владелец назвал это «квадратные прыщи», и
///    это ровно то, что border-radius умеет: он режет углы наружу, вогнутую
///    галтель им не построить.
///
/// Работает третий способ: блок с бордером. У него скругление гнёт и
/// наружный, и ВНУТРЕННИЙ контур, причём внутренний радиус равен
/// «наружный − толщина бордера». Кладём кольцо толщиной `radius` ровно по
/// границе выреза, расширив его на ту же толщину наружу, и задаём наружный
/// радиус `2 × radius`. Внутренний контур получается скруглённым на
/// `radius` — это и есть окно с закруглёнными краями, — а наружный контур
/// кольца лежит под баром и рельсами и не виден.
///
/// Угол дисплея при этом не трогается вовсе: кольцо живёт внутри экрана.
/// The ring border must shrink to fit the available chrome: if any edge is
/// thinner than `radius`, the ring's outer contour would paint over the
/// wallpaper. Clamp to the smallest available edge (`top_gap` is the bar
/// height / `wrap.top` override). Extracted so the clamp is unit-testable
/// without building the element tree.
fn aperture_ring_border(
    radius: f32,
    inset_left: f32,
    inset_right: f32,
    inset_bottom: f32,
    top_gap: f32,
) -> f32 {
    radius
        .min(inset_left)
        .min(inset_right)
        .min(inset_bottom)
        .min(top_gap)
        .max(0.0)
}

fn aperture_ring(
    chrome: gpui::Hsla,
    inset_left: f32,
    inset_right: f32,
    inset_bottom: f32,
    bar_h: f32,
    radius: f32,
) -> Vec<gpui::AnyElement> {
    if radius <= 0.0 {
        return Vec::new();
    }
    // Кольцо не может вылезти за пределы хрома: если край тоньше радиуса,
    // наружная часть кольца легла бы на обои. Ужимаем до доступного.
    let b = aperture_ring_border(radius, inset_left, inset_right, inset_bottom, bar_h);
    if b <= 0.0 {
        return Vec::new();
    }
    vec![
        div()
            .absolute()
            .left(px(inset_left - b))
            .right(px(inset_right - b))
            .top(px(bar_h - b))
            .bottom(px(inset_bottom - b))
            .border(px(b))
            .border_color(chrome)
            .rounded(px(b + radius))
            .into_any_element(),
    ]
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
        // The matte paints what the exclusive strips reserve — per-edge
        // insets read from the same sanitized source so the ring can
        // never diverge from the geometry clients are pushed by (T303
        // second drift, fixed; T311 D3 makes the values per-edge).
        // T318 эррата: РИСУЕМ не то же, что РЕЗЕРВИРУЕМ. Резервация края с
        // мапленным рельсом равна нулю (T314) — место держит сам рельс. Но
        // краска там нужна: внутренний контур апертуры скруглён, а значит в
        // углу хрома должно становиться БОЛЬШЕ, он заполняет угол. Если на
        // этом крае не рисовать, гнуть нечего — и попытка скруглить сам
        // рельс даёт вывернутую наизнанку кривизну (плашка со срезанными
        // краями и обои в вырезе, живая находка владельца 2026-08-19).
        // Поэтому под мапленным рельсом матте кладёт бордер шириной рельса:
        // прямая часть скрыта самим рельсом (тот же токен), а скруглённый
        // внутренний контур выступает в апертуру и заполняет угол.
        // ВЕРХНИЕ углы этим не лечатся: `border_t` = 0, и угол внутреннего
        // контура матте лежит на y=0 под баром. Верх апертуры рисует бар —
        // это T316.
        let inset_left = if rail_mapped(FrameSide::Left) {
            RAIL_WIDTH
        } else {
            wrap_inset_left(&cfg, false)
        };
        let inset_right = if rail_mapped(FrameSide::Right) {
            RAIL_WIDTH
        } else {
            wrap_inset_right(&cfg, false)
        };
        let inset_bottom = wrap_inset_bottom(&cfg);
        let radius = cfg.wrap.inner_radius;

        // Per-edge insets (T311 D3) — the matte's border mirrors exactly
        // what `wrap_window_options` reserves on each side. T303's "single
        // uniform ring" trick (one constant border offset with
        // `rounded(radius + inset)` inner contour) is the right shape when
        // the ring is uniform; T311 D3 changes each side independently, so
        // we set each border individually. The previous
        // `rounded(radius + inset)` formula no longer yields a clean inner
        // contour — `.rounded(px(radius))` gives the outer corners a
        // constant radius and the close corners are reworked on D4 (the
        // bar hides the top; the bottom is recomputed; the sides collapse
        // to a zero-thickness div border where a rail is mapped). Not
        // returning to T303's "five divs with corner patches" — T303
        // explicitly removed that pattern. No top border (bar is the top
        // edge). No background — the hole stays the wallpaper (spec
        // §5.1).
        div()
            .id("frame-wrap-matte")
            .size_full()
            .relative()
            // Плоские края — отдельным слоем, без скруглений. Позиционируется
            // абсолютно от корня, поэтому кольцо ниже считает отступы от края
            // экрана, а не от padding-box с бордерами.
            .child(
                div()
                    .absolute()
                    .inset_0()
                    .when(inset_left > 0.0, |d| d.border_l(px(inset_left)))
                    .when(inset_right > 0.0, |d| d.border_r(px(inset_right)))
                    .when(inset_bottom > 0.0, |d| d.border_b(px(inset_bottom)))
                    .border_t_0()
                    .border_color(theme.bg.tertiary),
            )
            .children(aperture_ring(
                theme.bg.tertiary,
                inset_left,
                inset_right,
                inset_bottom,
                shell_top_gap(&cfg),
                radius,
            ))
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
/// The matte is fullscreen on Layer::Top with exclusive zone `-1` (the
/// wlr-layer-shell opt-out: it must NOT reserve space — a fullscreen surface
/// with `exclusive != 0` reserves the whole screen — and `-1` also stops the
/// compositor from offsetting it by sibling panels' reservations, T308); the
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
    let cfg = cached_config();
    let inset_left = wrap_inset_left(&cfg, rail_mapped(FrameSide::Left));
    let inset_right = wrap_inset_right(&cfg, rail_mapped(FrameSide::Right));
    let inset_bottom = wrap_inset_bottom(&cfg);
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
        //
        // `exclusive_zone: Some(px(-1.))` is the wlr-layer-shell opt-out
        // (T305 blood fact, control_center.rs): with `None` the compositor
        // still offsets an anchor-only surface by EVERY sibling reservation
        // — with the left rail mapped its 40px exclusive zone plus our own
        // ExclLeft 16px sum into a 56px rightward shift of the matte
        // (measured live, T308), pushing the right ring off-screen. `-1`
        // opts the matte OUT of foreign reservations entirely, and then the
        // negative bottom/left margins T303 used to counteract them are not
        // just unnecessary but harmful: they push the matte to x=-16/y=16
        // (measured live) and the ring drifts left/up by the inset. With
        // `-1` the reservations are ignored, so a plain LEFT|BOTTOM matte
        // with zero margin lands flush at x=0,y=0 (bottom edge on the
        // screen bottom) and stays there with any panel combination.
        WrapRole::Matte => (
            Size::new(px(w), px(h)),
            Anchor::LEFT | Anchor::BOTTOM,
            "frame_wrap_matte",
            Layer::Top,
            Some(px(-1.)),
            None,
            // No margin: `-1` above already opts out of the bar/ExclBottom/
            // ExclLeft and side-panel reservations, so the matte covers
            // x0-2560, y0-1440 with zero compensation.
            None,
        ),
        WrapRole::ExclLeft => (
            // T321 эррата: размер клампится до 1px, зона остаётся сырой.
            // При мапленном рельсе инсет = 0, и нулевой размер в
            // `window.open` уходит в `viewport.set_destination(0, h)` —
            // протокольное нарушение `wp_viewport`, соединение убито.
            Size::new(px(inset_left.max(1.0)), px(h)),
            Anchor::TOP | Anchor::BOTTOM | Anchor::LEFT,
            "frame_wrap_excl_left",
            Layer::Overlay,
            Some(px(inset_left)),
            Some(Anchor::LEFT),
            None,
        ),
        WrapRole::ExclRight => (
            Size::new(px(inset_right.max(1.0)), px(h)),
            Anchor::TOP | Anchor::BOTTOM | Anchor::RIGHT,
            "frame_wrap_excl_right",
            Layer::Overlay,
            Some(px(inset_right)),
            Some(Anchor::RIGHT),
            None,
        ),
        WrapRole::ExclBottom => (
            Size::new(px(w), px(inset_bottom.max(1.0))),
            Anchor::LEFT | Anchor::RIGHT | Anchor::BOTTOM,
            "frame_wrap_excl_bottom",
            Layer::Overlay,
            Some(px(inset_bottom)),
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
/// surface fails, everything already opened is rolled back and `false` is
/// returned so the caller can keep the previous style alive instead of
/// landing in a frame-less half-state (T321).
fn open_wrap_windows(cx: &mut App) -> bool {
    {
        let slots = wrap_windows().lock().unwrap_or_else(|e| e.into_inner());
        if slots.matte.is_some() {
            return true;
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
                return false;
            }
        }
    }
    let mut slots = wrap_windows().lock().unwrap_or_else(|e| e.into_inner());
    for (role, handle) in opened {
        *slots.slot(role) = Some(handle);
    }
    true
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

const STYLE_ID_NORMAL: u8 = 0;
const STYLE_ID_WRAPPED: u8 = 1;
/// Last applied style — a Normal↔Wrapped transition is the only event that
/// re-triggers panel geometry via `after_apply`.
static LAST_STYLE: AtomicU8 = AtomicU8::new(STYLE_ID_NORMAL);

/// T312: `[bottom_strip]` is legacy. Log once at startup (never per
/// hot-reload) so a user whose `frame.toml` still carries the section is
/// told why it no longer does anything.
static BOTTOM_STRIP_LEGACY_LOGGED: AtomicBool = AtomicBool::new(false);

/// Last applied sanitized wrap geometry. T321: geometry is mutated live
/// (`window.resize` + `set_exclusive_zone` on the strips, repaint on the
/// matte) — the old close+open recreate raced the compositor and dropped the
/// adapter. The tracking only decides whether a live sync is needed.
static LAST_WRAP_GEOMETRY: OnceLock<Mutex<Option<WrapConfig>>> = OnceLock::new();

fn last_wrap_geometry() -> &'static Mutex<Option<WrapConfig>> {
    LAST_WRAP_GEOMETRY.get_or_init(|| Mutex::new(None))
}

/// Pure decision: does `current` wrap geometry differ from the last applied
/// one? `None` (first apply) counts as a change. Split out of
/// `apply_wrapped` so the live-sync rule is unit-testable without an `App`.
fn wrap_geometry_changed(last: Option<&WrapConfig>, current: &WrapConfig) -> bool {
    last != Some(current)
}

/// Last rail mapping the live wrap set was synchronized with
/// (`RAIL_LEFT_BIT` / `RAIL_RIGHT_BIT`). The exclusive strips' zone values
/// follow rail mapping (T314): a mapped rail IS the frame edge, its strip
/// zone collapses to 0; a hidden rail leaves the per-edge ring.
/// Change signal for the live `set_exclusive_zone` pass — the strips are
/// never recreated on a mapping change (the T311 D3 close+open attempt
/// drowned in Hyprland `Protocol error invalid_object`).
static LAST_RAIL_MAPPING: AtomicU8 = AtomicU8::new(0);

fn rail_mapping_bits() -> u8 {
    let mut bits = 0;
    if rail_mapped(FrameSide::Left) {
        bits |= RAIL_LEFT_BIT;
    }
    if rail_mapped(FrameSide::Right) {
        bits |= RAIL_RIGHT_BIT;
    }
    bits
}

/// Pure decision: does the rail mapping differ from the last synchronized
/// one? Split out of `apply_wrapped` so the live-zone rule is unit-testable
/// without an `App`.
fn wrap_rail_mapping_changed(last: u8, current: u8) -> bool {
    last != current
}

/// T314/T321: live geometry + rail-mapping sync over the already-open wrap
/// set. The strips paint nothing and take no input — a thickness edit only
/// changes their footprint and reservation, both live-mutable
/// (`window.resize` + `set_exclusive_zone`), and the matte's ring repaints
/// from `cached_config()`. Never close+open here: the recreate path raced
/// the compositor and dropped the adapter (T321), the same class as the
/// T311 D3 close+open attempt that drowned in `Protocol error invalid_object`.
fn sync_wrap_surfaces(cx: &mut App, cfg: &FrameConfig) {
    let (w, h) = crate::monitor::pult_display_id_or_primary(cx)
        .and_then(|id| cx.find_display(id))
        .map(|d| (f32::from(d.bounds().size.width), f32::from(d.bounds().size.height)))
        .unwrap_or((1920., 1080.));

    let inset_left = wrap_inset_left(cfg, rail_mapped(FrameSide::Left));
    let inset_right = wrap_inset_right(cfg, rail_mapped(FrameSide::Right));
    let inset_bottom = wrap_inset_bottom(cfg);

    let targets = [
        (
            WrapRole::ExclLeft,
            Size::new(px(inset_left), px(h)),
            inset_left,
        ),
        (
            WrapRole::ExclRight,
            Size::new(px(inset_right), px(h)),
            inset_right,
        ),
        (
            WrapRole::ExclBottom,
            Size::new(px(w), px(inset_bottom)),
            inset_bottom,
        ),
    ];
    let mut slots = wrap_windows().lock().unwrap_or_else(|e| e.into_inner());
    for (role, size, zone) in targets {
        let Some(handle) = slots.slot(role).as_ref() else {
            continue;
        };
        // T321 эррата: НУЛЕВОЙ размер в `resize` убивает соединение.
        // Форк клампит размер только на ветке `set_geometry`
        // (`Source/gpui_linux/.../wayland/window.rs:1553`,
        // `map_size(|v| if v <= 0 { 1 } else { v })`), а в
        // `viewport.set_destination` (`:1335`) уходит СЫРОЕ значение.
        // `set_destination(0, h)` — протокольное нарушение `wp_viewport`
        // («Size was <= 0»), после которого соединение убито и шелл
        // теряет все поверхности.
        //
        // Ноль здесь штатный: при мапленном рельсе `wrap_inset_left/right`
        // равен 0 — край держит сам рельс. Значит полосе нечего
        // резервировать, но и ресайзить её в ноль нельзя: оставляем
        // футпринт как есть (она ничего не красит и не берёт ввод) и
        // двигаем только эксклюзивную зону.
        //
        // Найдено исполнителем T322 с точной строкой форка; регресс мой —
        // в приёмке T321 я проверил смену геометрии и ни разу не открыл
        // после неё панель.
        let zero = f32::from(size.width) <= 0.0 || f32::from(size.height) <= 0.0;
        match handle.update(cx, |_, window: &mut Window, _| {
            if !zero {
                window.resize(size);
            }
            window.set_exclusive_zone(px(zone));
        }) {
            Ok(()) => tracing::info!("frame: {role:?} geometry synced zone={zone}"),
            Err(e) => tracing::warn!("frame: {role:?} geometry update could not reach window ({e})"),
        }
    }
    // The matte's per-edge borders read the same geometry/mapping — repaint
    // it in the same frame so paint and reservation never diverge again
    // (the T314 defect: the matte caught up on the next unrelated redraw).
    if let Some(matte) = slots.matte.as_ref() {
        if let Err(e) = matte.update(cx, |_, _window: &mut Window, cx| cx.notify()) {
            tracing::warn!("frame: wrap matte notify could not reach window ({e})");
        }
    }
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

fn apply_normal(cx: &mut App) {
    // Normal = no shell: no plate, no bottom strip, no exclusive strips.
    // Just make sure no wrap surfaces linger from a previous mode.
    close_wrap_windows(cx);
}

fn apply_wrapped(cx: &mut App, cfg: &FrameConfig) {
    // T319: a `wrap.top` override means the bar's own height is ignored for
    // the shell's top edge. Log once per apply (not per render) so a live
    // override is visible in the log but never spams.
    if cfg.wrap.top > 0.0 {
        tracing::info!(
            top = cfg.wrap.top,
            "frame: wrap.top override active — bar.toml [appearance] height ignored for the shell top edge"
        );
    }

    // T321: open the wrap set first (idempotent). Only a normal→wrapped
    // transition or the first apply actually opens surfaces; a live
    // thickness/radius edit mutates the already-open set in place
    // (`sync_wrap_surfaces`). Opening first means a failed open leaves the
    // previous Normal state intact instead of a frame-less half-state.
    let was_open = wrap_windows()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .matte
        .is_some();
    if !open_wrap_windows(cx) {
        tracing::warn!("frame: wrap open failed — previous frame style kept");
        return;
    }

    if was_open {
        // Live geometry + rail-mapping sync — never close+open (T321).
        let geometry_changed = {
            let mut slot = last_wrap_geometry().lock().unwrap_or_else(|e| e.into_inner());
            let changed = wrap_geometry_changed(slot.as_ref(), &cfg.wrap);
            *slot = Some(cfg.wrap.clone());
            changed
        };
        let mapping_changed = {
            let current = rail_mapping_bits();
            wrap_rail_mapping_changed(LAST_RAIL_MAPPING.swap(current, Ordering::Relaxed), current)
        };
        if geometry_changed || mapping_changed {
            sync_wrap_surfaces(cx, cfg);
        }
    } else {
        // Fresh open already baked the current geometry/mapping — record them
        // so the next apply doesn't see a spurious change.
        *last_wrap_geometry().lock().unwrap_or_else(|e| e.into_inner()) = Some(cfg.wrap.clone());
        LAST_RAIL_MAPPING.store(rail_mapping_bits(), Ordering::Relaxed);
    }
}

/// Live-apply the cached config (style, strip, wrap geometry). Idempotent.
/// Called on every `frame.toml` change (300 ms debounce), on rail
/// open/close, and once at startup.
pub fn apply(cx: &mut App) {
    let cfg = cached_config();
    // T312: [bottom_strip] is legacy — log once so a user whose frame.toml
    // carries the section is told why it does nothing. The non-default check
    // comes FIRST: `swap` before it would burn the one-shot on the very first
    // apply of a clean config, and a section added later by hot-reload would
    // then be swallowed in silence (errata to the T312 report's gate).
    if cfg.bottom_strip != BottomStripConfig::default()
        && !BOTTOM_STRIP_LEGACY_LOGGED.swap(true, Ordering::Relaxed)
    {
        tracing::warn!(
            "frame: [bottom_strip] is legacy — height/junction/enabled are no longer read; the wrapped bottom edge is now wrap.bottom"
        );
    }

    let style_id = match cfg.style {
        FrameStyle::Normal => STYLE_ID_NORMAL,
        FrameStyle::Wrapped => STYLE_ID_WRAPPED,
    };
    let changed = LAST_STYLE.swap(style_id, Ordering::Relaxed) != style_id;

    match cfg.style {
        FrameStyle::Normal => apply_normal(cx),
        FrameStyle::Wrapped => apply_wrapped(cx, &cfg),
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
/// surfaces are opened in Normal; the matte + exclusive strips open in
/// Wrapped.
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
        assert_eq!(cfg.style, FrameStyle::Normal);
        assert_eq!(cfg.wrap.top, DEFAULT_TOP);
        assert_eq!(cfg.wrap.left, DEFAULT_THICKNESS);
        assert_eq!(cfg.wrap.right, DEFAULT_THICKNESS);
        assert_eq!(cfg.wrap.bottom, DEFAULT_BOTTOM_THICKNESS);
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
    fn missing_style_is_normal() {
        let cfg: FrameConfig = toml::from_str("[bottom_strip]\nenabled=true\n").unwrap();
        assert_eq!(cfg.style, FrameStyle::Normal);
    }

    #[test]
    fn unknown_style_falls_back_to_normal() {
        let cfg: FrameConfig = toml::from_str("style = \"diagonal\"\n").unwrap();
        assert_eq!(cfg.style, FrameStyle::Normal);
    }

    #[test]
    fn style_normal_and_wrapped_parse() {
        let normal: FrameConfig = toml::from_str("style = \"normal\"\n").unwrap();
        assert_eq!(normal.style, FrameStyle::Normal);
        let wrapped: FrameConfig = toml::from_str("style = \"wrapped\"\n").unwrap();
        assert_eq!(wrapped.style, FrameStyle::Wrapped);
    }

    #[test]
    fn style_legacy_aliases_parse() {
        // T312: `hide`/`wrap` are aliases of the new names — an existing
        // `frame.toml` with the old value must not collapse to defaults (T268).
        let hide: FrameConfig = toml::from_str("style = \"hide\"\n").unwrap();
        assert_eq!(hide.style, FrameStyle::Normal);
        let wrap: FrameConfig = toml::from_str("style = \"wrap\"\n").unwrap();
        assert_eq!(wrap.style, FrameStyle::Wrapped);
    }

    #[test]
    fn style_parse_is_case_insensitive() {
        for s in ["WRAP", "Wrapped", "wRaPpEd"] {
            let cfg: FrameConfig = toml::from_str(&format!("style = \"{s}\"\n")).unwrap();
            assert_eq!(cfg.style, FrameStyle::Wrapped);
        }
        let normal: FrameConfig = toml::from_str("style = \"NORMAL\"\n").unwrap();
        assert_eq!(normal.style, FrameStyle::Normal);
    }

    #[test]
    fn unknown_style_keeps_other_fields() {
        // T312: the whole point of manual `deserialize_style` — a bad `style`
        // must fall back to the default WITHOUT wiping `wrap.thickness` /
        // `bottom_strip.height` from the same TOML (the T268 trap).
        let doc = "style = \"banana\"\n[wrap]\nthickness = 24.0\ninner_radius = 12.0\n[bottom_strip]\nheight = 8.0\n";
        let cfg: FrameConfig = toml::from_str(doc).unwrap();
        assert_eq!(cfg.style, FrameStyle::Normal);
        assert_eq!(cfg.wrap.left, 24.0);
        assert_eq!(cfg.wrap.right, 24.0);
        assert_eq!(cfg.wrap.inner_radius, 12.0);
        assert_eq!(cfg.bottom_strip.height, 8.0);
    }

    #[test]
    fn wrap_inset_zero_in_normal_thickness_in_wrapped() {
        let normal = FrameConfig {
            style: FrameStyle::Normal,
            ..FrameConfig::default()
        };
        let wrapped = FrameConfig {
            style: FrameStyle::Wrapped,
            wrap: WrapConfig {
                left: 4.0,
                right: 4.0,
                ..Default::default()
            },
            ..FrameConfig::default()
        };
        assert_eq!(wrap_inset_for(&normal), 0.0);
        assert_eq!(wrap_inset_for(&wrapped), 4.0);
        // No [wrap] section → default thickness drives the inset.
        let wrapped_default = FrameConfig {
            style: FrameStyle::Wrapped,
            ..FrameConfig::default()
        };
        assert_eq!(wrap_inset_for(&wrapped_default), DEFAULT_THICKNESS);
    }

    #[test]
    fn wrap_per_edge_insets_follow_rail_mapping() {
        // T311 D3 + T319: a side only reads `wrap.left`/`wrap.right` when
        // its rail is unmapped. When the rail is open the side collapses to
        // 0 — the rail already paints that edge, the wrap must not waste
        // pixels on top. Bottom always reads `wrap.bottom`.
        let mut cfg = FrameConfig {
            style: FrameStyle::Wrapped,
            ..FrameConfig::default()
        };
        cfg.wrap.left = 16.0;
        cfg.wrap.right = 16.0;
        cfg.wrap.bottom = 6.0;

        // No rails mapped → both sides read their own value, bottom reads
        // bottom.
        assert_eq!(wrap_inset_left(&cfg, false), 16.0);
        assert_eq!(wrap_inset_right(&cfg, false), 16.0);
        assert_eq!(wrap_inset_bottom(&cfg), 6.0);

        // Left rail mapped → left collapses to 0; right unchanged.
        assert_eq!(wrap_inset_left(&cfg, true), 0.0);
        assert_eq!(wrap_inset_right(&cfg, false), 16.0);
        // Right rail mapped → right collapses to 0; left unchanged.
        assert_eq!(wrap_inset_left(&cfg, false), 16.0);
        assert_eq!(wrap_inset_right(&cfg, true), 0.0);
        // Both rails mapped → both sides 0, bottom unaffected.
        assert_eq!(wrap_inset_left(&cfg, true), 0.0);
        assert_eq!(wrap_inset_right(&cfg, true), 0.0);
        assert_eq!(wrap_inset_bottom(&cfg), 6.0);

        // Normal style → everything collapses to 0 because no matte is
        // drawn, even with both rails gone.
        let normal = FrameConfig {
            style: FrameStyle::Normal,
            ..FrameConfig::default()
        };
        assert_eq!(wrap_inset_left(&normal, false), 0.0);
        assert_eq!(wrap_inset_right(&normal, false), 0.0);
        assert_eq!(wrap_inset_bottom(&normal), 0.0);
    }

    #[test]
    fn wrap_inset_left_zero_in_normal_thickness_in_wrapped() {
        // T311 D3: legacy `wrap_inset_for` still returns the rail-free
        // thickness value (same semantics it had pre-D3), so a config
        // consumer that does not yet know about per-edge insets does not
        // see surprise. The point of the per-edge helpers is to be the
        // recommended path; this entry point is kept only for tests and
        // any external reader (settings UI, IPC) that summarizes the
        // shape.
        let wrapped = FrameConfig {
            style: FrameStyle::Wrapped,
            ..FrameConfig::default()
        };
        assert_eq!(wrap_inset_for(&wrapped), DEFAULT_THICKNESS);
        let normal = FrameConfig {
            style: FrameStyle::Normal,
            ..FrameConfig::default()
        };
        assert_eq!(wrap_inset_for(&normal), 0.0);
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
    fn wrap_per_edge_thickness_clamped() {
        // T319: every edge (left/right/bottom) shares the same
        // MIN_THICKNESS..=MAX_THICKNESS clamp; `top` keeps 0 as the
        // follow-bar sentinel and clamps any other value the same way.
        let mut cfg = WrapConfig::default();
        assert_eq!(cfg.left, DEFAULT_THICKNESS);
        assert_eq!(cfg.right, DEFAULT_THICKNESS);
        assert_eq!(cfg.bottom, DEFAULT_BOTTOM_THICKNESS);
        assert_eq!(cfg.top, DEFAULT_TOP);

        cfg.left = 99.0;
        assert_eq!(cfg.sanitized().left, MAX_THICKNESS);
        cfg.left = -2.0;
        assert_eq!(cfg.sanitized().left, MIN_THICKNESS);
        cfg.left = 16.0;
        assert_eq!(cfg.sanitized().left, 16.0);

        cfg.right = 99.0;
        assert_eq!(cfg.sanitized().right, MAX_THICKNESS);
        cfg.right = -2.0;
        assert_eq!(cfg.sanitized().right, MIN_THICKNESS);

        cfg.bottom = 0.0;
        assert_eq!(cfg.sanitized().bottom, MIN_THICKNESS);
        cfg.bottom = 6.0;
        assert_eq!(cfg.sanitized().bottom, 6.0);

        // top: 0 stays 0 (follow bar), non-zero clamps like any edge.
        cfg.top = 0.0;
        assert_eq!(cfg.sanitized().top, 0.0);
        cfg.top = 99.0;
        assert_eq!(cfg.sanitized().top, MAX_THICKNESS);
        cfg.top = -1.0;
        assert_eq!(cfg.sanitized().top, MIN_THICKNESS);
    }

    #[test]
    fn legacy_thickness_aliases_still_read() {
        // T319: `thickness` and `bottom_thickness` must keep reading as
        // aliases (thickness → left+right, bottom_thickness → bottom) so an
        // existing `frame.toml` never collapses to defaults (T268 trap).
        let doc = "[wrap]\nthickness = 24.0\nbottom_thickness = 6.0\ninner_radius = 12.0\n";
        let cfg: FrameConfig = toml::from_str(doc).unwrap();
        assert_eq!(cfg.wrap.left, 24.0);
        assert_eq!(cfg.wrap.right, 24.0);
        assert_eq!(cfg.wrap.bottom, 6.0);
        assert_eq!(cfg.wrap.inner_radius, 12.0);
        assert_eq!(cfg.wrap.top, DEFAULT_TOP);
    }

    #[test]
    fn explicit_new_keys_win_over_aliases() {
        // T319: an explicit `left`/`bottom` beats the legacy alias for that
        // edge; the alias still feeds the other (unset) side.
        let doc = "[wrap]\nthickness = 24.0\nleft = 30.0\nbottom = 8.0\n";
        let cfg: FrameConfig = toml::from_str(doc).unwrap();
        assert_eq!(cfg.wrap.left, 30.0);
        assert_eq!(cfg.wrap.right, 24.0);
        assert_eq!(cfg.wrap.bottom, 8.0);
    }

    #[test]
    fn missing_wrap_keys_parse_to_defaults() {
        // T319/T311 D3: a `[wrap]` section with only some keys must NOT
        // fail parse — missing keys fall back to defaults (T268).
        let doc = "[wrap]\nleft = 20.0\n";
        let cfg: FrameConfig = toml::from_str(doc).unwrap();
        assert_eq!(cfg.wrap.left, 20.0);
        assert_eq!(cfg.wrap.right, DEFAULT_THICKNESS);
        assert_eq!(cfg.wrap.bottom, DEFAULT_BOTTOM_THICKNESS);
        assert_eq!(cfg.wrap.top, DEFAULT_TOP);
        assert_eq!(cfg.wrap.inner_radius, DEFAULT_INNER_RADIUS);
    }

    #[test]
    fn garbage_wrap_value_drops_only_that_key() {
        // T319 verification §3: a garbage `left` falls back to that key's
        // default while every other key and section survives.
        let doc = "[wrap]\nleft = \"много\"\nright = 22.0\nbottom = 7.0\n\n[bottom_strip]\nenabled = false\n";
        let cfg: FrameConfig = toml::from_str(doc).unwrap();
        assert_eq!(cfg.wrap.left, DEFAULT_THICKNESS);
        assert_eq!(cfg.wrap.right, 22.0);
        assert_eq!(cfg.wrap.bottom, 7.0);
        assert!(!cfg.bottom_strip.enabled);
    }

    #[test]
    fn aperture_ring_border_clamps_to_smallest_edge() {
        // T319: when an edge is thinner than the radius, the ring's outer
        // contour would paint on the wallpaper — clamp to the smallest
        // available edge. `top_gap` is the bar height / `wrap.top` override.
        let radius = 10.0;
        assert_eq!(aperture_ring_border(radius, 16.0, 16.0, 16.0, 30.0), 10.0);
        assert_eq!(aperture_ring_border(radius, 16.0, 16.0, 4.0, 30.0), 4.0);
        assert_eq!(aperture_ring_border(radius, 4.0, 16.0, 16.0, 30.0), 4.0);
        assert_eq!(aperture_ring_border(radius, 16.0, 2.0, 16.0, 30.0), 2.0);
        assert_eq!(aperture_ring_border(radius, 16.0, 16.0, 16.0, 8.0), 8.0);
        assert_eq!(aperture_ring_border(radius, 0.0, 16.0, 16.0, 30.0), 0.0);
    }

    #[test]
    fn shell_top_gap_follows_bar_then_overrides() {
        // T319: top = 0 → the live bar height; top > 0 → explicit override.
        // Normal has no wrap frame but panels still clear the bar.
        let bar = crate::state::bar_height_px();
        let follow = FrameConfig {
            style: FrameStyle::Wrapped,
            wrap: WrapConfig {
                top: 0.0,
                ..Default::default()
            },
            ..FrameConfig::default()
        };
        assert_eq!(shell_top_gap(&follow), bar);

        let overridden = FrameConfig {
            style: FrameStyle::Wrapped,
            wrap: WrapConfig {
                top: 42.0,
                ..Default::default()
            },
            ..FrameConfig::default()
        };
        assert_eq!(shell_top_gap(&overridden), 42.0);

        let normal = FrameConfig {
            style: FrameStyle::Normal,
            ..FrameConfig::default()
        };
        assert_eq!(shell_top_gap(&normal), bar);
    }

    #[test]
    fn bottom_thickness_round_trips_on_write() {
        // T311 D3: writing the `style` key (the `write_style` codepath used
        // by settings tab toggles) must NOT wipe a non-default
        // `bottom_thickness` in the surrounding file — same hygiene rule as
        // the existing `write_style_preserves_unknown_keys` test.
        let dir = std::env::temp_dir()
            .join(format!("chronos-frame-bt-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("frame.toml");
        std::fs::write(
            &path,
            "[wrap]\nthickness = 16.0\ninner_radius = 16.0\nbottom_thickness = 4.0\n",
        )
        .unwrap();
        write_style_at(&path, FrameStyle::Wrapped).unwrap();
        let doc: toml::Value =
            std::fs::read_to_string(&path).unwrap().parse().unwrap();
        assert_eq!(doc["style"], toml::Value::String("wrapped".into()));
        assert_eq!(
            doc["wrap"]["bottom_thickness"],
            toml::Value::Float(4.0)
        );
        let _ = std::fs::remove_dir_all(&dir);
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
            left: 16.0,
            right: 16.0,
            inner_radius: 16.0,
            bottom: 6.0,
            ..Default::default()
        };
        let same = WrapConfig {
            left: 16.0,
            right: 16.0,
            inner_radius: 16.0,
            bottom: 6.0,
            ..Default::default()
        };
        let thicker = WrapConfig {
            left: 24.0,
            right: 24.0,
            inner_radius: 16.0,
            bottom: 6.0,
            ..Default::default()
        };
        let rounder = WrapConfig {
            left: 16.0,
            right: 16.0,
            inner_radius: 32.0,
            bottom: 6.0,
            ..Default::default()
        };
        let slimmer_bottom = WrapConfig {
            left: 16.0,
            right: 16.0,
            inner_radius: 16.0,
            bottom: 4.0,
            ..Default::default()
        };

        // First apply (no recorded geometry yet) is a change.
        assert!(wrap_geometry_changed(None, &base));
        // Same geometry — no recreate.
        assert!(!wrap_geometry_changed(Some(&base), &same));
        // Thickness edit — recreate.
        assert!(wrap_geometry_changed(Some(&base), &thicker));
        // Radius edit — recreate (matte's ring radius repaints on recreate).
        assert!(wrap_geometry_changed(Some(&base), &rounder));
        // Bottom-thickness edit (T311 D3) — recreate.
        assert!(wrap_geometry_changed(Some(&base), &slimmer_bottom));
    }

    #[test]
    fn wrap_rail_mapping_changed_tracks_real_transitions() {
        // Steady state: the same mapping re-applied (hot-reload churn,
        // style transitions, repeated `apply`) must NOT trigger the live
        // zone pass.
        assert!(!wrap_rail_mapping_changed(0, 0));
        assert!(!wrap_rail_mapping_changed(
            RAIL_LEFT_BIT | RAIL_RIGHT_BIT,
            RAIL_LEFT_BIT | RAIL_RIGHT_BIT
        ));

        // A rail maps — its strip's zone collapses to 0 (the rail becomes
        // the frame edge).
        assert!(wrap_rail_mapping_changed(0, RAIL_LEFT_BIT));
        assert!(wrap_rail_mapping_changed(0, RAIL_RIGHT_BIT));

        // A rail hides — the ring returns to that edge.
        assert!(wrap_rail_mapping_changed(RAIL_LEFT_BIT, 0));
        assert!(wrap_rail_mapping_changed(RAIL_RIGHT_BIT, 0));

        // The second rail maps while the first stays — only the new edge
        // changes, but the pass is per-set, so it must run.
        assert!(wrap_rail_mapping_changed(
            RAIL_LEFT_BIT,
            RAIL_LEFT_BIT | RAIL_RIGHT_BIT
        ));
        // Mirror: one of two mapped rails hides.
        assert!(wrap_rail_mapping_changed(
            RAIL_LEFT_BIT | RAIL_RIGHT_BIT,
            RAIL_RIGHT_BIT
        ));

        // Both edges flip in one transition (left hides, right maps) —
        // still a single pass over both strips.
        assert!(wrap_rail_mapping_changed(RAIL_LEFT_BIT, RAIL_RIGHT_BIT));
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
        write_style_at(&path, FrameStyle::Wrapped).unwrap();
        let doc: toml::Value = std::fs::read_to_string(&path).unwrap().parse().unwrap();
        assert_eq!(doc["style"], toml::Value::String("wrapped".into()));
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
        write_style_at(&path, FrameStyle::Normal).unwrap();
        let doc: toml::Value = std::fs::read_to_string(&path).unwrap().parse().unwrap();
        assert_eq!(doc["style"], toml::Value::String("normal".into()));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
