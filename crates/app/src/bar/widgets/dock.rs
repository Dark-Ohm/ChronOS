//! Dock bar widget — pinned application icons in the left cluster.
//!
//! Replaces the standalone dock window (removed in #8). Reads `DockConfig`
//! from a Global cache (loaded once at init, invalidated by `DockConfigSignal`)
//! and `ApplicationsState` from `AppState` on every render.
//!
//! Left cluster layout (per `Top Bar.dc.html`):
//!   [Start] | [app icons...] | (then workspaces further right)

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use gpui::{
    AnyElement, App, InteractiveElement, MouseButton, Window, div, img, prelude::*, px,
};
use gpui::ImageSource;

use chronos_luau::bar::{BarSection, BarWidget};
use chronos_services::{AppEntry, Service};
use chronos_ui::Theme;

use crate::dock::config;
use crate::launcher::launch::launch;
use crate::state::AppState;

/// Cached icon path resolutions.
static ICON_CACHE: OnceLock<Mutex<HashMap<String, Option<PathBuf>>>> = OnceLock::new();

fn icon_cache() -> &'static Mutex<HashMap<String, Option<PathBuf>>> {
    ICON_CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

const ICON_PX: f32 = 18.0;

pub struct DockWidget;

impl BarWidget for DockWidget {
    fn name(&self) -> &str {
        "dock"
    }

    fn section(&self) -> BarSection {
        BarSection::Left
    }

    fn render(&self, _window: &mut Window, cx: &App) -> AnyElement {
        let theme = Theme::global(cx);

        // Read cached config (no disk I/O per render).
        let pinned = config::cached().pinned;

        // Read applications state.
        let entries = AppState::applications(cx).get().entries.clone();

        let icons = build_dock_icons(&pinned, &entries);

        // Start button — ChronOS hexagon glyph.
        let start_button = div()
            .id("dock-start")
            .h(px(24.))
            .w(px(24.))
            .flex()
            .items_center()
            .justify_center()
            .rounded_lg()
            .cursor_pointer()
            .hover(|s| s.bg(theme.interactive.hover))
            .on_click(move |_event, _window, cx: &mut App| {
                crate::launcher::toggle(cx);
            })
            .child(
                gpui::svg()
                    .path("icons/hexagon-sigil.svg")
                    .size(px(15.))
                    .text_color(theme.accent.primary),
            );

        // Divider after start button.
        let divider = div()
            .w(px(1.))
            .h(px(14.))
            .bg(theme.bg.secondary);

        // App icons.
        let app_icons: Vec<AnyElement> = icons
            .iter()
            .map(|(entry, icon_path)| {
                let entry = entry.clone();
                let icon_path = icon_path.clone();
                let label = entry.name.clone();
                let entry_id = entry.id.clone();

                let icon_elem = match icon_path {
                    Some(path) => {
                        let src: ImageSource = path.into();
                        img(src)
                            .w(px(ICON_PX))
                            .h(px(ICON_PX))
                            .into_any_element()
                    }
                    None => {
                        let letter = label
                            .chars()
                            .next()
                            .unwrap_or('?')
                            .to_uppercase()
                            .to_string();
                        div()
                            .w(px(ICON_PX))
                            .h(px(ICON_PX))
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded_md()
                            .bg(theme.bg.elevated)
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(theme.text.primary)
                                    .child(letter),
                            )
                            .into_any_element()
                    }
                };

                div()
                    .id(format!("dock-icon-{}", entry.id))
                    .h(px(24.))
                    .w(px(24.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded_lg()
                    .cursor_pointer()
                    .hover(|s| s.bg(theme.interactive.hover))
                    .on_click(move |_event, _window, _cx: &mut App| {
                        if let Err(e) = launch(&entry.exec) {
                            tracing::error!("dock: failed to launch {}: {e:#}", entry.name);
                        }
                    })
                    .on_mouse_down(
                        MouseButton::Right,
                        move |_event, _window, cx: &mut App| {
                            crate::dock::context_menu::open(cx, entry_id.clone());
                        },
                    )
                    .child(icon_elem)
                    .into_any_element()
            })
            .collect();

        div()
            .flex()
            .items_center()
            .gap(px(3.))
            .child(start_button)
            .child(divider)
            .children(app_icons)
            .into_any_element()
    }
}

/// Register the dock widget with the global bar registry.
pub fn register(cx: &mut App) {
    // Init dock globals (context menu + config change signal).
    cx.set_global(crate::dock::context_menu::DockMenuState::default());
    cx.set_global(crate::dock::signal::DockConfigSignal::default());

    // Load config cache from disk.
    config::reload_cache();

    cx.global_mut::<chronos_luau::bar::BarWidgetRegistry>()
        .register(Box::new(DockWidget));
}

// ── Icon resolution (ported from dock/view.rs) ──

fn build_dock_icons(pinned: &[String], entries: &[AppEntry]) -> Vec<(AppEntry, Option<PathBuf>)> {
    pinned
        .iter()
        .filter_map(|pin_id| {
            let entry = entries.iter().find(|e| e.id == *pin_id)?;
            let icon_path = entry.icon.as_deref().and_then(resolve_icon);
            Some((entry.clone(), icon_path))
        })
        .collect()
}

fn resolve_icon(name: &str) -> Option<PathBuf> {
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

    #[test]
    fn theme_chain_ends_with_hicolor() {
        let chain = theme_chain(&[]);
        assert!(chain.last().map(|s| s.as_str()) == Some("hicolor"));
    }

    #[test]
    fn build_dock_icons_skips_unresolved() {
        let entries = vec![
            AppEntry {
                id: "kitty".into(),
                name: "Kitty".into(),
                exec: "/usr/bin/kitty".into(),
                icon: Some("kitty".into()),
                terminal: false,
            },
            AppEntry {
                id: "notpinned".into(),
                name: "NotPinned".into(),
                exec: "/usr/bin/notpinned".into(),
                icon: None,
                terminal: false,
            },
        ];
        let pinned = vec!["kitty".to_string()];
        let icons = build_dock_icons(&pinned, &entries);
        assert!(icons.iter().any(|(e, _)| e.id == "kitty"));
        assert!(!icons.iter().any(|(e, _)| e.id == "notpinned"));
    }
}
