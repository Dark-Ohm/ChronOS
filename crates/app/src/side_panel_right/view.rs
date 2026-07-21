//! Right side panel view — window shell only in this task. MPRIS card
//! (task 9), system/network spectrum meters (task 10), and power row
//! (task 11) are added as children of `render()` in later tasks.

use gpui::{Context, IntoElement, Render, Window, div, prelude::*, px};

use chronos_ui::Theme;

pub struct SidePanelRightView {}

impl SidePanelRightView {
    pub fn new(_cx: &mut Context<Self>) -> Self {
        Self {}
    }
}

impl Render for SidePanelRightView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = Theme::global(cx);
        div()
            .size_full()
            .bg(theme.bg.secondary)
            .border_l_1()
            .border_color(theme.border.default)
            .p(px(20.))
            .text_color(theme.text.primary)
            .child("side panel: skeleton, tasks 9-11 fill this in")
    }
}
