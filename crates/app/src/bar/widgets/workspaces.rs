//! Workspaces widget for the bar — clickable dots that switch Hyprland
//! workspaces. The active workspace is highlighted via theme.accent.primary.
//!
//! Data comes from `AppState::compositor(cx)` (live `CompositorState`); a click
//! dispatches `CompositorCommand::FocusWorkspace(id)` straight to the
//! compositor service.

use gpui::{AnyElement, App, Window, div, prelude::*, px};

use chronos_luau::bar::{BarSection, BarWidget, BarWidgetRegistry};
use chronos_services::{CompositorCommand, Service};
use chronos_ui::Theme;

use crate::state::AppState;

/// Dot diameter in px (design/Top Bar.dc.html shows ~7px dots).
const DOT_SIZE: f32 = 7.0;

pub struct WorkspacesWidget;

impl BarWidget for WorkspacesWidget {
    fn name(&self) -> &str {
        "workspaces"
    }

    fn section(&self) -> BarSection {
        BarSection::Left
    }

    fn render(&self, _window: &mut Window, cx: &App) -> AnyElement {
        let compositor = AppState::compositor(cx);
        let state = compositor.get();

        // No compositor / no workspaces yet: render empty, don't panic.
        if state.workspaces.is_empty() {
            return div().into_any_element();
        }

        let theme = Theme::global(cx);

        let dots: Vec<AnyElement> = state
            .workspaces
            .iter()
            .map(|ws| {
                let id = ws.id;
                let bg = if ws.active {
                    theme.accent.primary
                } else {
                    theme.text.disabled
                };

                div()
                    .id(format!("workspace-dot-{id}"))
                    .cursor_pointer()
                    .w(px(DOT_SIZE))
                    .h(px(DOT_SIZE))
                    .rounded_full()
                    .bg(bg)
                    .on_click(move |_event, _window, cx: &mut App| {
                        let _ = AppState::compositor(cx)
                            .dispatch(CompositorCommand::FocusWorkspace(id));
                    })
                    .into_any_element()
            })
            .collect();

        div()
            .flex()
            .items_center()
            .gap(px(5.))
            .children(dots)
            .into_any_element()
    }
}

/// Register the workspaces widget with the global bar registry.
pub fn register(cx: &mut App) {
    cx.global_mut::<BarWidgetRegistry>()
        .register(Box::new(WorkspacesWidget));
}
