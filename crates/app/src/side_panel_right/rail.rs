//! Vertical icon-rail — switches the active tab of the IDE panel.
//!
//! One `on_hover`-free button per `PanelTab::ALL`; active tab gets an
//! `accent.primary` bar on its left edge + `interactive.hover` fill.
//! Design brief: `design.md` §"Shell-IDE правая панель (таб-контейнер)".

use gpui::{App, Context, Hsla, IntoElement, Window, div, prelude::*, px, svg};

use chronos_ui::Theme;

use crate::side_panel_right::surfaces;
use crate::side_panel_right::tabs::PanelTab;

use std::rc::Rc;

pub(crate) const RAIL_WIDTH: f32 = 44.;
const BUTTON_SIZE: f32 = 36.;

pub fn rail_button_bg(is_active: bool, theme: &Theme) -> Hsla {
    if is_active {
        theme.interactive.hover
    } else {
        gpui::transparent_black()
    }
}

pub fn render_rail(
    cx: &App,
    active: PanelTab,
    on_select: Rc<dyn Fn(PanelTab, &mut Window, &mut App) + 'static>,
    dock_content: bool,
    on_dock_toggle: Rc<dyn Fn(&mut Window, &mut App) + 'static>,
) -> impl IntoElement {
    let theme = Theme::global(cx);
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
        .border_l_1()
        .border_color(theme.border.default)
        .children(PanelTab::ALL.into_iter().map(|tab| {
            let is_active = tab == active;
            let on_select = on_select.clone();
            div()
                .id(("rail-tab", tab as usize))
                .relative()
                .flex()
                .items_center()
                .justify_center()
                .size(px(BUTTON_SIZE))
                .rounded(theme.radius)
                .bg(rail_button_bg(is_active, theme))
                .on_click(move |_, window, cx| on_select(tab, window, cx))
                .child(
                    svg()
                        .path(tab.icon_path())
                        .size(px(20.))
                        .text_color(if is_active {
                            theme.text.primary
                        } else {
                            theme.text.muted
                        }),
                )
                .when(is_active, |el| {
                    el.child(
                        div()
                            .absolute()
                            .left(px(-8.))
                            .top(px(BUTTON_SIZE / 2. - 10.))
                            .w(px(3.))
                            .h(px(20.))
                            .rounded(px(2.))
                            .bg(theme.accent.primary),
                    )
                })
        }))
        .child(div().flex_1()) // spacer
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
