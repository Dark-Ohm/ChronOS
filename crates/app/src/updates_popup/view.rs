//! Updates popup view — pending-update list + "Upgrade all" button.
//!
//! Rendering rules:
//!   * empty list  → "System is up to date", no footer button.
//!   * each row    → `name (AUR)?` left, `old → new` right.
//!   * footer      → "Upgrade all" button, only rendered when there is
//!                   something to upgrade.

use gpui::{
    AnyElement, App, Context, InteractiveElement, IntoElement, Render, Styled, Window, div,
    prelude::*, px,
};

use chronos_services::{PackageUpdate, Service, UpdateSource, UpgradeState};

use crate::state::AppState;
use crate::updates_popup::{LIST_MAX_H, close_this, max_visible_rows, upgrade_all};

use chronos_ui::Theme;

const ROW_PAD_Y: f32 = 6.;
const ROW_PAD_X: f32 = 12.;
const HEADER_PAD: f32 = 12.;

pub struct UpdatesPopupView {}

impl UpdatesPopupView {
    pub fn new(_cx: &mut App) -> Self {
        Self {}
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
        let divider = theme.bg.secondary;
        let radius = theme.radius;
        let radius_lg = theme.radius_lg;
        let accent = theme.accent.primary;
        let accent_hover = theme.accent.hover;
        let hover = theme.interactive.hover;
        let border_subtle = theme.border.subtle;

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
            // Truncate so the footer's "Upgrade all" button always stays
            // within the window's visible bounds (see MAX_POPUP_H comment in
            // mod.rs) — losing rows off the bottom is acceptable, losing the
            // only way to trigger an upgrade is not.
            let max_rows = max_visible_rows();
            let (shown, hidden) = if count > max_rows && max_rows > 0 {
                (max_rows - 1, count - (max_rows - 1))
            } else {
                (count, 0)
            };
            let mut rows: Vec<AnyElement> = updates[..shown]
                .iter()
                .map(|u| render_row(u, text_primary, text_muted, radius, hover, accent_hover))
                .collect();
            if hidden > 0 {
                rows.push(
                    div()
                        .w_full()
                        .px(px(ROW_PAD_X))
                        .py(px(ROW_PAD_Y))
                        .text_color(text_muted)
                        .child(format!("+{hidden} more (run checkupdates for the full list)"))
                        .into_any_element(),
                );
            }
            // Hard pixel clip: even if the truncation above under- or
            // over-estimates real row height, this guarantees the list can
            // never grow past LIST_MAX_H and push the footer out of the
            // window (see mod.rs `HEADER_BUDGET_H` comment).
            div()
                .w_full()
                .max_h(px(LIST_MAX_H))
                .overflow_hidden()
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

        div()
            .flex_col()
            .rounded(radius_lg)
            .bg(bg)
            .border_1()
            .border_color(border_subtle)
            .overflow_hidden()
            .child(header)
            .child(divider_line)
            .child(list)
            .child(footer)
    }
}

fn render_row(
    update: &PackageUpdate,
    text_primary: gpui::Hsla,
    text_muted: gpui::Hsla,
    radius: gpui::Pixels,
    hover: gpui::Hsla,
    accent_hover: gpui::Hsla,
) -> AnyElement {
    let is_aur = matches!(update.source, UpdateSource::Aur);
    let name_block: AnyElement = if is_aur {
        // AUR badge (pill) instead of a bare " (AUR)" text suffix — visual
        // parity with the design mockup (rounded pill, accent.hover bg on
        // lowered alpha, smaller radius). Rendered ONLY for AUR sources.
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
                .text_color(text_muted)
                .child(format!("{} → {}", update.old_version, update.new_version)),
        )
        .into_any_element()
}
