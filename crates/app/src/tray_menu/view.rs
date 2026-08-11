//! Tray menu popup view — renders the live `MenuNode` tree from the
//! `TrayMenuState` global.
//!
//! Rendering rules (per the brief):
//!   * leaf item          → label (muted color if `!enabled`)
//!   * separator          → thin divider (label/children ignored)
//!   * toggle item        → `✓` / `○` prefix (Checkmark/Radio)
//!   * submenu (children) → unfolded inline with indentation (no nested
//!                          windows in MVP)
//!   * empty label        → rendered as `…` (the known OpenCode service bug
//!                          where child labels arrive empty — pending fix)

use gpui::{
    AnyElement, App, Context, Div, InteractiveElement, Render, Window, div, prelude::*, px,
};

use chronos_services::MenuNode;

use crate::state::AppState;
use crate::tray_menu::TrayMenuState;
use crate::tray_menu::{click_item, close};

use chronos_ui::{
    Theme, WindowRootExt, elevation_apply_light_chrome, elevation_blur_layer,
};

/// Padding applied to each menu row (px).
const ROW_PAD_Y: f32 = 6.;
/// Row horizontal padding — design `.ci { padding: 0 10px }`.
const ROW_PAD_X: f32 = 10.;
/// Indentation per submenu nesting level (px).
const SUBMENU_INDENT: f32 = 16.;

/// Build a fresh, empty menu view.
impl TrayMenuView {
    pub fn new(_cx: &mut App) -> Self {
        Self {}
    }
}

pub struct TrayMenuView {}

impl Render for TrayMenuView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let service = cx.global::<TrayMenuState>().open_service.clone();
        let nodes = cx.global::<TrayMenuState>().nodes.clone();

        let theme = Theme::global(cx);

        let bg = theme.bg.primary;
        let text_primary = theme.text.primary;
        let text_muted = theme.text.muted;
        let divider = theme.bg.secondary;
        let radius = theme.radius;
        let radius_lg = theme.radius_lg;
        let hover = theme.interactive.hover;
        let accent = theme.accent.primary;
        let border_subtle = theme.border.subtle;

        // Elevated-surface shell (T128 depth language): blur + drop shadow +
        // Light-C glow/watermark — the shared "one popup component" recipe that
        // ties the tray menu and the dock context menu together.
        let elev = theme.elevation_popup();
        let blur_layer = elevation_blur_layer(&elev, radius_lg);

        let Some(service) = service else {
            // No menu open — empty surface.
            return div().into_any_element();
        };

        if nodes.is_empty() {
            // Menu requested but not yet fetched (or empty). Show a tiny
            // placeholder so the surface isn't a zero-size transparent void.
            let mut card = div()
                .window_font(theme)
                .relative()
                .flex_col()
                .rounded(radius_lg)
                .bg(bg.alpha(0.94))
                .border_1()
                .border_color(border_subtle)
                .shadow(elev.shadows.to_vec())
                .overflow_hidden();
            card = elevation_apply_light_chrome(&elev, card);
            return card
                .p(px(ROW_PAD_X))
                .text_color(text_muted)
                .child("…".to_string())
                .into_any_element();
        }

        let rows: Vec<AnyElement> = nodes
            .iter()
            .filter(|n| n.visible)
            .map(|node| {
                render_node(
                    node,
                    service.clone(),
                    &bg,
                    &text_primary,
                    &text_muted,
                    &divider,
                    radius,
                    hover,
                    accent,
                    0,
                )
            })
            .collect();

        // Menu root: elevated card shell (blur + shadow + Light-C chrome) with an
        // inner bounded scroll column. Design caps the menu at `viewport − 16px`
        // and scrolls rows past it; `mod.rs` already caps the window height, this
        // `.flex_1().min_h(0).overflow_y_scroll()` column takes that bounded
        // height and scrolls on overflow. `p(6)` + row `px(10)` ≈ design `.ctx-menu`
        // 6px padding + `.ci` `0 10px` content inset.
        let mut card = div()
            .window_font(theme)
            .relative()
            .flex_col()
            .rounded(radius_lg)
            .bg(bg.alpha(0.94))
            .border_1()
            .border_color(border_subtle)
            .shadow(elev.shadows.to_vec())
            .overflow_hidden();
        card = elevation_apply_light_chrome(&elev, card);
        card = card
            .child(blur_layer)
            .child(
                div()
                    .id("tray-menu-list")
                    .flex_1()
                    .min_h(px(0.))
                    .overflow_y_scroll()
                    .p(px(6.))
                    .children(rows),
            );
        card.into_any_element()
    }
}

