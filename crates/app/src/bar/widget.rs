// crates/app/src/bar/widget.rs
use gpui::{AnyElement, App, Global, Window};

use crate::bar::sections::BarSection;

pub trait BarWidget: 'static {
    fn section(&self) -> BarSection {
        BarSection::Left
    }
    fn render(&self, _window: &mut Window, _cx: &App) -> AnyElement;
}

pub struct BarWidgetRegistry {
    widgets: Vec<Box<dyn BarWidget>>,
}

impl Default for BarWidgetRegistry {
    fn default() -> Self {
        Self {
            widgets: Vec::new(),
        }
    }
}

impl Global for BarWidgetRegistry {}

impl BarWidgetRegistry {
    // Widget-registration API; not yet called until a widget registers itself.
    #[allow(dead_code)]
    pub fn register(&mut self, widget: Box<dyn BarWidget>) {
        self.widgets.push(widget);
    }

    pub fn widgets_for(&self, section: BarSection) -> impl Iterator<Item = &dyn BarWidget> {
        self.widgets
            .iter()
            .filter(move |w| w.section() == section)
            .map(|w| w.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{div, IntoElement};

    struct FakeWidget {
        section: BarSection,
    }

    impl BarWidget for FakeWidget {
        fn section(&self) -> BarSection {
            self.section
        }
        fn render(&self, _window: &mut Window, _cx: &App) -> AnyElement {
            div().into_any_element()
        }
    }

    #[test]
    fn register_then_filter_by_section() {
        let mut registry = BarWidgetRegistry::default();
        registry.register(Box::new(FakeWidget { section: BarSection::Left }));
        registry.register(Box::new(FakeWidget { section: BarSection::Right }));

        let left: Vec<&dyn BarWidget> = registry.widgets_for(BarSection::Left).collect();
        assert_eq!(left.len(), 1);
        assert_eq!(left[0].section(), BarSection::Left);

        let right: Vec<&dyn BarWidget> = registry.widgets_for(BarSection::Right).collect();
        assert_eq!(right.len(), 1);

        let center: Vec<&dyn BarWidget> = registry.widgets_for(BarSection::Center).collect();
        assert_eq!(center.len(), 0);
    }

    #[test]
    fn default_section_is_left() {
        struct Plain;
        impl BarWidget for Plain {
            fn render(&self, _w: &mut Window, _c: &App) -> AnyElement {
                div().into_any_element()
            }
        }
        let mut registry = BarWidgetRegistry::default();
        registry.register(Box::new(Plain));
        // Plain relies on the default trait method -> Left
        assert_eq!(registry.widgets_for(BarSection::Left).count(), 1);
        assert_eq!(registry.widgets_for(BarSection::Center).count(), 0);
    }
}
