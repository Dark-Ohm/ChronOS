//! Updates widget for the bar — icon + pending-update count, click opens the
//! updates popup (`crate::updates_popup`).
//!
//! Data comes from `AppState::aur(cx)` (`UpdatesState`, `crates/services/src/aur/`).

use gpui::{AnyElement, App, Window, div, prelude::*, px};

use chronos_luau::bar::{BarSection, BarWidget};
use chronos_services::{Service, UpdatesState};
use chronos_ui::Theme;

use crate::state::AppState;

/// Pure description of what the widget should display (unit-testable).
#[derive(Debug, PartialEq, Eq)]
struct UpdatesView {
    icon: &'static str,
    /// Empty when there are no pending updates (icon-only, muted).
    count_label: String,
    has_updates: bool,
}

fn describe(state: &UpdatesState) -> UpdatesView {
    let count = state.count();
    UpdatesView {
        icon: "⬆",
        count_label: if count > 0 {
            count.to_string()
        } else {
            String::new()
        },
        has_updates: count > 0,
    }
}

pub struct UpdatesWidget;

impl BarWidget for UpdatesWidget {
    fn name(&self) -> &str {
        "updates"
    }

    fn section(&self) -> BarSection {
        BarSection::Right
    }

    fn render(&self, _window: &mut Window, cx: &App) -> AnyElement {
        let aur = AppState::aur(cx);
        let state = aur.get();
        let theme = Theme::global(cx);
        let view = describe(&state);

        let color = if view.has_updates {
            theme.status.warning
        } else {
            theme.text.muted
        };

        let label = if view.has_updates {
            format!("{} {}", view.icon, view.count_label)
        } else {
            view.icon.to_string()
        };

        div()
            .id("bar-updates")
            .flex()
            .items_center()
            .gap(px(4.))
            .cursor_pointer()
            .px(px(6.))
            .py(px(2.))
            .rounded(theme.radius)
            .child(div().child(label).text_color(color))
            .on_click(|_event, window, cx: &mut App| {
                crate::updates_popup::toggle(window, cx);
            })
            .into_any_element()
    }
}

/// Register the updates widget with the global bar registry.
pub fn register(cx: &mut App) {
    cx.global_mut::<chronos_luau::bar::BarWidgetRegistry>()
        .register(Box::new(UpdatesWidget));
}

#[cfg(test)]
mod tests {
    use super::*;
    use chronos_services::{PackageUpdate, UpdateSource};

    fn update(name: &str) -> PackageUpdate {
        PackageUpdate {
            name: name.into(),
            old_version: "1".into(),
            new_version: "2".into(),
            source: UpdateSource::Official,
        }
    }

    #[test]
    fn describe_no_updates() {
        let v = describe(&UpdatesState::default());
        assert!(!v.has_updates);
        assert_eq!(v.count_label, "");
    }

    #[test]
    fn describe_with_updates() {
        let state = UpdatesState {
            updates: vec![update("a"), update("b"), update("c")],
        };
        let v = describe(&state);
        assert!(v.has_updates);
        assert_eq!(v.count_label, "3");
    }
}
