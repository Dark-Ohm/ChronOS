//! Shared icon resolution — maps icon names (freedesktop icon-theme names)
//! to on-disk paths by walking the icon theme hierarchy.
//!
//! Used by the dock widget, the launcher, the tray widget and tray/dock
//! context menus. Extracted from `bar/widgets/dock.rs` and merged with the
//! tray widget's resolver (T263) so every surface shares one resolution
//! path and one cache.
//!
//! Search covers the sizes/contexts both consumers need: app icons live in
//! `apps/` at 48–512, tray icons in `status|panel|actions|symbolic`
//! at small sizes — the menu rows (`.ci-ic`) use the same lookup as tray.
//! Names that miss every theme dir fall back to flat bases
//! (`/usr/share/pixmaps` & the flatpak equivalents) where files sit directly
//! in the dir with no theme/size hierarchy — e.g. `anydesk.svg` ships only
//! there (T265-0).

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
    cached_resolve_icon(name)
}

/// Cached resolution (the shared cache both dock/launcher and tray menus use).
pub fn cached_resolve_icon(icon_name: &str) -> Option<PathBuf> {
    let mut cache = icon_cache().lock().unwrap();
    if let Some(cached) = cache.get(icon_name) {
        return cached.clone();
    }
    let result = resolve_icon_uncached(icon_name);
    cache.insert(icon_name.to_string(), result.clone());
    result
}

fn resolve_icon_uncached(name: &str) -> Option<PathBuf> {
    let as_path = Path::new(name);
    if as_path.is_absolute() {
        return as_path.exists().then(|| as_path.to_path_buf());
    }

    let themed = themed_bases();
    let chain = theme_chain(&themed);
    search_themed(name, &themed, &chain).or_else(|| search_flat(name, &flat_bases()))
}

/// Bases holding theme hierarchies (`<base>/<theme>/<size>/<context>/`).
/// The flatpak exports matter on systems where apps install their icons into
/// the sandbox export tree instead of `/usr/share/icons` (T265-0).
fn themed_bases() -> Vec<PathBuf> {
    vec![
        PathBuf::from("/usr/share/icons"),
        PathBuf::from("/usr/local/share/icons"),
        dirs::home_dir()
            .map(|h| h.join(".local/share/icons"))
            .unwrap_or_default(),
        dirs::home_dir()
            .map(|h| h.join(".icons"))
            .unwrap_or_default(),
        PathBuf::from("/var/lib/flatpak/exports/share/icons"),
        dirs::home_dir()
            .map(|h| h.join(".local/share/flatpak/exports/share/icons"))
            .unwrap_or_default(),
    ]
}

/// Bases holding icon files directly, with no theme/size hierarchy — the
/// legacy `/usr/share/pixmaps` layout. freedesktop defines it as the unthemed
/// fallback, so it is searched only after every themed lookup misses.
fn flat_bases() -> Vec<PathBuf> {
    vec![
        PathBuf::from("/usr/share/pixmaps"),
        PathBuf::from("/var/lib/flatpak/exports/share/pixmaps"),
        dirs::home_dir()
            .map(|h| h.join(".local/share/flatpak/exports/share/pixmaps"))
            .unwrap_or_default(),
    ]
}

