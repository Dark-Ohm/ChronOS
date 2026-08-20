//! System settings — Bar page (T202). All controls inlined.
//!
//! T231: visual redesign — the full-width panel (up to MAX_WIDTH=960px) no
//! longer reads as a debug menu. What changed:
//! - Appearance block is a **responsive grid**: 2 columns on wide panels,
//!   1 column at/below `GRID_BREAKPOINT` (default docked width stays 1-col).
//! - Hypr modules render as a compact 2-3 column grid, not a wall of rows.
//! - Visual hierarchy: section headers (accent tick + semibold title + mono
//!   subtitle) vs setting labels (label + mono path).
//! - Controls: sliders with a thick track + bordered/shadowed thumb,
//!   `-`/`+` step buttons with borders, segmented chips with accent state.
//! - The whole content sits on a `theme.bg.elevated` card with the theme's
//!   elevation language (`elevation_popup` + `elevation_apply_light_chrome`).
//!
//! Behavior is untouched (T231 is visual only): `persist` still writes
//! through `bar_settings::apply_patch` (widgets/version survive), preset ids
//! stay `&'static str`, slider drag math is unchanged, "Open" still bumps the
//! `PreviewTarget` global.

use std::path::PathBuf;

use chronos_ui::{Theme, ThemeScheme, builtin_schemes, elevation_apply_light_chrome};
use gpui::{
    App, AnyElement, BoxShadow, ClickEvent, Context, DragMoveEvent, EmptyView, FontWeight,
    InteractiveElement, IntoElement, ParentElement, Render, ScrollHandle, SharedString, Styled,
    Window, div, prelude::*, px,
};

use crate::bar_settings::{
    BarSettingsPatch, EdgeChoice, ElevationChoice, PRESETS, WidthChoice, apply_patch, apply_preset,
    config_path, read_current,
};
use crate::side_panel_right::preview_target::{PreviewIntent, PreviewTarget};
use super::ui::{
    NoteSeverity, empty_state_note, is_wide, section_header, setting_label, setting_row,
};

// ── Geometry ────────────────────────────────────────────────────────────────

/// Slider geometry — thicker than the old 4px line (T231 verdict: "низкая
/// affordance"). Track 6px, thumb 16px with border + drop shadow.
const SLIDER_TW: f32 = 110.0;
const SLIDER_TRACK_H: f32 = 6.0;
const SLIDER_THUMB: f32 = 16.0;

// ── Value ranges (unchanged from T202; page clamps in the same places) ──────
const HEIGHT_MIN: f32 = 20.0;
const HEIGHT_MAX: f32 = 48.0;
const RADIUS_MAX: f32 = 16.0;
/// T266 alpha step for the −/+ buttons (the slider itself is continuous).
const ALPHA_STEP: f32 = 0.05;

// ── Drag markers ────────────────────────────────────────────────────────────
/// Own marker types so Height/Radius/Alpha drags never cross-fire.
pub struct HeightSliderDrag;
pub struct RadiusSliderDrag;
pub struct SurfaceAlphaSliderDrag;

// ── State ───────────────────────────────────────────────────────────────────

pub struct BarSettingsTab {
    current: BarSettingsPatch,
    error: Option<String>,
    applied_preset: Option<&'static str>,
    scroll: ScrollHandle,
    /// T196: cached Hypr module listing (name, path). Lazily loaded on first render.
    hypr_modules: Vec<(String, PathBuf)>,
    hypr_modules_loaded: bool,
    /// T266: keeps this page repainting when the background blur probe lands
    /// (the toggle flips from disabled to enabled/reason). Dropping it would
    /// silently freeze the toggle at the pre-probe state.
    _surface_effects_sub: gpui::Subscription,
}

impl BarSettingsTab {
    pub fn new(cx: &mut Context<Self>) -> Self {
        // T266: the blur global is installed by `surface_effects::init` in
        // main.rs before any panel can open — observe it to repaint on the
        // background probe result.
        let sub = cx.observe_global::<crate::surface_effects::SurfaceEffectsState>(|_, cx| {
            cx.notify();
        });
        Self {
            current: read_current(),
            error: None,
            applied_preset: None,
            scroll: ScrollHandle::new(),
            hypr_modules: Vec::new(),
            hypr_modules_loaded: false,
            _surface_effects_sub: sub,
        }
    }

    fn load_hypr_modules(&mut self) {
        if self.hypr_modules_loaded {
            return;
        }
        self.hypr_modules_loaded = true;
        let dir = match dirs::config_dir() {
            Some(d) => d.join("hypr/modules"),
            None => return,
        };
        let entries = match std::fs::read_dir(&dir) {
            Ok(e) => e,
            Err(_) => return,
        };
        let mut modules: Vec<(String, PathBuf)> = Vec::new();
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map_or(false, |e| e == "lua") {
                let name = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("unknown")
                    .to_string();
                modules.push((name, path));
            }
        }
        modules.sort_by(|a, b| a.0.cmp(&b.0));
        self.hypr_modules = modules;
    }

    /// Persist through `bar_settings::apply_patch` (raw toml edit — widgets and
    /// unknown keys survive, `version` is forced to 2). Errors surface in the
    /// banner, never panic.
    fn persist(&mut self, cx: &mut Context<Self>) {
        match apply_patch(&self.current) {
            Ok(()) => self.error = None,
            Err(e) => self.error = Some(e),
        }
        cx.notify();
    }

    fn apply_preset_id(&mut self, id: &'static str, cx: &mut Context<Self>) {
        match apply_preset(id) {
            Ok(p) => {
                self.current = p.appearance;
                self.applied_preset = Some(id);
                self.error = None;
            }
            Err(e) => self.error = Some(e),
        }
        cx.notify();
    }
}

