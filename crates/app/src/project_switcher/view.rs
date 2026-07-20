//! Project popup view — saved project list + "+ Добавить проект" row.
//!
//! Rendering rules:
//!   * empty list → muted hint row.
//!   * each row   → accent dot (active) + name, branch right-aligned mono.
//!   * footer     → "+ Добавить проект" (XDG portal directory picker).

use std::path::Path;

use gpui::{
    AnyElement, App, Context, InteractiveElement, IntoElement, Render, Styled, Window, div,
    prelude::*, px,
};

use chronos_ui::Theme;

use crate::project_switcher::{add_project, cached, close_this, current_branch, set_active};

const ROW_PAD_Y: f32 = 6.;
const ROW_PAD_X: f32 = 12.;
const HEADER_PAD: f32 = 12.;

pub struct ProjectPopupView {}

impl Render for ProjectPopupView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let config = cached();
        let theme = Theme::global(cx);
        let bg = theme.bg.primary;
        let text_primary = theme.text.primary;
        let text_secondary = theme.text.secondary;
        let text_muted = theme.text.muted;
        let divider = theme.bg.secondary;
        let radius = theme.radius;
        let radius_lg = theme.radius_lg;
        let accent = theme.accent.primary;
        let hover = theme.interactive.hover;
        let border_subtle = theme.border.subtle;
        let font_mono = theme.font_mono;

        let header = div()
            .w_full()
            .flex()
            .items_center()
            .justify_between()
            .px(px(HEADER_PAD))
            .py(px(ROW_PAD_Y))
            .child(div().text_color(text_primary).child("Проекты"))
            .child(
                div()
                    .id("project-popup-close")
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

        let list: AnyElement = if config.projects.is_empty() {
            div()
                .w_full()
                .px(px(ROW_PAD_X))
                .py(px(ROW_PAD_Y))
                .text_color(text_muted)
                .child("Пока пусто — добавь первый")
                .into_any_element()
        } else {
            let active = config.active.clone();
            let rows: Vec<AnyElement> = config
                .projects
                .iter()
                .enumerate()
                .map(|(i, entry)| {
                    let is_active = active.as_deref() == Some(entry.path.as_str());
                    let branch = current_branch(Path::new(&entry.path))
                        .unwrap_or_else(|| "—".to_string());
                    let path = entry.path.clone();
                    div()
                        .id(("project-row", i))
                        .w_full()
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap(px(8.))
                        .px(px(ROW_PAD_X))
                        .py(px(ROW_PAD_Y))
                        .rounded(radius)
                        .cursor_pointer()
                        .hover(|s| s.bg(hover))
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(8.))
                                .child(
                                    div()
                                        .w(px(6.))
                                        .h(px(6.))
                                        .rounded_full()
                                        .bg(if is_active { accent } else { divider }),
                                )
                                .child(
                                    div()
                                        .text_color(if is_active {
                                            text_primary
                                        } else {
                                            text_secondary
                                        })
                                        .child(entry.name.clone()),
                                ),
                        )
                        .child(
                            div()
                                .text_color(text_muted)
                                .font_family(font_mono)
                                .text_xs()
                                .child(branch),
                        )
                        .on_click(move |_event, window, cx: &mut App| {
                            set_active(path.clone(), window, cx);
                        })
                        .into_any_element()
                })
                .collect();
            div().w_full().flex_col().children(rows).into_any_element()
        };

        let add_row = div()
            .id("project-add")
            .w_full()
            .flex()
            .items_center()
            .gap(px(8.))
            .px(px(ROW_PAD_X))
            .py(px(ROW_PAD_Y))
            .rounded(radius)
            .cursor_pointer()
            .text_color(text_secondary)
            .hover(|s| s.bg(hover))
            .child("+ Добавить проект")
            .on_click(|_event, _window, cx: &mut App| {
                add_project(cx);
            });

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
            .child(divider_line2(divider))
            .child(add_row)
    }
}

fn divider_line2(color: gpui::Hsla) -> AnyElement {
    div().w_full().h(px(1.)).bg(color).into_any_element()
}
