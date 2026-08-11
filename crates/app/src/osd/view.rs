//! OSD volume strip view — progress bar + icon/label from `OsdPopupState`.
//!
//! Визуал бара портирован с эталона `docs/design/Volume-OSD.dc.html`
//! («Volume OSD — Decorative Bar»). Сверённые состояния (по РЕНДЕРЕННЫМ
//! вариантам в теле HTML, не по легенде — см. отчёт T262):
//!   * Normal (72%) / Full (100%) — заливка `#89b4fa`  → `status.info`
//!   * Low    (18%)            — заливка `#f9e2af`  → `status.warning`
//!   * Muted  (0%)             — заливка `#f38ba8`  → `status.error`
//! Трек тёмный (`#313244`), заливка скругляется, ширина плавно
//! переходит при смене громкости (`width .35s` ≈ `ease_out_quint`).

use gpui::{
    AnimationExt, App, Animation, Context, FontWeight, Render, Window, div, easing, prelude::*, px,
};

use chronos_ui::{Theme, WindowRootExt};

use crate::osd::OsdPopupState;

/// Empty view; all content is read from the `OsdPopupState` global.
pub struct OsdView {
    /// Fill width at the start of the current width transition.
    from_fraction: f32,
    /// Fill width the current transition animates towards (= live target).
    to_fraction: f32,
    /// Bumped on every volume change so `with_animation` replays the
    /// width tween (the OSD window is reused, not remounted, on change).
    anim_id: u64,
}

impl OsdView {
    pub fn new(_cx: &mut App) -> Self {
        Self {
            from_fraction: 0.0,
            to_fraction: 0.0,
            anim_id: 0,
        }
    }
}

/// Карта геометрии карточки — детерминированная, чтобы заливка не
/// вылезала за трек (старый код считал ширину заливки от 260px при
/// реальной ширине колонки ~248px → переполнение, скрытое `overflow_hidden`).
const CARD_W: f32 = 320.;
const CARD_PAD: f32 = 16.;
const ICON_W: f32 = 28.;
const GAP: f32 = 12.;
/// Ширина колонки с трекорм = карточка минус паддинги, иконка и зазор.
const TRACK_W: f32 = CARD_W - CARD_PAD * 2.0 - ICON_W - GAP;

/// Длительность перехода ширины заливки — эталон `transition:width .35s`.
const FILL_TRANSITION: std::time::Duration = std::time::Duration::from_millis(350);

impl Render for OsdView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = Theme::global(cx);
        let Some(display) = cx.global::<OsdPopupState>().display().cloned() else {
            return div().into_any_element();
        };

        let fraction = display.bar_fraction();
        let percent = display.percent_label();
        let muted = display.muted;
        let is_source = display.is_source;
        let name = display.name;

        let icon = if is_source {
            if muted { "🎤̸" } else { "🎤" }
        } else if muted {
            "🔇"
        } else if fraction < 0.01 {
            "🔈"
        } else if fraction < 0.5 {
            "🔉"
        } else {
            "🔊"
        };

        let kind_label = if is_source {
            "Микрофон"
        } else {
            "Громкость"
        };

        // === Семантический цвет заливки (по эталону) =========================
        // Normal/Full — синий (info); Low — жёлтый (warning); Muted — красный
        // (error). Все три уже есть в `theme.status` — хардкодить запрещено.
        let fill = if muted {
            theme.status.error
        } else if fraction < 0.5 {
            theme.status.warning
        } else {
            theme.status.info
        };

        let bg = theme.bg.elevated;
        let bar_track = theme.bg.secondary;
        let text_primary = if muted {
            theme.status.error
        } else {
            theme.text.primary
        };
        let text_secondary = theme.text.secondary;
        let radius = theme.radius_lg;
        // Трек/заливка — небольшое скругление (эталон 2px на 4px-треке;
        // здесь 8px-трек → 4px читается пропорционально).
        let bar_radius = px(4.);

        // === Ширина заливки: tween при смене громкости =======================
        if (fraction - self.to_fraction).abs() > 1e-3 {
            self.from_fraction = self.to_fraction;
            self.to_fraction = fraction;
            self.anim_id += 1;
        }
        let from = self.from_fraction;
        let to = self.to_fraction;
        let anim_id = self.anim_id;

        // Outer: full layer-shell width (BOTTOM|LEFT|RIGHT), centre the card.
        let card = div()
            .window_font(theme)
            .flex()
            .items_center()
            .gap(px(GAP))
            .w(px(CARD_W))
            .h(px(80.))
            .px(px(CARD_PAD))
            .py(px(12.))
            .rounded(radius)
            .bg(bg)
            .child(
                div()
                    .flex_none()
                    .w(px(ICON_W))
                    .text_color(fill)
                    .text_lg()
                    .child(icon.to_string()),
            )
            .child(
                div()
                    .flex_1()
                    .flex_col()
                    .gap(px(6.))
                    .child(
                        div()
                            .flex()
                            .justify_between()
                            .items_center()
                            .child(
                                div()
                                    .text_color(text_primary)
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child(kind_label.to_string()),
                            )
                            .child(div().text_color(text_secondary).child(if muted {
                                "mute".to_string()
                            } else {
                                format!("{percent}%")
                            })),
                    )
                    // Track + fill.
                    .child(
                        div()
                            .w(px(TRACK_W))
                            .h(px(8.))
                            .rounded(bar_radius)
                            .bg(bar_track)
                            .overflow_hidden()
                            .child(
                                div()
                                    .h_full()
                                    .rounded(bar_radius)
                                    .bg(fill)
                                    .with_animation(
                                        format!("osd-fill-{anim_id}"),
                                        Animation::new(FILL_TRANSITION)
                                            .with_easing(easing::ease_out_quint()),
                                        move |el, phase| {
                                            let w = from + (to - from) * phase;
                                            el.w(px(TRACK_W * w))
                                        },
                                    ),
                            ),
                    )
                    .child(
                        div()
                            .text_color(text_secondary)
                            .text_xs()
                            .child(if name.is_empty() { String::new() } else { name }),
                    ),
            );

        div()
            .size_full()
            .flex()
            .justify_center()
            .items_end()
            .child(card)
            .into_any_element()
    }
}
