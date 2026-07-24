//! Updates popup view — pending-update list + "Upgrade all" button.
//!
//! Pixel-faithful to `design/Updates Popup.dc.html` (dark reference + light
//! Light C variant). Every hex, padding, radius, font-size, font-weight here
//! comes from that mockup — do not re-derive by eye.

use gpui::{
    AnyElement, App, BoxShadow, Context, InteractiveElement, IntoElement, Render, ScrollHandle,
    Styled, Window, div, prelude::*, px, svg,
};

use chronos_services::{PackageUpdate, Service, UpdateSource, UpgradeState};

use crate::state::AppState;
use crate::updates_popup::{LIST_MAX_H, close_this, upgrade_all};

use chronos_ui::Theme;

// ── Geometry from mockup ────────────────────────────────────────────
const HEADER_PY: f32 = 12.;
const HEADER_PX: f32 = 14.;
const ROW_PY: f32 = 9.;
const ROW_PX: f32 = 14.;
const FOOTER_PY: f32 = 12.;
const FOOTER_PX: f32 = 14.;
const BTN_PY: f32 = 8.;

pub struct UpdatesPopupView {
    scroll: ScrollHandle,
}

impl UpdatesPopupView {
    pub fn new(_cx: &mut App) -> Self {
        Self {
            scroll: ScrollHandle::new(),
        }
    }
}

impl Render for UpdatesPopupView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let state = AppState::aur(cx).get();
        let updates = state.updates.clone();
        let count = updates.len();

        let theme = Theme::global(cx);
        let bg = theme.bg.primary;
        let text_primary = theme.text.primary;
        let text_muted = theme.text.muted;
        let text_secondary = theme.text.secondary;
        let border = theme.border.default;
        let radius = theme.radius;       // 6px
        let radius_lg = theme.radius_lg; // 12px
        let accent = theme.accent.primary;
        let accent_hover = theme.accent.hover;
        let hover = theme.interactive.hover;
        let font_mono = theme.font_mono;
        let font_mono = theme.font_mono;
        let is_light = theme.is_light;

        // ── Header ──────────────────────────────────────────────────
        let header = div()
            .w_full()
            .flex()
            .items_center()
            .justify_between()
            .px(px(HEADER_PX))
            .py(px(HEADER_PY))
            .border_b_1()
            .border_color(border)
            .child(
                div()
                    .text_color(text_primary)
                    .font_family(font_mono)
                    .text_size(theme.font_sizes.sm)
                    .child(if count > 0 {
                        format!("Updates ({count})")
                    } else {
                        "Updates".to_string()
                    }),
            )
            .child(
                div()
                    .id("updates-popup-close")
                    .cursor_pointer()
                    .w(px(22.))
                    .h(px(22.))
                    .rounded(radius)
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_color(text_muted)
                    .hover(|s| s.bg(hover))
                    .child(svg().path("icons/x.svg").size(px(13.)))
                    .on_click(|_event, window, cx: &mut App| {
                        close_this(window, cx);
                    }),
            );

        // ── List ────────────────────────────────────────────────────
        let list: AnyElement = if updates.is_empty() {
            div()
                .w_full()
                .px(px(ROW_PX))
                .py(px(ROW_PY))
                .text_color(text_muted)
                .font_family(font_mono)
                .text_size(theme.font_sizes.sm)
                .child("System is up to date")
                .into_any_element()
        } else {
            let rows: Vec<AnyElement> = updates
                .iter()
                .map(|u| {
                    render_row(
                        u,
                        text_primary,
                        text_secondary,
                        text_muted,
                        hover,
                        accent,
                        border,
                        font_mono,
                        radius,
                    )
                })
                .collect();
            div()
                .id("updates-popup-list")
                .w_full()
                .max_h(px(LIST_MAX_H))
                .overflow_y_scroll()
                .track_scroll(&self.scroll)
                .flex_col()
                .children(rows)
                .into_any_element()
        };

        // ── Footer ──────────────────────────────────────────────────
        let upgrade_state = state.upgrade_state;
        let footer: AnyElement = if updates.is_empty() && upgrade_state == UpgradeState::Idle {
            div().into_any_element()
        } else {
            let status_line: AnyElement = match upgrade_state {
                UpgradeState::Idle => div().into_any_element(),
                UpgradeState::Running => div().into_any_element(),
                UpgradeState::Done => div()
                    .w_full()
                    .px(px(FOOTER_PX))
                    .pb(px(2.))
                    .text_color(theme.status.success)
                    .font_family(font_mono)
                    .text_size(px(12.5))
                    .child("Upgrade complete")
                    .into_any_element(),
                UpgradeState::Failed => div()
                    .w_full()
                    .px(px(FOOTER_PX))
                    .pb(px(2.))
                    .text_color(theme.status.error)
                    .font_family(font_mono)
                    .text_size(px(12.5))
                    .child("Upgrade failed")
                    .into_any_element(),
            };

            let button: AnyElement = if upgrade_state == UpgradeState::Running {
                div()
                    .id("updates-popup-upgrade-all")
                    .w_full()
                    .flex()
                    .items_center()
                    .justify_center()
                    .py(px(BTN_PY))
                    .rounded(radius)
                    .border_1()
                    .border_color(text_muted)
                    .text_color(text_muted)
                    .font_family(font_mono)
                    .text_size(px(12.5))
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .child("Upgrading…")
                    .into_any_element()
            } else if !updates.is_empty() {
                div()
                    .id("updates-popup-upgrade-all")
                    .w_full()
                    .flex()
                    .items_center()
                    .justify_center()
                    .cursor_pointer()
                    .py(px(BTN_PY))
                    .rounded(radius)
                    .border_1()
                    .border_color(accent)
                    .text_color(accent)
                    .font_family(font_mono)
                    .text_size(px(12.5))
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .hover(|s| s.border_color(accent_hover).text_color(accent_hover))
                    .child("Upgrade all")
                    .on_click(|_event, window, cx: &mut App| {
                        upgrade_all(window, cx);
                    })
                    .into_any_element()
            } else {
                div().into_any_element()
            };

            div()
                .w_full()
                .flex_col()
                .child(status_line)
                .child(
                    div()
                        .w_full()
                        .px(px(FOOTER_PX))
                        .py(px(FOOTER_PY))
                        .child(button),
                )
                .into_any_element()
        };

        // ── Card ────────────────────────────────────────────────────
        let mut card = div()
            .relative()
            .flex_col()
            .rounded(px(10.)) // mockup: border-radius:10px
            .bg(bg)
            .border_1()
            .border_color(border)
            .overflow_hidden();

        if is_light {
            card = card
                .shadow(vec![
                    BoxShadow::new(px(0.), px(6.), gpui::rgba(0x3c40_6e29).into())
                        .blur_radius(px(24.)),
                    BoxShadow::new(px(0.), px(0.), gpui::rgba(0x007a_cc26).into())
                        .spread_radius(px(1.))
                        .inset(),
                ])
                .child(
                    div()
                        .absolute()
                        .top(px(0.))
                        .left(px(0.))
                        .right(px(0.))
                        .h(px(1.))
                        .bg(accent)
                        .opacity(0.4),
                )
                .child(
                    svg()
                        .path("icons/hexagon-sigil.svg")
                        .absolute()
                        .top(px(-30.))
                        .right(px(-30.))
                        .size(px(140.))
                        .text_color(accent)
                        .opacity(0.18),
                );
        }

        card.child(header).child(list).child(footer)
    }
}

