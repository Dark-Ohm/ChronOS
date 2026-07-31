//! Task runner for the Build/Logs panel (T178).
//!
//! No GPUI: pure process spawn, log buffer, and task list resolution.
//! UI lives in `side_panel_right/tab/build.rs`.

mod buffer;
mod project;
mod config;
mod runner;

pub use buffer::{DEFAULT_LOG_CAP, LogBuffer, LogLine, StreamKind};
pub use config::{
    LoadOutcome, ResolveTasks, TaskDef, TaskSource, TasksConfig, detect_cargo_tasks,
    load_tasks_config, parse_tasks_toml, resolve_tasks, tasks_config_path,
};
pub use project::{ActiveProject, load_active_project};
pub use runner::{RunStatus, TaskSession};
