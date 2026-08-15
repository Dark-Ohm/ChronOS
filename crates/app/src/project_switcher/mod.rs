//! Project switcher — persistent `{name, path}` list + active project.
//!
//! T279 / Slice A2: the popup lifecycle (`ProjectPopupState`, `view.rs`,
//! `open`/`close`/`toggle`) is gone — project selection now lives as an
//! embedded `ProjectTab` inside the left workspace content canvas. This
//! module keeps only the domain that `ProjectTab` (and any future caller)
//! needs: `ProjectsConfig` persistence, branch lookup, and the add/select
//! actions. `init` still reloads + logs the cache; it no longer registers a
//! popup global.
//!
//! Config: `~/.config/chronos/projects.toml` (same cached-load pattern as
//! `dock/config.rs`). Branch comes from parsing `.git/HEAD` directly (a
//! ~30-byte file read — no subprocess, no inotify). "Add project" opens the
//! real XDG portal directory picker via `ashpd`; the portal call runs on a
//! throwaway tokio runtime in its own thread because GPUI's executor is not
//! a tokio context (HANDOFF: spawn_blocking outside tokio hangs).

use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use gpui::App;
use serde::{Deserialize, Serialize};

// ── Config ──

static CONFIG_CACHE: OnceLock<Mutex<ProjectsConfig>> = OnceLock::new();

fn config_cache() -> &'static Mutex<ProjectsConfig> {
    CONFIG_CACHE.get_or_init(|| Mutex::new(ProjectsConfig::default()))
}

pub fn cached() -> ProjectsConfig {
    config_cache().lock().unwrap().clone()
}

/// Test-only: seed the in-memory cache without touching `projects.toml`.
#[cfg(test)]
pub(crate) fn set_cached_for_test(config: ProjectsConfig) {
    *config_cache().lock().unwrap() = config;
}

pub fn reload_cache() {
    *config_cache().lock().unwrap() = ProjectsConfig::load();
}

fn update_cache_and_save(config: ProjectsConfig) {
    if let Err(e) = config.save() {
        tracing::warn!("project_switcher: failed to save projects.toml: {e}");
    }
    *config_cache().lock().unwrap() = config;
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct ProjectEntry {
    pub name: String,
    pub path: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq, Eq)]
pub struct ProjectsConfig {
    /// Path of the active project (matches a `projects[].path`).
    pub active: Option<String>,
    #[serde(default)]
    pub projects: Vec<ProjectEntry>,
}

impl ProjectsConfig {
    pub fn load() -> Self {
        let path = config_path();
        match std::fs::read_to_string(&path) {
            Ok(content) => match toml::from_str::<ProjectsConfig>(&content) {
                Ok(config) => config,
                Err(e) => {
                    tracing::warn!(
                        "project_switcher: failed to parse projects.toml: {e}, using empty"
                    );
                    Self::default()
                }
            },
            Err(_) => Self::default(),
        }
    }

    pub fn save(&self) -> Result<(), std::io::Error> {
        let path = config_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(self).expect("ProjectsConfig is always serializable");
        std::fs::write(path, content)
    }

    pub fn active_entry(&self) -> Option<&ProjectEntry> {
        let active = self.active.as_deref()?;
        self.projects.iter().find(|p| p.path == active)
    }
}

fn config_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("~/.config"))
        .join("chronos/projects.toml")
}

// ── Git branch ──

/// Current branch of the repo at `path`, from `.git/HEAD` directly.
/// Handles worktrees (`.git` as a `gitdir: …` file) and detached HEAD
/// (short hash). `None` when `path` is not a git repo.
pub fn current_branch(path: &Path) -> Option<String> {
    let git = path.join(".git");
    let head_path = if git.is_file() {
        let content = std::fs::read_to_string(&git).ok()?;
        let dir = content.strip_prefix("gitdir:")?.trim();
        let dir = Path::new(dir);
        if dir.is_absolute() {
            dir.join("HEAD")
        } else {
            path.join(dir).join("HEAD")
        }
    } else {
        git.join("HEAD")
    };
    let head = std::fs::read_to_string(head_path).ok()?;
    let head = head.trim();
    match head.strip_prefix("ref: refs/heads/") {
        Some(branch) => Some(branch.to_string()),
        None => Some(head.chars().take(7).collect()),
    }
}

// ── Actions ──

/// Set the active project path and persist it. T279: the popup `close_this`
/// is gone — selecting a project from `ProjectTab` updates the config and
/// the workspace coordinator reloads scope. No `Window` argument.
pub(crate) fn set_active(path: String, cx: &mut App) {
    let mut config = cached();
    config.active = Some(path);
    update_cache_and_save(config);
    tracing::info!("project_switcher: active project set");
    let _ = cx; // no callback yet — ProjectTab drives the reload explicitly.
}

/// Remove a project from the list and persist. If the removed path was the
/// active project, `active` is cleared — the workspace coordinator's own
/// scope cleanup lives in `side_panel_left::remove_project_scope` (the
/// Project tab runs both). Returns `true` when an entry was removed.
pub(crate) fn remove_project(path: &str, cx: &mut App) -> bool {
    let mut config = cached();
    let before = config.projects.len();
    config.projects.retain(|p| p.path != path);
    if config.projects.len() == before {
        return false;
    }
    if config.active.as_deref() == Some(path) {
        config.active = None;
    }
    update_cache_and_save(config);
    tracing::info!("project_switcher: removed project {path}");
    let _ = cx;
    true
}

