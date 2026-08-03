//! System settings — Bar page (T202). All controls inlined.

use std::path::PathBuf;

use gpui::{
    Context, DragMoveEvent, EmptyView, InteractiveElement, IntoElement, ParentElement, Render,
    ScrollHandle, SharedString, Styled, Window, div, prelude::*, px,
};

use chronos_ui::Theme;
use crate::bar_settings::{
    BarSettingsPatch, EdgeChoice, ElevationChoice, PRESETS, WidthChoice, apply_patch, apply_preset,
    config_path, read_current,
};
use crate::side_panel_right::preview_target::{PreviewIntent, PreviewTarget};

const HEIGHT_MIN: f32 = 20.;
const HEIGHT_MAX: f32 = 48.;
const RADIUS_MAX: f32 = 16.;

pub struct HeightSliderDrag;
pub struct RadiusSliderDrag;

pub struct BarSettingsTab {
    current: BarSettingsPatch,
    error: Option<String>,
    applied_preset: Option<&'static str>,
    scroll: ScrollHandle,
    /// T196: cached Hypr module listing (name, path). Lazily loaded on first render.
    hypr_modules: Vec<(String, PathBuf)>,
    hypr_modules_loaded: bool,
}

impl BarSettingsTab {
    pub fn new(_cx: &mut Context<Self>) -> Self {
        Self {
            current: read_current(),
            error: None,
            applied_preset: None,
            scroll: ScrollHandle::new(),
            hypr_modules: Vec::new(),
            hypr_modules_loaded: false,
        }
    }

    fn load_hypr_modules(&mut self) {
        if self.hypr_modules_loaded { return; }
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
                let name = path.file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("unknown")
                    .to_string();
                modules.push((name, path));
            }
        }
        modules.sort_by(|a, b| a.0.cmp(&b.0));
        self.hypr_modules = modules;
    }

    fn persist(&mut self, cx: &mut Context<Self>) {
        match apply_patch(&self.current) { Ok(()) => self.error = None, Err(e) => self.error = Some(e), }
        cx.notify();
    }

    fn apply_preset_id(&mut self, id: &'static str, cx: &mut Context<Self>) {
        match apply_preset(id) {
            Ok(p) => { self.current = p.appearance; self.applied_preset = Some(id); self.error = None; }
            Err(e) => self.error = Some(e),
        }
        cx.notify();
    }
}

impl Render for BarSettingsTab {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = *Theme::global(cx);

        // ── Drag listeners (pattern: volume_popup line ~390) ─────────────
        let height_drag = cx.listener(
            move |this, ev: &DragMoveEvent<HeightSliderDrag>, _w, cx: &mut Context<BarSettingsTab>| {
                let rel_x = f32::from(ev.event.position.x - ev.bounds.origin.x);
                let w = f32::from(ev.bounds.size.width).max(1.0);
                let frac = (rel_x / w).clamp(0.0, 1.0);
                this.current.height = (HEIGHT_MIN + frac * (HEIGHT_MAX - HEIGHT_MIN)).clamp(HEIGHT_MIN, HEIGHT_MAX);
                this.persist(cx);
            },
        );
        let radius_drag = cx.listener(
            move |this, ev: &DragMoveEvent<RadiusSliderDrag>, _w, cx: &mut Context<BarSettingsTab>| {
                let rel_x = f32::from(ev.event.position.x - ev.bounds.origin.x);
                let w = f32::from(ev.bounds.size.width).max(1.0);
                let frac = (rel_x / w).clamp(0.0, 1.0);
                this.current.radius = (frac * RADIUS_MAX).clamp(0.0, RADIUS_MAX);
                this.persist(cx);
            },
        );

        // ── Click handlers ──────────────────────────────────────────────
        let hs = ((HEIGHT_MAX - HEIGHT_MIN) / 10.0).max(1.0);
        let rs = (RADIUS_MAX / 10.0).max(1.0);

