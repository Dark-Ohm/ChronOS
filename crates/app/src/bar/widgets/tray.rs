//! Tray widget for the bar — shows a row of system-tray item icons/badges.
//!
//! Data comes from `AppState::tray(cx)` (live `TrayState`). MVP rendering:
//! each item is a compact badge. If an `icon_name` is available we still use a
//! text fallback (freedesktop icon-theme lookup in GPUI is out of scope for the
//! MVP — pixmap rendering is also deferred; both noted in the OPENCODE report).
//! A click on a badge dispatches `TrayCommand::ActivateItem` to the tray
//! service (left-click activation, `StatusNotifierItem.Activate(0,0)`).

use gpui::{div, prelude::*, px, AnyElement, App, InteractiveElement, Window};

use chronos_luau::bar::{BarSection, BarWidget};
use chronos_services::{Service, TrayCommand};

use crate::state::AppState;

pub struct TrayWidget;

impl BarWidget for TrayWidget {
    fn name(&self) -> &str {
        "tray"
    }

    fn section(&self) -> BarSection {
        BarSection::Right
    }

    fn render(&self, _window: &mut Window, cx: &App) -> AnyElement {
        let tray = AppState::tray(cx);
        let state = tray.get();

        // No tray service / no items: render empty, don't panic.
        if state.items.is_empty() {
            return div().into_any_element();
        }

        let theme = chronos_ui::Theme::global(cx);
        let radius = theme.radius;

        let badges: Vec<AnyElement> = state
            .items
            .iter()
            .map(|item| {
                let id = item.id.clone();
                let label = item.label.clone();

                div()
                    .id(format!("tray-item-{id}"))
                    .cursor_pointer()
                    .px(px(6.))
                    .py(px(2.))
                    .rounded(radius)
                    .child(label)
                    .on_click(move |_event, _window, cx: &mut App| {
                        AppState::tray(cx)
                            .dispatch(TrayCommand::ActivateItem { service: id.clone() });
                    })
                    .into_any_element()
            })
            .collect();

        div()
            .flex()
            .items_center()
            .gap(px(4.))
            .children(badges)
            .into_any_element()
    }
}

/// Register the tray widget with the global bar registry.
pub fn register(cx: &mut App) {
    use chronos_luau::bar::BarWidgetRegistry;
    cx.global_mut::<BarWidgetRegistry>()
        .register(Box::new(TrayWidget));
}
