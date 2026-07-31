//! Build/Logs tab — run project tasks and stream output (T178).
//!
//! Engine: `chronos_services::tasks` (no GPUI). Project root comes from
//! `ProjectsConfig::active_entry()`, never `current_dir()`.

use std::path::PathBuf;
use std::time::Duration;

use chronos_services::tasks::{
    ResolveTasks, RunStatus, StreamKind, TaskDef, TaskSession, TaskSource,
    load_active_project, resolve_tasks,
};
use chronos_ui::Theme;
use gpui::{
    Context, FontWeight, InteractiveElement, IntoElement, ParentElement, Render, ScrollHandle,
    SharedString, StatefulInteractiveElement, Styled, Window, div, prelude::*, px,
};

const POLL_MS: u64 = 50;
const TASK_LIST_H: f32 = 160.;

/// How the tab resolved its project root (for honest empty states).
#[derive(Debug, Clone)]
enum ProjectCtx {
    Active { name: String, path: PathBuf },
    None,
}

pub struct BuildTab {
    project: ProjectCtx,
    tasks: Vec<TaskDef>,
    source: TaskSource,
    session: TaskSession,
    /// Autoscroll to bottom unless user scrolled up.
    stick_bottom: bool,
    scroll: ScrollHandle,
    /// Cached display lines (refreshed on poll).
    display_lines: Vec<(StreamKind, SharedString)>,
}

impl BuildTab {
    /// Lazy: only runs when the tab view is first created (first activation).
    pub fn new(cx: &mut Context<Self>) -> Self {
        tracing::info!("side_panel_right build: tab opened — loading tasks");
        let project = match load_active_project() {
            Some(p) => ProjectCtx::Active {
                name: p.name,
                path: p.path,
            },
            None => ProjectCtx::None,
        };

        let (tasks, source) = match &project {
            ProjectCtx::Active { path, .. } => {
                let ResolveTasks { tasks, source } = resolve_tasks(path);
                (tasks, source)
            }
            ProjectCtx::None => (
                Vec::new(),
                TaskSource::Empty {
                    looked_in: "no active project in ~/.config/chronos/projects.toml".into(),
                },
            ),
        };

        let mut this = Self {
            project,
            tasks,
            source,
            session: TaskSession::default(),
            stick_bottom: true,
            scroll: ScrollHandle::new(),
            display_lines: Vec::new(),
        };
        this.sync_display();
        this.start_poll(cx);
        this
    }

    fn start_poll(&self, cx: &mut Context<Self>) {
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(POLL_MS))
                    .await;
                let cont = this
                    .update(cx, |this, cx| {
                        this.session.poll();
                        this.sync_display();
                        if this.stick_bottom {
                            // ScrollHandle API: request bottom on next layout.
                            this.scroll.scroll_to_bottom();
                        }
                        cx.notify();
                        true
                    })
                    .unwrap_or(false);
                if !cont {
                    break;
                }
            }
        })
        .detach();
    }

    fn sync_display(&mut self) {
        self.display_lines = self
            .session
            .buffer()
            .lines()
            .iter()
            .map(|l| (l.stream, SharedString::from(l.text.clone())))
            .collect();
    }

    fn run_task(&mut self, task: &TaskDef, cx: &mut Context<Self>) {
        let ProjectCtx::Active { path, .. } = &self.project else {
            return;
        };
        self.stick_bottom = true;
        match self.session.start(task, path) {
            Ok(()) => {
                tracing::info!(task = %task.id, path = %path.display(), "build: task started");
            }
            Err(e) => {
                tracing::warn!(error = %e, "build: start failed");
            }
        }
        self.sync_display();
        cx.notify();
    }

    fn cancel(&mut self, cx: &mut Context<Self>) {
        self.session.cancel();
        self.sync_display();
        cx.notify();
    }

    fn status_label(status: &RunStatus) -> String {
        match status {
            RunStatus::Idle => "idle".into(),
            RunStatus::Running { started } => {
                format!("running… {:.1}s", started.elapsed().as_secs_f32())
            }
            RunStatus::Ok { code, duration } => {
                format!("ok (exit {code}, {:.1}s)", duration.as_secs_f32())
            }
            RunStatus::Failed {
                code,
                duration,
                detail,
            } => match code {
                Some(c) => format!("failed (exit {c}, {:.1}s) — {detail}", duration.as_secs_f32()),
                None => format!("failed ({:.1}s) — {detail}", duration.as_secs_f32()),
            },
        }
    }
}

impl Render for BuildTab {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = *Theme::global(cx);
        let running = self.session.status().is_running();
        let status_text = Self::status_label(self.session.status());
        let active_id = self.session.active_task_id().map(|s| s.to_string());