/// Render a single `MenuNode` (and, if it is a submenu, its children inline).
#[allow(clippy::too_many_arguments)]
fn render_node(
    node: &MenuNode,
    service: String,
    bg: &gpui::Hsla,
    text_primary: &gpui::Hsla,
    text_muted: &gpui::Hsla,
    divider: &gpui::Hsla,
    radius: gpui::Pixels,
    hover: gpui::Hsla,
    accent: gpui::Hsla,
    depth: u32,
) -> AnyElement {
    let indent = px(SUBMENU_INDENT * depth as f32);

    if node.separator {
        return div()
            .w_full()
            .h(px(1.))
            .my(px(4.))
            .ml(indent)
            .bg(*divider)
            .into_any_element();
    }

    let label = if node.label.is_empty() {
        "…".to_string()
    } else {
        node.label.clone()
    };

    // Toggle glyph. Design renders the check in the accent (`.ci-check`) and
    // only when checked; Radio keeps a persistent ○/◉ so the group state is
    // always visible. Rendered in a fixed 16px gutter below so all rows share
    // one left edge (design `.ci-check`/`.ci-ic` width).
    let (mark, mark_color) = match &node.toggle {
        Some((kind, checked)) => match kind {
            chronos_services::MenuToggleType::Radio => {
                if *checked {
                    (Some("◉".to_string()), accent)
                } else {
                    (Some("○".to_string()), *text_muted)
                }
            }
            chronos_services::MenuToggleType::Checkmark => {
                if *checked {
                    (Some("✓".to_string()), accent)
                } else {
                    (None, *text_muted)
                }
            }
        },
        None => (None, *text_muted),
    };

    let text_color = if node.enabled {
        *text_primary
    } else {
        *text_muted
    };

    let has_children = !node.children.is_empty();

    // Row chrome: 16px mark gutter + flex-1 label with ellipsis. `hover`
    // stays a no-op for disabled rows (design `.ci.disabled` has no wash),
    // and clicking only arms for enabled leaf items.
    let mut row = div()
        .w_full()
        .flex()
        .items_center()
        .gap(px(10.))
        .px(px(ROW_PAD_X))
        .py(px(ROW_PAD_Y))
        .rounded(radius)
        .ml(indent)
        .child(
            div()
                .w(px(16.))
                .flex()
                .items_center()
                .children(mark.iter().map(|m| div().text_color(mark_color).child(m.clone()))),
        )
        .child(
            div()
                .flex_1()
                .min_w(px(0.))
                .whitespace_nowrap()
                .overflow_hidden()
                .text_color(text_color)
                .child(label),
        );

    if node.enabled {
        row = row.hover(|s| s.bg(hover));
    }

    // Applying `on_click` folds `Div` into `Stateful<Div>`; we drop to
    // `AnyElement` exactly once at the end.
    let row_elem: AnyElement = if node.enabled && !has_children {
        let id = node.id;
        row.cursor_pointer()
            .id(format!("tray-menu-item-{id}"))
            .on_click(move |_event, window, cx: &mut App| {
                click_item(window, cx, id);
            })
            .into_any_element()
    } else {
        row.into_any_element()
    };

    // Inline submenu expansion (no nested windows in MVP).
    if has_children {
        let child_rows: Vec<AnyElement> = node
            .children
            .iter()
            .filter(|n| n.visible)
            .map(|child| {
                render_node(
                    child,
                    service.clone(),
                    bg,
                    text_primary,
                    text_muted,
                    divider,
                    radius,
                    hover,
                    accent,
                    depth + 1,
                )
            })
            .collect();
        div().w_full().children(child_rows).into_any_element()
    } else {
        row_elem
    }
}
