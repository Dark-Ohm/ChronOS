//! Tray widget for the bar — renders real system-tray icons.
//!
//! Data comes from `AppState::tray(cx)` (live `TrayState`). Rendering uses a
//! three-tier fallback chain per spec:
//!   1. `icon_name` → freedesktop icon-theme lookup (own hicolor-tree walk,
//!      no extra crate) → `img(path)`. Resolved paths are cached per
//!      `icon_name` in a thread-local map (render() fires on every notify).
//!      `icon_name` may itself be an absolute path — checked first.
//!   2. `icon_pixmap` → GPUI `RenderImage` built from raw RGBA (the service
//!      already did ARGB→RGBA; we do RGBA→BGRA here, since GPUI stores decoded
//!      images in BGRA — see `Source/gpui/src/assets.rs`).
//!   3. text badge (first letter of title/icon_name) — the OpenCode MVP.
//!
//! A click dispatches `TrayCommand::ActivateItem` (left-click activation,
//! `StatusNotifierItem.Activate(0,0)`) — unchanged from the MVP widget.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use gpui::{
    AnyElement, App, ObjectFit, RenderImage, Window, div, img, prelude::*, px,
};
use image::{Frame, RgbaImage};
use smallvec::SmallVec;

use chronos_luau::bar::{BarSection, BarWidget};
use chronos_services::{Service, TrayCommand, TrayItem, TrayPixmap};

use crate::state::AppState;

/// Rendered tray icon edge length, in CSS pixels.
const ICON_PX: f32 = 18.0;

pub struct TrayWidget;

impl BarWidget for TrayWidget {
    fn name(&self) -> &str {
        "tray"
    }

    fn section(&self) -> BarSection {
        BarSection::Right
    }

    fn render(&self, _window: &mut Window, cx: &App) -> AnyElement {
        let tray = AppState::tray(cx);
        let state = tray.get();

        if state.items.is_empty() {
            return div().into_any_element();
        }

        let theme = chronos_ui::Theme::global(cx);
        let radius = theme.radius;

        let badges: Vec<AnyElement> = state
            .items
            .iter()
            .map(|item| {
                let id = item.id.clone();

                div()
                    .id(format!("tray-item-{id}"))
                    .cursor_pointer()
                    .px(px(6.))
                    .py(px(2.))
                    .rounded(radius)
                    .child(render_icon(item))
                    .on_click(move |_event, _window, cx: &mut App| {
                        AppState::tray(cx)
                            .dispatch(TrayCommand::ActivateItem { service: id.clone() });
                    })
                    .into_any_element()
            })
            .collect();

        div()
            .flex()
            .items_center()
            .gap(px(4.))
            .children(badges)
            .into_any_element()
    }
}

/// Render a single tray item's icon, following the fallback chain:
/// `icon_name` (theme/absolute path) → `icon_pixmap` (raw RGBA) → letter.
fn render_icon(item: &TrayItem) -> AnyElement {
    // 1. icon_name → resolved file path (cached by icon_name).
    if let Some(name) = item.icon_name.as_deref() {
        if !name.is_empty() {
            if let Some(path) = cached_resolve_icon(name) {
                return img(path)
                    .w(px(ICON_PX))
                    .h(px(ICON_PX))
                    .object_fit(ObjectFit::Contain)
                    .into_any_element();
            }
        }
    }

    // 2. icon_pixmap → GPUI RenderImage from raw RGBA (RGBA→BGRA for GPU).
    if let Some(pm) = item.icon_pixmap.as_ref() {
        if let Some(rendered) = pixmap_render_image(pm) {
            return img(rendered)
                .w(px(ICON_PX))
                .h(px(ICON_PX))
                .object_fit(ObjectFit::Contain)
                .into_any_element();
        }
    }

    // 3. Letter fallback (OpenCode MVP badge).
    div().child(item.label.clone()).into_any_element()
}

/// Build a GPUI `RenderImage` from a raw RGBA `TrayPixmap`.
///
/// GPUI stores decoded images in **BGRA** (see `Source/gpui/src/assets.rs`:
/// "A cached and processed image, in BGRA format"; all file decoders in
/// `img.rs` do `pixel.swap(0, 2)` RGBA→BGRA before `Frame::new`). The service
/// already converted ARGB→RGBA, so here we only do the final RGBA→BGRA swap.
fn pixmap_render_image(pm: &TrayPixmap) -> Option<Arc<RenderImage>> {
    let mut data = pm.data.clone();
    for pixel in data.chunks_exact_mut(4) {
        // RGBA [R,G,B,A] -> BGRA [B,G,R,A]
        pixel.swap(0, 2);
    }
    let buffer = RgbaImage::from_raw(pm.width, pm.height, data)?;
    let frame = Frame::new(buffer);
    Some(Arc::new(RenderImage::new(SmallVec::from_elem(frame, 1))))
}

