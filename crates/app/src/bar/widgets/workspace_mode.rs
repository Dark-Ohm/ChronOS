//! Переключатель режима рабочего пространства для бара — иконка + подпись,
//! клик переключает Developer ⇄ Gamer.
//!
//! Живёт СТРОГО в правом кластере левее часов: `STYLE.md` фиксирует CAVA по
//! центру и часы крайними справа, и этот виджет их не двигает.
//!
//! Риск T159/Q4: это первый виджет бара с тремя независимыми `on_click` на
//! разных `div` внутри одного `render()`. Каждый `div` получает уникальный
//! `.id()` — без этого GPUI хеширует контент и может слить обработчики.
//! Event bubbling: в GPUI `on_click` на дочернем `div` не всплывает к
//! родительскому — проверено на практике dock.rs (два типа событий на одном
//! элементе). Здесь каждый `div` отдельный, так что риск минимален.

use gpui::{AnyElement, App, Window, div, prelude::*, px, svg};

use chronos_luau::bar::{BarSection, BarWidget};
use chronos_ui::Theme;

use crate::workspace_mode;

pub struct WorkspaceModeWidget;

impl BarWidget for WorkspaceModeWidget {
    fn name(&self) -> &str {
        "workspace_mode"
    }

    fn section(&self) -> BarSection {
        BarSection::Right
    }

    fn render(&self, _window: &mut Window, cx: &App) -> AnyElement {
        let theme = Theme::global(cx);
        let mode = workspace_mode::current(cx);

        let pill = div()
            .id("bar-workspace-mode")
            .flex()
            .items_center()
            .gap(px(5.))
            .cursor_pointer()
            .px(px(7.))
            .py(px(2.))
            .rounded(theme.radius)
            .hover(|s| s.bg(theme.interactive.hover))
            .child(
                svg()
                    .path(mode.icon_path())
                    .size(px(12.))
                    .text_color(theme.text.secondary),
            )
            .child(
                div()
                    .child(mode.label())
                    .text_color(theme.text.secondary)
                    .text_size(px(12.)),
            )
            .on_click(|_event, _window, cx: &mut App| {
                workspace_mode::toggle(cx);
            });

        let mut row = div().flex().items_center().gap(px(6.));

        if let Some(prompt) = workspace_mode::pending(cx) {
            row = row.child(
                div()
                    .id("workspace-mode-prompt-banner")
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .px(px(7.))
                    .py(px(2.))
                    .rounded(theme.radius)
                    .bg(theme.bg.elevated)
                    .child(
                        div()
                            .child(format!("Перейти в {}?", prompt.target.label()))
                            .text_color(theme.text.primary)
                            .text_size(px(12.)),
                    )
                    .child(
                        div()
                            .id("workspace-mode-prompt-yes")
                            .cursor_pointer()
                            .px(px(4.))
                            .rounded(px(3.))
                            .hover(|s| s.bg(theme.interactive.hover))
                            .child("Да")
                            .text_color(theme.accent.primary)
                            .text_size(px(12.))
                            .on_click(|_event, _window, cx: &mut App| {
                                workspace_mode::accept_prompt(cx);
                            }),
                    )
                    .child(
                        div()
                            .id("workspace-mode-prompt-no")
                            .cursor_pointer()
                            .px(px(4.))
                            .rounded(px(3.))
                            .hover(|s| s.bg(theme.interactive.hover))
                            .child("Нет")
                            .text_color(theme.text.muted)
                            .text_size(px(12.))
                            .on_click(|_event, _window, cx: &mut App| {
                                workspace_mode::dismiss_prompt(cx, false);
                            }),
                    )
                    .child(
                        div()
                            .id("workspace-mode-prompt-never")
                            .cursor_pointer()
                            .px(px(4.))
                            .rounded(px(3.))
                            .hover(|s| s.bg(theme.interactive.hover))
                            .child("Не спрашивать")
                            .text_color(theme.text.muted)
                            .text_size(px(12.))
                            .on_click(|_event, _window, cx: &mut App| {
                                workspace_mode::dismiss_prompt(cx, true);
                            }),
                    ),
            );
        }

        row.child(pill).into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chronos_luau::bar::{BarSection, BarWidget};

    #[test]
    fn widget_name_and_section_are_stable() {
        let w = WorkspaceModeWidget;
        assert_eq!(w.name(), "workspace_mode");
        assert!(matches!(w.section(), BarSection::Right));
    }
}
