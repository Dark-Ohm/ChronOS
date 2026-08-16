//! Files provider (`/` / `~`): directory completion + `xdg-open` on Enter.

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use super::{ProviderAction, ProviderResult};

/// Soft cap on listed paths per keystroke (directory listing, not a scan).
const MAX_RESULTS: usize = 30;

/// List directory entries under the typed path's parent, filtered by the last
/// path component (case-insensitive prefix).
pub fn results(path: &str) -> Vec<ProviderResult> {
    let expanded = expand_tilde(path);
    // `~` alone means "list $HOME", not "show the home dir as one entry".
    let (dir, prefix) = if path == "~" {
        (PathBuf::from(&expanded), String::new())
    } else {
        split_dir_prefix(&expanded)
    };

    let Ok(entries) = std::fs::read_dir(&dir) else {
        return vec![ProviderResult {
            id: "files-error".into(),
            label: format!("cannot read {}", dir.display()),
            detail: None,
            glyph: '/',
            action: ProviderAction::None,
        }];
    };

    let prefix_lower = prefix.to_lowercase();
    let mut matches: Vec<PathBuf> = entries
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            let name = p
                .file_name()
                .map(|n| n.to_string_lossy().to_lowercase())
                .unwrap_or_default();
            name.starts_with(&prefix_lower)
        })
        .collect();
    matches.sort();
    matches.truncate(MAX_RESULTS);

    matches
        .into_iter()
        .map(|p| ProviderResult {
            id: format!("files-{}", p.display()),
            label: p
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default(),
            detail: Some(p.display().to_string()),
            glyph: if p.is_dir() { '▸' } else { '·' },
            action: ProviderAction::OpenPath(p.display().to_string()),
        })
        .collect()
}

/// Open a path with `xdg-open` (detached via setsid so it survives chronos).
pub fn open(path: &str) -> Result<()> {
    Command::new("setsid")
        .arg("xdg-open")
        .arg(path)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .stdin(Stdio::null())
        .spawn()
        .context("failed to run xdg-open")?;
    Ok(())
}

/// Expand a leading `~` / `~/...` to `$HOME`. Falls back to `/` when `HOME`
/// is unset (unlikely in a user session).
fn expand_tilde(raw: &str) -> String {
    if raw == "~" {
        std::env::var("HOME").unwrap_or_else(|_| "/".to_string())
    } else if let Some(rest) = raw.strip_prefix("~/") {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/".to_string());
        format!("{}/{}", home.trim_end_matches('/'), rest)
    } else {
        raw.to_string()
    }
}

/// Split a path into `(directory to list, name prefix to filter)`. A trailing
/// slash (or empty) means "list that directory entirely".
fn split_dir_prefix(path: &str) -> (PathBuf, String) {
    let p = Path::new(path);
    if path.is_empty() || path.ends_with('/') {
        return (p.to_path_buf(), String::new());
    }
    match (p.parent(), p.file_name()) {
        (Some(parent), Some(name)) => {
            let parent = if parent.as_os_str().is_empty() {
                Path::new("/")
            } else {
                parent
            };
            (
                parent.to_path_buf(),
                name.to_string_lossy().to_string(),
            )
        }
        _ => (PathBuf::from("/"), path.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_plain_path_into_parent_and_prefix() {
        let (dir, prefix) = split_dir_prefix("/home/neo/Do");
        assert_eq!(dir, PathBuf::from("/home/neo"));
        assert_eq!(prefix, "Do");
    }

    #[test]
    fn trailing_slash_lists_the_directory() {
        let (dir, prefix) = split_dir_prefix("/home/neo/");
        assert_eq!(dir, PathBuf::from("/home/neo/"));
        assert_eq!(prefix, "");
    }

    #[test]
    fn root_alone_lists_root() {
        let (dir, prefix) = split_dir_prefix("/");
        assert_eq!(dir, PathBuf::from("/"));
        assert_eq!(prefix, "");
    }

    #[test]
    fn single_component_lists_root() {
        let (dir, prefix) = split_dir_prefix("home");
        assert_eq!(dir, PathBuf::from("/"));
        assert_eq!(prefix, "home");
    }

    #[test]
    fn relative_path_keeps_relative_parent() {
        let (dir, prefix) = split_dir_prefix("foo/bar");
        assert_eq!(dir, PathBuf::from("foo"));
        assert_eq!(prefix, "bar");
    }
}
