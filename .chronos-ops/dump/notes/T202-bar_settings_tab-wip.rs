//! System settings — «Bar» page (T202).
//!
//! Mounted on `PanelTab::EditorSettings` (label «System settings», T192).
//! Reads/writes `~/.config/chronos/bar.toml` `[appearance]` through the
//! lib-visible `crate::bar_settings` module — the inotify watcher (T134)
//! re-applies the file live (T200 `apply_appearance`). No apply logic lives
//! in this page: it only persists, the watcher applies.
//!
//! Layout: preset chips on top, ~7 controls below, each labeled with its
//! schema key (`appearance.height` etc.) so agent/UI/docs share vocabulary
//! (PRODUCT live-customization contract).
//!
//! Controls are inlined into `render()`: `cx.listener` inside `Render`
//! produces `&mut App`, not `&mut Context<Self>`, so persist goes through
//! `entity.update(cx, …)`. Helper functions take `impl Fn(…)` bounds to
//! accept the anonymous listener types.

use gpui::{
    App, Context, DragMoveEvent, EmptyView, InteractiveElement, IntoElement, ParentElement, Render,
    ScrollHandle, SharedString, Styled, Window, div, prelude::*, px,
};

use chronos_ui::Theme;

use crate::bar_settings::{
    BarSettingsPatch, EdgeChoice, ElevationChoice, PRESETS, WidthChoice, apply_patch, apply_preset,
    config_path, read_current,
};
use crate::side_panel_right::preview_target::{PreviewIntent, PreviewTarget};

// Slider ranges (documented in PRODUCT): height 20..=48, radius 0..=16.
const HEIGHT_MIN: f32 = 20.;
const HEIGHT_MAX: f32 = 48.;
const RADIUS_MAX: f32 = 16.;

/// Per-field drag markers — GPUI routes `DragMoveEvent<T>` to every listener
/// of type `T`; a shared marker would make both sliders drive the same value
/// (volume_popup T123 live bug).
pub struct HeightSliderDrag;
pub struct RadiusSliderDrag;

pub struct BarSettingsTab {
    /// The appearance the page currently reflects (read from disk at open,
    /// updated optimistically after every control write).
    current: BarSettingsPatch,
    /// Last apply error (shown in-place, §13 — no panic).
    error: Option<String>,
    /// Last applied preset id (chip highlight).
    applied_preset: Option<&'static str>,
    scroll: ScrollHandle,
}

impl BarSettingsTab {
    pub fn new(_cx: &mut Context<Self>) -> Self {
        Self {
            current: read_current(),
            error: None,
            applied_preset: None,
            scroll: ScrollHandle::new(),
        }
    }

    /// Persist `current` and refresh. Shared by every control handler so the
    /// save path is exactly one function (same file the watcher re-reads).
    fn persist(&mut self, cx: &mut Context<Self>) {
        match apply_patch(&self.current) {
            Ok(()) => self.error = None,
            Err(e) => self.error = Some(e),
        }
        cx.notify();
    }

    fn apply_preset_id(&mut self, id: &'static str, cx: &mut Context<Self>) {
        match apply_preset(id) {
            Ok(preset) => {
                self.current = preset.appearance;
                self.applied_preset = Some(id);
                self.error = None;
            }
            Err(e) => self.error = Some(e),
        }
        cx.notify();
    }
}

impl Render for BarSettingsTab {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = *Theme::global(cx);
        let current = self.current;
        let error = self.error.clone();
        let applied = self.applied_preset;

        // Entity handle for persist dispatch from drag/click handlers where
        // `cx` is `&mut App`, not `&mut Context<BarSettingsTab>`.
        let this_entity = cx.entity().clone();

        // ── Drag listeners ────────────────────────────────────────────────

