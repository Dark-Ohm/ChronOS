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

use chronos_ui::Theme;

/// Padding applied to each menu row (px).
const ROW_PAD_Y: f32 = 6.;
const ROW_PAD_X: f32 = 12.;
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

        let bg = theme.bg.elevated;
        let text_primary = theme.text.primary;
        let text_muted = theme.text.muted;
        let divider = theme.bg.secondary;
        let radius = theme.radius;
        let radius_lg = theme.radius_lg;

        let Some(service) = service else {
            // No menu open — empty surface.
            return div().into_any_element();
        };

        if nodes.is_empty() {
            // Menu requested but not yet fetched (or empty). Show a tiny
            // placeholder so the surface isn't a zero-size transparent void.
            return div()
                .flex_col()
                .rounded(radius_lg)
                .bg(bg)
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
                    0,
                )
            })
            .collect();

        div()
            .flex_col()
            .rounded(radius_lg)
            .bg(bg)
            .overflow_hidden()
            .children(rows)
            .into_any_element()
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

    // Toggle prefix.
    let prefix = match &node.toggle {
        Some((kind, checked)) => {
            let mark = match kind {
                chronos_services::MenuToggleType::Radio => {
                    if *checked {
                        "◉ "
                    } else {
                        "○ "
                    }
                }
                chronos_services::MenuToggleType::Checkmark => {
                    if *checked {
                        "✓ "
                    } else {
                        "☐ "
                    }
                }
            };
            mark.to_string()
        }
        None => String::new(),
    };

    let text_color = if node.enabled {
        *text_primary
    } else {
        *text_muted
    };

    let has_children = !node.children.is_empty();

    // Build the row. Applying `on_click` flips `Div` into `Stateful<Div>`, so
    // we keep the builder as a trait object (`impl IntoElement`) only at the
    // point we stop chaining — here we build the leaf row then fold to
    // `AnyElement` exactly once.
    // Build the row. Applying `on_click` flips `Div` into `Stateful<Div>`, so
    // we compute the whole row as a single `AnyElement` (with or without the
    // click handler) rather than trying to reassign one `Div`.
    let row_elem: AnyElement = if node.enabled && !has_children {
        let id = node.id;
        div()
            .w_full()
            .flex()
            .items_center()
            .px(px(ROW_PAD_X))
            .py(px(ROW_PAD_Y))
            .rounded(radius)
            .ml(indent)
            .cursor_pointer()
            .id(format!("tray-menu-item-{id}"))
            .on_click(move |_event, window, cx: &mut App| {
                click_item(window, cx, id);
            })
            .child(
                div()
                    .text_color(text_color)
                    .child(format!("{prefix}{label}")),
            )
            .into_any_element()
    } else {
        div()
            .w_full()
            .flex()
            .items_center()
            .px(px(ROW_PAD_X))
            .py(px(ROW_PAD_Y))
            .rounded(radius)
            .ml(indent)
            .child(
                div()
                    .text_color(text_color)
                    .child(format!("{prefix}{label}")),
            )
            .into_any_element()
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
                    depth + 1,
                )
            })
            .collect();
        div().w_full().children(child_rows).into_any_element()
    } else {
        row_elem
    }
}
