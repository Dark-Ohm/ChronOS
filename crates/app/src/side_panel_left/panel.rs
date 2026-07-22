use gpui::*;

use super::SidePanelLeft;

pub fn render_panel(
    _panel: &SidePanelLeft,
    _window: &mut Window,
    _cx: &mut Context<SidePanelLeft>,
) -> impl IntoElement {
    div().w_full().h_full()
}
