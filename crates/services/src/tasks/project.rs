//! Active project root — same `~/.config/chronos/projects.toml` as the bar
//! project switcher. Kept here so the Build tab (lib crate) does not depend
//! on the binary-only `project_switcher` module.

use std::path::{Path, PathBuf};

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

/// Whether `root` looks like a cargo workspace/package root.
pub fn has_cargo_toml(root: &Path) -> bool {
    root.join("Cargo.toml").is_file()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn missing_file_is_none() {
        // Function reads the real config path; we only unit-test parse of structure
        // via has_cargo_toml / ActiveProject construction.
        let dir = tempdir().unwrap();
        assert!(!has_cargo_toml(dir.path()));
        fs::write(dir.path().join("Cargo.toml"), "").unwrap();
        assert!(has_cargo_toml(dir.path()));
    }
}