        // Header: project + overall status
        let header = div()
            .id("build-header")
            .flex()
            .flex_col()
            .gap(px(4.))
            .px(px(12.))
            .py(px(10.))
            .border_b_1()
            .border_color(theme.border.subtle)
            .child(
                div()
                    .text_size(px(13.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(theme.text.primary)
                    .child("Build / Logs"),
            )
            .child(match &self.project {
                ProjectCtx::Active { name, path } => div()
                    .text_size(px(11.))
                    .font_family(theme.font_mono)
                    .text_color(theme.text.muted)
                    .child(format!("{name} — {}", path.display())),
                ProjectCtx::None => div()
                    .text_size(px(12.))
                    .text_color(theme.status.error)
                    .child(
                        "No active project. Set `active` in ~/.config/chronos/projects.toml \
                         (project switcher).",
                    ),
            })
            .child(
                div()
                    .text_size(px(11.))
                    .text_color(theme.text.muted)
                    .child(status_text),
            );

        // Task list or honest empty
        let mut task_panel = div()
            .id("build-tasks")
            .flex()
            .flex_col()
            .gap(px(4.))
            .px(px(8.))
            .py(px(8.))
            .max_h(px(TASK_LIST_H))
            .overflow_y_scroll()
            .border_b_1()
            .border_color(theme.border.subtle);

        match (&self.project, self.tasks.is_empty()) {
            (ProjectCtx::None, _) => {
                task_panel = task_panel.child(
                    div()
                        .px(px(8.))
                        .py(px(6.))
                        .text_size(px(12.))
                        .text_color(theme.text.muted)
                        .child("Tasks unavailable without an active project."),
                );
            }
            (_, true) => {
                let looked = match &self.source {
                    TaskSource::Empty { looked_in } => looked_in.clone(),
                    _ => "tasks.toml / Cargo.toml".into(),
                };
                task_panel = task_panel.child(
                    div()
                        .px(px(8.))
                        .py(px(6.))
                        .text_size(px(12.))
                        .text_color(theme.text.muted)
                        .child(format!("No tasks found. Looked in: {looked}")),
                );
            }
            _ => {
                for task in &self.tasks {
                    let task = task.clone();
                    let is_active = active_id.as_deref() == Some(task.id.as_str());
                    let label = task.label.clone();
                    let can_run = !running && matches!(self.project, ProjectCtx::Active { .. });

                    let row = div()
                        .id(SharedString::from(format!("build-task-{}", task.id)))
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .px(px(8.))
                        .py(px(6.))
                        .rounded_md()
                        .bg(if is_active {
                            theme.interactive.hover
                        } else {
                            theme.bg.elevated
                        })
                        .child(
                            div()
                                .flex_1()
                                .min_w(px(0.))
                                .text_size(px(12.))
                                .text_color(theme.text.primary)
                                .child(label),
                        )
                        .child(
                            div()
                                .id(SharedString::from(format!("build-run-{}", task.id)))
                                .px(px(10.))
                                .py(px(4.))
                                .rounded_md()
                                .cursor_pointer()
                                .bg(if can_run {
                                    theme.accent.primary
                                } else {
                                    theme.border.subtle
                                })
                                .text_color(if can_run {
                                    theme.text.primary
                                } else {
                                    theme.text.muted
                                })
                                .text_size(px(11.))
                                .when(can_run, |el| {
                                    el.on_click(cx.listener({
                                        let task = task.clone();
                                        move |this, _e, _w, cx| {
                                            this.run_task(&task, cx);
                                        }
                                    }))
                                })
                                .child(if running && is_active {
                                    "…"
                                } else {
                                    "Run"
                                }),
                        );
                    task_panel = task_panel.child(row);
                }
            }
        }

        if running {
            task_panel = task_panel.child(
                div()
                    .id("build-cancel")
                    .mt(px(4.))
                    .px(px(10.))
                    .py(px(6.))
                    .rounded_md()
                    .cursor_pointer()
                    .bg(theme.status.error)
                    .text_color(theme.text.primary)
                    .text_size(px(11.))
                    .on_click(cx.listener(|this, _e, _w, cx| this.cancel(cx)))
                    .child("Cancel"),
            );
        }

        // Log output
        let mut log = div()
            .id("build-log")
            .flex_1()
            .min_h(px(0.))
            .overflow_y_scroll()
            .track_scroll(&self.scroll)
            .flex()
            .flex_col()
            .px(px(10.))
            .py(px(8.))
            .font_family(theme.font_mono)
            .text_size(px(11.));

        if self.session.buffer().is_truncated() {
            log = log.child(
                div()
                    .mb(px(6.))
                    .text_color(theme.status.warning)
                    .child(format!(
                        "… {} earlier lines dropped (cap {})",
                        self.session.buffer().dropped(),
                        self.session.buffer().cap()
                    )),
            );
        }

        if self.display_lines.is_empty() && !running {
            log = log.child(
                div()
                    .text_color(theme.text.muted)
                    .child("Output will appear here when a task runs."),
            );
        }

        for (i, (stream, text)) in self.display_lines.iter().enumerate() {
            let color = match stream {
                StreamKind::Stdout => theme.text.primary,
                StreamKind::Stderr => theme.status.error,
                StreamKind::System => theme.status.warning,
            };
            log = log.child(
                div()
                    .id(SharedString::from(format!("build-line-{i}")))
                    .text_color(color)
                    .child(text.clone()),
            );
        }

        div()
            .size_full()
            .flex()
            .flex_col()
            .child(header)
            .child(task_panel)
            .child(log)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::TestAppContext;

    fn install_theme(cx: &mut TestAppContext) {
        cx.update(|cx| {
            cx.set_global(Theme::default());
        });
    }

    #[gpui::test]
    fn build_tab_creates_without_panic(cx: &mut TestAppContext) {
        install_theme(cx);
        let view = cx.new(BuildTab::new);
        cx.update_entity(&view, |this, _cx| {
            // Must not invent ok status.
            assert!(matches!(this.session.status(), RunStatus::Idle));
        });
    }

    #[gpui::test]
    fn status_label_idle_is_not_ok() {
        assert_eq!(BuildTab::status_label(&RunStatus::Idle), "idle");
        assert!(!BuildTab::status_label(&RunStatus::Idle).contains("ok"));
    }
}
