//! System widget for the bar — hexagon-sigil icon, click opens the system popup
//! (brightness + power profile + gaming mode). Anchored popup: captures layout
//! bounds via a zero-opacity canvas, then opens the popup anchored to those
//! bounds on `on_mouse_down`.
//!
//! Always visible (desktop or laptop, with or without a battery). On a
//! desktop without a physical battery this is the only entry point into
//! power/brightness controls — the legacy `battery.rs` widget renders an
//! empty div there and is unclickable. The battery widget is **not**
//! removed by this module; both can coexist (battery shows on laptops,
//! system shows everywhere).

use gpui::{
    AnyElement, App, Bounds, MouseButton, Pixels, Window, canvas, div, prelude::*, px, svg,
};
use std::cell::Cell;
use std::rc::Rc;

use chronos_luau::bar::{BarSection, BarWidget};
use chronos_ui::Theme;

pub struct SystemWidget {
    bounds: Rc<Cell<Bounds<Pixels>>>,
}

impl SystemWidget {
    pub fn new() -> Self {
        Self {
            bounds: Rc::new(Cell::new(Bounds::default())),
        }
    }
}

impl BarWidget for SystemWidget {
    fn name(&self) -> &str {
        "system"
    }

    fn section(&self) -> BarSection {
        BarSection::Right
    }

    fn render(&self, _window: &mut Window, cx: &App) -> AnyElement {
        let theme = Theme::global(cx);

        let bounds_cell = self.bounds.clone();
        let content = div()
            .id("bar-system")
            .flex()
            .items_center()
            .cursor_pointer()
            .px(px(6.))
            .py(px(2.))
            .rounded(theme.radius)
            .hover(|s| s.bg(theme.interactive.hover))
            .child(
                svg()
                    .path("icons/hexagon-core.svg")
                    .size(px(13.))
                    .text_color(theme.accent.primary),
            );

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
            .child(content.on_mouse_down(MouseButton::Left, {
                let bounds_cell = self.bounds.clone();
                move |_event, window, cx: &mut App| {
                    if crate::edit_mode::is_active(cx) {
                        return;
                    }
                    let anchor_rect = bounds_cell.get();
                    let parent = window.window_handle();
                    crate::system_popup::toggle(anchor_rect, parent, window, cx);
                }
            }))
            .into_any_element()
    }
}

/// Register the system widget with the global bar registry.
pub fn register(cx: &mut App) {
    cx.global_mut::<chronos_luau::bar::BarWidgetRegistry>()
        .register(Box::new(SystemWidget::new()));
}
