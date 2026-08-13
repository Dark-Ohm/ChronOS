//! Vertical icon-rail — switches the active tab of the IDE panel.
//!
//! One `on_hover`-free button per tab in the **resolved mode set** (not the
//! full catalog); active tab gets an `accent.primary` bar on its left edge +
//! `interactive.hover` fill. Design brief: `design.md` §"Shell-IDE правая
//! панель (таб-контейнер)".
//!
//! T219: The rail is split into two groups — **top** (above the spacer) and
//! **bottom** (between the spacer and the dock toggle). In edit mode, each
//! icon gets ▲/▼ move arrows and an `accent.primary.opacity(0.45)` frame,
//! mirroring the bar's edit chrome.

use gpui::{App, Hsla, IntoElement, Window, div, prelude::*, px, svg};

use chronos_ui::Theme;

use crate::side_panel_right::surfaces;
use crate::side_panel_right::tabs::PanelTab;

use std::rc::Rc;

// T204: single source of truth for rail width lives in `side_panel_right::mod`
// (`RAIL_WIDTH = 36`). Re-export so this module can never drift from it.
pub(crate) use super::RAIL_WIDTH;
const BUTTON_SIZE: f32 = 28.;

pub fn rail_button_bg(is_active: bool, theme: &Theme) -> Hsla {
    if is_active {
        theme.interactive.hover
    } else {
        gpui::transparent_black()
    }
}

/// Render a single rail icon button. In edit mode, wraps it with ▲/▼ arrows
/// and an accent frame (mirroring bar's `render_widget_slot`).
fn render_rail_button(
    tab: PanelTab,
    is_active: bool,
    editing: bool,
    on_select: Rc<dyn Fn(PanelTab, &mut Window, &mut App) + 'static>,
    on_move: Rc<dyn Fn(PanelTab, isize, &mut App) + 'static>,
    theme: &Theme,
) -> impl IntoElement {
    let icon = div()
        .id(("rail-tab", tab as usize))
        .relative()
        .flex()
        .items_center()
        .justify_center()
        .size(px(BUTTON_SIZE))
        .rounded(theme.radius)
        .bg(rail_button_bg(is_active, theme))
        .on_click({
            let on_select = on_select.clone();
            move |_, window, cx| on_select(tab, window, cx)
        })
        .child(
            svg()
                .path(tab.icon_path())
                .size(px(18.))
                .text_color(if is_active {
                    theme.text.primary
                } else {
                    theme.text.muted
                }),
        )
        .when(is_active, |el| {
            // Active indicator bar — flush against the rail's screen-ward edge.
            el.child(
                div()
                    .absolute()
                    .left(px(-4.))
                    .top(px(BUTTON_SIZE / 2. - 10.))
                    .w(px(3.))
                    .h(px(20.))
                    .rounded(px(2.))
                    .bg(theme.accent.primary),
            )
        });

    if !editing {
        return icon.into_any_element();
    }

    // Edit mode: accent frame + ▲/▼ arrows.
    let arrow_up_id = format!("rail-edit-up-{tab:?}");
    let arrow_down_id = format!("rail-edit-down-{tab:?}");
    let on_move_up = on_move.clone();
    let on_move_down = on_move.clone();

    div()
        .flex()
        .flex_col()
        .items_center()
        .gap(px(0.))
        .child(
            // ▲ arrow (move up = delta -1 in rail coordinates).
            div()
                .id(arrow_up_id)
                .flex_none()
                .w(px(BUTTON_SIZE))
                .h(px(10.))
                .flex()
                .items_center()
                .justify_center()
                .cursor_pointer()
                .text_size(px(8.))
                .text_color(theme.text.secondary)
                .hover(|s| s.bg(theme.border.subtle).text_color(theme.text.primary))
                .on_click(move |_, _, cx| on_move_up(tab, -1, cx))
                .child("▲"),
        )
        .child(
            div()
                .flex()
                .items_center()
                .justify_center()
                .rounded(theme.radius)
                .border_1()
                .border_color(theme.accent.primary.opacity(0.45))
                .child(icon),
        )
        .child(
            // ▼ arrow (move down = delta +1 in rail coordinates).
            div()
                .id(arrow_down_id)
                .flex_none()
                .w(px(BUTTON_SIZE))
                .h(px(10.))
                .flex()
                .items_center()
                .justify_center()
                .cursor_pointer()
                .text_size(px(8.))
                .text_color(theme.text.secondary)
                .hover(|s| s.bg(theme.border.subtle).text_color(theme.text.primary))
                .on_click(move |_, _, cx| on_move_down(tab, 1, cx))
                .child("▼"),
        )
        .into_any_element()
}

