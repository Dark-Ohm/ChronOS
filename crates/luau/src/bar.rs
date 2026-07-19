//! Bar widget trait, sections, and global registry.
//! Moved from crates/app to avoid circular dependencies — the watcher
//! in crates/luau needs BarWidgetRegistry access via cx.update_global.

use gpui::{AnyElement, App, Global, Window};

/// Which horizontal section a widget renders into.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BarSection {
    Left,
    Center,
    Right,
}

/// Bar thickness in logical pixels.
pub const BAR_HEIGHT: f32 = 30.0;

/// Bar background color (0xRRGGBB).
pub const BAR_COLOR: u32 = 0x1e1e2e;

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
    pub fn register(&mut self, widget: Box<dyn BarWidget>) {
        self.widgets.push(widget);
    }

    pub fn replace_by_name(&mut self, name: &str, widget: Box<dyn BarWidget>) -> Option<Box<dyn BarWidget>> {
        if let Some(pos) = self.widgets.iter().position(|w| w.name() == name) {
            let old = std::mem::replace(&mut self.widgets[pos], widget);
            Some(old)
        } else {
            self.widgets.push(widget);
            None
        }
    }

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
    fn replace_by_name_swaps_existing() {
        let mut registry = BarWidgetRegistry::default();
        registry.register(Box::new(FakeWidget { name: "w1".into(), section: BarSection::Left }));
        let old = registry.replace_by_name("w1", Box::new(FakeWidget { name: "w1".into(), section: BarSection::Right }));
        assert!(old.is_some());
        assert_eq!(registry.widgets_for(BarSection::Right).count(), 1);
        assert_eq!(registry.widgets_for(BarSection::Left).count(), 0);
    }

    #[test]
    fn unregister_by_name_removes() {
        let mut registry = BarWidgetRegistry::default();
        registry.register(Box::new(FakeWidget { name: "w1".into(), section: BarSection::Left }));
        let removed = registry.unregister_by_name("w1");
        assert!(removed.is_some());
        assert_eq!(registry.widgets_for(BarSection::Left).count(), 0);
    }
}
