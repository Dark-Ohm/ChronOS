//! Vertical icon-rail — switches the active tab of the IDE panel.
//!
//! One `on_hover`-free button per tab in the **resolved mode set** (not the
//! full catalog); active tab gets a pill background (`accent.primary` with
//! theme-dependent alpha). Design brief: `design.md` §"Shell-IDE правая
//! панель (таб-контейнер)". T315: 3px accent strip removed (tab-strip
//! idiom, contradicts rounded aperture corners).
//!
//! T219: The rail is split into two groups — **top** (above the spacer) and
//! **bottom** (between the spacer and the dock toggle). In edit mode, each
//! icon gets ▲/▼ move arrows and an `accent.primary.opacity(0.45)` frame,
//! mirroring the bar's edit chrome.

use gpui::{App, Bounds, Hsla, IntoElement, SharedString, Window, canvas, div, prelude::*, px, svg};

use chronos_ui::Theme;

use crate::side_panel_right::surfaces;
use crate::side_panel_right::tabs::PanelTab;
use crate::workspace_mode;

use std::cell::Cell;
use std::rc::Rc;

// T204: single source of truth for rail width lives in `side_panel_right::mod`
// (`RAIL_WIDTH = 36`). Re-export so this module can never drift from it.
pub(crate) use super::RAIL_WIDTH;
const BUTTON_SIZE: f32 = 28.;