// ── Render ──────────────────────────────────────────────────────────────────

impl Render for BarSettingsTab {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = *Theme::global(cx);
        let elev = theme.elevation_popup();

        // ── Responsive breakpoint (T231) ─────────────────────────────
        let is_wide = is_wide(cx);

        // ── Drag listeners (pattern: volume_popup, logic unchanged) ──
        let height_drag = cx.listener(
            move |this, ev: &DragMoveEvent<HeightSliderDrag>, _w, cx: &mut Context<BarSettingsTab>| {
                let frac = slider_frac(
                    f32::from(ev.event.position.x - ev.bounds.origin.x),
                    f32::from(ev.bounds.size.width),
                );
                this.current.height = (HEIGHT_MIN + frac * (HEIGHT_MAX - HEIGHT_MIN))
                    .clamp(HEIGHT_MIN, HEIGHT_MAX);
                this.persist(cx);
            },
        );
        let radius_drag = cx.listener(
            move |this, ev: &DragMoveEvent<RadiusSliderDrag>, _w, cx: &mut Context<BarSettingsTab>| {
                let frac = slider_frac(
                    f32::from(ev.event.position.x - ev.bounds.origin.x),
                    f32::from(ev.bounds.size.width),
                );
                this.current.radius = (frac * RADIUS_MAX).clamp(0.0, RADIUS_MAX);
                this.persist(cx);
            },
        );
        // T266: alpha slider — live-applies on every drag sample (no
        // «apply» step). The theme watcher may reapply after its debounce;
        // both paths are idempotent because the same value is persisted.
        let alpha_drag = cx.listener(
            move |this, ev: &DragMoveEvent<SurfaceAlphaSliderDrag>, _w, cx: &mut Context<BarSettingsTab>| {
                let frac = slider_frac(
                    f32::from(ev.event.position.x - ev.bounds.origin.x),
                    f32::from(ev.bounds.size.width),
                );
                let floor = Theme::global(cx).surface.min_alpha;
                let alpha = alpha_from_frac(frac, floor);
                if let Err(e) = crate::theme_config::persist_surface_alpha(alpha) {
                    this.error = Some(e);
                }
                crate::theme_config::apply(cx);
                cx.notify();
            },
        );

        // ── Click handlers (logic unchanged) ──────────────────────────
        let hs = ((HEIGHT_MAX - HEIGHT_MIN) / 10.0).max(1.0);
        let rs = (RADIUS_MAX / 10.0).max(1.0);

        let h_minus = cx.listener(move |this, _ev, _w, cx: &mut Context<BarSettingsTab>| {
            this.current.height = (this.current.height - hs).clamp(HEIGHT_MIN, HEIGHT_MAX);
            this.persist(cx);
        });
        let h_plus = cx.listener(move |this, _ev, _w, cx: &mut Context<BarSettingsTab>| {
            this.current.height = (this.current.height + hs).clamp(HEIGHT_MIN, HEIGHT_MAX);
            this.persist(cx);
        });
        let r_minus = cx.listener(move |this, _ev, _w, cx: &mut Context<BarSettingsTab>| {
            this.current.radius = (this.current.radius - rs).clamp(0.0, RADIUS_MAX);
            this.persist(cx);
        });
        let r_plus = cx.listener(move |this, _ev, _w, cx: &mut Context<BarSettingsTab>| {
            this.current.radius = (this.current.radius + rs).clamp(0.0, RADIUS_MAX);
            this.persist(cx);
        });
        // T266 alpha −/+ step buttons — same live-apply path as the drag.
        let a_minus = cx.listener(move |this, _ev, _w, cx: &mut Context<BarSettingsTab>| {
            let floor = Theme::global(cx).surface.min_alpha;
            let cur = Theme::global(cx).surface.alpha;
            let alpha = (cur - ALPHA_STEP).max(floor);
            if let Err(e) = crate::theme_config::persist_surface_alpha(alpha) {
                this.error = Some(e);
            }
            crate::theme_config::apply(cx);
            cx.notify();
        });
        let a_plus = cx.listener(move |this, _ev, _w, cx: &mut Context<BarSettingsTab>| {
            let cur = Theme::global(cx).surface.alpha;
            let alpha = (cur + ALPHA_STEP).min(1.0);
            if let Err(e) = crate::theme_config::persist_surface_alpha(alpha) {
                this.error = Some(e);
            }
            crate::theme_config::apply(cx);
            cx.notify();
        });

        let edge_top = cx.listener(move |this, _ev, _w, cx: &mut Context<BarSettingsTab>| {
            this.current.edge = EdgeChoice::Top;
            this.persist(cx);
        });
        let edge_bottom = cx.listener(move |this, _ev, _w, cx: &mut Context<BarSettingsTab>| {
            this.current.edge = EdgeChoice::Bottom;
            this.persist(cx);
        });

        let w_full = cx.listener(move |this, _ev, _w, cx: &mut Context<BarSettingsTab>| {
            this.current.width = WidthChoice::Full;
            this.persist(cx);
        });
        let w_70 = cx.listener(move |this, _ev, _w, cx: &mut Context<BarSettingsTab>| {
            this.current.width = WidthChoice::Fraction70;
            this.persist(cx);
        });
        let w_50 = cx.listener(move |this, _ev, _w, cx: &mut Context<BarSettingsTab>| {
            this.current.width = WidthChoice::Fraction50;
            this.persist(cx);
        });

