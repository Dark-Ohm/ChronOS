//! Dock view: horizontal row of pinned application icons.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use chronos_services::{AppEntry, Service};
use chronos_ui::Theme;
use gpui::ImageSource;
use gpui::{
    App, Context, Focusable, InteractiveElement, IntoElement, MouseButton, Render, Styled, Window,
    div, img, prelude::*, px,
};

use crate::dock::config::DockConfig;
use crate::launcher::launch::launch;
use crate::state;

const DOCK_HEIGHT: f32 = 56.;
const ICON_SIZE: f32 = 40.;
const ICON_PADDING: f32 = 8.;

/// Cached icon path resolutions. Populated lazily, never invalidated
/// (icon themes rarely change mid-session).
static ICON_CACHE: OnceLock<std::sync::Mutex<HashMap<String, Option<PathBuf>>>> = OnceLock::new();

fn icon_cache() -> &'static std::sync::Mutex<HashMap<String, Option<PathBuf>>> {
    ICON_CACHE.get_or_init(|| std::sync::Mutex::new(HashMap::new()))
}

pub struct DockView {
    icons: Vec<(AppEntry, Option<PathBuf>)>,
    entries: Vec<AppEntry>,
    focus: gpui::FocusHandle,
}

impl DockView {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let config = DockConfig::load();
        let entries = state::AppState::applications(cx).get().entries.clone();
        let icons = build_dock_icons(&config.pinned, &entries);

        let view = Self {
            icons,
            entries,
            focus: cx.focus_handle(),
        };

        // Subscribe to applications — pinned list may gain/lose entries
        // when packages are installed/removed.
        let signal = state::AppState::applications(cx).subscribe();
        state::watch(cx, signal, |this, state, cx| {
            this.entries = state.entries.clone();
            let config = DockConfig::load();
            this.icons = build_dock_icons(&config.pinned, &this.entries);
            cx.notify();
        });

        // Subscribe to dock config changes (e.g. unpin from context menu).
        let config_signal = cx.global::<crate::dock::signal::DockConfigSignal>()
            .signal
            .signal();
        state::watch(cx, config_signal, |this, _, cx| {
            let config = DockConfig::load();
            this.icons = build_dock_icons(&config.pinned, &this.entries);
            cx.notify();
        });

        view
    }
}

impl Render for DockView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = Theme::global(cx);

        div()
            .track_focus(&self.focus)
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .gap(px(ICON_PADDING))
            .bg(theme.bg.primary)
            .children(self.icons.iter().map(|(entry, icon_path)| {
                let entry = entry.clone();
                let icon_path = icon_path.clone();
                let label = entry.name.clone();
                let entry_id = entry.id.clone();

                div()
                    .id(format!("dock-icon-{}", entry.id))
                    .h(px(DOCK_HEIGHT))
                    .w(px(DOCK_HEIGHT))
                    .flex()
                    .flex_col()
                    .items_center()
                    .justify_center()
                    .rounded_lg()
                    .hover(|s| s.bg(theme.interactive.hover))
                    .cursor_pointer()
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
                    .child(match icon_path {
                        Some(path) => {
                            let src: ImageSource = path.into();
                            img(src)
                                .w(px(ICON_SIZE))
                                .h(px(ICON_SIZE))
                                .into_any_element()
                        }
                        None => {
                            // Fallback: first letter of name as text icon.
                            let letter = label
                                .chars()
                                .next()
                                .unwrap_or('?')
                                .to_uppercase()
                                .to_string();
                            div()
                                .w(px(ICON_SIZE))
                                .h(px(ICON_SIZE))
                                .flex()
                                .items_center()
                                .justify_center()
                                .rounded_md()
                                .bg(theme.bg.elevated)
                                .child(div().text_lg().text_color(theme.text.primary).child(letter))
                                .into_any_element()
                        }
                    })
            }))
    }
}

impl Focusable for DockView {
    fn focus_handle(&self, _cx: &App) -> gpui::FocusHandle {
        self.focus.clone()
    }
}

/// Build the list of pinned icons with resolved paths from the full entries list.
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

/// Resolve an icon name (from AppEntry.icon) to an absolute path.
/// Caches results for the process lifetime.
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
        let _ = resolve_icon("nonexistent-icon-xyz"); // second call hits cache
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
        // Only kitty should be in the dock (it's in pinned).
        assert!(icons.iter().any(|(e, _)| e.id == "kitty"));
        assert!(!icons.iter().any(|(e, _)| e.id == "notpinned"));
    }
}
