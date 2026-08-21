//! Runtime Edit Mode — layout customization flag (T134).
//!
//! When active: bar shows EDIT chrome; widget open-popups should yield to
//! layout affordances. Config writes go through `bar::layout_config`.

use gpui::{App, Global};

#[derive(Default, Debug, Clone, Copy)]
pub struct EditModeState {
    pub active: bool,
}

impl Global for EditModeState {}

pub fn init(cx: &mut App) {
    cx.set_global(EditModeState::default());
}

pub fn is_active(cx: &App) -> bool {
    cx.try_global::<EditModeState>()
        .map(|s| s.active)
        .unwrap_or(false)
}

pub fn toggle(cx: &mut App) {
    let active = {
        let s = cx.global_mut::<EditModeState>();
        s.active = !s.active;
        s.active
    };
    // Entering edit mode re-chromes the bar, so any bar-anchored popup must
    // yield — their anchor geometry is about to change and their
    // click-catchers would fight the edit affordances.
    if active {
        crate::volume_popup::close(cx);
        crate::calendar_popup::close(cx);
    }
    tracing::info!(active, "edit_mode: toggled");
    cx.refresh_windows();
}
