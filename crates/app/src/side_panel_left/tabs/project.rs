//! T279 / Slice A2 — the Project Switcher embedded as a tab.
//!
//! Replaces the popup (`project_switcher/view.rs`, deleted). Reads
//! `ProjectsConfig` via `project_switcher::cached()` and renders the project
//! list, active branch, and add/remove/Files/Terminal actions inside the
//! content canvas. Domain (config persistence, branch lookup, portal add)
//! stays in `project_switcher`; this tab is only the embedded view.
//! Files/Terminal forward to the right panel's
//! `open_files_at` / `open_terminal_at` reducers.
//!
//! Selection is one coordinator transaction: the tab forwards a
//! `ProjectEvent::Select(path)` to the `WorkspaceView` coordinator FIRST
//! (which runs `switch_project` — clear session, clear the chat column,
//! set path), and only then persists `ProjectsConfig.active` via
//! `project_switcher::set_active`. T279 round 2 fixed the order: the old
//! project's transcript must be gone from the screen BEFORE the config
//! names the new active project.

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
    /// User clicked "+ Add" and the portal picker returned `path` —
    /// coordinator runs the same transaction as `Select` (persist already
    /// happened inside `add_project`, so the coordinator must NOT persist
    /// again — only clear + load).
    Add(PathBuf),
    /// User removed a project — coordinator drops the config entry and, if
    /// it was active, clears the chat/session scope.
    Remove(PathBuf),
    /// Open the project root in the right panel's Files tab.
    OpenInFiles(PathBuf),
    /// Open a shell at the project root in the right panel's Terminal tab.
    OpenInTerminal(PathBuf),
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
            // T266: the project tab's plate follows surface alpha.
            .bg(theme.surface_color(theme.bg.primary))
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
                                // T279 round 2: `add_project` persists the
                                // new project, then invokes `on_added` —
                                // the callback emits `Add(path)` so the
                                // coordinator runs the same clear+load
                                // transaction as a plain Select.
                                let coordinator = this.coordinator.clone();
                                add_project(cx, move |path, cx| {
                                    if let Some(view) = coordinator.upgrade() {
                                        view.update(cx, |view, cx| {
                                            view.on_project_event(
                                                ProjectEvent::Add(path),
                                                cx,
                                            );
                                        });
                                    }
                                });
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
                            // T279 round 2: clear+load FIRST (the
                            // coordinator clears the chat column via
                            // `clear_for_project`), persist `active` LAST —
                            // the old transcript must be gone before the
                            // config names the new project.
                            this.emit(ProjectEvent::Select(path.clone()), cx);
                            set_active(path.to_string_lossy().to_string(), cx);
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
                                    .flex()
                                    .items_center()
                                    .gap(px(10.))
                                    .child(
                                        div()
                                            .text_size(theme.font_sizes.xs)
                                            .text_color(theme.text.muted)
                                            .child(branch.unwrap_or_default()),
                                    )
                                    // Row actions: Files / Terminal /
                                    // Remove. Each stops the click from
                                    // reaching the row's Select handler
                                    // (GPUI dispatches to the innermost
                                    // handler first; `stop_propagation`
                                    // keeps the row from switching the
                                    // active project on an action click).
                                    .child(
                                        div()
                                            .id(("project-files", i))
                                            .text_size(theme.font_sizes.xs)
                                            .text_color(theme.text.muted)
                                            .cursor_pointer()
                                            .hover(|s| s.text_color(theme.text.primary))
                                            .child("Files")
                                            .on_click({
                                                let path = path.clone();
                                                cx.listener(move |this, _e, _w, cx| {
                                                    cx.stop_propagation();
                                                    this.emit(
                                                        ProjectEvent::OpenInFiles(path.clone()),
                                                        cx,
                                                    );
                                                })
                                            }),
                                    )
                                    .child(
                                        div()
                                            .id(("project-terminal", i))
                                            .text_size(theme.font_sizes.xs)
                                            .text_color(theme.text.muted)
                                            .cursor_pointer()
                                            .hover(|s| s.text_color(theme.text.primary))
                                            .child("Term")
                                            .on_click({
                                                let path = path.clone();
                                                cx.listener(move |this, _e, _w, cx| {
                                                    cx.stop_propagation();
                                                    this.emit(
                                                        ProjectEvent::OpenInTerminal(
                                                            path.clone(),
                                                        ),
                                                        cx,
                                                    );
                                                })
                                            }),
                                    )
                                    .child(
                                        div()
                                            .id(("project-remove", i))
                                            .text_size(theme.font_sizes.xs)
                                            .text_color(theme.text.muted)
                                            .cursor_pointer()
                                            .hover(|s| s.text_color(theme.status.error))
                                            .child("✕")
                                            .on_click({
                                                let path = path.clone();
                                                cx.listener(move |this, _e, _w, cx| {
                                                    cx.stop_propagation();
                                                    this.emit(
                                                        ProjectEvent::Remove(path.clone()),
                                                        cx,
                                                    );
                                                })
                                            }),
                                    ),
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

    /// Add carries the picked path — the coordinator runs the same
    /// clear+load transaction as Select, so the payload must survive.
    /// (Round-1 had a unit `Add` + a `matches!(Add, Add)` tautology —
    /// removed in round 2.)
    #[test]
    fn project_event_add_carries_path() {
        let p = PathBuf::from("/home/neo/added-proj");
        match ProjectEvent::Add(p.clone()) {
            ProjectEvent::Add(got) => assert_eq!(got, p),
            _ => panic!("expected Add variant"),
        }
    }

    /// Remove / OpenInFiles / OpenInTerminal all carry the project path —
    /// the coordinator and the right-panel reducers need it (scope clear,
    /// Files root, terminal cwd). Guards against a future unit-variant
    /// refactor.
    #[test]
    fn project_event_actions_carry_path() {
        let p = PathBuf::from("/home/neo/proj");
        for event in [
            ProjectEvent::Remove(p.clone()),
            ProjectEvent::OpenInFiles(p.clone()),
            ProjectEvent::OpenInTerminal(p.clone()),
        ] {
            let got = match event {
                ProjectEvent::Remove(x)
                | ProjectEvent::OpenInFiles(x)
                | ProjectEvent::OpenInTerminal(x) => x,
                _ => panic!("expected an action variant"),
            };
            assert_eq!(got, p);
        }
    }

    /// Source contract (plan Task 4 Step 3 + round-2 order fix): inside the
    /// project-row click handler the coordinator `emit(Select)` must run
    /// BEFORE `set_active` persists `ProjectsConfig.active` — the old
    /// transcript is cleared before the config names the new project.
    /// Mirrors the T278 source-scan gate pattern.
    #[test]
    fn select_click_emits_before_persist() {
        let src = include_str!("project.rs");
        let emit_pos = src
            .find("this.emit(ProjectEvent::Select")
            .expect("project.rs must emit Select from the row click");
        let persist_pos = src
            .find("set_active(path.to_string_lossy()")
            .expect("project.rs must persist via set_active");
        assert!(
            emit_pos < persist_pos,
            "Select must be emitted before set_active persists (emit@{emit_pos} vs persist@{persist_pos})"
        );
    }
}