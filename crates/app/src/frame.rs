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
/// the strip height (T284 spec §3). T315 artboard: 10px = 25% of rail
/// width, visible but not dominant.
const DEFAULT_INNER_RADIUS: f32 = 10.0;
const MIN_RADIUS: f32 = 0.0;
const MAX_RADIUS: f32 = 64.0;
/// Default `wrap.thickness` — the chrome ring width on edges WITHOUT a
/// rail. Equal to the default `inner_radius` so the corner rounding reads
/// proportional to the frame; the old figure (`bottom_strip.height`) was
/// tuned for the thin Hide strip and made the wrap frame read as a cheap
/// 4px line (T303).
const DEFAULT_THICKNESS: f32 = 16.0;
const MIN_THICKNESS: f32 = 1.0;
const MAX_THICKNESS: f32 = 64.0;
/// Default `wrap.bottom_thickness` — the bottom plate that survives on
/// wrap shells regardless of rail mapping. T311 D3: lower than the
/// lateral edges because the bottom edge never hosts a rail and a tall
/// strip there is pure dead screen estate. T315 artboard: 12px =
/// 40% of bar height (30), 30% of rail width (40) — subordinate
/// but not a rendering artifact (was 6, inherited from Hide mode).
///
/// T318 эррата (владелец, 2026-08-19): 12 читалось всё ещё тонковато рядом
/// с рельсом в 40 и баром в 30. Привязан к `DEFAULT_THICKNESS` — низ и
/// боковое кольцо теперь одна величина, а не два независимых числа
/// (старая претензия из диагноза T315, пункт 5). Крутится живьём:
/// `wrap.bottom_thickness` в `~/.config/chronos/frame.toml`, hot-reload.
const DEFAULT_BOTTOM_THICKNESS: f32 = DEFAULT_THICKNESS;

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

/// `[wrap]` section — frame thickness, bottom-plate thickness and inner
/// corner radius. T311 D3: thickness semantics is **per-edge** — the
/// running frame has zero thickness on an edge covered by a rail, full
/// `thickness` on an edge without a rail, `bottom_thickness` along the
/// bottom (rail-free edges) and zero at the top (the bar is the top
/// edge). All three fields exist for compatibility and to express the
/// distinct roles; reading code uses the per-edge helpers
/// (`wrap_inset_left/right/bottom()`), the raw fields stay as config
/// inputs only.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct WrapConfig {
    /// Chromium ring width on rail-free edges.
    pub thickness: f32,
    /// Inner corner radius — painted by the matte (and by the bar at the
    /// top). `inner_radius = 0` disables rounding entirely.
    pub inner_radius: f32,
    /// Bottom-plate thickness — lower than `thickness` because the
    /// bottom edge never carries a rail, and is the only remaining
    /// lateral plate after rail sides collapse (T311 D3).
    pub bottom_thickness: f32,
}

impl Default for WrapConfig {
    fn default() -> Self {
        Self {
            thickness: DEFAULT_THICKNESS,
            inner_radius: DEFAULT_INNER_RADIUS,
            bottom_thickness: DEFAULT_BOTTOM_THICKNESS,
        }
    }
}

