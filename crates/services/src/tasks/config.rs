//! Task list config — `~/.config/chronos/tasks.toml` + cargo autodetection.
//!
//! Format (documented for HANDOFF):
//! ```toml
//! [[tasks]]
//! id = "build"
//! label = "cargo build"
//! command = "cargo"
//! args = ["build"]
//! ```
//! Missing file → empty list (caller may autodect). Broken TOML → warn + empty.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// One runnable task definition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskDef {
    /// Stable id for UI keys (e.g. `"cargo-build"`).
    pub id: String,
    /// Human label shown in the panel.
    pub label: String,
    /// Executable name or path (`"cargo"`, `"make"`, …).
    pub command: String,
    /// Arguments after the command.
    #[serde(default)]
    pub args: Vec<String>,
}

/// Root of `tasks.toml`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TasksConfig {
    #[serde(default)]
    pub tasks: Vec<TaskDef>,
}

/// Soft-parse TOML text. Invalid → empty + log-friendly message.
pub fn parse_tasks_toml(text: &str) -> Result<TasksConfig, String> {
    toml::from_str::<TasksConfig>(text).map_err(|e| e.to_string())
}

/// Load `~/.config/chronos/tasks.toml` if present.
///
/// - missing file → `Ok(None)`
/// - unreadable → `Ok(None)` + caller can log
/// - bad TOML → `Ok(Some(empty))` is wrong — return `Err` message for warn
pub fn load_tasks_config() -> LoadOutcome {
    let path = tasks_config_path();
    match std::fs::read_to_string(&path) {
        Ok(content) => match parse_tasks_toml(&content) {
            Ok(cfg) => LoadOutcome::Loaded(cfg),
            Err(e) => LoadOutcome::ParseError {
                path,
                message: e,
            },
        },
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => LoadOutcome::Missing,
        Err(e) => LoadOutcome::IoError {
            path,
            message: e.to_string(),
        },
    }
}

/// Result of trying to read the user task list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoadOutcome {
    Missing,
    Loaded(TasksConfig),
    ParseError { path: PathBuf, message: String },
    IoError { path: PathBuf, message: String },
}

pub fn tasks_config_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("~/.config"))
        .join("chronos/tasks.toml")
}

/// If `root` contains `Cargo.toml`, return the four default cargo tasks.
pub fn detect_cargo_tasks(root: &Path) -> Vec<TaskDef> {
    if !root.join("Cargo.toml").is_file() {
        return Vec::new();
    }
    cargo_default_tasks()
}

fn cargo_default_tasks() -> Vec<TaskDef> {
    vec![
        TaskDef {
            id: "cargo-build".into(),
            label: "cargo build".into(),
            command: "cargo".into(),
            args: vec!["build".into()],
        },
        TaskDef {
            id: "cargo-test".into(),
            label: "cargo test".into(),
            command: "cargo".into(),
            args: vec!["test".into()],
        },
        TaskDef {
            id: "cargo-clippy".into(),
            label: "cargo clippy".into(),
            command: "cargo".into(),
            args: vec!["clippy".into(), "--all-targets".into()],
        },
        TaskDef {
            id: "cargo-run".into(),
            label: "cargo run".into(),
            command: "cargo".into(),
            args: vec!["run".into()],
        },
    ]
}

/// Resolve the task list for a project root: user config first, else cargo detect.
pub fn resolve_tasks(project_root: &Path) -> ResolveTasks {
    match load_tasks_config() {
        LoadOutcome::Loaded(cfg) if !cfg.tasks.is_empty() => ResolveTasks {
            tasks: cfg.tasks,
            source: TaskSource::UserConfig,
        },
        LoadOutcome::Loaded(_) => {
            // Empty config file — fall through to detect.
            let tasks = detect_cargo_tasks(project_root);
            ResolveTasks {
                source: if tasks.is_empty() {
                    TaskSource::Empty {
                        looked_in: format!(
                            "{} (empty), {}",
                            tasks_config_path().display(),
                            project_root.join("Cargo.toml").display()
                        ),
                    }
                } else {
                    TaskSource::CargoToml
                },
                tasks,
            }
        }
        LoadOutcome::Missing => {
            let tasks = detect_cargo_tasks(project_root);
            ResolveTasks {
                source: if tasks.is_empty() {
                    TaskSource::Empty {
                        looked_in: format!(
                            "{} (missing), {}",
                            tasks_config_path().display(),
                            project_root.join("Cargo.toml").display()
                        ),
                    }
                } else {
                    TaskSource::CargoToml
                },
                tasks,
            }
        }
        LoadOutcome::ParseError { path, message } => {
            tracing::warn!(
                path = %path.display(),
                error = %message,
                "tasks: failed to parse tasks.toml — using empty list"
            );
            ResolveTasks {
                tasks: Vec::new(),
                source: TaskSource::Empty {
                    looked_in: format!("{} (parse error: {message})", path.display()),
                },
            }
        }
        LoadOutcome::IoError { path, message } => {
            tracing::warn!(
                path = %path.display(),
                error = %message,
                "tasks: failed to read tasks.toml"
            );
            ResolveTasks {
                tasks: Vec::new(),
                source: TaskSource::Empty {
                    looked_in: format!("{} (io: {message})", path.display()),
                },
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolveTasks {
    pub tasks: Vec<TaskDef>,
    pub source: TaskSource,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskSource {
    UserConfig,
    CargoToml,
    Empty { looked_in: String },
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn parse_valid_tasks_toml() {
        let text = r#"
[[tasks]]
id = "build"
label = "cargo build"
command = "cargo"
args = ["build"]
"#;
        let cfg = parse_tasks_toml(text).unwrap();
        assert_eq!(cfg.tasks.len(), 1);
        assert_eq!(cfg.tasks[0].id, "build");
        assert_eq!(cfg.tasks[0].args, vec!["build"]);
    }

    #[test]
    fn parse_broken_toml_is_err() {
        assert!(parse_tasks_toml("[[[ not valid").is_err());
    }

    #[test]
    fn parse_empty_tasks_ok() {
        let cfg = parse_tasks_toml("").unwrap();
        assert!(cfg.tasks.is_empty());
    }

    #[test]
    fn detect_cargo_when_manifest_present() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("Cargo.toml"), "[package]\nname=\"x\"\n").unwrap();
        let tasks = detect_cargo_tasks(dir.path());
        assert_eq!(tasks.len(), 4);
        assert!(tasks.iter().any(|t| t.id == "cargo-build"));
    }

    #[test]
    fn detect_empty_without_manifest() {
        let dir = tempdir().unwrap();
        assert!(detect_cargo_tasks(dir.path()).is_empty());
    }
}
