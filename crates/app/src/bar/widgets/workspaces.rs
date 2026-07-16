//! Workspaces widget for the bar — clickable badges that switch Hyprland
//! workspaces. The active workspace is highlighted via the theme.
//!
//! Data comes from `AppState::compositor(cx)` (live `CompositorState`); a click
//! dispatches `CompositorCommand::FocusWorkspace(id)` straight to the
//! compositor service. No extra `services` surface is needed — `FocusWorkspace`
//! was already wired through `hyprland::execute_command`.

use gpui::{div, prelude::*, px, AnyElement, App, InteractiveElement, Window, rgba};

use chronos_luau::bar::{BarSection, BarWidget, BarWidgetRegistry};
use chronos_services::{CompositorCommand, Service};
use chronos_ui::Theme;

use crate::state::AppState;

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
        let active_bg = theme.accent.primary;
        let active_fg = rgba(0xffffffff).into();
        let idle_bg = theme.bg.secondary;
        let idle_fg = theme.text.muted;
        let focus_border = theme.border.focused;

        let badges: Vec<AnyElement> = state
            .workspaces
            .iter()
            .map(|ws| {
                let id = ws.id;
                let label = if ws.name.is_empty() {
                    id.to_string()
                } else {
                    ws.name.clone()
                };
                let (bg, fg) = if ws.active {
                    (active_bg, active_fg)
                } else {
                    (idle_bg, idle_fg)
                };

                div()
                    .id(format!("workspace-badge-{id}"))
                    .cursor_pointer()
                    .px(px(8.))
                    .py(px(2.))
                    .rounded(theme.radius)
                    .bg(bg)
                    .text_color(fg)
                    .when(ws.active, |el| el.border_l_2().border_color(focus_border))
                    .on_click(move |_event, _window, cx: &mut App| {
                        let _ = AppState::compositor(cx)
                            .dispatch(CompositorCommand::FocusWorkspace(id));
                    })
                    .child(label)
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

/// Register the workspaces widget with the global bar registry.
pub fn register(cx: &mut App) {
    cx.global_mut::<BarWidgetRegistry>()
        .register(Box::new(WorkspacesWidget));
}
