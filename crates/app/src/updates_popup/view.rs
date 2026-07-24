//! Updates popup view — pending-update list + "Upgrade all" button.
//!
//! Rendering rules:
//!   * empty list  → "System is up to date", no footer button.
//!   * each row    → `name (AUR)?` left, `old → new` right.
//!   * footer      → "Upgrade all" button, only rendered when there is
//!                   something to upgrade.

use gpui::{
    AnyElement, App, BoxShadow, Context, InteractiveElement, IntoElement, Render, ScrollHandle,
    Styled, Window, div, prelude::*, px, svg,
};

use chronos_services::{PackageUpdate, Service, UpdateSource, UpgradeState};

use crate::state::AppState;
use crate::updates_popup::{LIST_MAX_H, close_this, upgrade_all};

use chronos_ui::Theme;

const ROW_PAD_Y: f32 = 6.;
const ROW_PAD_X: f32 = 12.;
const HEADER_PAD: f32 = 12.;

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
        let divider = theme.bg.secondary;
        let radius = theme.radius;
        let radius_lg = theme.radius_lg;
        let accent = theme.accent.primary;
        let accent_hover = theme.accent.hover;
        let hover = theme.interactive.hover;
        let border_default = theme.border.default;

        let header = div()
            .w_full()
            .flex()
            .items_center()
            .justify_between()
            .px(px(HEADER_PAD))
            .py(px(ROW_PAD_Y))
            .child(div().text_color(text_primary).child(if count > 0 {
                format!("Updates ({count})")
            } else {
                "Updates".to_string()
            }))
            .child(
                div()
                    .id("updates-popup-close")
                    .cursor_pointer()
                    .px(px(6.))
                    .rounded(radius)
                    .text_color(text_muted)
                    .hover(|s| s.bg(hover))
                    .child("✕")
                    .on_click(|_event, window, cx: &mut App| {
                        close_this(window, cx);
                    }),
            );

        let divider_line = div().w_full().h(px(1.)).bg(divider);

        let list: AnyElement = if updates.is_empty() {
            div()
                .w_full()
                .px(px(ROW_PAD_X))
                .py(px(ROW_PAD_Y))
                .text_color(text_muted)
                .child("System is up to date")
                .into_any_element()
        } else {
            let rows: Vec<AnyElement> = updates
                .iter()
                .map(|u| render_row(u, text_primary, text_secondary, text_muted, radius, hover, accent_hover))
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

        let upgrade_state = state.upgrade_state;
        let footer: AnyElement = if updates.is_empty() && upgrade_state == UpgradeState::Idle {
            div().into_any_element()
        } else {
            let status_line: AnyElement = match upgrade_state {
                UpgradeState::Idle => div().into_any_element(),
                // Во время работы статус несёт САМА кнопка («Upgrading…»),
                // поэтому отдельной строки нет: она дублировала бы текст и
                // съедала FOOTER_BUDGET_H, которого при полном списке ровно
                // 64px на весь футер (эррата Архитектора при приёмке №12).
                UpgradeState::Running => div().into_any_element(),
                UpgradeState::Done => div()
                    .w_full()
                    .px(px(HEADER_PAD))
                    .pb(px(2.))
                    .text_color(theme.status.success)
                    .child("Upgrade complete")
                    .into_any_element(),
                UpgradeState::Failed => div()
                    .w_full()
                    .px(px(HEADER_PAD))
                    .pb(px(2.))
                    .text_color(theme.status.error)
                    .child("Upgrade failed")
                    .into_any_element(),
            };

            let button: AnyElement = if upgrade_state == UpgradeState::Running {
                // Blocked during upgrade.
                div()
                    .id("updates-popup-upgrade-all")
                    .w_full()
                    .flex()
                    .items_center()
                    .justify_center()
                    .px(px(ROW_PAD_X))
                    .py(px(ROW_PAD_Y))
                    .rounded(radius)
                    .bg(theme.interactive.active)
                    .text_color(text_muted)
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
                    .px(px(ROW_PAD_X))
                    .py(px(ROW_PAD_Y))
                    .rounded(radius)
                    .bg(accent)
                    .hover(|s| s.bg(accent_hover))
                    .text_color(text_primary)
                    .child("Upgrade all")
                    .on_click(|_event, window, cx: &mut App| {
                        upgrade_all(window, cx);
                    })
                    .into_any_element()
            } else {
                // No updates left after a successful upgrade — no button.
                div().into_any_element()
            };

            div()
                .w_full()
                .flex_col()
                .child(status_line)
                .child(
                    div()
                        .w_full()
                        .px(px(HEADER_PAD))
                        .py(px(ROW_PAD_Y))
                        .child(button),
                )
                .into_any_element()
        };

        let is_light = theme.is_light;

        let mut card = div()
            .relative()
            .flex_col()
            .rounded(radius_lg)
            .bg(bg)
            .border_1()
            .border_color(border_default)
            .overflow_hidden();

        if is_light {
            card = card
                .shadow(vec![
                    // outer elevated shadow
                    BoxShadow::new(px(0.), px(6.), gpui::rgba(0x3c40_6e29).into())
                        .blur_radius(px(24.)),
                    // inner accent border glow
                    BoxShadow::new(px(0.), px(0.), gpui::rgba(0x007a_cc26).into())
                        .spread_radius(px(1.))
                        .inset(),
                ])
                .child(
                    // glow-top hairline — accent at low opacity
                    // (CSS gradient approximated as solid line)
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
                    // watermark hexagon sigil
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

        card.child(header).child(divider_line).child(list).child(footer)
    }
}

fn render_row(
    update: &PackageUpdate,
    text_primary: gpui::Hsla,
    text_secondary: gpui::Hsla,
    text_muted: gpui::Hsla,
    radius: gpui::Pixels,
    hover: gpui::Hsla,
    accent_hover: gpui::Hsla,
) -> AnyElement {
    let is_aur = matches!(update.source, UpdateSource::Aur);
    let name_block: AnyElement = if is_aur {
        div()
            .flex()
            .items_center()
            .gap(px(6.))
            .child(div().text_color(text_primary).child(update.name.clone()))
            .child(
                div()
                    .rounded(radius)
                    .px(px(6.))
                    .py(px(1.))
                    .bg(accent_hover)
                    .opacity(0.18)
                    .text_color(accent_hover)
                    .text_xs()
                    .child("AUR"),
            )
            .into_any_element()
    } else {
        div().text_color(text_primary).child(update.name.clone()).into_any_element()
    };
    div()
        .w_full()
        .flex()
        .items_center()
        .justify_between()
        .px(px(ROW_PAD_X))
        .py(px(ROW_PAD_Y))
        .rounded(radius)
        .hover(|s| s.bg(hover))
        .child(name_block)
        .child(
            div()
                .text_color(text_secondary)
                .child(format!("{} → {}", update.old_version, update.new_version)),
        )
        .into_any_element()
}