// ── icon-theme resolution ──────────────────────────────────────────────

const ICON_SIZES: &[&str] = &[
    "scalable", "symbolic", "48x48", "32x32", "24x24", "22x22", "16x16",
];
const ICON_CONTEXTS: &[&str] = &[
    "devices", "apps", "actions", "status", "categories", "mimetypes", "panel", "places",
];
const ICON_EXTS: &[&str] = &["svg", "png"];

thread_local! {
    static ICON_CACHE: std::cell::RefCell<HashMap<String, Option<PathBuf>>> =
        std::cell::RefCell::new(HashMap::new());
}

fn cached_resolve_icon(icon_name: &str) -> Option<PathBuf> {
    if let Some(hit) = ICON_CACHE.with(|c| c.borrow().get(icon_name).cloned()) {
        return hit;
    }
    let resolved = resolve_icon_path(icon_name);
    ICON_CACHE.with(|c| {
        c.borrow_mut().insert(icon_name.to_string(), resolved.clone());
    });
    resolved
}

fn resolve_icon_path(icon_name: &str) -> Option<PathBuf> {
    let as_path = Path::new(icon_name);
    if as_path.is_absolute() {
        return as_path.exists().then(|| as_path.to_path_buf());
    }
    let theme = current_icon_theme();
    for base in icon_search_dirs() {
        for theme_name in [theme.as_str(), "hicolor"] {
            if theme_name.is_empty() { continue; }
            for size in ICON_SIZES {
                for ctx in ICON_CONTEXTS {
                    for ext in ICON_EXTS {
                        let candidate = base
                            .join(theme_name).join(size).join(ctx)
                            .join(icon_name).with_extension(ext);
                        if candidate.exists() { return Some(candidate); }
                    }
                }
            }
        }
    }
    None
}

fn icon_search_dirs() -> Vec<PathBuf> {
    let mut search_dirs = vec![
        PathBuf::from("/usr/share/icons"),
        PathBuf::from("/usr/local/share/icons"),
    ];
    if let Some(home) = dirs::home_dir() {
        search_dirs.push(home.join(".local/share/icons"));
        search_dirs.push(home.join(".icons"));
    }
    search_dirs
}

fn current_icon_theme() -> String {
    static THEME: OnceLock<String> = OnceLock::new();
    THEME
        .get_or_init(|| read_gtk_icon_theme().unwrap_or_else(|| "hicolor".to_string()))
        .clone()
}

fn read_gtk_icon_theme() -> Option<String> {
    let path = dirs::config_dir()?.join("gtk-3.0/settings.ini");
    let content = std::fs::read_to_string(&path).ok()?;
    for raw in content.lines() {
        let line = raw.trim();
        if let Some(rest) = line.strip_prefix("gtk-icon-theme-name") {
            let rest = rest.trim_start_matches([' ', '=']);
            let value = rest.split('#').next().unwrap_or(rest).trim();
            if !value.is_empty() { return Some(value.to_string()); }
        }
    }
    None
}

pub fn register(cx: &mut App) {
    use chronos_luau::bar::BarWidgetRegistry;
    cx.global_mut::<BarWidgetRegistry>()
        .register(Box::new(TrayWidget));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bogus_icon_name_resolves_to_none() {
        assert!(resolve_icon_path("chronos-totally-bogus-icon-xyz-9999").is_none());
        assert!(cached_resolve_icon("chronos-totally-bogus-icon-xyz-9999").is_none());
    }

    #[test]
    fn missing_absolute_path_resolves_to_none() {
        assert!(resolve_icon_path("/nonexistent/chronos-icon-xyz.png").is_none());
    }

    #[test]
    fn pixmap_render_image_swaps_rgba_to_bgra() {
        let pm = TrayPixmap { width: 1, height: 1, data: vec![0x10, 0x20, 0x30, 0xFF] };
        let img = pixmap_render_image(&pm).expect("render image builds");
        assert_eq!(img.frame_count(), 1);
        let bytes = img.as_bytes(0).expect("frame bytes present");
        assert_eq!(bytes, &[0x30, 0x20, 0x10, 0xFF]);
    }

    #[test]
    fn pixmap_render_image_bad_length_yields_none() {
        let pm = TrayPixmap { width: 2, height: 2, data: vec![0; 4] };
        assert!(pixmap_render_image(&pm).is_none());
    }
}
