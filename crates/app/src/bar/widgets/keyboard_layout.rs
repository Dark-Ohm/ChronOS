//! Раскладка клавиатуры в баре — текстовая пилюля (US / RU / IL).
//!
//! Данные: `AppState::compositor(cx)` — живое `CompositorState.keyboard_layout`
//! (полное XKB-имя вроде `"English (US)"` / `"Russian"` / `"Hebrew"`). Клик
//! циклит раскладку через `CompositorCommand::CycleKeyboardLayout`.
//!
//! Внимание: `switchxkblayout` — hyprctl-сабкоманда, НЕ Lua-диспетчер. Через
//! `/dispatch` её слать нельзя (Lua-Hyprland сожрёт как Lua и молча упадёт) —
//! `execute_command` знает про `CycleKeyboardLayout` и пишет строку сырой
//! (см. `crates/services/src/compositor/hyprland.rs`).

use gpui::{AnyElement, App, Window, div, prelude::*, px};

use chronos_luau::bar::{BarSection, BarWidget};
use chronos_services::{CompositorCommand, Service};
use chronos_ui::Theme;

use crate::state::AppState;

/// Сократить полное имя раскладки до 2-буквенной метки капсом:
/// `"English (US)"` → `"US"`, `"Russian"` → `"RU"`, `"Hebrew"` → `"HE"`.
/// Незнакомая строка без `" ("` — первые 2 символа капсом. Пустая (`""`) —
/// пустая строка (до первого события/фетча не паниковать).
pub fn abbreviate(name: &str) -> String {
    if name.is_empty() {
        return String::new();
    }
    // `"English (US)"` → `"US"`: регион в скобках, первые 2 символа.
    if let Some(i) = name.find(" (") {
        let region: String = name[i + 2..]
            .chars()
            .filter(|c| !c.is_whitespace())
            .take(2)
            .collect();
        if region.len() == 2 {
            return region.to_uppercase();
        }
    }
    // Без скобок (`"Russian"`) или незнакомая строка — первые 2 символа.
    let head: String = name.chars().take(2).collect();
    if head.len() == 2 {
        head.to_uppercase()
    } else {
        name.to_uppercase()
    }
}

pub struct KeyboardLayoutWidget;

impl BarWidget for KeyboardLayoutWidget {
    fn name(&self) -> &str {
        "keyboard_layout"
    }

    fn section(&self) -> BarSection {
        BarSection::Right
    }

    fn render(&self, _window: &mut Window, cx: &App) -> AnyElement {
        let compositor = AppState::compositor(cx);
        let label = abbreviate(&compositor.get().keyboard_layout);
        let theme = Theme::global(cx);

        div()
            .id("bar-keyboard-layout")
            .flex()
            .items_center()
            .cursor_pointer()
            .px(px(7.))
            .py(px(2.))
            .rounded(theme.radius)
            .hover(|s| s.bg(theme.interactive.hover))
            .child(
                div()
                    .child(label)
                    .text_color(theme.text.secondary)
                    .text_size(px(12.)),
            )
            .on_click(|_event, _window, cx: &mut App| {
                let _ = AppState::compositor(cx).dispatch(CompositorCommand::CycleKeyboardLayout);
            })
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chronos_luau::bar::{BarSection, BarWidget};

    #[test]
    fn widget_name_and_section_are_stable() {
        let w = KeyboardLayoutWidget;
        assert_eq!(w.name(), "keyboard_layout");
        assert!(matches!(w.section(), BarSection::Right));
    }

    #[test]
    fn abbreviate_english_us() {
        assert_eq!(abbreviate("English (US)"), "US");
    }

    #[test]
    fn abbreviate_russian() {
        assert_eq!(abbreviate("Russian"), "RU");
    }

    #[test]
    fn abbreviate_hebrew() {
        assert_eq!(abbreviate("Hebrew"), "HE");
    }

    #[test]
    fn abbreviate_empty_is_empty() {
        assert_eq!(abbreviate(""), "");
    }

    #[test]
    fn abbreviate_unknown_no_parens_takes_first_two() {
        assert_eq!(abbreviate("Cangjie"), "CA");
    }
}