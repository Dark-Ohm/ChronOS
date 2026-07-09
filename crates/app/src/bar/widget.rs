// crates/app/src/bar/widget.rs
use gpui::{AnyElement, App, Global, Window};

use crate::bar::sections::BarSection;

pub trait BarWidget: 'static {
    fn name(&self) -> &str {
        "unnamed"
    }
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
    #[allow(dead_code)]
    pub fn register(&mut self, widget: Box<dyn BarWidget>) {
        self.widgets.push(widget);
    }

    /// Replace a widget by name. If found, swaps it and returns the old widget.
    /// If not found, pushes the new widget and returns None.
    pub fn replace_by_name(&mut self, name: &str, widget: Box<dyn BarWidget>) -> Option<Box<dyn BarWidget>> {
        if let Some(pos) = self.widgets.iter().position(|w| w.name() == name) {
            let old = std::mem::replace(&mut self.widgets[pos], widget);
            Some(old)
        } else {
            self.widgets.push(widget);
            None
        }
    }

    /// Remove a widget by name. Returns the removed widget if found.
    #[allow(dead_code)]
    pub fn unregister_by_name(&mut self, name: &str) -> Option<Box<dyn BarWidget>> {
        if let Some(pos) = self.widgets.iter().position(|w| w.name() == name) {
            Some(self.widgets.remove(pos))
        } else {
            None
        }
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
        name: String,
        section: BarSection,
    }

    impl BarWidget for FakeWidget {
        fn name(&self) -> &str {
            &self.name
        }
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
        registry.register(Box::new(FakeWidget { name: "a".into(), section: BarSection::Left }));
        registry.register(Box::new(FakeWidget { name: "b".into(), section: BarSection::Right }));

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
        assert_eq!(registry.widgets_for(BarSection::Left).count(), 1);
        assert_eq!(registry.widgets_for(BarSection::Center).count(), 0);
    }

    #[test]
    fn replace_by_name_swaps_existing() {
        let mut registry = BarWidgetRegistry::default();
        registry.register(Box::new(FakeWidget { name: "w1".into(), section: BarSection::Left }));
        let old = registry.replace_by_name("w1", Box::new(FakeWidget { name: "w1".into(), section: BarSection::Right }));
        assert!(old.is_some());
        assert_eq!(registry.widgets_for(BarSection::Right).count(), 1);
        assert_eq!(registry.widgets_for(BarSection::Left).count(), 0);
    }

    #[test]
    fn replace_by_name_pushes_new() {
        let mut registry = BarWidgetRegistry::default();
        let old = registry.replace_by_name("new", Box::new(FakeWidget { name: "new".into(), section: BarSection::Center }));
        assert!(old.is_none());
        assert_eq!(registry.widgets_for(BarSection::Center).count(), 1);
    }

    #[test]
    fn unregister_by_name_removes() {
        let mut registry = BarWidgetRegistry::default();
        registry.register(Box::new(FakeWidget { name: "w1".into(), section: BarSection::Left }));
        let removed = registry.unregister_by_name("w1");
        assert!(removed.is_some());
        assert_eq!(registry.widgets_for(BarSection::Left).count(), 0);
    }

    #[test]
    fn unregister_by_name_returns_none_for_missing() {
        let mut registry = BarWidgetRegistry::default();
        let removed = registry.unregister_by_name("nope");
        assert!(removed.is_none());
    }
}
