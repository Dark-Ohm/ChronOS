//! Built-in bar widgets.

mod battery;
mod clock;
// mod network;  // выключено Архитектором: ServiceStatus::Failed не существует — фикс за Autohand
mod workspaces;

use gpui::App;

use chronos_luau::bar::BarWidgetRegistry;

/// Register all built-in bar widgets with the global registry.
/// Called once at startup from [`crate::bar::init`].
pub fn register_builtin(cx: &mut App) {
    clock::register(cx);
    // ── Other agents append below (one mod + one call each) ──
    workspaces::register(cx);
    cx.global_mut::<chronos_luau::bar::BarWidgetRegistry>()
        .register(Box::new(battery::BatteryWidget));
    // network: выключено до фикса Autohand (см. выше)
    // tray::register(cx);
}