/// T315: active tab gets a pill background (`accent.primary` with
/// theme-dependent alpha), mirrored from the left rail.
pub fn rail_button_bg(is_active: bool, theme: &Theme) -> Hsla {
    if is_active {
        let alpha = if theme.is_light { 0.12 } else { 0.15 };
        theme.accent.primary.alpha(alpha)
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
    on_select: Rc<dyn Fn(PanelTab, Bounds<gpui::Pixels>, &mut Window, &mut App) + 'static>,
    on_move: Rc<dyn Fn(PanelTab, isize, &mut App) + 'static>,
    theme: &Theme,
) -> impl IntoElement {
    // T305: the icon's LIVE laid-out bounds (window-local) are captured on
    // paint and handed to the click handler — the control-center popup anchors
    // to them (never a cached constant, never `window.bounds()`).
    let bounds_cell: Rc<Cell<Bounds<gpui::Pixels>>> = Rc::new(Cell::new(Bounds::default()));
    let icon = div()
        .id(("rail-tab", tab as usize))
        .relative()
        .flex()
        .items_center()
        .justify_center()
        .size(px(BUTTON_SIZE))
        .rounded(theme.radius)
        .bg(rail_button_bg(is_active, theme))
        .child({
            let cell_for_canvas = bounds_cell.clone();
            canvas(
                |bounds, _window, _cx| bounds,
                move |_bounds, captured, _window, _cx| cell_for_canvas.set(captured),
            )
            .absolute()
            .inset_0()
        })
        .on_click({
            let on_select = on_select.clone();
            let bounds_cell = bounds_cell.clone();
            move |_, window, cx| on_select(tab, bounds_cell.get(), window, cx)
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
        );

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
    on_select: &Rc<dyn Fn(PanelTab, Bounds<gpui::Pixels>, &mut Window, &mut App) + 'static>,
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
    on_select: Rc<dyn Fn(PanelTab, Bounds<gpui::Pixels>, &mut Window, &mut App) + 'static>,
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
        // T266: the rail's own plate follows surface alpha.
        .bg(theme.surface_color(surfaces::chrome(theme)))
        // T318 эррата: рельс НЕ скругляется — зеркально левому. Скругляется
        // дыра, а не кромка: в углу апертуры хрома должно становиться больше,
        // а `rounded_tl/bl` на рельсе срезали материал и открывали обои.
        // Внутренний контур рисует матте (`frame.rs`).
        // T315: border removed — same reason as left rail. The rail is the
        // frame edge, not a separate panel.
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
        // T292: workspace-mode button — sits above the dock toggle, is NOT a
        // PanelTab (mode is not a reorderable tab). Click toggles
        // Developer ⇄ Gamer; while a switch prompt is pending the same click
        // opens the inline confirm row instead of toggling.
        .child(render_mode_button(cx, theme))
        .when(workspace_mode::pending(cx).is_some(), |rail| {
            rail.child(render_mode_prompt(theme))
        })
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

/// T292: workspace-mode button — above the dock toggle, not a `PanelTab`.
///
/// Shows the current mode's icon (gamepad / mode-daily) with a faint accent
/// tint that is deliberately distinct from the active-tab accent bar so the
/// two are never confused. Click toggles Developer ⇄ Gamer; while a switch
/// prompt is pending the click opens the inline confirm row instead.
fn render_mode_button(cx: &App, theme: &Theme) -> impl IntoElement {
    let mode = workspace_mode::current(cx);
    let has_pending = workspace_mode::pending(cx).is_some();

    let on_click = move |_event: &gpui::ClickEvent, _window: &mut Window, cx: &mut App| {
        if workspace_mode::pending(cx).is_some() {
            // Pending prompt owns the click — the inline row below handles
            // Да/Нет/Не спрашивать. Toggling here would dismiss the prompt
            // silently, so we no-op and let the row decide.
            return;
        }
        workspace_mode::toggle(cx);
    };

    div()
        .id("rail-workspace-mode")
        .flex()
        .items_center()
        .justify_center()
        .size(px(BUTTON_SIZE))
        .rounded(theme.radius)
        // Faint accent tint marks "this is the mode switch", not a tab.
        .bg(if has_pending {
            theme.accent.primary.opacity(0.22)
        } else {
            theme.accent.primary.opacity(0.10)
        })
        .cursor_pointer()
        .hover(|s| s.bg(theme.accent.primary.opacity(0.18)))
        .on_click(on_click)
        .child(
            svg()
                .path(mode.icon_path())
                .size(px(18.))
                .text_color(theme.accent.primary),
        )
}

/// T292: inline confirm row shown only while `workspace_mode::pending` is set
/// (the old bar banner moves here with the widget). Three honest actions, no
/// second surface — fits inside the 36px rail as a vertical stack.
fn render_mode_prompt(theme: &Theme) -> impl IntoElement {
    div()
        .id("rail-workspace-mode-prompt")
        .flex()
        .flex_col()
        .items_stretch()
        .gap(px(2.))
        .w(px(BUTTON_SIZE))
        .child(prompt_item("Да", theme, |cx: &mut App| {
            workspace_mode::accept_prompt(cx)
        }))
        .child(prompt_item("Нет", theme, |cx: &mut App| {
            workspace_mode::dismiss_prompt(cx, false)
        }))
        .child(prompt_item("Не спраш.", theme, |cx: &mut App| {
            workspace_mode::dismiss_prompt(cx, true)
        }))
}

fn prompt_item(
    label: &'static str,
    theme: &Theme,
    on_click: impl Fn(&mut App) + 'static,
) -> impl IntoElement {
    div()
        .id(SharedString::from(format!("rail-mode-prompt-{label}")))
        .w_full()
        .py(px(3.))
        .rounded(px(3.))
        .flex()
        .items_center()
        .justify_center()
        .text_size(px(9.))
        .text_color(theme.text.muted)
        .cursor_pointer()
        .hover(|s| s.bg(theme.interactive.hover).text_color(theme.text.primary))
        .on_click(move |_event, _window, cx: &mut App| on_click(cx))
        .child(label)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chronos_ui::Theme;

    use crate::workspace_mode::WorkspaceMode;

    #[test]
    fn active_tab_uses_pill_fill_inactive_is_transparent() {
        // T315: active tab gets accent.primary with theme-dependent alpha,
        // not interactive.hover.
        let theme = Theme::default();
        let alpha = if theme.is_light { 0.12 } else { 0.15 };
        assert_eq!(rail_button_bg(true, &theme), theme.accent.primary.alpha(alpha));
        assert_eq!(rail_button_bg(false, &theme), gpui::transparent_black());
    }

    #[test]
    fn mode_button_is_not_a_panel_tab() {
        // T292: the workspace-mode control is a rail button, not a reorderable
        // `PanelTab` — it must never appear in the tab catalog.
        assert!(
            PanelTab::ALL.iter().all(|t| t.id() != "workspace_mode"),
            "workspace_mode must not leak into PanelTab::ALL"
        );
    }

    #[test]
    fn mode_button_click_toggles_mode() {
        // T292: the rail mode button calls `workspace_mode::toggle`, whose
        // pure behaviour is Developer ⇄ Gamer (pending prompt is a no-op for
        // the toggle, handled by the inline row). Mirrors `WorkspaceMode::other`.
        assert_eq!(WorkspaceMode::Developer.other(), WorkspaceMode::Gamer);
        assert_eq!(WorkspaceMode::Gamer.other(), WorkspaceMode::Developer);
    }
}
