//! Center-section clock widget — `HH:MM · dd мес` in Russian.
//! Updates every second via the bar refresh-bridge (1-second ticker).

use chrono::{Datelike, Local};

use gpui::{AnyElement, App, Window, div, px, text, prelude::*};

use chronos_luau::bar::{BarSection, BarWidget};
use chronos_ui::Theme;

/// Russian month abbreviations (lowercase, 3 letters).
const MONTHS_RU: [&str; 12] = [
    "\u{0438}\u{043D}\u{0432}", // янв
    "\u{0444}\u{0435}\u{0432}", // фев
    "\u{043C}\u{0430}\u{0440}", // мар
    "\u{0430}\u{043F}\u{0440}", // апр
    "\u{043C}\u{0430}\u{0439}", // май
    "\u{0438}\u{044E}\u{043D}", // июн
    "\u{0438}\u{044E}\u{043B}", // июл
    "\u{0430}\u{0432}\u{0433}", // авг
    "\u{0441}\u{0435}\u{043D}", // сен
    "\u{043E}\u{043A}\u{0442}", // окт
    "\u{043D}\u{043E}\u{044F}", // ноя
    "\u{0434}\u{0435}\u{043A}", // дек
];

/// Bar clock widget — center section.
pub struct ClockWidget;

impl BarWidget for ClockWidget {
    fn name(&self) -> &str {
        "clock"
    }

    fn section(&self) -> BarSection {
        BarSection::Center
    }

    fn render(&self, _window: &mut Window, cx: &App) -> AnyElement {
        let now = Local::now();
        let month_idx = (now.month0()) as usize;
        let label = format!(
            "{} \u{00B7} {} {}",
            now.format("%H:%M"),
            now.day(),
            MONTHS_RU[month_idx],
        );

        div()
            .px(px(8.))
            .text_color(Theme::global(cx).text.primary)
            .text_size(Theme::global(cx).font_sizes.sm)
            .child(text!(label))
            .into_any_element()
    }
}

/// Register the clock widget with the global bar registry.
pub fn register(cx: &mut App) {
    cx.global_mut::<chronos_luau::bar::BarWidgetRegistry>()
        .register(Box::new(ClockWidget));
}
