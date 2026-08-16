//! Dock config change signal.
//!
//! A global `Mutable<()>` that fires whenever the pinned list changes
//! (e.g. after unpin). The dock watches this to rebuild its icon list.

use futures_signals::signal::{Mutable, MutableSignal};
use gpui::Global;

/// Global signal for dock config changes.
pub struct DockConfigSignal {
    pub signal: Mutable<()>,
}

impl Default for DockConfigSignal {
    fn default() -> Self {
        Self {
            signal: Mutable::new(()),
        }
    }
}

impl Global for DockConfigSignal {}

impl DockConfigSignal {
    /// Stream of dock config changes, for `state::watch` in bar/mod.rs.
    /// Callers must ensure the global is already registered (`dock::widgets
    /// register()` at bar init, before any window opens).
    pub fn signal(cx: &gpui::App) -> MutableSignal<()> {
        cx.global::<DockConfigSignal>().signal.signal()
    }
}

/// Notify all dock views that the config changed.
pub fn notify_config_changed(cx: &mut gpui::App) {
    *cx.global::<DockConfigSignal>().signal.lock_mut() = ();
}

/// Global hover state for the dock context menu's "Unpin" row.
/// `on_hover` can only touch globals/`cx.notify()` — not `&mut self` — so the
/// view writes here and reads it back on the next `render` to toggle the
/// 2px accent-bar.
pub struct DockMenuHoverSignal(pub bool);

impl Global for DockMenuHoverSignal {}