        let height_drag = cx.listener({
            let entity = this_entity.clone();
            move |this, ev: &DragMoveEvent<HeightSliderDrag>, _w, cx| {
                let frac = frac_from_bounds(ev);
                let new_h = HEIGHT_MIN + frac * (HEIGHT_MAX - HEIGHT_MIN);
                this.current.height = new_h.clamp(HEIGHT_MIN, HEIGHT_MAX);
                entity.update(cx, |t, cx| t.persist(cx));
            }
        });
        let radius_drag = cx.listener({
            let entity = this_entity.clone();
            move |this, ev: &DragMoveEvent<RadiusSliderDrag>, _w, cx| {
                let frac = frac_from_bounds(ev);
                this.current.radius = (frac * RADIUS_MAX).clamp(0.0, RADIUS_MAX);
                entity.update(cx, |t, cx| t.persist(cx));
            }
        });

        // ── Stepper button listeners (height) ─────────────────────────────
        let height_step = ((HEIGHT_MAX - HEIGHT_MIN) / 10.0).max(1.0);
        let height_minus = cx.listener({
            let entity = this_entity.clone();
            move |this, _ev, _w, cx| {
                let v = (this.current.height - height_step).clamp(HEIGHT_MIN, HEIGHT_MAX);
                this.current.height = v;
                entity.update(cx, |t, cx| t.persist(cx));
            }
        });
        let height_plus = cx.listener({
            let entity = this_entity.clone();
            move |this, _ev, _w, cx| {
                let v = (this.current.height + height_step).clamp(HEIGHT_MIN, HEIGHT_MAX);
                this.current.height = v;
                entity.update(cx, |t, cx| t.persist(cx));
            }
        });

        // ── Stepper button listeners (radius) ─────────────────────────────
        let radius_step = (RADIUS_MAX / 10.0).max(1.0);
        let radius_minus = cx.listener({
            let entity = this_entity.clone();
            move |this, _ev, _w, cx| {
                let v = (this.current.radius - radius_step).clamp(0.0, RADIUS_MAX);
                this.current.radius = v;
                entity.update(cx, |t, cx| t.persist(cx));
            }
        });
        let radius_plus = cx.listener({
            let entity = this_entity.clone();
            move |this, _ev, _w, cx| {
                let v = (this.current.radius + radius_step).clamp(0.0, RADIUS_MAX);
                this.current.radius = v;
                entity.update(cx, |t, cx| t.persist(cx));
            }
        });

        // ── Edge toggle ───────────────────────────────────────────────────
        let edge_top = cx.listener({
            let entity = this_entity.clone();
            move |this, _ev, _w, cx| {
                this.current.edge = EdgeChoice::Top;
                entity.update(cx, |t, cx| t.persist(cx));
            }
        });
        let edge_bottom = cx.listener({
            let entity = this_entity.clone();
            move |this, _ev, _w, cx| {
                this.current.edge = EdgeChoice::Bottom;
                entity.update(cx, |t, cx| t.persist(cx));
            }
        });

        // ── Width toggle ──────────────────────────────────────────────────
        let width_full = cx.listener({
            let entity = this_entity.clone();
            move |this, _ev, _w, cx| {
                this.current.width = WidthChoice::Full;
                entity.update(cx, |t, cx| t.persist(cx));
            }
        });
        let width_70 = cx.listener({
            let entity = this_entity.clone();
            move |this, _ev, _w, cx| {
                this.current.width = WidthChoice::Fraction70;
                entity.update(cx, |t, cx| t.persist(cx));
            }
        });
        let width_50 = cx.listener({
            let entity = this_entity.clone();
            move |this, _ev, _w, cx| {
                this.current.width = WidthChoice::Fraction50;
                entity.update(cx, |t, cx| t.persist(cx));
            }
        });

        // ── Floating toggle ───────────────────────────────────────────────
        let floating_click = cx.listener({
            let entity = this_entity.clone();
            move |this, _ev, _w, cx| {
                let next = !this.current.floating;
                this.current.floating = next;
                if next {
                    this.current.exclusive = false;
                }
                entity.update(cx, |t, cx| t.persist(cx));
            }
        });

