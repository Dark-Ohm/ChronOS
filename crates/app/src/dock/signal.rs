//! Dock config change signal.
//!
//! A global `Mutable<()>` that fires whenever the pinned list changes
//! (e.g. after unpin). The dock watches this to rebuild its icon list.

use futures_signals::signal::Mutable;
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

/// Notify all dock views that the config changed.
pub fn notify_config_changed(cx: &mut gpui::App) {
    *cx.global::<DockConfigSignal>().signal.lock_mut() = ();
}
