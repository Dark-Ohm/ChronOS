//! Tray widget for the bar — renders real system-tray icons.
//!
//! Data comes from `AppState::tray(cx)` (live `TrayState`). Rendering uses a
//! three-tier fallback chain per spec:
//!   1. `icon_name` → shared freedesktop icon-theme lookup
//!      (`crate::icon_resolution`) → `img(path)`. Resolved paths are cached
//!      per `icon_name` (render() fires on every notify).
//!      `icon_name` may itself be an absolute path — checked first.
//!   2. `icon_pixmap` → GPUI `RenderImage` built from raw RGBA (the service
//!      already did ARGB→RGBA; we do RGBA→BGRA here, since GPUI stores decoded
//!      images in BGRA — see `Source/gpui/src/assets.rs`).
//!   3. text badge (first letter of title/icon_name) — the OpenCode MVP.
//!
//! A click dispatches `TrayCommand::ActivateItem` (left-click activation,
//! `StatusNotifierItem.Activate(0,0)`) — unchanged from the MVP widget.

use std::cell::Cell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

use gpui::{
    AnyElement, App, Bounds, InteractiveElement, MouseButton, ObjectFit, Pixels, RenderImage, Window,
    canvas, div, img, prelude::*, px,
};
use image::{Frame, RgbaImage};
use smallvec::SmallVec;

use chronos_luau::bar::{BarSection, BarWidget};
use chronos_services::{Service, TrayCommand, TrayItem, TrayPixmap, TrayState};

use crate::state::AppState;

/// Rendered tray icon edge length, in CSS pixels.
const ICON_PX: f32 = 18.0;

pub struct TrayWidget {
    /// Captured on-screen bounds per tray item id — the anchor rect for the
    /// context popup (canon `positionRoot`: menu opens at the click point).
    bounds: Rc<std::cell::RefCell<HashMap<String, Rc<Cell<Bounds<Pixels>>>>>>,
}

impl TrayWidget {
    pub fn new() -> Self {
        Self {
            bounds: Rc::new(std::cell::RefCell::new(HashMap::new())),
        }
    }
}

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

        let theme = chronos_ui::Theme::global(cx);
        let radius = theme.radius;

        // Filter → dedupe → cap (in that order, per task spec).
        let prepared = prepare_tray_items(&state, MAX_TRAY_ITEMS);

        if prepared.visible.is_empty() {
            return div().into_any_element();
        }

        let mut badges: Vec<AnyElement> = prepared
            .visible
            .iter()
            .map(|item| {
                let id = item.id.clone();
                // Per-item bounds cell: the popup anchors to THIS icon, not
                // the whole tray block (canon: menu follows the clicked icon).
                let bounds_cell = self
                    .bounds
                    .borrow_mut()
                    .entry(id.clone())
                    .or_insert_with(|| Rc::new(Cell::new(Bounds::default())))
                    .clone();
                // Separate clone for the right-click handler so the left-click
                // `move` closure doesn't consume `id`.
                let id_right = id.clone();
                let bounds_cell_right = bounds_cell.clone();

                let badge = div()
                    .id(format!("tray-item-{id}"))
                    .cursor_pointer()
                    .px(px(6.))
                    .py(px(2.))
                    .rounded(radius)
                    .hover(|s| s.bg(theme.interactive.hover))
                    .child(render_icon(item))
                    .on_click(move |_event, _window, cx: &mut App| {
                        if crate::edit_mode::is_active(cx) {
                            return;
                        }
                        AppState::tray(cx).dispatch(TrayCommand::ActivateItem {
                            service: id.clone(),
                        });
                    })
                    // Right-click opens the DBusMenu context popup (toggle),
                    // anchored to this icon's bounds (canon `positionRoot`).
                    // Left-click ActivateItem above is intentionally untouched.
                    .on_mouse_down(MouseButton::Right, move |_event, window, cx: &mut App| {
                        if crate::edit_mode::is_active(cx) {
                            return;
                        }
                        let anchor_rect = bounds_cell_right.get();
                        let parent = window.window_handle();
                        crate::tray_menu::toggle(anchor_rect, parent, window, cx, id_right.clone());
                    });

                // Canvas captures this icon's live bounds into `bounds_cell`
                // every prepaint (same pattern as `bar/widgets/volume.rs`).
                div()
                    .relative()
                    .child(
                        canvas(
                            move |bounds, _window, _cx| bounds,
                            move |_bounds, captured, _window, _cx| {
                                bounds_cell.set(captured);
                            },
                        )
                        .absolute()
                        .size_full(),
                    )
                    .child(badge)
                    .into_any_element()
            })
            .collect();

        // Overflow indicator: `+N` in the same muted-badge language as the
        // bell/update counters, only when the cap bit off real items.
        if prepared.overflow > 0 {
            badges.push(
                div()
                    .id("tray-overflow")
                    .font_family(theme.font_mono)
                    .text_size(theme.font_sizes.sm)
                    .text_color(theme.text.muted)
                    .child(format!("+{}", prepared.overflow))
                    .into_any_element(),
            );
        }

        div()
            .flex()
            .items_center()
            .gap(px(4.))
            .children(badges)
            .into_any_element()
    }
}