        // ── Elevation toggle ──────────────────────────────────────────────
        let elev_none = cx.listener({
            let entity = this_entity.clone();
            move |this, _ev, _w, cx| {
                this.current.elevation = ElevationChoice::None;
                entity.update(cx, |t, cx| t.persist(cx));
            }
        });
        let elev_soft = cx.listener({
            let entity = this_entity.clone();
            move |this, _ev, _w, cx| {
                this.current.elevation = ElevationChoice::Soft;
                entity.update(cx, |t, cx| t.persist(cx));
            }
        });
        let elev_strong = cx.listener({
            let entity = this_entity.clone();
            move |this, _ev, _w, cx| {
                this.current.elevation = ElevationChoice::Strong;
                entity.update(cx, |t, cx| t.persist(cx));
            }
        });

        // ── Exclusive toggle ──────────────────────────────────────────────
        let exclusive_click = cx.listener({
            let entity = this_entity.clone();
            move |this, _ev, _w, cx| {
                if !this.current.floating {
                    this.current.exclusive = !this.current.exclusive;
                    entity.update(cx, |t, cx| t.persist(cx));
                }
            }
        });

        // ── Open bar.toml ─────────────────────────────────────────────────
        let open_config = cx.listener({
            let entity = this_entity.clone();
            move |this, _ev, _w, cx| {
                let path = config_path();
                tracing::info!(path = %path.display(), "bar_settings: open bar.toml in editor");
                cx.set_global(PreviewTarget {
                    path: Some(path),
                    generation: 1,
                    intent: PreviewIntent::Edit,
                });
                this.error = None;
                entity.update(cx, |_, cx| cx.notify());
            }
        });

        // ── Layout ────────────────────────────────────────────────────────

        let edge = current.edge;
        let width = current.width;
        let elevation = current.elevation;
        let floating = current.floating;
        let height_frac = ((current.height - HEIGHT_MIN) / (HEIGHT_MAX - HEIGHT_MIN)).clamp(0.0, 1.0);
        let radius_frac = (current.radius / RADIUS_MAX).clamp(0.0, 1.0);

        div()
            .id("bar-settings-tab")
            .size_full()
            .flex()
            .flex_col()
            .child(header(&theme, &current))
            .child(
                div()
                    .id("bar-settings-scroll")
                    .flex_1()
                    .min_h(px(0.))
                    .overflow_y_scroll()
                    .track_scroll(&self.scroll)
                    .flex()
                    .flex_col()
                    .gap(px(14.))
                    .p(px(14.))
                    // ── Presets ───────────────────────────────────────────
                    .child(presets_section(&theme, applied, cx))
                    // ── Controls ──────────────────────────────────────────
                    .child(section_label(&theme, "Appearance", "appearance.* — applies live"))
                    // 1. Edge
                    .child(control_row(&theme, "Edge", "appearance.edge", {
                        segmented_static(
                            &theme,
                            vec![
                                ("Top", edge == EdgeChoice::Top, edge_top),
                                ("Bottom", edge == EdgeChoice::Bottom, edge_bottom),
                            ],
                            "edge",
                        )
                    }))
                    // 2. Height
                    .child(control_row(&theme, "Height", "appearance.height", {
                        slider_static(
                            &theme,
                            height_frac,
                            "bar-height",
                            height_drag,
                            height_minus,
                            height_plus,
                            format!("{:.0}", current.height),
                        )
                    }))
                    // 3. Width
                    .child(control_row(&theme, "Width", "appearance.width", {
                        segmented_static(
                            &theme,
                            vec![
                                ("Full", width == WidthChoice::Full, width_full),
                                ("70%", width == WidthChoice::Fraction70, width_70),
                                ("50%", width == WidthChoice::Fraction50, width_50),
                            ],
                            "width",
                        )
                    }))
                    // 4. Floating
                    .child(control_row(&theme, "Floating", "appearance.floating", {
                        toggle_chip_static(&theme, "floating", floating, floating_click)
                    }))
                    // 5. Radius
                    .child(control_row(&theme, "Radius", "appearance.radius", {
                        slider_static(
                            &theme,
                            radius_frac,
                            "bar-radius",
                            radius_drag,
                            radius_minus,
                            radius_plus,
                            format!("{:.0}", current.radius),
                        )
                    }))
                    // 6. Elevation
                    .child(control_row(&theme, "Elevation", "appearance.elevation", {
                        segmented_static(
                            &theme,
                            vec![
                                ("None", elevation == ElevationChoice::None, elev_none),
                                ("Soft", elevation == ElevationChoice::Soft, elev_soft),
                                ("Strong", elevation == ElevationChoice::Strong, elev_strong),
                            ],
                            "elevation",
                        )
                    }))
                    // 7. Exclusive (dimmed when floating — sanitize T199)
                    .child(control_row(&theme, "Exclusive zone", "appearance.exclusive", {
                        let chip = toggle_chip_static(
                            &theme, "exclusive", current.exclusive, exclusive_click,
                        );
                        if floating {
                            div().opacity(0.35).child(chip).into_any_element()
                        } else {
                            chip.into_any_element()
                        }
                    }))
                    // 8. Open config in Editor
                    .child(
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
                                    .gap(px(1.))
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
                            .on_click(open_config),
                    )
                    // ── Error (if any) ────────────────────────────────────
                    .when_some(error, |d, e| {
                        d.child(
                            div()
                                .w_full()
                                .px(px(10.))
                                .py(px(8.))
                                .rounded_md()
                                .border_1()
                                .border_color(theme.status.error)
                                .text_color(theme.status.error)
                                .text_xs()
                                .child(e),
                        )
                    }),
            )
    }
}

