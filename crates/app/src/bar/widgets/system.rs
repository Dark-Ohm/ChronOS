//! System widget for the bar — hexagon-sigil icon. T290: a click opens the
//! left-panel `Display` tab (brightness + wallpapers) instead of the old
//! system popup. The popup is gone; the Display tab is the brightness entry
//! point now.
//!
//! Always visible (desktop or laptop, with or without a battery). On a
//! desktop without a physical battery this is the only entry point into
//! brightness/wallpaper controls — the legacy `battery.rs` widget renders an
//! empty div there and is unclickable. The battery widget is **not**
//! removed by this module; both can coexist (battery shows on laptops,
//! system shows everywhere).

use gpui::{AnyElement, App, MouseButton, Window, div, prelude::*, px, svg};

use chronos_luau::bar::{BarSection, BarWidget};
use chronos_ui::Theme;

pub struct SystemWidget;

impl SystemWidget {
    pub fn new() -> Self {
        Self
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

        div()
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
            )
            .on_mouse_down(MouseButton::Left, |_event, _window, cx: &mut App| {
                if crate::edit_mode::is_active(cx) {
                    return;
                }
                crate::side_panel_left::select_tab(
                    crate::side_panel_left::tabs::LeftTab::Display,
                    cx,
                );
            })
            .into_any_element()
    }
}

/// Register the system widget with the global bar registry.
pub fn register(cx: &mut App) {
    cx.global_mut::<chronos_luau::bar::BarWidgetRegistry>()
        .register(Box::new(SystemWidget::new()));
}
