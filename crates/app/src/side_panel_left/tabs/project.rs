//! T279 / Slice A2 — the Project Switcher embedded as a tab.
//!
//! Replaces the popup (`project_switcher/view.rs`, deleted). Reads
//! `ProjectsConfig` via `project_switcher::cached()` and renders the project
//! list, active branch, and add/select actions inside the content canvas.
//! Domain (config persistence, branch lookup, portal add) stays in
//! `project_switcher`; this tab is only the embedded view.
//!
//! Selection is one coordinator transaction: the tab calls
//! `project_switcher::set_active` (updates + persists config), then forwards a
//! `ProjectEvent::Select(path)` to the `WorkspaceView` coordinator, which
//! runs `project_switch_transition` (clear session, set path) and reloads.

use std::path::PathBuf;

use gpui::{Context, IntoElement, Render, WeakEntity, Window, div, prelude::*, px};

use chronos_ui::{Theme, WindowRootExt};

use crate::project_switcher::{add_project, cached, current_branch, set_active};
use crate::side_panel_left::workspace_view::WorkspaceView;

/// Event emitted by `ProjectTab` to the workspace coordinator.
#[derive(Clone, Debug)]
pub enum ProjectEvent {
    /// User selected a project — coordinator clears session scope + loads.
    Select(PathBuf),
    /// User clicked "+ Add" — portal picker handled by `add_project`.
    Add,
}

/// The Project Switcher embedded as a content-canvas tab.
pub struct ProjectTab {
    /// Weak handle to the owning `WorkspaceView` — forwards events to the
    /// coordinator without owning panel state.
    coordinator: WeakEntity<WorkspaceView>,
}

impl ProjectTab {
    pub fn new(coordinator: WeakEntity<WorkspaceView>) -> Self {
        Self { coordinator }
    }

    fn emit(&self, event: ProjectEvent, cx: &mut Context<Self>) {
        if let Some(view) = self.coordinator.upgrade() {
            view.update(cx, |view, cx| view.on_project_event(event, cx));
        }
    }
}

impl Render for ProjectTab {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = *Theme::global(cx);
        let config = cached();

        div()
            .id("left-project-tab")
            .window_font(&theme)
            .size_full()
            .flex()
            .flex_col()
            .bg(theme.bg.primary)
            // Header.
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .px(px(12.))
                    .py(px(10.))
                    .border_b_1()
                    .border_color(theme.border.subtle)
                    .child(
                        div()
                            .text_size(px(14.))
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(theme.text.primary)
                            .child("Project"),
                    )
                    .child(
                        div()
                            .id("project-add")
                            .text_size(px(13.))
                            .text_color(theme.text.secondary)
                            .cursor_pointer()
                            .hover(|s| s.text_color(theme.text.primary))
                            .child("+ Add")
                            .on_click(cx.listener(|this, _e, _w, cx| {
                                add_project(cx);
                                this.emit(ProjectEvent::Add, cx);
                            })),
                    ),
            )
            // Project rows.
            .children(config.projects.iter().enumerate().map(|(i, entry)| {
                let is_active = config.active.as_deref() == Some(entry.path.as_str());
                let branch = current_branch(std::path::Path::new(&entry.path));
                let path = PathBuf::from(&entry.path);
                div()
                    .id(("project-row", i))
                    .flex()
                    .flex_col()
                    .px(px(12.))
                    .py(px(8.))
                    .border_b_1()
                    .border_color(theme.border.subtle)
                    .cursor_pointer()
                    .when(is_active, |el| el.bg(theme.interactive.active))
                    .hover(|s| s.bg(theme.interactive.hover))
                    .on_click({
                        let path = path.clone();
                        cx.listener(move |this, _e, _w, cx| {
                            set_active(path.to_string_lossy().to_string(), cx);
                            this.emit(ProjectEvent::Select(path.clone()), cx);
                        })
                    })
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .text_size(px(13.))
                                    .font_weight(if is_active {
                                        gpui::FontWeight::SEMIBOLD
                                    } else {
                                        gpui::FontWeight::NORMAL
                                    })
                                    .text_color(if is_active {
                                        theme.text.primary
                                    } else {
                                        theme.text.secondary
                                    })
                                    .child(entry.name.clone()),
                            )
                            .child(
                                div()
                                    .text_size(theme.font_sizes.xs)
                                    .text_color(theme.text.muted)
                                    .child(branch.unwrap_or_default()),
                            ),
                    )
                    .child(
                        div()
                            .text_size(theme.font_sizes.xs)
                            .text_color(theme.text.muted)
                            .child(entry.path.clone()),
                    )
            }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// ProjectEvent carries the path through — guards against a future
    /// refactor dropping the path payload (the coordinator needs it to
    /// clear + load scope).
    #[test]
    fn project_event_select_carries_path() {
        let p = PathBuf::from("/home/neo/proj");
        match ProjectEvent::Select(p.clone()) {
            ProjectEvent::Select(got) => assert_eq!(got, p),
            _ => panic!("expected Select variant"),
        }
    }

    /// Add variant is a unit — it signals the coordinator to refresh after
    /// the portal picker completes.
    #[test]
    fn project_event_add_is_unit() {
        assert!(matches!(ProjectEvent::Add, ProjectEvent::Add));
    }
}