        let h_minus = cx.listener(move |this, _ev: &gpui::ClickEvent, _w, cx: &mut Context<BarSettingsTab>| { this.current.height = (this.current.height - hs).clamp(HEIGHT_MIN, HEIGHT_MAX); this.persist(cx); });
        let h_plus  = cx.listener(move |this, _ev: &gpui::ClickEvent, _w, cx: &mut Context<BarSettingsTab>| { this.current.height = (this.current.height + hs).clamp(HEIGHT_MIN, HEIGHT_MAX); this.persist(cx); });
        let r_minus = cx.listener(move |this, _ev: &gpui::ClickEvent, _w, cx: &mut Context<BarSettingsTab>| { this.current.radius = (this.current.radius - rs).clamp(0.0, RADIUS_MAX); this.persist(cx); });
        let r_plus  = cx.listener(move |this, _ev: &gpui::ClickEvent, _w, cx: &mut Context<BarSettingsTab>| { this.current.radius = (this.current.radius + rs).clamp(0.0, RADIUS_MAX); this.persist(cx); });

        let edge_top    = cx.listener(move |this, _ev, _w, cx: &mut Context<BarSettingsTab>| { this.current.edge = EdgeChoice::Top; this.persist(cx); });
        let edge_bottom = cx.listener(move |this, _ev, _w, cx: &mut Context<BarSettingsTab>| { this.current.edge = EdgeChoice::Bottom; this.persist(cx); });

        let w_full = cx.listener(move |this, _ev, _w, cx: &mut Context<BarSettingsTab>| { this.current.width = WidthChoice::Full; this.persist(cx); });
        let w_70   = cx.listener(move |this, _ev, _w, cx: &mut Context<BarSettingsTab>| { this.current.width = WidthChoice::Fraction70; this.persist(cx); });
        let w_50   = cx.listener(move |this, _ev, _w, cx: &mut Context<BarSettingsTab>| { this.current.width = WidthChoice::Fraction50; this.persist(cx); });

        let on_float = cx.listener(move |this, _ev, _w, cx: &mut Context<BarSettingsTab>| { let n = !this.current.floating; this.current.floating = n; if n { this.current.exclusive = false; } this.persist(cx); });

        let ev_none   = cx.listener(move |this, _ev, _w, cx: &mut Context<BarSettingsTab>| { this.current.elevation = ElevationChoice::None; this.persist(cx); });
        let ev_soft   = cx.listener(move |this, _ev, _w, cx: &mut Context<BarSettingsTab>| { this.current.elevation = ElevationChoice::Soft; this.persist(cx); });
        let ev_strong = cx.listener(move |this, _ev, _w, cx: &mut Context<BarSettingsTab>| { this.current.elevation = ElevationChoice::Strong; this.persist(cx); });

        let on_excl = cx.listener(move |this, _ev, _w, cx: &mut Context<BarSettingsTab>| { if !this.current.floating { this.current.exclusive = !this.current.exclusive; this.persist(cx); } });

        let on_open = cx.listener(move |this, _ev, _w, cx: &mut Context<BarSettingsTab>| {
            let p = config_path();
            cx.set_global(PreviewTarget { path: Some(p), generation: 1, intent: PreviewIntent::Edit });
            this.error = None;
            cx.notify();
        });

        // T196: Theme toggle — T211: went through `update_global::<Theme,_>`
        // which panics (`no state of type Theme exists`) when the listener's
        // app context can't resolve the Theme global. Reuse the IPC/hot-reload
        // path `theme_config::toggle` instead — it `set_global`s (never panics),
        // persists scheme, syncs the gpui-component theme and refreshes windows.
        // Degrade+log on persist failure (no `expect`).
        let theme_scheme = if Theme::global(cx).is_light { "Light" } else { "Default" };
        let is_light = Theme::global(cx).is_light;
        let toggle_theme = cx.listener(move |this, _ev, _w, cx: &mut Context<BarSettingsTab>| {
            crate::theme_config::toggle(cx);
            this.error = None;
            cx.notify();
        });