/// Result of `prepare_tray_items`: the items to actually render plus the
/// number of hidden-overflow items (for the `+N` badge).
pub struct PreparedTray<'a> {
    /// Items passing filter + dedupe, capped to `max`.
    pub visible: Vec<&'a TrayItem>,
    /// How many usable items were dropped past the cap.
    pub overflow: usize,
}

/// Maximum number of tray icons shown; extras collapse into a `+N` badge.
const MAX_TRAY_ITEMS: usize = 8;

/// D-Bus name owner prefix of a registered service string.
///
/// A `StatusNotifierItem` id is either a unique bus name (`:1.75`) or a
/// well-known name (`org.kde.StatusNotifierItem-1234-1`). For unique names
/// the owner is the whole id up to the first `/`; for well-known names we
/// take the name before the dash-instance suffix so multiple items from the
/// same application collapse together. This is what the dedupe keys on.
pub fn bus_name(id: &str) -> &str {
    let no_path = id.split('/').next().unwrap_or(id);
    if let Some(idx) = no_path.find('-') {
        // `:1.75` has no dash → stays whole; `org.kde.StatusNotifierItem-1234-1`
        // → `org.kde.StatusNotifierItem`. Unique `:N.M` names are never split.
        if no_path.starts_with(':') {
            no_path
        } else {
            &no_path[..idx]
        }
    } else {
        no_path
    }
}

/// An item is worth rendering only if the user can recognise/click it:
/// it needs either an icon (name or pixmap) or a non-empty title. Vivaldi
/// (Chromium) registers anonymous `StatusNotifierItem`s with neither — they
/// show as a blank glyph and are meaningless, so we drop them here. The
/// service keeps the full bus truth untouched (needed for menus/debugging).
pub fn is_useful(item: &TrayItem) -> bool {
    let has_icon =
        item.icon_name.as_ref().is_some_and(|n| !n.is_empty()) || item.icon_pixmap.is_some();
    let has_title = item.title.as_ref().is_some_and(|t| !t.is_empty());
    has_icon || has_title
}

/// Collapse several items from one bus owner into a single icon.
///
/// Order is preserved (newest last, per `TrayState`). For each bus owner the
/// first *useful* item wins; if none of an owner's items are useful they are
/// all dropped by the caller's filter anyway.
pub fn dedupe_by_bus<'a>(items: &[&'a TrayItem]) -> Vec<&'a TrayItem> {
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for item in items {
        if seen.insert(bus_name(&item.id)) {
            out.push(*item);
        }
    }
    out
}

/// Keep at most `max` items; return the kept list and the number dropped.
pub fn apply_cap<'a>(items: &[&'a TrayItem], max: usize) -> (Vec<&'a TrayItem>, usize) {
    if items.len() <= max {
        (items.to_vec(), 0)
    } else {
        let overflow = items.len() - max;
        (items[..max].to_vec(), overflow)
    }
}