/// Render a group of tabs vertically.
fn render_group(
    tabs: &[PanelTab],
    active: PanelTab,
    editing: bool,
    on_select: &Rc<dyn Fn(PanelTab, &mut Window, &mut App) + 'static>,
    on_move: &Rc<dyn Fn(PanelTab, isize, &mut App) + 'static>,
    theme: &Theme,
) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .items_center()
        .gap(px(4.))
        .children(tabs.iter().map(move |&tab| {
            render_rail_button(
                tab,
                tab == active,
                editing,
                on_select.clone(),
                on_move.clone(),
                theme,
            )
        }))
}

pub fn render_rail(
    cx: &App,
    top_tabs: &[PanelTab],
    bottom_tabs: &[PanelTab],
    active: PanelTab,
    on_select: Rc<dyn Fn(PanelTab, &mut Window, &mut App) + 'static>,
    dock_content: bool,
    on_dock_toggle: Rc<dyn Fn(&mut Window, &mut App) + 'static>,
    editing: bool,
    on_move: Rc<dyn Fn(PanelTab, isize, &mut App) + 'static>,
) -> impl IntoElement {
    let theme = Theme::global(cx);
    // Clone Rc's before the closures capture them — each iterator
    // needs its own reference.
    let on_select_top = on_select.clone();
    let on_select_bot = on_select.clone();
    let on_move_top = on_move.clone();
    let on_move_bot = on_move.clone();

    div()
        .id("side-panel-right-rail")
        .flex()
        .flex_col()
        .items_center()
        .gap(px(4.))
        .py(px(8.))
        .w(px(RAIL_WIDTH))
        .h_full()
        .bg(surfaces::chrome(theme))
        // T267 errata (2026-08-13): the border is UNCONDITIONAL. Open, it is
        // the divider between content and rail; collapsed, it is the panel's
        // only outer edge — the body div drops its own border together with
        // its background when `content_open` is false, so gating this one on
        // the same flag left rail-only mode with no separator at all.
        // Token is `border.subtle`, same as bar and left panel (T267).
        .border_l_1()
        .border_color(theme.border.subtle)
        // Top group
        .children(top_tabs.iter().map(move |&tab| {
            render_rail_button(
                tab,
                tab == active,
                editing,
                on_select_top.clone(),
                on_move_top.clone(),
                theme,
            )
        }))
        // Spacer — pushes bottom group down.
        .child(div().flex_1())
        // Bottom group
        .children(bottom_tabs.iter().map(move |&tab| {
            render_rail_button(
                tab,
                tab == active,
                editing,
                on_select_bot.clone(),
                on_move_bot.clone(),
                theme,
            )
        }))
        // Dock toggle — always last, below bottom group.
        .child({
            let docked = dock_content;
            let on_dock_toggle = on_dock_toggle.clone();
            div()
                .id("dock-toggle-right")
                .w(px(BUTTON_SIZE))
                .h(px(BUTTON_SIZE))
                .rounded(px(6.))
                .flex()
                .items_center()
                .justify_center()
                .text_size(px(11.))
                .text_color(if docked {
                    theme.accent.primary
                } else {
                    theme.text.muted
                })
                .cursor_pointer()
                .hover(|s| s.bg(theme.border.subtle))
                .on_click(move |_, window, cx| on_dock_toggle(window, cx))
                .child(if docked { "⊞" } else { "⊟" })
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chronos_ui::Theme;

    #[test]
    fn active_tab_uses_interactive_hover_fill_inactive_is_transparent() {
        let theme = Theme::default();
        assert_eq!(rail_button_bg(true, &theme), theme.interactive.hover);
        assert_eq!(rail_button_bg(false, &theme), gpui::transparent_black());
    }
}
