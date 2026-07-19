//! Project switcher — persistent `{name, path}` list + active project whose
//! git branch shows in the bar pill (`bar/widgets/project.rs`).
//!
//! Config: `~/.config/chronos/projects.toml` (same cached-load pattern as
//! `dock/config.rs`). Branch comes from parsing `.git/HEAD` directly (a
//! ~30-byte file read on the bar's 1s ticker — no subprocess, no inotify).
//! "Add project" opens the real XDG portal directory picker via `ashpd`;
//! the portal call runs on a throwaway tokio runtime in its own thread
//! because GPUI's executor is not a tokio context (HANDOFF: spawn_blocking
//! outside tokio hangs).

pub mod view;

use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use gpui::{
    App, Bounds, DisplayId, Global, Size, Window, WindowBackgroundAppearance, WindowBounds,
    WindowHandle, WindowKind, WindowOptions, layer_shell::*, point, prelude::*, px,
};
use serde::{Deserialize, Serialize};

use crate::project_switcher::view::ProjectPopupView;

const POPUP_WIDTH: f32 = 300.;
const POPUP_MARGIN_TOP: f32 = 36.;
const POPUP_MARGIN_RIGHT: f32 = 8.;
const HEADER_H: f32 = 40.;
const ROW_H: f32 = 34.;
const ADD_ROW_H: f32 = 38.;
const MAX_ROWS: usize = 12;

// ── Config ──

static CONFIG_CACHE: OnceLock<Mutex<ProjectsConfig>> = OnceLock::new();

fn config_cache() -> &'static Mutex<ProjectsConfig> {
    CONFIG_CACHE.get_or_init(|| Mutex::new(ProjectsConfig::default()))
}

pub fn cached() -> ProjectsConfig {
    config_cache().lock().unwrap().clone()
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
        let content =
            toml::to_string_pretty(self).expect("ProjectsConfig is always serializable");
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

// ── Popup lifecycle (updates_popup pattern: close_this guard, no
//    focus-loss close) ──

#[derive(Default)]
pub struct ProjectPopupState {
    handle: Option<WindowHandle<ProjectPopupView>>,
}

impl Global for ProjectPopupState {}

fn popup_height(project_count: usize) -> f32 {
    HEADER_H + (project_count.clamp(0, MAX_ROWS) as f32) * ROW_H + ADD_ROW_H
}

fn window_options(display_id: Option<DisplayId>, height: f32) -> WindowOptions {
    WindowOptions {
        display_id,
        titlebar: None,
        window_bounds: Some(WindowBounds::Windowed(Bounds {
            origin: point(px(0.), px(0.)),
            size: Size::new(px(POPUP_WIDTH), px(height)),
        })),
        app_id: Some("chronos-project-popup".to_string()),
        window_background: WindowBackgroundAppearance::Transparent,
        kind: WindowKind::LayerShell(LayerShellOptions {
            namespace: "project-popup".to_string(),
            layer: Layer::Overlay,
            anchor: Anchor::TOP | Anchor::RIGHT,
            exclusive_zone: None,
            margin: Some((px(POPUP_MARGIN_TOP), px(POPUP_MARGIN_RIGHT), px(0.), px(0.))),
            keyboard_interactivity: KeyboardInteractivity::None,
            ..Default::default()
        }),
        ..Default::default()
    }
}

pub fn open(cx: &mut App) {
    if cx.global::<ProjectPopupState>().handle.is_some() {
        return;
    }
    let display_id = crate::monitor::pult_display(cx);
    let height = popup_height(cached().projects.len());
    match cx.open_window(window_options(display_id, height), |_, app_cx| {
        app_cx.new(|_| ProjectPopupView {})
    }) {
        Ok(handle) => cx.global_mut::<ProjectPopupState>().handle = Some(handle),
        Err(err) => tracing::warn!("project_switcher: failed to open popup: {err}"),
    }
}

pub fn close(cx: &mut App) {
    if let Some(handle) = cx.global_mut::<ProjectPopupState>().handle.take() {
        let _ = handle.update(cx, |_, window: &mut Window, _| window.remove_window());
    }
}

pub(crate) fn close_this(window: &mut Window, cx: &mut App) {
    let this = window.window_handle();
    let tracked = cx
        .global::<ProjectPopupState>()
        .handle
        .as_ref()
        .map(|h| **h == this)
        .unwrap_or(false);
    if tracked {
        cx.global_mut::<ProjectPopupState>().handle.take();
    }
    window.remove_window();
}

pub fn toggle(_window: &mut Window, cx: &mut App) {
    let is_open = cx.global::<ProjectPopupState>().handle.is_some();
    if is_open {
        close(cx);
    } else {
        open(cx);
    }
}

/// Repaint + resize the open popup after the project list changed.
fn refresh_popup(cx: &mut App) {
    let handle = cx.global::<ProjectPopupState>().handle.clone();
    if let Some(handle) = handle {
        let height = popup_height(cached().projects.len());
        let _ = handle.update(cx, |_, window: &mut Window, view_cx| {
            window.resize(Size::new(px(POPUP_WIDTH), px(height)));
            view_cx.notify();
        });
    }
}

// ── Actions ──

pub(crate) fn set_active(path: String, window: &mut Window, cx: &mut App) {
    let mut config = cached();
    config.active = Some(path);
    update_cache_and_save(config);
    close_this(window, cx);
}

/// "+ Add project": XDG portal directory picker on a dedicated thread.
/// ashpd runs on its async-io reactor (the feature set the gpui fork already
/// pins — tokio feature conflicts at unification), so a plain
/// `futures::executor::block_on` drives it; the result comes back through a
/// tokio oneshot (a plain future — awaitable on the GPUI executor).
pub(crate) fn add_project(cx: &mut App) {
    let (tx, rx) = tokio::sync::oneshot::channel::<Option<PathBuf>>();

    std::thread::spawn(move || {
        let picked = async_io::block_on(async {
            let request = ashpd::desktop::file_chooser::SelectedFiles::open_file()
                .title("Добавить проект")
                .directory(true)
                .send()
                .await;
            match request.and_then(|r| r.response()) {
                Ok(files) => files.uris().first().and_then(|uri| file_uri_to_path(uri.as_str())),
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
            refresh_popup(cx);
        });
    })
    .detach();
}

/// `file:///home/x/my%20dir` → `/home/x/my dir`. Portal always returns
/// `file://` URIs with percent-encoding; anything else → None.
fn file_uri_to_path(uri: &str) -> Option<PathBuf> {
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
    cx.set_global(ProjectPopupState::default());
    reload_cache();
    tracing::info!(
        "project_switcher: loaded {} projects",
        cached().projects.len()
    );
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
}