        // ── Render state ─────────────────────────────────────────────────
        let cur = self.current;
        let error = self.error.clone();
        let applied = self.applied_preset;
        let edge = cur.edge; let width = cur.width; let elevation = cur.elevation; let floating = cur.floating;
        let h_frac = ((cur.height - HEIGHT_MIN) / (HEIGHT_MAX - HEIGHT_MIN)).clamp(0.0, 1.0);
        let r_frac = (cur.radius / RADIUS_MAX).clamp(0.0, 1.0);
        let track_bg = gpui::Hsla::from(gpui::rgba(0x0000_0047));
        let track_fill = gpui::Hsla::from(gpui::rgba(0x0000_006b));
        let thumb = gpui::Hsla::from(gpui::rgba(0xFFFF_FFE5));
        const TW: f32 = 110.; const TH: f32 = 4.; const TB: f32 = 13.;

        div().id("bar-settings-tab").size_full().flex().flex_col()
            .child(div().w_full().px(px(14.)).py(px(12.)).border_b_1().border_color(theme.border.default).flex().flex_col().gap(px(2.))
                .child(div().text_color(theme.text.primary).text_size(px(13.)).font_weight(gpui::FontWeight::SEMIBOLD).child("Bar"))
                .child(div().text_color(theme.text.muted).text_xs().font_family(theme.font_mono).child(format!("[appearance] · {} · {:.0}px", match edge { EdgeChoice::Top => "top", EdgeChoice::Bottom => "bottom" }, cur.height))))
            .child(
                div().id("bar-settings-scroll").flex_1().min_h(px(0.)).overflow_y_scroll().track_scroll(&self.scroll).flex().flex_col().gap(px(14.)).p(px(14.))
                    // Presets
                    .child({
                        let mut chips = Vec::new();
                        for p in PRESETS {
                            let id = p.id; let active = applied == Some(id);
                            chips.push(div().id(SharedString::from(format!("preset-{id}"))).flex_col().flex_1().px(px(10.)).py(px(8.)).rounded_md().border_1()
                                .border_color(if active { theme.accent.primary } else { theme.border.subtle })
                                .bg(if active { theme.accent.primary.opacity(0.14) } else { gpui::transparent_black() }).cursor_pointer()
                                .hover(move |s| { s.bg(if active { theme.accent.primary.opacity(0.14) } else { theme.interactive.hover }) })
                                .child(div().text_color(if active { theme.accent.primary } else { theme.text.primary }).text_size(px(12.)).font_weight(gpui::FontWeight::MEDIUM).child(p.name))
                                .child(div().text_color(theme.text.muted).text_xs().child(p.description))
                                .on_click(cx.listener(move |this, _ev, _w, cx| { this.apply_preset_id(id, cx); })).into_any_element());
                        }
                        div().w_full().flex_col().gap(px(6.))
                            .child(div().w_full().flex_col().gap(px(2.)).child(div().text_color(theme.text.primary).text_size(px(12.)).font_weight(gpui::FontWeight::SEMIBOLD).child("Presets")).child(div().text_color(theme.text.muted).text_xs().child("apply live · written to bar.toml")))
                            .child(div().w_full().flex().gap(px(8.)).children(chips))
                    })
                    // Appearance header
                    .child(div().w_full().flex_col().gap(px(2.)).child(div().text_color(theme.text.primary).text_size(px(12.)).font_weight(gpui::FontWeight::SEMIBOLD).child("Appearance")).child(div().text_color(theme.text.muted).text_xs().child("appearance.* — applies live")))
                    // Edge
                    .child(div().w_full().flex().items_center().justify_between().gap(px(10.))
                        .child(div().flex_col().gap(px(1.)).child(div().text_color(theme.text.primary).text_size(px(12.)).child("Edge")).child(div().text_color(theme.text.muted).text_xs().font_family(theme.font_mono).child("appearance.edge")))
                        .child({ let a = edge == EdgeChoice::Top; let b = edge == EdgeChoice::Bottom; div().flex().gap(px(2.)).p(px(2.)).rounded_md().border_1().border_color(theme.border.subtle).children(vec![
                            div().id("edge-seg-0").px(px(9.)).py(px(5.)).rounded_md().cursor_pointer().text_size(px(11.5)).font_family(theme.font_mono).bg(if a { theme.accent.primary.opacity(0.16) } else { gpui::transparent_black() }).text_color(if a { theme.accent.primary } else { theme.text.secondary }).hover(move |s| { s.bg(if a { theme.accent.primary.opacity(0.16) } else { theme.interactive.hover }) }).on_click(edge_top).child("Top").into_any_element(),
                            div().id("edge-seg-1").px(px(9.)).py(px(5.)).rounded_md().cursor_pointer().text_size(px(11.5)).font_family(theme.font_mono).bg(if b { theme.accent.primary.opacity(0.16) } else { gpui::transparent_black() }).text_color(if b { theme.accent.primary } else { theme.text.secondary }).hover(move |s| { s.bg(if b { theme.accent.primary.opacity(0.16) } else { theme.interactive.hover }) }).on_click(edge_bottom).child("Bottom").into_any_element(),
                        ]) }))
                    // Height
                    .child(div().w_full().flex().items_center().justify_between().gap(px(10.))
                        .child(div().flex_col().gap(px(1.)).child(div().text_color(theme.text.primary).text_size(px(12.)).child("Height")).child(div().text_color(theme.text.muted).text_xs().font_family(theme.font_mono).child("appearance.height")))
                        .child(div().flex().items_center().gap(px(8.))
                            .child(div().id("bar-h-minus").size(px(22.)).flex().items_center().justify_center().rounded_md().cursor_pointer().text_color(theme.text.secondary).hover(|s| s.bg(theme.interactive.hover)).on_click(h_minus).child("\u{2212}"))
                            .child(div().id("bar-h-track").relative().w(px(TW)).h(px(TH + 10.)).flex().items_center().cursor_pointer().on_drag(HeightSliderDrag, |_, _, _, cx| cx.new(|_| EmptyView)).on_drag_move(height_drag)
                                .child(div().w_full().h(px(TH)).rounded(px(2.)).bg(track_bg).relative()
                                    .child(div().absolute().left(px(0.)).top(px(0.)).bottom(px(0.)).w(px(TW * h_frac)).rounded(px(2.)).bg(track_fill))
                                    .child(div().absolute().top(px((TH - TB) / 2.)).left(px(TW * h_frac - TB / 2.)).size(px(TB)).rounded(px(TB / 2.)).bg(thumb))))
                            .child(div().id("bar-h-plus").size(px(22.)).flex().items_center().justify_center().rounded_md().cursor_pointer().text_color(theme.text.secondary).hover(|s| s.bg(theme.interactive.hover)).on_click(h_plus).child("+"))
                            .child(div().font_family(theme.font_mono).text_size(px(11.)).text_color(theme.text.muted).child(format!("{:.0}", cur.height)))))
                    // Width
                    .child(div().w_full().flex().items_center().justify_between().gap(px(10.))
                        .child(div().flex_col().gap(px(1.)).child(div().text_color(theme.text.primary).text_size(px(12.)).child("Width")).child(div().text_color(theme.text.muted).text_xs().font_family(theme.font_mono).child("appearance.width")))
                        .child({ let a = width == WidthChoice::Full; let b = width == WidthChoice::Fraction70; let c = width == WidthChoice::Fraction50; div().flex().gap(px(2.)).p(px(2.)).rounded_md().border_1().border_color(theme.border.subtle).children(vec![
                            div().id("width-0").px(px(9.)).py(px(5.)).rounded_md().cursor_pointer().text_size(px(11.5)).font_family(theme.font_mono).bg(if a { theme.accent.primary.opacity(0.16) } else { gpui::transparent_black() }).text_color(if a { theme.accent.primary } else { theme.text.secondary }).hover(move |s| { s.bg(if a { theme.accent.primary.opacity(0.16) } else { theme.interactive.hover }) }).on_click(w_full).child("Full").into_any_element(),
                            div().id("width-1").px(px(9.)).py(px(5.)).rounded_md().cursor_pointer().text_size(px(11.5)).font_family(theme.font_mono).bg(if b { theme.accent.primary.opacity(0.16) } else { gpui::transparent_black() }).text_color(if b { theme.accent.primary } else { theme.text.secondary }).hover(move |s| { s.bg(if b { theme.accent.primary.opacity(0.16) } else { theme.interactive.hover }) }).on_click(w_70).child("70%").into_any_element(),
                            div().id("width-2").px(px(9.)).py(px(5.)).rounded_md().cursor_pointer().text_size(px(11.5)).font_family(theme.font_mono).bg(if c { theme.accent.primary.opacity(0.16) } else { gpui::transparent_black() }).text_color(if c { theme.accent.primary } else { theme.text.secondary }).hover(move |s| { s.bg(if c { theme.accent.primary.opacity(0.16) } else { theme.interactive.hover }) }).on_click(w_50).child("50%").into_any_element(),
                        ]) }))
                    // Floating
                    .child(div().w_full().flex().items_center().justify_between().gap(px(10.))
                        .child(div().flex_col().gap(px(1.)).child(div().text_color(theme.text.primary).text_size(px(12.)).child("Floating")).child(div().text_color(theme.text.muted).text_xs().font_family(theme.font_mono).child("appearance.floating")))
                        .child(div().id("bar-ctrl-floating").px(px(10.)).py(px(5.)).rounded_md().cursor_pointer().text_size(px(11.5)).font_family(theme.font_mono)
                            .bg(if floating { theme.accent.primary.opacity(0.16) } else { gpui::transparent_black() }).text_color(if floating { theme.accent.primary } else { theme.text.secondary })
                            .hover(move |s| { s.bg(if floating { theme.accent.primary.opacity(0.16) } else { theme.interactive.hover }) }).border_1().border_color(if floating { theme.accent.primary } else { theme.border.subtle })
                            .child(if floating { "on" } else { "off" }).on_click(on_float)))
                    // Radius
                    .child(div().w_full().flex().items_center().justify_between().gap(px(10.))
                        .child(div().flex_col().gap(px(1.)).child(div().text_color(theme.text.primary).text_size(px(12.)).child("Radius")).child(div().text_color(theme.text.muted).text_xs().font_family(theme.font_mono).child("appearance.radius")))
                        .child(div().flex().items_center().gap(px(8.))
                            .child(div().id("bar-r-minus").size(px(22.)).flex().items_center().justify_center().rounded_md().cursor_pointer().text_color(theme.text.secondary).hover(|s| s.bg(theme.interactive.hover)).on_click(r_minus).child("\u{2212}"))
                            .child(div().id("bar-r-track").relative().w(px(TW)).h(px(TH + 10.)).flex().items_center().cursor_pointer().on_drag(RadiusSliderDrag, |_, _, _, cx| cx.new(|_| EmptyView)).on_drag_move(radius_drag)
                                .child(div().w_full().h(px(TH)).rounded(px(2.)).bg(track_bg).relative()
                                    .child(div().absolute().left(px(0.)).top(px(0.)).bottom(px(0.)).w(px(TW * r_frac)).rounded(px(2.)).bg(track_fill))
                                    .child(div().absolute().top(px((TH - TB) / 2.)).left(px(TW * r_frac - TB / 2.)).size(px(TB)).rounded(px(TB / 2.)).bg(thumb))))
                            .child(div().id("bar-r-plus").size(px(22.)).flex().items_center().justify_center().rounded_md().cursor_pointer().text_color(theme.text.secondary).hover(|s| s.bg(theme.interactive.hover)).on_click(r_plus).child("+"))
                            .child(div().font_family(theme.font_mono).text_size(px(11.)).text_color(theme.text.muted).child(format!("{:.0}", cur.radius)))))
                    // Elevation
                    .child(div().w_full().flex().items_center().justify_between().gap(px(10.))
                        .child(div().flex_col().gap(px(1.)).child(div().text_color(theme.text.primary).text_size(px(12.)).child("Elevation")).child(div().text_color(theme.text.muted).text_xs().font_family(theme.font_mono).child("appearance.elevation")))
                        .child({ let a = elevation == ElevationChoice::None; let b = elevation == ElevationChoice::Soft; let c = elevation == ElevationChoice::Strong; div().flex().gap(px(2.)).p(px(2.)).rounded_md().border_1().border_color(theme.border.subtle).children(vec![
                            div().id("elev-0").px(px(9.)).py(px(5.)).rounded_md().cursor_pointer().text_size(px(11.5)).font_family(theme.font_mono).bg(if a { theme.accent.primary.opacity(0.16) } else { gpui::transparent_black() }).text_color(if a { theme.accent.primary } else { theme.text.secondary }).hover(move |s| { s.bg(if a { theme.accent.primary.opacity(0.16) } else { theme.interactive.hover }) }).on_click(ev_none).child("None").into_any_element(),
                            div().id("elev-1").px(px(9.)).py(px(5.)).rounded_md().cursor_pointer().text_size(px(11.5)).font_family(theme.font_mono).bg(if b { theme.accent.primary.opacity(0.16) } else { gpui::transparent_black() }).text_color(if b { theme.accent.primary } else { theme.text.secondary }).hover(move |s| { s.bg(if b { theme.accent.primary.opacity(0.16) } else { theme.interactive.hover }) }).on_click(ev_soft).child("Soft").into_any_element(),
                            div().id("elev-2").px(px(9.)).py(px(5.)).rounded_md().cursor_pointer().text_size(px(11.5)).font_family(theme.font_mono).bg(if c { theme.accent.primary.opacity(0.16) } else { gpui::transparent_black() }).text_color(if c { theme.accent.primary } else { theme.text.secondary }).hover(move |s| { s.bg(if c { theme.accent.primary.opacity(0.16) } else { theme.interactive.hover }) }).on_click(ev_strong).child("Strong").into_any_element(),
                        ]) }))
                    // Exclusive
                    .child(div().w_full().flex().items_center().justify_between().gap(px(10.))
                        .child(div().flex_col().gap(px(1.)).child(div().text_color(theme.text.primary).text_size(px(12.)).child("Exclusive zone")).child(div().text_color(theme.text.muted).text_xs().font_family(theme.font_mono).child("appearance.exclusive")))
                        .child({ let chip = div().id("bar-ctrl-exclusive").px(px(10.)).py(px(5.)).rounded_md().cursor_pointer().text_size(px(11.5)).font_family(theme.font_mono)
                            .bg(if cur.exclusive { theme.accent.primary.opacity(0.16) } else { gpui::transparent_black() }).text_color(if cur.exclusive { theme.accent.primary } else { theme.text.secondary })
                            .hover(move |s| { s.bg(if cur.exclusive { theme.accent.primary.opacity(0.16) } else { theme.interactive.hover }) }).border_1().border_color(if cur.exclusive { theme.accent.primary } else { theme.border.subtle })
                            .child(if cur.exclusive { "on" } else { "off" }).on_click(on_excl);
                            if floating { div().opacity(0.35).child(chip).into_any_element() } else { chip.into_any_element() } }))
                    // Theme toggle (T196)
                    .child(div().w_full().flex_col().gap(px(2.)).child(div().text_color(theme.text.primary).text_size(px(12.)).font_weight(gpui::FontWeight::SEMIBOLD).child("Theme")).child(div().text_color(theme.text.muted).text_xs().child("theme.toml — hot-reload")))
                    .child(div().id("sys-theme-toggle").w_full().flex().justify_between().items_center().px(px(12.)).py(px(9.)).rounded_md().border_1().border_color(theme.border.subtle)
                        .child(div().flex_col().gap(px(1.)).child(div().text_color(theme.text.primary).text_size(px(12.)).child(if is_light { "☀ Light" } else { "🌙 Dark" })).child(div().text_color(theme.text.muted).text_xs().font_family(theme.font_mono).child(theme_scheme)))
                        .child(div().id("sys-theme-btn").px(px(10.)).py(px(5.)).rounded_md().cursor_pointer().text_size(px(11.5)).font_family(theme.font_mono).bg(theme.accent.primary.opacity(0.16)).text_color(theme.accent.primary).border_1().border_color(theme.accent.primary).hover(|s| s.bg(theme.accent.primary.opacity(0.28))).child("Toggle").on_click(toggle_theme)))
                    // Hypr modules (T196)
                    .child(div().w_full().flex_col().gap(px(2.)).child(div().text_color(theme.text.primary).text_size(px(12.)).font_weight(gpui::FontWeight::SEMIBOLD).child("Hypr modules")).child(div().text_color(theme.text.muted).text_xs().child("~/.config/hypr/modules/ — click to open in Editor")))
                    .child({
                        self.load_hypr_modules();
                        let mut rows: Vec<gpui::AnyElement> = Vec::new();
                        if self.hypr_modules.is_empty() {
                            rows.push(div().w_full().px(px(12.)).py(px(9.)).rounded_md().border_1().border_color(theme.border.subtle).text_color(theme.text.muted).text_xs().child("No modules found in ~/.config/hypr/modules/").into_any_element());
                        }
                        for (name, path) in &self.hypr_modules {
                            let p = path.clone();
                            rows.push(div().id(SharedString::from(format!("hypr-mod-{name}"))).w_full().flex().justify_between().items_center().px(px(12.)).py(px(9.)).rounded_md().border_1().border_color(theme.border.subtle).cursor_pointer().hover(|s| s.bg(theme.interactive.hover))
                                .child(div().flex_col().gap(px(1.)).child(div().text_color(theme.text.primary).text_size(px(12.)).font_family(theme.font_mono).child(name.clone())).child(div().text_color(theme.text.muted).text_xs().child(path.display().to_string())))
                                .child(div().text_color(theme.accent.primary).text_size(px(11.)).child("Open"))
                                .on_click(cx.listener(move |this, _ev, _w, cx| {
                                    cx.set_global(PreviewTarget { path: Some(p.clone()), generation: 1, intent: PreviewIntent::View });
                                    this.error = None;
                                    cx.notify();
                                })).into_any_element());
                        }
                        div().w_full().flex_col().gap(px(4.)).children(rows)
                    })
                    // About (T196)
                    .child(div().w_full().flex_col().gap(px(2.)).child(div().text_color(theme.text.primary).text_size(px(12.)).font_weight(gpui::FontWeight::SEMIBOLD).child("About")).child(div().text_color(theme.text.muted).text_xs().child("Build info")))
                    .child(div().w_full().flex_col().px(px(12.)).py(px(9.)).rounded_md().border_1().border_color(theme.border.subtle).gap(px(4.))
                        .child(div().flex().justify_between().child(div().text_color(theme.text.primary).text_size(px(12.)).child("ChronOS shell")).child(div().text_color(theme.text.muted).text_xs().font_family(theme.font_mono).child(env!("CARGO_PKG_VERSION"))))
                        .child(div().flex().justify_between().child(div().text_color(theme.text.muted).text_xs().child("Desktop shell for Hyprland")).child(div().text_color(theme.text.muted).text_xs().font_family(theme.font_mono).child("Apache-2.0")))
                        .child(div().flex().justify_between().child(div().text_color(theme.text.muted).text_xs().child("Rust + GPUI + mlua/LuauJIT")).child(div().text_color(theme.text.muted).text_xs().font_family(theme.font_mono).child("2026"))))
                    // Open config
                    .child(div().id("bar-settings-open-config").w_full().flex().justify_between().items_center().px(px(12.)).py(px(9.)).rounded_md().border_1().border_color(theme.border.subtle).cursor_pointer().hover(|s| s.bg(theme.interactive.hover))
                        .child(div().flex_col().gap(px(1.)).child(div().text_color(theme.text.primary).text_size(px(12.)).child("Open bar.toml")).child(div().text_color(theme.text.muted).text_xs().font_family(theme.font_mono).child("~/.config/chronos/bar.toml")))
                        .child(div().text_color(theme.accent.primary).text_size(px(12.)).child("Edit")).on_click(on_open))
                    // Error
                    .when_some(error, |d, e| { d.child(div().w_full().px(px(10.)).py(px(8.)).rounded_md().border_1().border_color(theme.status.error).text_color(theme.status.error).text_xs().child(e)) }),
            )
    }
}

#[cfg(test)]
mod tests { #[test] fn placeholder() {} }
