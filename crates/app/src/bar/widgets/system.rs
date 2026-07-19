//! System widget for the bar — ⚙ icon, click opens the system popup
//! (brightness + power profile + gaming mode).
//!
//! Always visible (desktop or laptop, with or without a battery). On a
//! desktop without a physical battery this is the only entry point into
//! power/brightness controls — the legacy `battery.rs` widget renders an
//! empty div there and is unclickable. The battery widget is **not**
//! removed by this module; both can coexist (battery shows on laptops,
//! system shows everywhere).

use gpui::{AnyElement, App, Window, div, prelude::*, px};

use chronos_luau::bar::{BarSection, BarWidget};
use chronos_ui::Theme;

pub struct SystemWidget;

impl BarWidget for SystemWidget {
    fn name(&self) -> &str {
        "system"
    }

    fn section(&self) -> BarSection {
        BarSection::Right
    }

    fn render(&self, _window: &mut Window, cx: &App) -> AnyElement {
        let theme = Theme::global(cx);
        let color = theme.text.secondary;

        // Glyph icon (⚙) — simpler than SVG hexagon line-art, and the brief
        // explicitly allows a glyph when SVG in GPUI is a hassle.
        div()
            .id("bar-system")
            .flex()
            .items_center()
            .cursor_pointer()
            .px(px(6.))
            .py(px(2.))
            .rounded(theme.radius)
            .child(div().child("⚙").text_color(color))
            .on_click(|_event, window, cx: &mut App| {
                crate::system_popup::toggle(window, cx);
            })
            .into_any_element()
    }
}

/// Register the system widget with the global bar registry.
pub fn register(cx: &mut App) {
    cx.global_mut::<chronos_luau::bar::BarWidgetRegistry>()
        .register(Box::new(SystemWidget));
}