impl WrapConfig {
    /// Clamp `thickness` and `bottom_thickness` into
    /// `MIN_THICKNESS..=MAX_THICKNESS` and `inner_radius` into
    /// `MIN_RADIUS..=MAX_RADIUS` (0 disables the corner rounding entirely).
    pub fn sanitized(&self) -> Self {
        let mut out = self.clone();
        out.thickness = clamp_thickness(out.thickness, "wrap.thickness");
        out.bottom_thickness =
            clamp_thickness(out.bottom_thickness, "wrap.bottom_thickness");
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
///
/// Kept as the "rail-free edge default" — `wrap_inset_left` /
/// `wrap_inset_right` for side correctness, this for the legacy callers
/// that only need a non-sided value (e.g. tests). Deprecated for new
/// call sites — use a per-edge helper instead.
pub fn wrap_inset_for(cfg: &FrameConfig) -> f32 {
    match cfg.style {
        FrameStyle::Hide => 0.0,
        FrameStyle::Wrap => cfg.wrap.sanitized().thickness,
    }
}

/// `wrap_inset()` reads `wrap_inset_for(cached_config())` — the rail-free
/// default. Use per-edge helpers when the side matters.
pub fn wrap_inset() -> f32 {
    wrap_inset_for(&cached_config())
}

// ── Per-edge insets (T311 D3) ─────────────────────────────────────────────────
//
// The wrap frame is no longer a uniform ring. Top always collapses to 0
// (the bar paints the top edge); bottom carries `bottom_thickness` (the
// only edge with no rail counterpart); left/right collapse to 0 on the
// side that hosts a rail and stay at `wrap.thickness` on the rail-free
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

/// Top edge — always 0; the bar is the top edge of the chrome, not the
/// frame.
pub fn wrap_inset_top() -> f32 {
    0.0
}

/// Bottom edge — the rail-free edge that always carries the plate.
pub fn wrap_inset_bottom(cfg: &FrameConfig) -> f32 {
    match cfg.style {
        FrameStyle::Hide => 0.0,
        FrameStyle::Wrap => cfg.wrap.sanitized().bottom_thickness,
    }
}

/// Bottom edge, reading from the cache (convenience for callers with an
/// `&App` that already knows `bottom_thickness` is rail-independent).
pub fn wrap_inset_bottom_cached() -> f32 {
    wrap_inset_bottom(&cached_config())
}

/// Left edge — 0 when left rail is mapped (the rail paints that edge),
/// otherwise `wrap.thickness`.
pub fn wrap_inset_left(cfg: &FrameConfig, left_rail_mapped: bool) -> f32 {
    match cfg.style {
        FrameStyle::Hide => 0.0,
        FrameStyle::Wrap if left_rail_mapped => 0.0,
        FrameStyle::Wrap => cfg.wrap.sanitized().thickness,
    }
}

/// Left edge, reading from the live cache.
pub fn wrap_inset_left_cached(left_rail_mapped: bool) -> f32 {
    wrap_inset_left(&cached_config(), left_rail_mapped)
}

/// Right edge — mirror of `wrap_inset_left`.
pub fn wrap_inset_right(cfg: &FrameConfig, right_rail_mapped: bool) -> f32 {
    match cfg.style {
        FrameStyle::Hide => 0.0,
        FrameStyle::Wrap if right_rail_mapped => 0.0,
        FrameStyle::Wrap => cfg.wrap.sanitized().thickness,
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
    let b = radius
        .min(inset_left)
        .min(inset_right)
        .min(inset_bottom)
        .min(bar_h)
        .max(0.0);
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
            RAIL_INSET
        } else {
            wrap_inset_left(&cfg, false)
        };
        let inset_right = if rail_mapped(FrameSide::Right) {
            RAIL_INSET
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
                crate::state::bar_height_px(),
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
            Size::new(px(inset_left), px(h)),
            Anchor::TOP | Anchor::BOTTOM | Anchor::LEFT,
            "frame_wrap_excl_left",
            Layer::Overlay,
            Some(px(inset_left)),
            Some(Anchor::LEFT),
            None,
        ),
        WrapRole::ExclRight => (
            Size::new(px(inset_right), px(h)),
            Anchor::TOP | Anchor::BOTTOM | Anchor::RIGHT,
            "frame_wrap_excl_right",
            Layer::Overlay,
            Some(px(inset_right)),
            Some(Anchor::RIGHT),
            None,
        ),
        WrapRole::ExclBottom => (
            Size::new(px(w), px(inset_bottom)),
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

/// Last rail mapping the live wrap set was synchronized with
/// (`RAIL_LEFT_BIT` / `RAIL_RIGHT_BIT`). The exclusive strips' zone values
/// follow rail mapping (T314): a mapped rail IS the frame edge, its strip
/// zone collapses to 0; a hidden rail leaves the `wrap.thickness` ring.
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
/// one? Split out of `apply_wrap` so the live-zone rule is unit-testable
/// without an `App`.
fn wrap_rail_mapping_changed(last: u8, current: u8) -> bool {
    last != current
}

/// T314: live exclusive-zone pass over the already-open strips — the only
/// mutation a rail mapping change performs. Never close+open here. The
/// strips paint nothing and take no input, so the zone number alone
/// describes the reservation (wlr-layer-shell `set_exclusive_zone` is a
/// value, independent from the surface's pixel footprint — same contract
/// as `side_panel_right/mod.rs:272`).
fn sync_wrap_excl_zones(cx: &mut App, cfg: &FrameConfig) {
    let targets = [
        (
            WrapRole::ExclLeft,
            wrap_inset_left(cfg, rail_mapped(FrameSide::Left)),
        ),
        (
            WrapRole::ExclRight,
            wrap_inset_right(cfg, rail_mapped(FrameSide::Right)),
        ),
    ];
    let mut slots = wrap_windows().lock().unwrap_or_else(|e| e.into_inner());
    for (role, zone) in targets {
        let Some(handle) = slots.slot(role).as_ref() else {
            continue;
        };
        match handle.update(cx, |_, window: &mut Window, _| {
            window.set_exclusive_zone(px(zone));
        }) {
            Ok(()) => tracing::info!("frame: {role:?} exclusive zone set to {zone}"),
            Err(e) => tracing::warn!("frame: {role:?} zone update could not reach window ({e})"),
        }
    }
    // The matte's per-edge borders read the same rail mapping — repaint it
    // in the same frame so paint and reservation never diverge again
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
    // T314: rail mapping is the second live signal. The recreate path bakes
    // the current mapping into the new surfaces; a mapping-only change runs
    // the live zone pass on the existing set instead (never close+open).
    let mapping_changed = {
        let current = rail_mapping_bits();
        wrap_rail_mapping_changed(LAST_RAIL_MAPPING.swap(current, Ordering::Relaxed), current)
    };

    if geometry_changed {
        close_wrap_windows(cx);
    } else if mapping_changed {
        sync_wrap_excl_zones(cx, cfg);
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
        assert_eq!(cfg.wrap.bottom_thickness, DEFAULT_BOTTOM_THICKNESS);
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
    fn wrap_per_edge_insets_follow_rail_mapping() {
        // T311 D3: the only way a side reads `wrap.thickness` is when its
        // rail is unmapped. When the rail is open the side collapses to 0
        // — the rail already paints that edge, the wrap must not waste
        // pixels on top.
        let mut cfg = FrameConfig {
            style: FrameStyle::Wrap,
            ..FrameConfig::default()
        };
        cfg.wrap.thickness = 16.0;
        cfg.wrap.bottom_thickness = 6.0;

        // No rails mapped → both sides read thickness, bottom reads
        // bottom_thickness, top reads 0 (bar covers it).
        assert_eq!(wrap_inset_left(&cfg, false), 16.0);
        assert_eq!(wrap_inset_right(&cfg, false), 16.0);
        assert_eq!(wrap_inset_bottom(&cfg), 6.0);
        assert_eq!(wrap_inset_top(), 0.0);

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

        // Hide style → everything collapses to 0 because no matte is
        // drawn, even with both rails gone.
        let hide = FrameConfig {
            style: FrameStyle::Hide,
            ..FrameConfig::default()
        };
        assert_eq!(wrap_inset_left(&hide, false), 0.0);
        assert_eq!(wrap_inset_right(&hide, false), 0.0);
        assert_eq!(wrap_inset_bottom(&hide), 0.0);
    }

    #[test]
    fn wrap_inset_left_zero_in_hide_thickness_in_wrap() {
        // T311 D3: legacy `wrap_inset_for` still returns the rail-free
        // thickness value (same semantics it had pre-D3), so a config
        // consumer that does not yet know about per-edge insets does not
        // see surprise. The point of the per-edge helpers is to be the
        // recommended path; this entry point is kept only for tests and
        // any external reader (settings UI, IPC) that summarizes the
        // shape.
        let wrap = FrameConfig {
            style: FrameStyle::Wrap,
            ..FrameConfig::default()
        };
        assert_eq!(wrap_inset_for(&wrap), DEFAULT_THICKNESS);
        let hide = FrameConfig {
            style: FrameStyle::Hide,
            ..FrameConfig::default()
        };
        assert_eq!(wrap_inset_for(&hide), 0.0);
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
        assert_eq!(cfg.bottom_thickness, DEFAULT_BOTTOM_THICKNESS);
        cfg.thickness = 99.0;
        assert_eq!(cfg.sanitized().thickness, MAX_THICKNESS);
        cfg.thickness = -2.0;
        assert_eq!(cfg.sanitized().thickness, MIN_THICKNESS);
        cfg.thickness = 16.0;
        assert_eq!(cfg.sanitized().thickness, 16.0);
    }

    #[test]
    fn wrap_bottom_thickness_clamped() {
        // T311 D3: bottom_thickness shares the same `MIN_THICKNESS..=
        // MAX_THICKNESS` clamp as `thickness` — same accept range, separate
        // value.
        let mut cfg = WrapConfig::default();
        cfg.bottom_thickness = 99.0;
        assert_eq!(cfg.sanitized().bottom_thickness, MAX_THICKNESS);
        cfg.bottom_thickness = 0.0;
        assert_eq!(cfg.sanitized().bottom_thickness, MIN_THICKNESS);
        cfg.bottom_thickness = 6.0;
        assert_eq!(cfg.sanitized().bottom_thickness, 6.0);
    }

    #[test]
    fn missing_bottom_thickness_parses_to_default() {
        // T311 D3: a `[wrap]` section without `bottom_thickness` must NOT
        // fail parse — old `frame.toml` files keep loading, the new field
        // silently gets the default. The T268 incident makes this a hard
        // requirement (whole-file fallback to defaults on any parse miss).
        let doc = "[wrap]\nthickness = 24.0\ninner_radius = 12.0\n";
        let cfg: FrameConfig = toml::from_str(doc).unwrap();
        assert_eq!(cfg.wrap.thickness, 24.0);
        assert_eq!(cfg.wrap.inner_radius, 12.0);
        assert_eq!(cfg.wrap.bottom_thickness, DEFAULT_BOTTOM_THICKNESS);
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
        write_style_at(&path, FrameStyle::Wrap).unwrap();
        let doc: toml::Value =
            std::fs::read_to_string(&path).unwrap().parse().unwrap();
        assert_eq!(doc["style"], toml::Value::String("wrap".into()));
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
            thickness: 16.0,
            inner_radius: 16.0,
            bottom_thickness: 6.0,
        };
        let same = WrapConfig {
            thickness: 16.0,
            inner_radius: 16.0,
            bottom_thickness: 6.0,
        };
        let thicker = WrapConfig {
            thickness: 24.0,
            inner_radius: 16.0,
            bottom_thickness: 6.0,
        };
        let rounder = WrapConfig {
            thickness: 16.0,
            inner_radius: 32.0,
            bottom_thickness: 6.0,
        };
        let slimmer_bottom = WrapConfig {
            thickness: 16.0,
            inner_radius: 16.0,
            bottom_thickness: 4.0,
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
