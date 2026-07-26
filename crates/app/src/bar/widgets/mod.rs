//! Built-in bar widgets.

mod battery;
mod cava;
mod clock;
mod dock;
mod mpris;
mod network;
mod notification_bell;
mod project;
mod separator;
mod system;
mod tray;
mod updates;
mod volume;
mod workspaces;

use chronos_luau::bar::{BarSection, BarWidget, BarWidgetRegistry};
use gpui::{AnyElement, App, Window};

use super::layout_config::BarLayoutConfig;

/// Forces a placement section independent of the widget's default.
struct ForcedSection {
    inner: Box<dyn BarWidget>,
    section: BarSection,
}

impl BarWidget for ForcedSection {
    fn name(&self) -> &str {
        self.inner.name()
    }

    fn section(&self) -> BarSection {
        self.section
    }

    fn render(&self, window: &mut Window, cx: &App) -> AnyElement {
        self.inner.render(window, cx)
    }
}

/// Build a widget instance for `name` into `section` (no register).
fn instantiate(name: &str, section: BarSection) -> Option<Box<dyn BarWidget>> {
    let inner: Box<dyn BarWidget> = match name {
        "dock" => Box::new(dock::DockWidget),
        "separator" => Box::new(separator::Separator { section }),
        "workspaces" => Box::new(workspaces::WorkspacesWidget),
        "mpris" => Box::new(mpris::MprisWidget),
        "cava" => Box::new(cava::CavaWidget),
        "project" => Box::new(project::ProjectWidget),
        "volume" => Box::new(volume::VolumeWidget::new()),
        "network" => Box::new(network::NetworkWidget::new()),
        "tray" => Box::new(tray::TrayWidget),
        "updates" => Box::new(updates::UpdatesWidget::new()),
        "system" => Box::new(system::SystemWidget::new()),
        "notification_bell" => Box::new(notification_bell::NotificationBellWidget::new()),
        "battery" => Box::new(battery::BatteryWidget),
        "clock" => Box::new(clock::ClockWidget),
        _ => return None,
    };
    // Separator already carries section.
    if name == "separator" {
        Some(inner)
    } else {
        Some(Box::new(ForcedSection { inner, section }))
    }
}

/// Clear registry and re-register widgets from layout (T134).
pub fn apply_layout(cx: &mut App, cfg: &BarLayoutConfig) {
    {
        let reg = cx.global_mut::<BarWidgetRegistry>();
        reg.clear();
    }
    for (name, section) in cfg.slots() {
        match instantiate(&name, section) {
            Some(w) => {
                cx.global_mut::<BarWidgetRegistry>().register(w);
            }
            None => {
                tracing::warn!("bar: cannot instantiate widget '{name}'");
            }
        }
    }
}

/// Register all built-in bar widgets from config (or default).
/// Called once at startup from [`crate::bar::init`].
pub fn register_builtin(cx: &mut App) {
    let cfg = BarLayoutConfig::load().sanitized();
    super::layout_config::update_cache(cfg.clone());
    apply_layout(cx, &cfg);
}