/// Full pipeline: filter anonymous items → dedupe by bus owner → cap.
/// Pure and side-effect-free (render() may run many times per frame).
pub fn prepare_tray_items<'a>(state: &'a TrayState, max: usize) -> PreparedTray<'a> {
    let useful: Vec<&TrayItem> = state.items.iter().filter(|i| is_useful(i)).collect();
    let deduped = dedupe_by_bus(&useful);
    let (visible, overflow) = apply_cap(&deduped, max);
    PreparedTray { visible, overflow }
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

    // 2. icon_pixmap → cached GPUI RenderImage (RGBA→BGRA for GPU).
    if let Some(pm) = item.icon_pixmap.as_ref() {
        if let Some(rendered) = cached_pixmap_render_image(&item.id, pm) {
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

/// Pixmap `RenderImage` cache keyed by `item.id`. Invalidation by
/// `(data_len, width, height)` — avoids rebuilding RenderImage on every
/// render tick (the bar redraws every second via the clock ticker).
thread_local! {
    static PIXMAP_CACHE: std::cell::RefCell<HashMap<String, (usize, u32, u32, Arc<RenderImage>)>> =
        std::cell::RefCell::new(HashMap::new());
}

fn cached_pixmap_render_image(item_id: &str, pm: &TrayPixmap) -> Option<Arc<RenderImage>> {
    let meta = (pm.data.len(), pm.width, pm.height);
    if let Some((old_len, old_w, old_h, cached)) =
        PIXMAP_CACHE.with(|c| c.borrow().get(item_id).cloned())
    {
        if (old_len, old_w, old_h) == meta {
            return Some(cached);
        }
    }
    let rendered = pixmap_render_image(pm)?;
    PIXMAP_CACHE.with(|c| {
        c.borrow_mut().insert(
            item_id.to_string(),
            (meta.0, meta.1, meta.2, rendered.clone()),
        );
    });
    Some(rendered)
}

// ── icon-theme resolution ──────────────────────────────────────────────
// Resolution + cache live in the shared `crate::icon_resolution` module
// (merged here in T263 so tray and context menus share one cache).
pub use crate::icon_resolution::cached_resolve_icon;

pub fn register(cx: &mut App) {
    use chronos_luau::bar::BarWidgetRegistry;
    cx.global_mut::<BarWidgetRegistry>()
        .register(Box::new(TrayWidget::new()));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bogus_icon_name_resolves_to_none() {
        assert!(cached_resolve_icon("chronos-totally-bogus-icon-xyz-9999").is_none());
    }

    #[test]
    fn pixmap_render_image_swaps_rgba_to_bgra() {
        let pm = TrayPixmap {
            width: 1,
            height: 1,
            data: vec![0x10, 0x20, 0x30, 0xFF],
        };
        let img = pixmap_render_image(&pm).expect("render image builds");
        assert_eq!(img.frame_count(), 1);
        let bytes = img.as_bytes(0).expect("frame bytes present");
        assert_eq!(bytes, &[0x30, 0x20, 0x10, 0xFF]);
    }

    #[test]
    fn pixmap_render_image_bad_length_yields_none() {
        let pm = TrayPixmap {
            width: 2,
            height: 2,
            data: vec![0; 4],
        };
        assert!(pixmap_render_image(&pm).is_none());
    }

    // ── tray clutter defence (task №16) ───────────────────────────────

    fn mk_item(id: &str, title: Option<&str>, icon: Option<&str>) -> TrayItem {
        TrayItem {
            id: id.to_string(),
            title: title.map(|s| s.to_string()),
            icon_name: icon.map(|s| s.to_string()),
            icon_pixmap: None,
            label: "?".to_string(),
            menu_path: None,
            menu: None,
        }
    }

    #[test]
    fn bus_name_splits_path_and_wellknown() {
        assert_eq!(bus_name(":1.75"), ":1.75");
        assert_eq!(
            bus_name(":1.75/org/chromium/StatusNotifierItem/15"),
            ":1.75"
        );
        assert_eq!(
            bus_name("org.kde.StatusNotifierItem-1234-1"),
            "org.kde.StatusNotifierItem"
        );
        assert_eq!(
            bus_name("org.kde.StatusNotifierItem-1234-1/Menu"),
            "org.kde.StatusNotifierItem"
        );
    }

    #[test]
    fn anonymous_item_is_filtered_out() {
        let junk = mk_item(":1.75/org/chromium/StatusNotifierItem/15", Some(""), None);
        assert!(!is_useful(&junk));
        let with_title = mk_item(":1.70", Some("Wireless"), None);
        assert!(is_useful(&with_title));
        let with_icon = mk_item(":1.71", Some(""), Some("network-wireless"));
        assert!(is_useful(&with_icon));
    }

    #[test]
    fn dedupe_collapses_same_bus_owner() {
        let items = vec![
            mk_item(":1.75/org/chromium/StatusNotifierItem/15", None, None),
            mk_item(":1.75/org/chromium/StatusNotifierItem/16", None, None),
            mk_item(":1.75/org/chromium/StatusNotifierItem/17", None, None),
        ];
        let refs: Vec<&TrayItem> = items.iter().collect();
        let deduped = dedupe_by_bus(&refs);
        assert_eq!(deduped.len(), 1);
        assert_eq!(deduped[0].id, ":1.75/org/chromium/StatusNotifierItem/15");
    }

    #[test]
    fn cap_limits_to_max_with_overflow() {
        let items: Vec<TrayItem> = (1..=12)
            .map(|i| mk_item(&format!(":1.{i}"), Some("App"), Some("icon")))
            .collect();
        let refs: Vec<&TrayItem> = items.iter().collect();
        let (kept, overflow) = apply_cap(&refs, 8);
        assert_eq!(kept.len(), 8);
        assert_eq!(overflow, 4);
    }

    #[test]
    fn prepare_pipeline_filter_dedupe_cap() {
        let mut state = TrayState::default();
        for n in 1..=13 {
            state.items.push(mk_item(
                &format!(":1.75/org/chromium/StatusNotifierItem/{n}"),
                Some(""),
                None,
            ));
        }
        state.items.push(mk_item(
            ":1.50/org/ayatana/NotificationItem/udiskie",
            Some("udiskie"),
            None,
        ));
        state
            .items
            .push(mk_item(":1.60", Some("Wireless"), Some("network-wireless")));

        let prepared = prepare_tray_items(&state, 8);
        assert_eq!(prepared.visible.len(), 2);
        assert_eq!(prepared.overflow, 0);
        assert!(
            prepared
                .visible
                .iter()
                .any(|i| i.title.as_deref() == Some("udiskie"))
        );
        assert!(
            prepared
                .visible
                .iter()
                .any(|i| i.title.as_deref() == Some("Wireless"))
        );
    }
}
