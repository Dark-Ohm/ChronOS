//! Bell widget for the bar — glyph + unread-count badge, click opens the
//! notification-history popup (`crate::notifications::history_popup`).
//!
//! Data comes from `AppState::notification(cx)` (`NotificationState`,
//! `crates/services/src/notification/`). The unread badge is shown only when
//! `unread > 0`; opening the popup dispatches `MarkAllRead`, clearing it.

use gpui::{AnyElement, App, Window, div, prelude::*, px, svg};

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
            .text_color(if view.unread > 0 { theme.text.primary } else { muted });

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
            bell = bell.child(
                div()
                    .flex()
                    .items_center()
                    .justify_center()
                    .min_w(px(16.))
                    .h(px(16.))
                    .px(px(4.))
                    .rounded(px(8.))
                    .bg(badge_color)
                    .text_color(theme.text.primary)
                    .text_xs()
                    .font_weight(gpui::FontWeight::BOLD)
                    .child(label),
            );
        }

        bell.on_click(|_event, window, cx: &mut App| {
            crate::notifications::history_popup::toggle(window, cx);
        })
        .into_any_element()
    }
}

/// Register the bell widget with the global bar registry.
pub fn register(cx: &mut App) {
    cx.global_mut::<chronos_luau::bar::BarWidgetRegistry>()
        .register(Box::new(NotificationBellWidget));
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
