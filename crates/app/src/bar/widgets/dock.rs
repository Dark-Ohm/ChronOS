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

use gpui::ImageSource;
use gpui::{AnyElement, App, InteractiveElement, MouseButton, Window, div, img, prelude::*, px};

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

/// Pins we already warned about this process (render is every frame — no flood).
static SKIP_WARNED: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();

fn skip_warned() -> &'static Mutex<HashSet<String>> {
    SKIP_WARNED.get_or_init(|| Mutex::new(HashSet::new()))
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

        // Mode/scene composition (scene > mode default > stored dock.toml).
        let pinned = config::resolve_pinned(cx);

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
        let divider = div().w(px(1.)).h(px(14.)).bg(theme.bg.secondary);

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
                        img(src).w(px(ICON_PX)).h(px(ICON_PX)).into_any_element()
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
                            .child(div().text_sm().text_color(theme.text.primary).child(letter))
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
                    .on_mouse_down(MouseButton::Right, move |_event, _window, cx: &mut App| {
                        crate::dock::context_menu::open(cx, entry_id.clone());
                    })
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

/// Install dock globals once and load `dock.toml` into the in-memory cache.
///
/// Called from [`super::register_builtin`] at bar init. Idempotent: repeated
/// calls do not replace an existing [`DockMenuState`] / [`DockConfigSignal`]
/// (layout reloads must not wipe an open context menu).
///
/// Widget instances themselves are owned by [`super::apply_layout`] (T134) —
/// this function does **not** register `DockWidget` into `BarWidgetRegistry`.
pub fn register(cx: &mut App) {
    if !cx.has_global::<crate::dock::context_menu::DockMenuState>() {
        cx.set_global(crate::dock::context_menu::DockMenuState::default());
    }
    if !cx.has_global::<crate::dock::signal::DockConfigSignal>() {
        cx.set_global(crate::dock::signal::DockConfigSignal::default());
    }
    config::reload_cache();
}

// ── Icon resolution (ported from dock/view.rs) ──

/// Why a pinned id was dropped from the dock strip (testable, no log capture).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PinSkipReason {
    /// No `.desktop` entry with this basename among scanned applications.
    NoAppEntry,
}

impl PinSkipReason {
    fn as_str(&self) -> &'static str {
        match self {
            Self::NoAppEntry => "no AppEntry (no matching .desktop basename)",
        }
    }
}

/// Resolve one pin against the applications catalog.
///
/// `Ok((entry, icon_path))` — show it (icon may still be `None` → letter glyph).
/// `Err(reason)` — drop it; caller must log.
fn resolve_pin(
    pin_id: &str,
    entries: &[AppEntry],
) -> Result<(AppEntry, Option<PathBuf>), PinSkipReason> {
    let entry = entries
        .iter()
        .find(|e| e.id == pin_id)
        .ok_or(PinSkipReason::NoAppEntry)?;
    let icon_path = entry.icon.as_deref().and_then(resolve_icon);
    Ok((entry.clone(), icon_path))
}

fn build_dock_icons(pinned: &[String], entries: &[AppEntry]) -> Vec<(AppEntry, Option<PathBuf>)> {
    pinned
        .iter()
        .filter_map(|pin_id| match resolve_pin(pin_id, entries) {
            Ok(pair) => Some(pair),
            Err(reason) => {
                // Once per pin_id per process — render runs every frame.
                let first = skip_warned()
                    .lock()
                    .map(|mut s| s.insert(pin_id.clone()))
                    .unwrap_or(true);
                if first {
                    tracing::warn!(
                        pin = %pin_id,
                        reason = reason.as_str(),
                        "dock: skipping pinned app"
                    );
                }
                None
            }
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
        // Pin present in catalog stays; pin absent is dropped (and warned).
        let pinned = vec!["kitty".to_string(), "ghost".to_string()];
        let icons = build_dock_icons(&pinned, &entries);
        assert!(icons.iter().any(|(e, _)| e.id == "kitty"));
        assert!(!icons.iter().any(|(e, _)| e.id == "ghost"));
        assert!(!icons.iter().any(|(e, _)| e.id == "notpinned"));
    }

    #[test]
    fn resolve_pin_reports_no_app_entry() {
        let entries = vec![AppEntry {
            id: "kitty".into(),
            name: "Kitty".into(),
            exec: "/usr/bin/kitty".into(),
            icon: Some("kitty".into()),
            terminal: false,
        }];
        assert_eq!(
            resolve_pin("firefox", &entries).unwrap_err(),
            PinSkipReason::NoAppEntry
        );
        assert!(resolve_pin("kitty", &entries).is_ok());
    }

    #[test]
    fn resolve_pin_allows_missing_icon() {
        let entries = vec![AppEntry {
            id: "thunar".into(),
            name: "Thunar".into(),
            exec: "thunar".into(),
            icon: None,
            terminal: false,
        }];
        let (entry, icon) = resolve_pin("thunar", &entries).expect("entry exists");
        assert_eq!(entry.id, "thunar");
        assert!(icon.is_none());
    }

    /// Globals install once; `apply_layout` rebuilds the registry without
    /// wiping `DockMenuState` (regression for the T166/T170 panic path).
    #[gpui::test]
    fn dock_globals_survive_apply_layout(cx: &mut gpui::TestAppContext) {
        use chronos_luau::bar::BarWidgetRegistry;
        use crate::bar::layout_config::BarLayoutConfig;
        use crate::bar::widgets;
        use crate::dock::context_menu::DockMenuState;
        use crate::dock::signal::DockConfigSignal;

        cx.update(|cx| {
            cx.set_global(BarWidgetRegistry::default());
            widgets::dock::register(cx);
            assert!(cx.has_global::<DockMenuState>());
            assert!(cx.has_global::<DockConfigSignal>());

            // Stamp entry_id without opening a window (no Theme/Wayland needed).
            cx.global_mut::<DockMenuState>()
                .set_entry_id_for_test(Some("marker".into()));
            assert_eq!(cx.global::<DockMenuState>().entry_id(), Some("marker"));

            let cfg = BarLayoutConfig::default();
            widgets::apply_layout(cx, &cfg);
            assert!(
                cx.has_global::<DockMenuState>(),
                "apply_layout must not remove DockMenuState"
            );
            assert_eq!(
                cx.global::<DockMenuState>().entry_id(),
                Some("marker"),
                "apply_layout must not re-set DockMenuState"
            );
            assert!(cx.has_global::<DockConfigSignal>());

            // Second register is idempotent (does not wipe marker).
            widgets::dock::register(cx);
            assert_eq!(cx.global::<DockMenuState>().entry_id(), Some("marker"));
        });
    }
}
