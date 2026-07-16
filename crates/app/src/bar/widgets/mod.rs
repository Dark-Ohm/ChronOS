//! Built-in bar widgets.

mod clock;
mod workspaces;

use gpui::App;

use chronos_luau::bar::BarWidgetRegistry;

/// Register all built-in bar widgets with the global registry.
/// Called once at startup from [`crate::bar::init`].
pub fn register_builtin(cx: &mut App) {
    clock::register(cx);
    // ── Other agents append below (one mod + one call each) ──
    workspaces::register(cx);
    // battery::register(cx);
    // network::register(cx);
    // tray::register(cx);
}