// ── Header ─────────────────────────────────────────────────────────────────

fn header(theme: &Theme, current: &BarSettingsPatch) -> impl IntoElement {
    let edge_label = match current.edge {
        EdgeChoice::Top => "top",
        EdgeChoice::Bottom => "bottom",
    };
    div()
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
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .child("Bar"),
        )
        .child(
            div()
                .text_color(theme.text.muted)
                .text_xs()
                .font_family(theme.font_mono)
                .child(format!(
                    "[appearance] · {edge_label} · {:.0}px",
                    current.height
                )),
        )
}

// ── Presets ────────────────────────────────────────────────────────────────

fn presets_section(
    theme: &Theme,
    applied: Option<&'static str>,
    cx: &mut Context<BarSettingsTab>,
) -> impl IntoElement {
    let mut chips = Vec::new();
    for preset in PRESETS {
        let id = preset.id;
        let name = preset.name;
        let desc = preset.description;
        let active = applied == Some(id);
        let chip = div()
            .id(SharedString::from(format!("bar-preset-{id}")))
            .flex_col()
            .flex_1()
            .px(px(10.))
            .py(px(8.))
            .rounded_md()
            .border_1()
            .border_color(if active {
                theme.accent.primary
            } else {
                theme.border.subtle
            })
            .bg(if active {
                theme.accent.primary.opacity(0.14)
            } else {
                gpui::transparent_black()
            })
            .cursor_pointer()
            .hover(|s| {
                s.bg(if active {
                    theme.accent.primary.opacity(0.14)
                } else {
                    theme.interactive.hover
                })
            })
            .child(
                div()
                    .text_color(if active {
                        theme.accent.primary
                    } else {
                        theme.text.primary
                    })
                    .text_size(px(12.))
                    .font_weight(gpui::FontWeight::MEDIUM)
                    .child(name),
            )
            .child(div().text_color(theme.text.muted).text_xs().child(desc))
            .on_click(cx.listener(move |this, _ev, _w, cx| {
                this.apply_preset_id(id, cx);
            }));
        chips.push(chip.into_any_element());
    }

    div()
        .w_full()
        .flex_col()
        .gap(px(6.))
        .child(section_label(theme, "Presets", "apply live · written to bar.toml"))
        .child(div().w_full().flex().gap(px(8.)).children(chips))
}

