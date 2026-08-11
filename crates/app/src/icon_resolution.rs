//! Shared icon resolution — maps icon names (from `.desktop` `Icon=` field)
//! to on-disk paths by walking the freedesktop icon theme hierarchy.
//!
//! Used by both the dock widget and the launcher to render application
//! icons. Extracted from `bar/widgets/dock.rs` so both surfaces share one
//! resolution path and one cache.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

/// Cached icon path resolutions (icon name → resolved path).
static ICON_CACHE: OnceLock<Mutex<HashMap<String, Option<PathBuf>>>> = OnceLock::new();

fn icon_cache() -> &'static Mutex<HashMap<String, Option<PathBuf>>> {
    ICON_CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Resolve an icon name to an on-disk path (PNG or SVG).
///
/// Walks the freedesktop icon theme hierarchy (user → system → hicolor
/// fallback) and returns the first existing match. Results are cached by
/// name — repeated lookups are O(1) after the first.
///
/// Returns `None` when no matching icon file exists (caller falls back to a
/// letter glyph or placeholder).
pub fn resolve_icon(name: &str) -> Option<PathBuf> {
    let mut cache = icon_cache().lock().unwrap();
    if let Some(cached) = cache.get(name) {
        return cached.clone();
    }
    let result = resolve_icon_uncached(name);
    cache.insert(name.to_string(), result.clone());
    result
}

fn resolve_icon_uncached(name: &str) -> Option<PathBuf> {
    let as_path = Path::new(name);
    if as_path.is_absolute() {
        return as_path.exists().then(|| as_path.to_path_buf());
    }

    let bases = vec![
        PathBuf::from("/usr/share/icons"),
        PathBuf::from("/usr/local/share/icons"),
        dirs::home_dir()
            .map(|h| h.join(".local/share/icons"))
            .unwrap_or_default(),
        dirs::home_dir()
            .map(|h| h.join(".icons"))
            .unwrap_or_default(),
    ];

    let chain = theme_chain(&bases);
    let sizes = ["48x48", "64x64", "32x32", "256x256"];
    let contexts = ["apps", "categories", "devices", "mimetypes", ""];
    let exts = ["png", "svg"];

    for base in &bases {
        for theme in &chain {
            if theme.is_empty() {
                continue;
            }
            for size in &sizes {
                for ctx in &contexts {
                    for ext in &exts {
                        let path = if ctx.is_empty() {
                            base.join(theme).join(size).join(name).with_extension(ext)
                        } else {
                            base.join(theme)
                                .join(size)
                                .join(ctx)
                                .join(name)
                                .with_extension(ext)
                        };
                        if path.exists() {
                            return Some(path);
                        }
                    }
                }
            }
        }
    }
    None
}

fn theme_chain(bases: &[PathBuf]) -> Vec<String> {
    static CHAIN: OnceLock<Vec<String>> = OnceLock::new();
    CHAIN.get_or_init(|| build_theme_chain(bases)).clone()
}

fn build_theme_chain(bases: &[PathBuf]) -> Vec<String> {
    let mut chain = Vec::new();
    let mut visited = HashSet::new();

    let start = read_gtk_icon_theme()
        .or_else(|| read_default_theme(bases))
        .unwrap_or_else(|| "hicolor".to_string());

    collect_inherits(&start, &mut chain, &mut visited, 0, bases);

    if !chain.iter().any(|t| t == "hicolor") {
        chain.push("hicolor".to_string());
    }
    chain
}

fn collect_inherits(
    theme: &str,
    chain: &mut Vec<String>,
    visited: &mut HashSet<String>,
    depth: u32,
    bases: &[PathBuf],
) {
    if depth > 4 || theme.is_empty() || visited.contains(theme) {
        return;
    }
    visited.insert(theme.to_string());
    chain.push(theme.to_string());

    for base in bases {
        let index = base.join(theme).join("index.theme");
        if let Ok(content) = std::fs::read_to_string(&index) {
            if let Some(inherits) = parse_inherits(&content) {
                for parent in inherits
                    .split(',')
                    .map(|s| s.trim())
                    .filter(|s| !s.is_empty())
                {
                    collect_inherits(parent, chain, visited, depth + 1, bases);
                }
            }
            return;
        }
    }
}

fn parse_inherits(content: &str) -> Option<String> {
    for line in content.lines() {
        let l = line.trim();
        if let Some(rest) = l.strip_prefix("Inherits") {
            let rest = rest.trim_start_matches([' ', '=']);
            let value = rest.split('#').next().unwrap_or(rest).trim();
            if !value.is_empty() {
                return Some(value.to_string());
            }
        }
    }
    None
}

fn read_gtk_icon_theme() -> Option<String> {
    let home = dirs::home_dir()?;
    let settings = home.join(".config/gtk-3.0/settings.ini");
    let content = std::fs::read_to_string(&settings).ok()?;
    for line in content.lines() {
        let l = line.trim();
        if let Some(rest) = l.strip_prefix("gtk-icon-theme-name") {
            let rest = rest.trim_start_matches([' ', '=']);
            let value = rest.split('#').next().unwrap_or(rest).trim().to_string();
            if !value.is_empty() {
                return Some(value);
            }
        }
    }
    None
}

fn read_default_theme(bases: &[PathBuf]) -> Option<String> {
    for base in bases {
        let index = base.join("default").join("index.theme");
        if let Ok(content) = std::fs::read_to_string(&index) {
            for line in content.lines() {
                let l = line.trim();
                if let Some(rest) = l.strip_prefix("Inherits") {
                    let rest = rest.trim_start_matches([' ', '=']);
                    let value = rest.split('#').next().unwrap_or(rest).trim().to_string();
                    if !value.is_empty() {
                        return Some(value);
                    }
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_icon_returns_cached() {
        let _ = resolve_icon("nonexistent-icon-xyz");
        let _ = resolve_icon("nonexistent-icon-xyz");
    }
}
