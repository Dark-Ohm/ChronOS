//! Active project root — same `~/.config/chronos/projects.toml` as the bar
//! project switcher. Kept here so the Build tab (lib crate) does not depend
//! on the binary-only `project_switcher` module.

use std::path::PathBuf;

use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct ProjectEntry {
    name: String,
    path: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
struct ProjectsFile {
    active: Option<String>,
    #[serde(default)]
    projects: Vec<ProjectEntry>,
}

/// Active project name + path, if configured and present in the list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveProject {
    pub name: String,
    pub path: PathBuf,
}

pub fn projects_config_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("~/.config"))
        .join("chronos/projects.toml")
}

/// Soft-load active project. Missing/broken config → `None` (no panic).
pub fn load_active_project() -> Option<ActiveProject> {
    let path = projects_config_path();
    let content = std::fs::read_to_string(&path).ok()?;
    let cfg: ProjectsFile = match toml::from_str(&content) {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!(
                path = %path.display(),
                error = %e,
                "tasks: failed to parse projects.toml"
            );
            return None;
        }
    };
    let active = cfg.active.as_deref()?;
    cfg.projects.into_iter().find(|p| p.path == active).map(|p| {
        ActiveProject {
            name: p.name,
            path: PathBuf::from(p.path),
        }
    })
}

// Cargo-root detection lives in `config::detect_cargo_tasks`, which checks the
// manifest itself — a second copy here was dead code kept alive by its own test.