// ── Row ─────────────────────────────────────────────────────────────
#[allow(clippy::too_many_arguments)]
fn render_row(
    update: &PackageUpdate,
    text_primary: gpui::Hsla,
    text_secondary: gpui::Hsla,
    text_muted: gpui::Hsla,
    hover: gpui::Hsla,
    accent: gpui::Hsla,
    border: gpui::Hsla,
    font_mono: &'static str,
    radius: gpui::Pixels,
) -> AnyElement {
    let is_aur = matches!(update.source, UpdateSource::Aur);

    let name_block: AnyElement = if is_aur {
        div()
            .flex()
            .items_center()
            .gap(px(7.))
            .min_w(px(0.))
            .child(
                div()
                    .text_color(text_primary)
                    .font_family(font_mono)
                    .text_size(px(12.5))
                    .font_weight(gpui::FontWeight::MEDIUM)
                    .whitespace_nowrap()
                    .overflow_hidden()
                    .text_ellipsis()
                    .child(update.name.clone()),
            )
            .child(
                div()
                    .flex_none()
                    .rounded(radius)
                    .px(px(5.))
                    .py(px(1.))
                    .border_1()
                    .border_color(gpui::Hsla::from(gpui::rgba(0xcb_a6_f74d)))
                    .bg(gpui::Hsla::from(gpui::rgba(0xcb_a6_f71f)))
                    .text_color(gpui::Hsla::from(gpui::rgba(0xcb_a6_f7ff)))
                    .font_family(font_mono)
                    .text_size(px(9.5))
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .child("AUR"),
            )
            .into_any_element()
    } else {
        div()
            .text_color(text_primary)
            .font_family(font_mono)
            .text_size(px(12.5))
            .font_weight(gpui::FontWeight::MEDIUM)
            .whitespace_nowrap()
            .overflow_hidden()
            .text_ellipsis()
            .child(update.name.clone())
            .into_any_element()
    };

    div()
        .w_full()
        .flex()
        .items_center()
        .justify_between()
        .gap(px(10.))
        .px(px(ROW_PX))
        .py(px(ROW_PY))
        .border_b_1()
        .border_color(border)
        .hover(|s| s.bg(hover))
        .child(name_block)
        .child(
            div()
                .flex_none()
                .font_family(font_mono)
                .text_size(px(11.))
                .flex()
                .items_center()
                .gap(px(5.))
                .child(div().text_color(text_muted).child(update.old_version.clone()))
                .child(div().text_color(text_muted).child("→"))
                .child(
                    div()
                        .text_color(text_secondary)
                        .child(update.new_version.clone()),
                ),
        )
        .into_any_element()
}
