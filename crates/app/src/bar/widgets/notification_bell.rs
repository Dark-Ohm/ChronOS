//! Bell widget for the bar — glyph + unread-count badge, click opens the
//! Notifications tab on the right panel (T293).
//!
//! Data comes from `AppState::notification(cx)` (`NotificationState`,
//! `crates/services/src/notification/`). The unread badge is shown only when
//! `unread > 0`; opening the tab dispatches `MarkAllRead`, clearing it.

use gpui::{AnyElement, App, MouseButton, Window, div, prelude::*, px, svg};

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

pub struct NotificationBellWidget;

impl NotificationBellWidget {
    pub fn new() -> Self {
        Self
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

        // T293: bell click opens the Notifications tab on the right panel.
        bell.on_mouse_down(MouseButton::Left, move |_event, _window, cx: &mut App| {
            if crate::edit_mode::is_active(cx) {
                return;
            }
            crate::side_panel_right::select_tab(
                crate::side_panel_right::tabs::PanelTab::Notifications,
                cx,
            );
        })
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