/// "+ Add project": XDG portal directory picker on a dedicated thread.
/// ashpd runs on its async-io reactor (the feature set the gpui fork already
/// pins — tokio feature conflicts at unification), so a plain
/// `futures::executor::block_on` drives it; the result comes back through a
/// tokio oneshot (a plain future — awaitable on the GPUI executor).
///
/// T279: the popup `refresh_popup` is gone — the list repaints via the
/// `ProjectTab` entity observing the config cache on next render.
///
/// T279 round 2: `on_added` fires after the persist so the caller (the
/// Project tab) can forward the picked path to the workspace coordinator
/// — Add must run the same clear+load transaction as Select.
pub(crate) fn add_project(
    cx: &mut App,
    on_added: impl Fn(PathBuf, &mut App) + Send + 'static,
) {
    let (tx, rx) = tokio::sync::oneshot::channel::<Option<PathBuf>>();

    std::thread::spawn(move || {
        let picked = async_io::block_on(async {
            let request = ashpd::desktop::file_chooser::SelectedFiles::open_file()
                .title("Добавить проект")
                .directory(true)
                .send()
                .await;
            match request.and_then(|r| r.response()) {
                Ok(files) => files
                    .uris()
                    .first()
                    .and_then(|uri| file_uri_to_path(uri.as_str())),
                Err(e) => {
                    tracing::info!("project_switcher: picker cancelled/failed: {e}");
                    None
                }
            }
        });
        if tx.send(picked).is_err() {
            tracing::warn!("project_switcher: picker result receiver dropped");
        }
    });

    cx.spawn(async move |cx| {
        let Ok(Some(path)) = rx.await else {
            return;
        };
        let _ = cx.update(|cx: &mut App| {
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| path.display().to_string());
            let path_str = path.display().to_string();
            let mut config = cached();
            if !config.projects.iter().any(|p| p.path == path_str) {
                config.projects.push(ProjectEntry {
                    name,
                    path: path_str.clone(),
                });
            }
            config.active = Some(path_str);
            update_cache_and_save(config);
            tracing::info!("project_switcher: added project, reloading cache");
            on_added(path, cx);
        });
    })
    .detach();
}

/// `file:///home/x/my%20dir` → `/home/x/my dir`. Portal always returns
/// `file://` URIs with percent-encoding; anything else → None.
pub(crate) fn file_uri_to_path(uri: &str) -> Option<PathBuf> {
    let encoded = uri.strip_prefix("file://")?;
    let mut bytes = Vec::with_capacity(encoded.len());
    let mut chars = encoded.bytes();
    while let Some(b) = chars.next() {
        if b == b'%' {
            let hi = chars.next()?;
            let lo = chars.next()?;
            let hex = [hi, lo];
            let hex = std::str::from_utf8(&hex).ok()?;
            bytes.push(u8::from_str_radix(hex, 16).ok()?);
        } else {
            bytes.push(b);
        }
    }
    use std::os::unix::ffi::OsStringExt;
    Some(PathBuf::from(std::ffi::OsString::from_vec(bytes)))
}

pub fn init(cx: &mut App) {
    // T279: no popup global to register — the embedded `ProjectTab` owns the
    // selection surface. Init still reloads the cache and logs the count.
    reload_cache();
    tracing::info!(
        "project_switcher: loaded {} projects",
        cached().projects.len()
    );
    let _ = cx;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_roundtrip() {
        let config = ProjectsConfig {
            active: Some("/a".into()),
            projects: vec![ProjectEntry {
                name: "a".into(),
                path: "/a".into(),
            }],
        };
        let s = toml::to_string(&config).unwrap();
        let back: ProjectsConfig = toml::from_str(&s).unwrap();
        assert_eq!(config, back);
    }

    #[test]
    fn empty_config_parses() {
        let config: ProjectsConfig = toml::from_str("").unwrap();
        assert!(config.projects.is_empty());
        assert!(config.active.is_none());
    }

    #[test]
    fn active_entry_matches_by_path() {
        let config = ProjectsConfig {
            active: Some("/b".into()),
            projects: vec![
                ProjectEntry {
                    name: "a".into(),
                    path: "/a".into(),
                },
                ProjectEntry {
                    name: "b".into(),
                    path: "/b".into(),
                },
            ],
        };
        assert_eq!(config.active_entry().unwrap().name, "b");
    }

    #[test]
    fn branch_of_this_repo_is_readable() {
        // The ChronOS repo itself is a live fixture.
        let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let repo = manifest.parent().unwrap().parent().unwrap();
        let branch = current_branch(repo);
        assert!(branch.is_some(), "expected a branch for {repo:?}");
        assert!(!branch.unwrap().is_empty());
    }

    #[test]
    fn branch_of_non_repo_is_none() {
        assert_eq!(current_branch(Path::new("/tmp")), None);
    }

    /// T279: no popup artifacts remain in the public API after carve.
    #[test]
    fn file_uri_to_path_decodes_percent() {
        let p = file_uri_to_path("file:///home/x/my%20dir").unwrap();
        assert_eq!(p, PathBuf::from("/home/x/my dir"));
    }
}