        let on_float = cx.listener(move |this, _ev, _w, cx: &mut Context<BarSettingsTab>| {
            let n = !this.current.floating;
            this.current.floating = n;
            if n {
                this.current.exclusive = false;
            }
            this.persist(cx);
        });

        let ev_none = cx.listener(move |this, _ev, _w, cx: &mut Context<BarSettingsTab>| {
            this.current.elevation = ElevationChoice::None;
            this.persist(cx);
        });
        let ev_soft = cx.listener(move |this, _ev, _w, cx: &mut Context<BarSettingsTab>| {
            this.current.elevation = ElevationChoice::Soft;
            this.persist(cx);
        });
        let ev_strong = cx.listener(move |this, _ev, _w, cx: &mut Context<BarSettingsTab>| {
            this.current.elevation = ElevationChoice::Strong;
            this.persist(cx);
        });

        let on_excl = cx.listener(move |this, _ev, _w, cx: &mut Context<BarSettingsTab>| {
            if !this.current.floating {
                this.current.exclusive = !this.current.exclusive;
                this.persist(cx);
            }
        });

        // T284: Frame theme — writes `frame.toml [style]` through the
        // frame's own RMW helper (never `bar.toml`); the 300 ms frame
        // watcher applies it live.
        let frame_normal = cx.listener(move |this, _ev, _w, cx: &mut Context<BarSettingsTab>| {
            if let Err(e) = crate::frame::write_style(crate::frame::FrameStyle::Normal) {
                this.error = Some(e);
            }
            cx.notify();
        });
        let frame_wrapped = cx.listener(move |this, _ev, _w, cx: &mut Context<BarSettingsTab>| {
            if let Err(e) = crate::frame::write_style(crate::frame::FrameStyle::Wrapped) {
                this.error = Some(e);
            }
            cx.notify();
        });

        // T266: blur toggle — goes through `surface_effects::set_blur_enabled`
        // (bridge first, persist only on success). The toggle renders disabled
        // until the background probe lands and while the module is missing.
        let on_blur_toggle = cx.listener(move |this, _ev, _w, cx: &mut Context<BarSettingsTab>| {
            let next = !crate::surface_effects::current(cx).persisted_blur;
            match crate::surface_effects::set_blur_enabled(next, cx) {
                Ok(()) => this.error = None,
                Err(e) => this.error = Some(e),
            }
            cx.notify();
        });

        let on_open = cx.listener(move |this, _ev, _w, cx: &mut Context<BarSettingsTab>| {
            let p = config_path();
            cx.set_global(PreviewTarget {
                path: Some(p),
                generation: 1,
                intent: PreviewIntent::Edit,
            });
            this.error = None;
            cx.notify();
        });


        // ── Render state ──────────────────────────────────────────────
        let cur = self.current;
        let error = self.error.clone();
        let applied = self.applied_preset;
        let edge = cur.edge;
        let width = cur.width;
        let elevation = cur.elevation;
        let floating = cur.floating;
        let cur_style = crate::frame::cached_config().style;
        let h_frac = ((cur.height - HEIGHT_MIN) / (HEIGHT_MAX - HEIGHT_MIN)).clamp(0.0, 1.0);
        let r_frac = (cur.radius / RADIUS_MAX).clamp(0.0, 1.0);
        // T266: slider position from the effective alpha, inverted to the
        // floor..=1.0 range (frac 0 = floor, frac 1 = opaque).
        let a_frac = ((theme.surface.alpha - theme.surface.min_alpha)
            / (1.0 - theme.surface.min_alpha))
        .clamp(0.0, 1.0);
        // T266: blur toggle state from the bridge global.
        let blur_state = crate::surface_effects::current(cx);
        let blur_on = blur_state.persisted_blur;
        let blur_enabled_ctrl = blur_state.probed && blur_state.capability
            == chronos_services::compositor::BlurCapability::Available;

        self.load_hypr_modules();

