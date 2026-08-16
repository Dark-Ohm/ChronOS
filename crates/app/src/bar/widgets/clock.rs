//! Clock widget — `HH:MM · dd мес` in Russian, rightmost in the right cluster.
//! Updates every second via the bar refresh-bridge (1-second ticker).
//!
//! Clicking the clock opens the calendar popup (`crate::calendar_popup`).

use chrono::{Datelike, Local};

use gpui::{AnyElement, App, Bounds, MouseButton, Pixels, Window, canvas, div, prelude::*, px, text};
use std::cell::Cell;
use std::rc::Rc;

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

/// Bar clock widget — right section, registered last (rightmost edge).
pub struct ClockWidget {
    bounds: Rc<Cell<Bounds<Pixels>>>,
}

impl ClockWidget {
    pub fn new() -> Self {
        Self {
            bounds: Rc::new(Cell::new(Bounds::default())),
        }
    }
}

impl BarWidget for ClockWidget {
    fn name(&self) -> &str {
        "clock"
    }

    fn section(&self) -> BarSection {
        BarSection::Right
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

        let bounds_cell = self.bounds.clone();
        let row = div()
            .id("bar-clock")
            .px(px(8.))
            .py(px(2.))
            .rounded(Theme::global(cx).radius)
            .hover(|s| s.bg(Theme::global(cx).interactive.hover))
            .cursor_pointer()
            .text_color(Theme::global(cx).text.primary)
            .text_size(Theme::global(cx).font_sizes.sm)
            .font_family(Theme::global(cx).font_mono)
            .child(text!(label))
            .on_mouse_down(MouseButton::Left, {
                let bounds_cell = self.bounds.clone();
                move |_event, window, cx: &mut App| {
                    if crate::edit_mode::is_active(cx) {
                        return;
                    }
                    let anchor_rect = bounds_cell.get();
                    let parent = window.window_handle();
                    crate::calendar_popup::toggle(anchor_rect, parent, window, cx);
                }
            });

        div()
            .relative()
            .child(
                canvas(
                    move |bounds, _window, _cx| bounds,
                    move |_bounds, captured, _window, _cx| {
                        bounds_cell.set(captured);
                    },
                )
                .absolute()
                .size_full(),
            )
            .child(row)
            .into_any_element()
    }
}

/// Register the clock widget with the global bar registry.
pub fn register(cx: &mut App) {
    cx.global_mut::<chronos_luau::bar::BarWidgetRegistry>()
        .register(Box::new(ClockWidget::new()));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clock_label_format() {
        let now = Local::now();
        let month_idx = (now.month0()) as usize;
        let label = format!(
            "{} · {} {}",
            now.format("%H:%M"),
            now.day(),
            MONTHS_RU[month_idx],
        );
        assert!(label.contains('·'));
        assert!(label.len() > 5);
    }

    #[test]
    fn clock_widget_name() {
        let w = ClockWidget::new();
        assert_eq!(w.name(), "clock");
        assert_eq!(w.section(), BarSection::Right);
    }

    #[test]
    fn russian_months_count() {
        assert_eq!(MONTHS_RU.len(), 12);
    }
}
