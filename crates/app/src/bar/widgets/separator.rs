//! Thin vertical separator between widget groups — mockup convention
//! (`design/Top Bar.dc.html`): 1px × 14px, `bg.elevated`.

use gpui::{AnyElement, App, Window, div, prelude::*, px};

use chronos_luau::bar::{BarSection, BarWidget};
use chronos_ui::Theme;

pub struct Separator {
    section: BarSection,
}

impl BarWidget for Separator {
    fn name(&self) -> &str {
        "separator"
    }

    fn section(&self) -> BarSection {
        self.section
    }

    fn render(&self, _window: &mut Window, cx: &App) -> AnyElement {
        div()
            .w(px(1.))
            .h(px(14.))
            .bg(Theme::global(cx).bg.elevated)
            .into_any_element()
    }
}

/// Register a separator into `section` at the current registration position.
/// Order matters: the registry renders widgets in registration order.
pub fn register(section: BarSection, cx: &mut App) {
    cx.global_mut::<chronos_luau::bar::BarWidgetRegistry>()
        .register(Box::new(Separator { section }));
}