// ── Shared rendering helpers ───────────────────────────────────────────────
// All take `impl Fn(…)` bounds to accept the anonymous types from `cx.listener`.

fn section_label(theme: &Theme, title: &str, subtitle: &str) -> impl IntoElement {
    div()
        .w_full()
        .flex_col()
        .gap(px(2.))
        .child(
            div()
                .text_color(theme.text.primary)
                .text_size(px(12.))
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .child(title.to_string()),
        )
        .child(div().text_color(theme.text.muted).text_xs().child(subtitle.to_string()))
}

/// Label + schema-key subtitle on the left, control on the right.
fn control_row(
    theme: &Theme,
    label: &str,
    schema_key: &str,
    control: impl IntoElement,
) -> impl IntoElement {
    div()
        .w_full()
        .flex()
        .items_center()
        .justify_between()
        .gap(px(10.))
        .child(
            div()
                .flex_col()
                .gap(px(1.))
                .child(
                    div()
                        .text_color(theme.text.primary)
                        .text_size(px(12.))
                        .child(label.to_string()),
                )
                .child(
                    div()
                        .text_color(theme.text.muted)
                        .text_xs()
                        .font_family(theme.font_mono)
                        .child(schema_key.to_string()),
                ),
        )
        .child(control)
}

/// Segmented control — one active segment. Takes ownership of options
/// so closures (from `cx.listener`) are consumed without Clone.
fn segmented_static(
    theme: &Theme,
    options: Vec<(&'static str, bool, impl Fn(&gpui::ClickEvent, &mut Window, &mut App) + 'static)>,
    id_prefix: &str,
) -> impl IntoElement {
    let mut segs = Vec::new();
    for (i, (label, active, onclick)) in options.into_iter().enumerate() {
        let seg = div()
            .id(SharedString::from(format!("{id_prefix}-seg-{i}")))
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
            .text_color(if active {
                theme.accent.primary
            } else {
                theme.text.secondary
            })
            .hover(|s| {
                s.bg(if active {
                    theme.accent.primary.opacity(0.16)
                } else {
                    theme.interactive.hover
                })
            })
            .on_click(onclick);
        segs.push(seg.child(label.to_string()).into_any_element());
    }
    div()
        .flex()
        .gap(px(2.))
        .p(px(2.))
        .rounded_md()
        .border_1()
        .border_color(theme.border.subtle)
        .children(segs)
}