fn search_themed(name: &str, bases: &[PathBuf], chain: &[String]) -> Option<PathBuf> {
    // App icons prefer large sizes; tray/menu icons prefer small/symbolic.
    // `symbolic` dirs hold single-color SVGs — tried for every name since
    // tray icons commonly resolve there. 128x128/512x512 are required for
    // apps that ship only large rasters (CMakeSetup, chatbox — T265-0).
    let sizes = [
        "scalable", "symbolic", "48x48", "64x64", "32x32", "256x256", "512x512", "128x128",
        "24x24", "22x22", "16x16",
    ];
    // `legacy` holds the freedesktop-named icons in the Adwaita*Legacy themes
    // (`network-wired` and friends, used by plenty of .desktop files).
    let contexts = [
        "apps",
        "categories",
        "devices",
        "mimetypes",
        "actions",
        "status",
        "panel",
        "places",
        "legacy",
        "",
    ];
    let exts = ["png", "svg"];

    for base in bases {
        for theme in chain {
            if theme.is_empty() {
                continue;
            }
            for size in &sizes {
                for ctx in &contexts {
                    for ext in &exts {
                        // Append the extension, never `with_extension`: icon
                        // names are dotted reverse-DNS more often than not
                        // (`org.xfce.thunar`), and `with_extension` would treat
                        // `thunar` as an extension and REPLACE it, producing
                        // `org.xfce.svg`. That silently lost every such icon.
                        let file = format!("{name}.{ext}");
                        let path = if ctx.is_empty() {
                            base.join(theme).join(size).join(&file)
                        } else {
                            base.join(theme).join(size).join(ctx).join(&file)
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

fn search_flat(name: &str, bases: &[PathBuf]) -> Option<PathBuf> {
    for base in bases {
        for ext in ["png", "svg"] {
            // Same dotted-name trap as in `search_themed`.
            let path = base.join(format!("{name}.{ext}"));
            if path.exists() {
                return Some(path);
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

    #[test]
    fn bogus_icon_name_resolves_to_none() {
        assert!(resolve_icon("chronos-totally-bogus-icon-xyz-9999").is_none());
    }

    #[test]
    fn missing_absolute_path_resolves_to_none() {
        assert!(resolve_icon("/nonexistent/chronos-icon-xyz.png").is_none());
    }

    #[test]
    fn themed_search_covers_128_and_512() {
        let dir = std::env::temp_dir().join(format!("chronos-sizes-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);

        std::fs::create_dir_all(dir.join("hicolor/512x512/apps")).unwrap();
        std::fs::write(dir.join("hicolor/512x512/apps/wide512.png"), b"").unwrap();
        std::fs::create_dir_all(dir.join("hicolor/128x128/apps")).unwrap();
        std::fs::write(dir.join("hicolor/128x128/apps/mid128.png"), b"").unwrap();

        let bases = vec![dir.clone()];
        let chain = vec!["hicolor".to_string()];
        let wide = search_themed("wide512", &bases, &chain);
        let mid = search_themed("mid128", &bases, &chain);
        let found = (wide, mid);
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(found.0, Some(dir.join("hicolor/512x512/apps/wide512.png")));
        assert_eq!(found.1, Some(dir.join("hicolor/128x128/apps/mid128.png")));
    }

    #[test]
    fn flat_fallback_finds_pixmaps_layout() {
        let dir = std::env::temp_dir().join(format!("chronos-pixmaps-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);

        // Themed base has the hierarchy but NOT the icon; the flat base holds
        // it directly — the /usr/share/pixmaps situation (anydesk.svg).
        let themed = dir.join("icons");
        let flat = dir.join("pixmaps");
        std::fs::create_dir_all(themed.join("hicolor/48x48/apps")).unwrap();
        std::fs::create_dir_all(&flat).unwrap();
        std::fs::write(flat.join("flatonly.svg"), b"").unwrap();

        let chain = vec!["hicolor".to_string()];
        let result = search_themed("flatonly", std::slice::from_ref(&themed), &chain)
            .or_else(|| search_flat("flatonly", std::slice::from_ref(&flat)));
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(result, Some(flat.join("flatonly.svg")));
    }

    #[test]
    fn themed_hit_wins_over_flat() {
        let dir = std::env::temp_dir().join(format!("chronos-order-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);

        let themed = dir.join("icons");
        let flat = dir.join("pixmaps");
        std::fs::create_dir_all(themed.join("hicolor/48x48/apps")).unwrap();
        std::fs::write(themed.join("hicolor/48x48/apps/dupe.png"), b"").unwrap();
        std::fs::create_dir_all(&flat).unwrap();
        std::fs::write(flat.join("dupe.png"), b"").unwrap();

        let chain = vec!["hicolor".to_string()];
        let result = search_themed("dupe", std::slice::from_ref(&themed), &chain)
            .or_else(|| search_flat("dupe", std::slice::from_ref(&flat)));
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(result, Some(themed.join("hicolor/48x48/apps/dupe.png")));
    }

    /// Reverse-DNS icon names are the norm (`org.xfce.thunar`), and
    /// `Path::with_extension` used to eat the last dotted segment, so every
    /// such icon silently resolved to nothing (caught live, T265-0 follow-up).
    #[test]
    fn dotted_icon_names_keep_all_their_segments() {
        let dir = std::env::temp_dir().join(format!("chronos-dotted-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);

        let themed = dir.join("icons");
        let flat = dir.join("pixmaps");
        std::fs::create_dir_all(themed.join("hicolor/scalable/apps")).unwrap();
        std::fs::write(
            themed.join("hicolor/scalable/apps/org.xfce.thunar.svg"),
            b"",
        )
        .unwrap();
        std::fs::create_dir_all(&flat).unwrap();
        std::fs::write(flat.join("com.example.flat.png"), b"").unwrap();

        let chain = vec!["hicolor".to_string()];
        let themed_hit = search_themed("org.xfce.thunar", std::slice::from_ref(&themed), &chain);
        let flat_hit = search_flat("com.example.flat", std::slice::from_ref(&flat));
        let _ = std::fs::remove_dir_all(&dir);

        assert_eq!(
            themed_hit,
            Some(themed.join("hicolor/scalable/apps/org.xfce.thunar.svg"))
        );
        assert_eq!(flat_hit, Some(flat.join("com.example.flat.png")));
    }

    /// `network-wired` and friends live under `legacy/` in the Adwaita*Legacy
    /// themes; without that context they were unreachable.
    #[test]
    fn legacy_context_is_searched() {
        let dir = std::env::temp_dir().join(format!("chronos-legacy-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);

        std::fs::create_dir_all(dir.join("AdwaitaLegacy/16x16/legacy")).unwrap();
        std::fs::write(dir.join("AdwaitaLegacy/16x16/legacy/network-wired.png"), b"").unwrap();

        let bases = vec![dir.clone()];
        let chain = vec!["AdwaitaLegacy".to_string()];
        let hit = search_themed("network-wired", &bases, &chain);
        let _ = std::fs::remove_dir_all(&dir);

        assert_eq!(
            hit,
            Some(dir.join("AdwaitaLegacy/16x16/legacy/network-wired.png"))
        );
    }

    #[test]
    fn collect_inherits_walks_chain() {
        let dir = std::env::temp_dir().join(format!("chronos-icons-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);

        std::fs::create_dir_all(dir.join("main")).unwrap();
        std::fs::write(
            dir.join("main/index.theme"),
            "[Icon Theme]\nInherits=parent\n",
        )
        .unwrap();
        std::fs::create_dir_all(dir.join("parent")).unwrap();
        std::fs::write(
            dir.join("parent/index.theme"),
            "[Icon Theme]\nName=Parent\n",
        )
        .unwrap();

        let bases = vec![dir.clone()];
        let mut chain = Vec::new();
        let mut visited = HashSet::new();
        collect_inherits("main", &mut chain, &mut visited, 0, &bases);
        collect_inherits("hicolor", &mut chain, &mut visited, 0, &bases);
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(chain, vec!["main", "parent", "hicolor"]);
    }

    #[test]
    fn collect_inherits_handles_cycles() {
        let dir = std::env::temp_dir().join(format!("chronos-cycle-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);

        std::fs::create_dir_all(dir.join("a")).unwrap();
        std::fs::write(dir.join("a/index.theme"), "[Icon Theme]\nInherits=b\n").unwrap();
        std::fs::create_dir_all(dir.join("b")).unwrap();
        std::fs::write(dir.join("b/index.theme"), "[Icon Theme]\nInherits=a\n").unwrap();

        let bases = vec![dir.clone()];
        let mut chain = Vec::new();
        let mut visited = HashSet::new();
        collect_inherits("a", &mut chain, &mut visited, 0, &bases);
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(chain, vec!["a", "b"]);
    }

    #[test]
    fn collect_inherits_respects_depth_limit() {
        let dir = std::env::temp_dir().join(format!("chronos-depth-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);

        for i in 0..6u32 {
            let d = dir.join(format!("d{i}"));
            std::fs::create_dir_all(&d).unwrap();
            if i < 5 {
                std::fs::write(
                    d.join("index.theme"),
                    format!("[Icon Theme]\nInherits=d{}\n", i + 1),
                )
                .unwrap();
            } else {
                std::fs::write(d.join("index.theme"), "[Icon Theme]\nName=Last\n").unwrap();
            }
        }

        let bases = vec![dir.clone()];
        let mut chain = Vec::new();
        let mut visited = HashSet::new();
        collect_inherits("d0", &mut chain, &mut visited, 0, &bases);
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(chain, vec!["d0", "d1", "d2", "d3", "d4"]);
    }

    #[test]
    fn read_default_theme_from_index_theme() {
        let dir = std::env::temp_dir().join(format!("chronos-default-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);

        std::fs::create_dir_all(dir.join("default")).unwrap();
        std::fs::write(
            dir.join("default/index.theme"),
            "[Icon Theme]\nInherits=Adwaita\n",
        )
        .unwrap();

        let bases = vec![dir.clone()];
        let theme = read_default_theme(&bases);
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(theme, Some("Adwaita".to_string()));
    }
}
