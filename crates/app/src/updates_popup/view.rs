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

use chronos_services::{PackageUpdate, Service, UpdateSource};

use crate::state::AppState;
use crate::updates_popup::{close_this, upgrade_all};

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
        let bg = theme.bg.elevated;
        let text_primary = theme.text.primary;
        let text_muted = theme.text.muted;
        let divider = theme.bg.secondary;
        let radius = theme.radius;
        let radius_lg = theme.radius_lg;
        let accent = theme.accent.primary;
        let accent_hover = theme.accent.hover;
        let hover = theme.interactive.hover;

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
                .map(|u| render_row(u, text_primary, text_muted, radius))
                .collect();
            div().w_full().flex_col().children(rows).into_any_element()
        };

        let footer: AnyElement = if updates.is_empty() {
            div().into_any_element()
        } else {
            div()
                .w_full()
                .px(px(HEADER_PAD))
                .py(px(ROW_PAD_Y))
                .child(
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
                        }),
                )
                .into_any_element()
        };

        div()
            .flex_col()
            .rounded(radius_lg)
            .bg(bg)
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
) -> AnyElement {
    let source_tag = match update.source {
        UpdateSource::Official => "",
        UpdateSource::Aur => " (AUR)",
    };
    div()
        .w_full()
        .flex()
        .items_center()
        .justify_between()
        .px(px(ROW_PAD_X))
        .py(px(ROW_PAD_Y))
        .rounded(radius)
        .child(
            div()
                .text_color(text_primary)
                .child(format!("{}{}", update.name, source_tag)),
        )
        .child(
            div()
                .text_color(text_muted)
                .child(format!("{} → {}", update.old_version, update.new_version)),
        )
        .into_any_element()
}