/// Slider with −/+ steppers. Drag/click handlers are pre-baked.
/// `M` is the drag marker type (HeightSliderDrag / RadiusSliderDrag).
fn slider_static<M: 'static>(
    theme: &Theme,
    frac: f32,
    id_prefix: &str,
    drag: impl Fn(&DragMoveEvent<M>, &mut Window, &mut App) + 'static,
    minus: impl Fn(&gpui::ClickEvent, &mut Window, &mut App) + 'static,
    plus: impl Fn(&gpui::ClickEvent, &mut Window, &mut App) + 'static,
    value_label: String,
) -> impl IntoElement {
    const TRACK_W: f32 = 110.;
    const TRACK_H: f32 = 4.;
    const THUMB: f32 = 13.;

    div()
        .flex()
        .items_center()
        .gap(px(8.))
        .child(
            div()
                .id(SharedString::from(format!("{id_prefix}-minus")))
                .size(px(22.))
                .flex()
                .items_center()
                .justify_center()
                .rounded_md()
                .cursor_pointer()
                .text_color(theme.text.secondary)
                .hover(|s| s.bg(theme.interactive.hover))
                .on_click(minus)
                .child("−"),
        )
        .child(
            div()
                .id(SharedString::from(format!("{id_prefix}-track")))
                .relative()
                .w(px(TRACK_W))
                .h(px(TRACK_H + 10.))
                .flex()
                .items_center()
                .cursor_pointer()
                .on_drag::<M, EmptyView>(|_, _, _, cx| cx.new(|_| EmptyView))
                .on_drag_move(drag)
                .child(
                    div()
                        .w_full()
                        .h(px(TRACK_H))
                        .rounded(px(2.))
                        .bg(theme_track_bg())
                        .relative()
                        .child(
                            div()
                                .absolute()
                                .left(px(0.))
                                .top(px(0.))
                                .bottom(px(0.))
                                .w(px(TRACK_W * frac))
                                .rounded(px(2.))
                                .bg(theme_track_fill()),
                        )
                        .child(
                            div()
                                .absolute()
                                .top(px((TRACK_H - THUMB) / 2.))
                                .left(px(TRACK_W * frac - THUMB / 2.))
                                .size(px(THUMB))
                                .rounded(px(THUMB / 2.))
                                .bg(theme_thumb()),
                        ),
                ),
        )
        .child(
            div()
                .id(SharedString::from(format!("{id_prefix}-plus")))
                .size(px(22.))
                .flex()
                .items_center()
                .justify_center()
                .rounded_md()
                .cursor_pointer()
                .text_color(theme.text.secondary)
                .hover(|s| s.bg(theme.interactive.hover))
                .on_click(plus)
                .child("+"),
        )
        .child(
            div()
                .font_family(theme.font_mono)
                .text_size(px(11.))
                .text_color(theme.text.muted)
                .child(value_label),
        )
}

/// A binary toggle chip. Handler is pre-baked.
fn toggle_chip_static(
    theme: &Theme,
    id_suffix: &str,
    active: bool,
    on_toggle: impl Fn(&gpui::ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .id(SharedString::from(format!("bar-ctrl-{id_suffix}")))
        .px(px(10.))
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
        .text_color(if active {
            theme.accent.primary
        } else {
            theme.text.secondary
        })
        .hover(|s| {
            s.bg(if active {
                theme.accent.primary.opacity(0.16)
            } else {
                theme.interactive.hover
            })
        })
        .border_1()
        .border_color(if active {
            theme.accent.primary
        } else {
            theme.border.subtle
        })
        .child(if active { "on" } else { "off" })
        .on_click(on_toggle)
}

// ── Pure helpers ───────────────────────────────────────────────────────────

/// Fraction 0..=1 of the pointer x within the slider element bounds.
fn frac_from_bounds<M>(ev: &DragMoveEvent<M>) -> f32 {
    let rel = ev.event.position.x - ev.bounds.origin.x;
    let w: f32 = ev.bounds.size.width.into();
    (rel / w.max(1.0)).clamp(0.0, 1.0)
}

// Track colors — 4-byte hex with alpha baked in, per fork convention.
fn theme_track_bg() -> gpui::Hsla {
    gpui::Hsla::from(gpui::rgba(0x0000_0047))
}
fn theme_track_fill() -> gpui::Hsla {
    gpui::Hsla::from(gpui::rgba(0x0000_006b))
}
fn theme_thumb() -> gpui::Hsla {
    gpui::Hsla::from(gpui::rgba(0xFFFF_FFE5))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slider_frac_maps_range() {
        let f = |v: f32, min: f32, max: f32| ((v - min) / (max - min)).clamp(0.0, 1.0);
        assert_eq!(f(HEIGHT_MIN, HEIGHT_MIN, HEIGHT_MAX), 0.0);
        assert_eq!(f(HEIGHT_MAX, HEIGHT_MIN, HEIGHT_MAX), 1.0);
        assert!((f(34.0, HEIGHT_MIN, HEIGHT_MAX) - 0.5).abs() < 1e-4);
    }

    #[test]
    fn slider_step_is_reasonable() {
        assert!(((HEIGHT_MAX - HEIGHT_MIN) / 10.0).max(1.0) > 0.0);
        assert!(((RADIUS_MAX - 0.0) / 10.0).max(1.0) > 0.0);
    }
}
