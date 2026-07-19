//! Built-in bar widgets.

mod battery;
mod cava;
mod clock;
mod dock;
mod mpris;
mod network;
mod notification_bell;
mod separator;
mod system;
mod tray;
mod updates;
mod volume;
mod workspaces;

use chronos_luau::bar::BarSection;
use gpui::App;

/// Register all built-in bar widgets with the global registry.
/// Called once at startup from [`crate::bar::init`].
///
/// ORDER MATTERS: within a section, widgets render in registration order
/// (see `design/Top Bar.dc.html` + STYLE.md «Bar layout — ПРИНЯТЫЕ решения»).
pub fn register_builtin(cx: &mut App) {
    // Left: Start+dock → | → workspace dots.
    dock::register(cx);
    separator::register(BarSection::Left, cx);
    workspaces::register(cx);

    // Center: MPRIS to the left of CAVA; CAVA holds the middle.
    mpris::register(cx);
    cava::register(cx);

    // Right: controls cluster → | → battery → clock (rightmost).
    volume::register(cx);
    network::register(cx);
    tray::register(cx);
    updates::register(cx);
    system::register(cx);
    notification_bell::register(cx);
    separator::register(BarSection::Right, cx);
    cx.global_mut::<chronos_luau::bar::BarWidgetRegistry>()
        .register(Box::new(battery::BatteryWidget));
    clock::register(cx);
}
