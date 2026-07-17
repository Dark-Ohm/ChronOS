//! Built-in bar widgets.

mod battery;
mod clock;
mod mpris;
mod network;
mod tray;
mod volume;
mod workspaces;

use gpui::App;

/// Register all built-in bar widgets with the global registry.
/// Called once at startup from [`crate::bar::init`].
pub fn register_builtin(cx: &mut App) {
    clock::register(cx);
    // ── Other agents append below (one mod + one call each) ──
    workspaces::register(cx);
    cx.global_mut::<chronos_luau::bar::BarWidgetRegistry>()
        .register(Box::new(battery::BatteryWidget));
    network::register(cx);
    tray::register(cx);
    volume::register(cx);
    mpris::register(cx);
}