        // ── Header ────────────────────────────────────────────────────
        let header = div()
            .id("bar-settings-header")
            .w_full()
            .px(px(14.))
            .py(px(12.))
            .border_b_1()
            .border_color(theme.border.default)
            .flex()
            .flex_col()
            .gap(px(2.))
            .child(
                div()
                    .text_color(theme.text.primary)
                    .text_size(px(13.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .child("Bar"),
            )
            .child(
                div()
                    .text_color(theme.text.muted)
                    .text_xs()
                    .font_family(theme.font_mono)
                    .child(format!(
                        "[appearance] · {} · {:.0}px",
                        match edge {
                            EdgeChoice::Top => "top",
                            EdgeChoice::Bottom => "bottom",
                        },
                        cur.height
                    )),
            );

        // ── Elevated card wrapping all scrollable content (T231 §5) ──
        // `.id()` must come AFTER `elevation_apply_light_chrome` — that
        // helper takes a bare `Div`, and `.id()` upgrades to `Stateful<Div>`.
        let mut card = div()
            .relative()
            .w_full()
            .flex()
            .flex_col()
            .gap(px(16.))
            .px(px(16.))
            .py(px(16.))
            .bg(theme.bg.elevated)
            .border_1()
            .border_color(theme.border.subtle)
            .rounded(elev.radius)
            .shadow(elev.shadows.to_vec());
        card = elevation_apply_light_chrome(&elev, card);
        let mut card = card.id("bar-settings-card");

        // ── Presets ───────────────────────────────────────────────────
        card = card
            .child(section_header(theme, "Presets", "apply live · written to bar.toml"))
            .child({
                let mut chips = Vec::new();
                for p in PRESETS {
                    let id = p.id;
                    let active = applied == Some(id);
                    chips.push(preset_chip(
                        theme,
                        &format!("preset-{id}"),
                        p.name,
                        p.description,
                        active,
                        cx.listener(move |this, _ev, _w, cx: &mut Context<BarSettingsTab>| {
                            this.apply_preset_id(id, cx);
                        }),
                    ));
                }
                div()
                    .w_full()
                    .flex()
                    .flex_wrap()
                    .gap(px(8.))
                    .children(chips)
            });

        // ── Appearance — responsive grid (T231 §1) ────────────────────
        card = card
            .child(section_header(
                theme,
                "Appearance",
                "appearance.* — applies live",
            ))
            .child(
                div()
                    .grid()
                    .w_full()
                    .gap(px(10.))
                    .when(is_wide, |d| d.grid_cols(2))
                    .when(!is_wide, |d| d.grid_cols(1))
                    .child(setting_row(
                        setting_label(theme, "Edge", "appearance.edge"),
                        segmented(
                            theme,
                            vec![
                                seg_chip(
                                    theme,
                                    "edge-seg-0",
                                    "Top",
                                    edge == EdgeChoice::Top,
                                    edge_top,
                                ),
                                seg_chip(
                                    theme,
                                    "edge-seg-1",
                                    "Bottom",
                                    edge == EdgeChoice::Bottom,
                                    edge_bottom,
                                ),
                            ],
                        ),
                    ))
                    .child(setting_row(
                        setting_label(theme, "Height", "appearance.height"),
                        slider_control(
                            theme,
                            h_frac,
                            h_minus,
                            h_plus,
                            HeightSliderDrag,
                            height_drag,
                            "bar-h-minus",
                            "bar-h-track",
                            "bar-h-plus",
                        ),
                    ))
                    .child(setting_row(
                        setting_label(theme, "Width", "appearance.width"),
                        segmented(
                            theme,
                            vec![
                                seg_chip(
                                    theme,
                                    "width-seg-0",
                                    "Full",
                                    width == WidthChoice::Full,
                                    w_full,
                                ),
                                seg_chip(
                                    theme,
                                    "width-seg-1",
                                    "70%",
                                    width == WidthChoice::Fraction70,
                                    w_70,
                                ),
                                seg_chip(
                                    theme,
                                    "width-seg-2",
                                    "50%",
                                    width == WidthChoice::Fraction50,
                                    w_50,
                                ),
                            ],
                        ),
                    ))
                    .child(setting_row(
                        setting_label(theme, "Floating", "appearance.floating"),
                        onoff_chip(theme, "bar-ctrl-floating", floating, on_float),
                    ))
                    .child(setting_row(
                        setting_label(theme, "Radius", "appearance.radius"),
                        slider_control(
                            theme,
                            r_frac,
                            r_minus,
                            r_plus,
                            RadiusSliderDrag,
                            radius_drag,
                            "bar-r-minus",
                            "bar-r-track",
                            "bar-r-plus",
                        ),
                    ))
                    // T266: surface transparency — third slider in the same
                    // row family (same `slider_control`, same geometry). The
                    // low end maps to the scheme's measured readability floor
                    // (`min_alpha`), not 0.0; default sits at the opaque end.
                    .child(setting_row(
                        setting_label(
                            theme,
                            "Surface opacity",
                            "theme.toml surface_alpha",
                        ),
                        slider_control(
                            theme,
                            a_frac,
                            a_minus,
                            a_plus,
                            SurfaceAlphaSliderDrag,
                            alpha_drag,
                            "bar-a-minus",
                            "bar-a-track",
                            "bar-a-plus",
                        ),
                    ))
                    // T266: compositor blur — separate toggle next to the
                    // alpha slider (blur is GPU-costly; users want it
                    // independent of alpha). Disabled with a reason while the
                    // probe is in flight or the module is missing.
                    .child(setting_row(
                        setting_label(
                            theme,
                            "Blur",
                            "theme.toml blur_enabled · hyprctl eval",
                        ),
                        if blur_enabled_ctrl {
                            onoff_chip(theme, "bar-blur-toggle", blur_on, on_blur_toggle)
                        } else {
                            div()
                                .id("bar-blur-toggle-disabled")
                                .px(px(10.))
                                .py(px(5.))
                                .rounded_md()
                                .text_size(px(11.5))
                                .font_family(theme.font_mono)
                                .text_color(theme.text.disabled)
                                .border_1()
                                .border_color(theme.border.subtle)
                                .opacity(0.6)
                                .child(match blur_state.capability {
                                    chronos_services::compositor::BlurCapability::ModuleMissing => {
                                        "import 45-surface-effects-chronos.lua"
                                    }
                                    chronos_services::compositor::BlurCapability::Unsupported => {
                                        "compositor: no blur"
                                    }
                                    chronos_services::compositor::BlurCapability::Available => {
                                        "checking…"
                                    }
                                })
                                .into_any_element()
                        },
                    ))
                    .child(setting_row(
                        setting_label(theme, "Elevation", "appearance.elevation"),
                        segmented(
                            theme,
                            vec![
                                seg_chip(
                                    theme,
                                    "elev-seg-0",
                                    "None",
                                    elevation == ElevationChoice::None,
                                    ev_none,
                                ),
                                seg_chip(
                                    theme,
                                    "elev-seg-1",
                                    "Soft",
                                    elevation == ElevationChoice::Soft,
                                    ev_soft,
                                ),
                                seg_chip(
                                    theme,
                                    "elev-seg-2",
                                    "Strong",
                                    elevation == ElevationChoice::Strong,
                                    ev_strong,
                                ),
                            ],
                        ),
                    ))
                    .child(setting_row(
                        setting_label(theme, "Exclusive zone", "appearance.exclusive"),
                        {
                            let chip = onoff_chip(theme, "bar-ctrl-exclusive", cur.exclusive, on_excl);
                            if floating {
                                div().opacity(0.35).child(chip).into_any_element()
                            } else {
                                chip
                            }
                        },
                    ))
                    .child(setting_row(
                        setting_label(theme, "Frame", "frame.toml style"),
                        segmented(
                            theme,
                            vec![
                                seg_chip(
                                    theme,
                                    "frame-seg-normal",
                                    "Normal",
                                    cur_style == crate::frame::FrameStyle::Normal,
                                    frame_normal,
                                ),
                                seg_chip(
                                    theme,
                                    "frame-seg-wrapped",
                                    "Wrapped",
                                    cur_style == crate::frame::FrameStyle::Wrapped,
                                    frame_wrapped,
                                ),
                            ],
                        ),
                    )),
            );

        // ── Theme picker (T313) ────────────────────────────────────────
        // One swatch card per `builtin_schemes()` entry. Swatch colors come
        // from the scheme's own `Theme` (never hardcoded here) — the strip is
        // bg.primary/secondary/tertiary/elevated + an accent dot. Active
        // scheme is matched by color core (surface alpha/blur are overlaid
        // by the config, so a whole-theme equality would misfire on
        // translucent setups).
        let mut swatches: Vec<AnyElement> = Vec::new();
        for scheme in builtin_schemes() {
            let name = scheme.name;
            let active = scheme_core_matches(&scheme.theme, &theme);
            swatches.push(theme_swatch_card(
                theme,
                &scheme,
                active,
                cx.listener(move |this, _ev, _w, cx: &mut Context<BarSettingsTab>| {
                    crate::theme_config::select(name, cx);
                    this.error = None;
                    cx.notify();
                }),
            ));
        }
        card = card
            .child(section_header(theme, "Theme", "theme.toml — hot-reload"))
            .child(
                div()
                    .id("sys-theme-swatches")
                    .grid()
                    .w_full()
                    .gap(px(8.))
                    .when(is_wide, |d| d.grid_cols(2))
                    .when(!is_wide, |d| d.grid_cols(1))
                    .children(swatches),
            );

        // ── Hypr modules — compact grid on wide (T231 §4) ────────────
        card = card
            .child(section_header(
                theme,
                "Hypr modules",
                "~/.config/hypr/modules/ — click to open in Editor",
            ))
            .child({
                let mut rows: Vec<AnyElement> = Vec::new();
                if self.hypr_modules.is_empty() {
                    // T269: the shared note, not a bordered one-off — bordered
                    // empty states are drift (T252 canon has no such variant).
                    rows.push(empty_state_note(
                        theme,
                        "No modules found in ~/.config/hypr/modules/",
                        NoteSeverity::Muted,
                    ));
                }
                for (name, path) in &self.hypr_modules {
                    let p = path.clone();
                    let display = path.display().to_string();
                    rows.push(module_card(
                        theme,
                        &format!("hypr-mod-{name}"),
                        name,
                        &display,
                        cx.listener(move |this, _ev, _w, cx: &mut Context<BarSettingsTab>| {
                            cx.set_global(PreviewTarget {
                                path: Some(p.clone()),
                                generation: 1,
                                intent: PreviewIntent::View,
                            });
                            this.error = None;
                            cx.notify();
                        }),
                    ));
                }
                let n = self.hypr_modules.len();
                div()
                    .grid()
                    .w_full()
                    .gap(px(8.))
                    .when(is_wide && n >= 3, |d| d.grid_cols(3))
                    .when(is_wide && n == 2, |d| d.grid_cols(2))
                    .when(!is_wide && n > 0, |d| d.grid_cols(1))
                    .children(rows)
            });

        // ── About ─────────────────────────────────────────────────────
        card = card
            .child(section_header(theme, "About", "Build info"))
            .child(
                div()
                    .w_full()
                    .flex_col()
                    .gap(px(4.))
                    .px(px(12.))
                    .py(px(9.))
                    .rounded_md()
                    .border_1()
                    .border_color(theme.border.subtle)
                    .child(
                        div()
                            .flex()
                            .justify_between()
                            .child(
                                div()
                                    .text_color(theme.text.primary)
                                    .text_size(px(12.))
                                    .child("ChronOS shell"),
                            )
                            .child(
                                div()
                                    .text_color(theme.text.muted)
                                    .text_xs()
                                    .font_family(theme.font_mono)
                                    .child(env!("CARGO_PKG_VERSION")),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .justify_between()
                            .child(
                                div()
                                    .text_color(theme.text.muted)
                                    .text_xs()
                                    .child("Desktop shell for Hyprland"),
                            )
                            .child(
                                div()
                                    .text_color(theme.text.muted)
                                    .text_xs()
                                    .font_family(theme.font_mono)
                                    .child("Apache-2.0"),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .justify_between()
                            .child(
                                div()
                                    .text_color(theme.text.muted)
                                    .text_xs()
                                    .child("Rust + GPUI + mlua"),
                            )
                            .child(
                                div()
                                    .text_color(theme.text.muted)
                                    .text_xs()
                                    .font_family(theme.font_mono)
                                    .child("LuauJIT"),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .justify_between()
                            .child(
                                div()
                                    .text_color(theme.text.muted)
                                    .text_xs()
                                    .child("offline by design"),
                            )
                            .child(
                                div()
                                    .text_color(theme.text.muted)
                                    .text_xs()
                                    .font_family(theme.font_mono)
                                    .child("no network · no telemetry"),
                            ),
                    ),
            );

        // ── Open config action ────────────────────────────────────────
        card = card.child(
            div()
                .id("bar-settings-open-config")
                .w_full()
                .flex()
                .justify_between()
                .items_center()
                .px(px(12.))
                .py(px(9.))
                .rounded_md()
                .border_1()
                .border_color(theme.border.subtle)
                .cursor_pointer()
                .hover(|s| s.bg(theme.interactive.hover))
                .child(
                    div()
                        .flex_col()
                        .gap(px(2.))
                        .child(
                            div()
                                .text_color(theme.text.primary)
                                .text_size(px(12.))
                                .child("Open bar.toml"),
                        )
                        .child(
                            div()
                                .text_color(theme.text.muted)
                                .text_xs()
                                .font_family(theme.font_mono)
                                .child("~/.config/chronos/bar.toml"),
                        ),
                )
                .child(
                    div()
                        .text_color(theme.accent.primary)
                        .text_size(px(12.))
                        .child("Edit"),
                )
                .on_click(on_open),
        );

        // ── Error banner ──────────────────────────────────────────────
        let card = card.when_some(error, |d, e| {
            d.child(
                div()
                    .w_full()
                    .px(px(12.))
                    .py(px(9.))
                    .rounded_md()
                    .border_1()
                    .border_color(theme.status.error)
                    .text_color(theme.status.error)
                    .text_xs()
                    .font_family(theme.font_mono)
                    .child(e),
            )
        });

        // ── Root ──────────────────────────────────────────────────────
        div()
            .id("bar-settings-tab")
            .w_full()
            .min_h(px(0.))
            .flex()
            .flex_col()
            .child(header)
            .child(
                div()
                    .id("bar-settings-scroll")
                    .flex_1()
                    .min_h(px(0.))
                    .overflow_y_scroll()
                    .track_scroll(&self.scroll)
                    .p(px(14.))
                    .child(card),
            )
    }
}

// ── Visual helpers ──────────────────────────────────────────────────────────

/// Group of segment chips in a bordered control capsule (Edge/Width/Elevation).
fn segmented(theme: Theme, chips: Vec<AnyElement>) -> AnyElement {
    div()
        .flex()
        .items_center()
        .gap(px(2.))
        .p(px(2.))
        .rounded_md()
        .border_1()
        .border_color(theme.border.subtle)
        .children(chips)
        .into_any_element()
}

/// Segmented-control chip — accent state (T231 §3 keeps the accent language).
fn seg_chip<F>(theme: Theme, id: &str, label: &str, active: bool, on_click: F) -> AnyElement
where
    F: Fn(&ClickEvent, &mut Window, &mut App) + 'static,
{
    let id = SharedString::from(id);
    let label = SharedString::from(label);
    div()
        .id(id)
        .px(px(9.))
        .py(px(5.))
        .rounded_md()
        .cursor_pointer()
        .text_size(px(11.5))
        .font_family(theme.font_mono)
        .bg(if active {
            theme.accent.primary.opacity(0.16)
        } else {
            gpui::transparent_black()
        })
        .text_color(if active { theme.accent.primary } else { theme.text.secondary })
        .border_1()
        .border_color(if active { theme.accent.primary } else { theme.border.subtle })
        .hover(move |s| {
            if active {
                s.bg(theme.accent.primary.opacity(0.16))
            } else {
                s.bg(theme.interactive.hover)
            }
        })
        .on_click(on_click)
        .child(label)
        .into_any_element()
}

/// Preset chip — wider hit area, full title + subtitle (T231 §2).
fn preset_chip<F>(theme: Theme, id: &str, name: &str, desc: &str, active: bool, on_click: F) -> AnyElement
where
    F: Fn(&ClickEvent, &mut Window, &mut App) + 'static,
{
    let id = SharedString::from(id);
    let name = SharedString::from(name);
    let desc = SharedString::from(desc);
    div()
        .id(id)
        .flex_1()
        .min_w(px(96.))
        .flex_col()
        .gap(px(2.))
        .px(px(10.))
        .py(px(6.))
        .rounded_md()
        .cursor_pointer()
        .bg(if active {
            theme.accent.primary.opacity(0.16)
        } else {
            theme.bg.secondary.opacity(0.5)
        })
        .border_1()
        .border_color(if active { theme.accent.primary } else { theme.border.subtle })
        .hover(move |s| {
            if active {
                s.bg(theme.accent.primary.opacity(0.16))
            } else {
                s.bg(theme.interactive.hover)
            }
        })
        .on_click(on_click)
        .child(
            div()
                .text_size(px(11.5))
                .font_weight(FontWeight::MEDIUM)
                .text_color(if active { theme.accent.primary } else { theme.text.primary })
                .child(name),
        )
        .child(
            div()
                .text_color(if active { theme.text.secondary } else { theme.text.muted })
                .text_xs()
                .font_family(theme.font_mono)
                .child(desc),
        )
        .into_any_element()
}

/// True when a scheme's color core (everything except the surface tokens)
/// equals the active theme. Surface alpha/blur are overlaid by the config on
/// top of any scheme, so a whole-`Theme` comparison would never match a
/// translucent setup.
fn scheme_core_matches(scheme: &Theme, active: &Theme) -> bool {
    scheme.bg == active.bg
        && scheme.text == active.text
        && scheme.border == active.border
        && scheme.accent == active.accent
        && scheme.status == active.status
        && scheme.interactive == active.interactive
}

/// T313 theme picker card: live palette strip (bg.primary/secondary/
/// tertiary/elevated + accent dot) with the scheme name underneath. Active
/// card wears the accent border/state language shared with `seg_chip` and
/// `onoff_chip`. Colors are read from the scheme's own `Theme` — hardcoding
/// hexes here would drift from the palette on the next scheme edit.
fn theme_swatch_card<F>(
    theme: Theme,
    scheme: &ThemeScheme,
    active: bool,
    on_click: F,
) -> AnyElement
where
    F: Fn(&ClickEvent, &mut Window, &mut App) + 'static,
{
    let id = SharedString::from(format!("theme-swatch-{}", scheme.name));
    let name = SharedString::from(scheme.name);
    let s = &scheme.theme;
    div()
        .id(id)
        .flex_col()
        .gap(px(6.))
        .px(px(10.))
        .py(px(8.))
        .rounded_md()
        .cursor_pointer()
        .bg(if active {
            theme.accent.primary.opacity(0.16)
        } else {
            theme.bg.secondary.opacity(0.5)
        })
        .border_1()
        .border_color(if active { theme.accent.primary } else { theme.border.subtle })
        .hover(move |s| {
            if active {
                s.bg(theme.accent.primary.opacity(0.16))
            } else {
                s.bg(theme.interactive.hover)
            }
        })
        .on_click(on_click)
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(8.))
                .w_full()
                .child(
                    div()
                        .flex_1()
                        .flex()
                        .h(px(18.))
                        .rounded(px(4.))
                        .overflow_hidden()
                        .border_1()
                        .border_color(theme.border.subtle)
                        .child(div().flex_1().bg(s.bg.primary))
                        .child(div().flex_1().bg(s.bg.secondary))
                        .child(div().flex_1().bg(s.bg.tertiary))
                        .child(div().flex_1().bg(s.bg.elevated)),
                )
                .child(
                    div()
                        .size(px(18.))
                        .rounded_full()
                        .bg(s.accent.primary)
                        .border_1()
                        .border_color(theme.border.subtle),
                ),
        )
        .child(
            div()
                .text_size(px(11.5))
                .font_weight(FontWeight::MEDIUM)
                .text_color(if active { theme.accent.primary } else { theme.text.primary })
                .child(name),
        )
        .into_any_element()
}

/// On/off chip (Floating, Exclusive zone) — same accent-state language.
fn onoff_chip<F>(theme: Theme, id: &str, on: bool, on_click: F) -> AnyElement
where
    F: Fn(&ClickEvent, &mut Window, &mut App) + 'static,
{
    div()
        .id(SharedString::from(id))
        .px(px(10.))
        .py(px(5.))
        .rounded_md()
        .cursor_pointer()
        .text_size(px(11.5))
        .font_family(theme.font_mono)
        .bg(if on {
            theme.accent.primary.opacity(0.16)
        } else {
            gpui::transparent_black()
        })
        .text_color(if on { theme.accent.primary } else { theme.text.secondary })
        .border_1()
        .border_color(if on { theme.accent.primary } else { theme.border.subtle })
        .hover(move |s| {
            if on {
                s.bg(theme.accent.primary.opacity(0.16))
            } else {
                s.bg(theme.interactive.hover)
            }
        })
        .on_click(on_click)
        .child(if on { "on" } else { "off" })
        .into_any_element()
}

/// `-`/`+` step button — border + hover bg, same weight as segments (T231 §3).
fn step_button<F>(theme: Theme, id: &str, label: &str, on_click: F) -> AnyElement
where
    F: Fn(&ClickEvent, &mut Window, &mut App) + 'static,
{
    let id = SharedString::from(id);
    let label = SharedString::from(label);
    div()
        .id(id)
        .w(px(24.))
        .h(px(24.))
        .flex()
        .items_center()
        .justify_center()
        .rounded_md()
        .cursor_pointer()
        .text_size(px(13.))
        .text_color(theme.text.secondary)
        .border_1()
        .border_color(theme.border.subtle)
        .bg(gpui::transparent_black())
        .hover(move |s| s.bg(theme.interactive.hover))
        .on_click(on_click)
        .child(label)
        .into_any_element()
}

/// The slider face: 6px track (muted), accent fill, 16px thumb with border
/// + drop shadow. Purely visual — drag wiring lives in `slider_control`.
fn slider_face(theme: Theme, frac: f32) -> AnyElement {
    let frac = frac.clamp(0.0, 1.0);
    let fill_w = SLIDER_TW * frac;
    let thumb_left = (SLIDER_TW * frac - SLIDER_THUMB / 2.0).clamp(0.0, SLIDER_TW - SLIDER_THUMB);
    let track_top = (SLIDER_THUMB - SLIDER_TRACK_H) / 2.0;

    div()
        .relative()
        .w(px(SLIDER_TW))
        .h(px(SLIDER_THUMB))
        .child(
            div()
                .absolute()
                .left(px(0.))
                .top(px(track_top))
                .w(px(SLIDER_TW))
                .h(px(SLIDER_TRACK_H))
                .rounded(px(SLIDER_TRACK_H / 2.0))
                .bg(theme.interactive.hover),
        )
        .child(
            div()
                .absolute()
                .left(px(0.))
                .top(px(track_top))
                .w(px(fill_w))
                .h(px(SLIDER_TRACK_H))
                .rounded(px(SLIDER_TRACK_H / 2.0))
                .bg(theme.accent.primary),
        )
        .child(
            div()
                .absolute()
                .left(px(thumb_left))
                .size(px(SLIDER_THUMB))
                .rounded(px(SLIDER_THUMB / 2.0))
                .bg(theme.text.primary)
                .border_1()
                .border_color(theme.border.subtle)
                .shadow(vec![BoxShadow::new(px(0.), px(2.), theme.bg.tertiary.opacity(0.35))
                    .blur_radius(px(6.))]),
        )
        .into_any_element()
}

/// Full slider control: step buttons + draggable track + numeric readout.
/// Generic over the drag marker so Height and Radius share one helper while
/// `on_drag` still routes to the right marker type.
///
/// `pub(crate)` so the launcher settings page (T265-G) reuses the same slider
/// instead of forking its own (spec: "свой слайдер не писать").
pub(crate) fn slider_control<D, F1, F2>(
    theme: Theme,
    frac: f32,
    minus: F1,
    plus: F2,
    drag_marker: D,
    drag: impl Fn(&DragMoveEvent<D>, &mut Window, &mut App) + 'static,
    minus_id: &str,
    track_id: &str,
    plus_id: &str,
) -> AnyElement
where
    D: 'static,
    F1: Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    F2: Fn(&ClickEvent, &mut Window, &mut App) + 'static,
{
    div()
        .flex()
        .items_center()
        .gap(px(8.))
        .child(step_button(theme, minus_id, "−", minus))
        .child(
            div()
                .id(SharedString::from(track_id))
                .relative()
                .w(px(SLIDER_TW))
                .h(px(SLIDER_THUMB))
                .flex()
                .items_center()
                .cursor_pointer()
                .on_drag(drag_marker, |_, _, _, cx| cx.new(|_| EmptyView))
                .on_drag_move(drag)
                .child(slider_face(theme, frac)),
        )
        .child(step_button(theme, plus_id, "+", plus))
        .into_any_element()
}

/// Hypr-module card: name (mono) + path (muted, ellipsis) + Open link.
/// Compact grid cells on wide panels (T231 §4).
fn module_card<F>(theme: Theme, id: &str, name: &str, path: &str, on_click: F) -> AnyElement
where
    F: Fn(&ClickEvent, &mut Window, &mut App) + 'static,
{
    let id = SharedString::from(id);
    let name = SharedString::from(name);
    let path = SharedString::from(path);
    div()
        .id(id)
        .w_full()
        .flex_col()
        .gap(px(4.))
        .px(px(10.))
        .py(px(8.))
        .rounded_md()
        .border_1()
        .border_color(theme.border.subtle)
        .bg(theme.bg.secondary.opacity(0.5))
        .cursor_pointer()
        .hover(|s| s.bg(theme.interactive.hover))
        .on_click(on_click)
        .child(
            div()
                .text_color(theme.text.primary)
                .text_size(px(11.))
                .font_weight(FontWeight::MEDIUM)
                .font_family(theme.font_mono)
                .truncate()
                .child(name),
        )
        .child(
            div()
                .text_color(theme.text.muted)
                .text_xs()
                .font_family(theme.font_mono)
                .truncate()
                .child(path),
        )
        .child(
            div()
                .flex()
                .justify_between()
                .child(div().text_xs().text_color(theme.text.muted).child("Open"))
                .child(
                    div()
                        .text_xs()
                        .font_family(theme.font_mono)
                        .text_color(theme.accent.primary)
                        .child("▸"),
                ),
        )
        .into_any_element()
}

// ── Pure helpers ────────────────────────────────────────────────────────────

/// Pointer x relative to the track → 0..=1 fraction (same math as T202).
fn slider_frac(rel_x: f32, w: f32) -> f32 {
    (rel_x / w.max(1.0)).clamp(0.0, 1.0)
}

/// Slider fraction → surface alpha in `floor..=1.0`. frac 0 maps to the
/// scheme floor (not 0.0 — readability), frac 1 to opaque.
fn alpha_from_frac(frac: f32, floor: f32) -> f32 {
    floor + frac.clamp(0.0, 1.0) * (1.0 - floor)
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slider_frac_clamps_and_handles_zero_width() {
        assert_eq!(slider_frac(0.0, 100.0), 0.0);
        assert_eq!(slider_frac(50.0, 100.0), 0.5);
        assert_eq!(slider_frac(100.0, 100.0), 1.0);
        assert_eq!(slider_frac(200.0, 100.0), 1.0);
        assert_eq!(slider_frac(-10.0, 100.0), 0.0);
        assert_eq!(slider_frac(5.0, 0.0), 1.0, "zero width must not divide by zero");
    }

    #[test]
    fn slider_fraction_maps_to_theme_floor_and_one() {
        assert_eq!(alpha_from_frac(0.0, 0.62), 0.62);
        assert_eq!(alpha_from_frac(1.0, 0.62), 1.0);
        assert_eq!(alpha_from_frac(0.5, 0.62), 0.81);
        // Floor 1.0 (Task 1 conservative) pins the whole slider to opaque.
        assert_eq!(alpha_from_frac(0.0, 1.0), 1.0);
        assert_eq!(alpha_from_frac(1.0, 1.0), 1.0);
    }

}
