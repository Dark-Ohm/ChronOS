//! Bell widget for the bar — glyph + unread-count badge, click opens the
//! notification-history popup (`crate::notifications::history_popup`).
//!
//! Data comes from `AppState::notification(cx)` (`NotificationState`,
//! `crates/services/src/notification/`). The unread badge is shown only when
//! `unread > 0`; opening the popup dispatches `MarkAllRead`, clearing it.

use gpui::{
    AnyElement, App, Bounds, MouseButton, Pixels, Window, canvas, div, prelude::*, px, svg,
};
use std::cell::Cell;
use std::rc::Rc;

use chronos_luau::bar::{BarSection, BarWidget};
use chronos_services::{NotificationState, Service};
use chronos_ui::Theme;

use crate::state::AppState;

/// Pure description of what the widget should display (unit-testable).
#[derive(Debug, PartialEq, Eq)]
struct BellView {
    /// Glyph shown (always the bell).
    icon: &'static str,
    /// Unread count; 0 means no badge.
    unread: usize,
}

fn describe(state: &NotificationState) -> BellView {
    BellView {
        icon: "icons/bell.svg",
        unread: state.unread,
    }
}

pub struct NotificationBellWidget {
    /// Captured bell bounds for the anchored popup (T117 pattern).
    bounds: Rc<Cell<Bounds<Pixels>>>,
}

impl NotificationBellWidget {
    pub fn new() -> Self {
        Self {
            bounds: Rc::new(Cell::new(Bounds::default())),
        }
    }
}

impl BarWidget for NotificationBellWidget {
    fn name(&self) -> &str {
        "notification_bell"
    }

    fn section(&self) -> BarSection {
        BarSection::Right
    }

    fn render(&self, _window: &mut Window, cx: &App) -> AnyElement {
        let state = AppState::notification(cx).get();
        let theme = Theme::global(cx);
        let view = describe(&state);

        let muted = theme.text.muted;
        let badge_color = theme.status.error; // red dot for unread

        let glyph = svg()
            .path(view.icon)
            .size(px(13.))
            .text_color(if view.unread > 0 {
                theme.text.primary
            } else {
                muted
            });

        // Bell + optional red badge (count, capped at 99 for the label).
        let mut bell = div()
            .id("bar-notification-bell")
            .flex()
            .items_center()
            .gap(px(4.))
            .cursor_pointer()
            .px(px(6.))
            .py(px(2.))
            .rounded(theme.radius)
            .hover(|s| s.bg(theme.interactive.hover))
            .child(glyph);

        if view.unread > 0 {
            let label = if view.unread > 99 {
                "99+".to_string()
            } else {
                view.unread.to_string()
            };
            // Число, а не пилюля-кружок (решение пользователя 2026-07-20):
            // счётчик живёт нашим mono-шрифтом в цвете статуса, заливки нет.
            bell = bell.child(
                div()
                    .font_family(theme.font_mono)
                    .text_size(theme.font_sizes.sm)
                    .text_color(badge_color)
                    .child(label),
            );
        }

        // T117 lessons: (1) `.relative()` wrapping canvas + hit target;
        // (2) `on_mouse_down(Left)` for grab-popups, not `on_click`;
        // (3) bounds captured into a `Rc<Cell<…>>` field, not a local
        // that dies when `render` returns.
        let bounds_cell = self.bounds.clone();
        div()
            .relative()
            .child(
                canvas(
                    move |bounds, _window, _cx| bounds,
                    move |bounds, captured, _window, _cx| {
                        bounds_cell.set(captured);
                        let _ = bounds;
                    },
                )
                .absolute()
                .size_full(),
            )
            .child(bell.on_mouse_down(MouseButton::Left, {
                let bounds_cell = self.bounds.clone();
                move |_event, window, cx: &mut App| {
                    if crate::edit_mode::is_active(cx) {
                        return;
                    }
                    let anchor_rect = bounds_cell.get();
                    let parent = window.window_handle();
                    crate::notifications::history_popup::toggle(anchor_rect, parent, window, cx);
                }
            }))
            .into_any_element()
    }
}

/// Register the bell widget with the global bar registry.
pub fn register(cx: &mut App) {
    cx.global_mut::<chronos_luau::bar::BarWidgetRegistry>()
        .register(Box::new(NotificationBellWidget::new()));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn describe_no_unread() {
        let v = describe(&NotificationState::default());
        assert_eq!(v.unread, 0);
        assert_eq!(v.icon, "icons/bell.svg");
    }

    #[test]
    fn describe_with_unread() {
        let mut s = NotificationState::default();
        s.unread = 3;
        let v = describe(&s);
        assert_eq!(v.unread, 3);
    }
}
