//! Notifications tab (T293) — right-panel replacement for the history popup.
//!
//! Renders the same notification history list (newest-first, urgency strip,
//! monogram, actions, dismiss, Clear all) as the former popup, but inside
//! the right panel's scroll viewport. The card renderer is shared with the
//! (transient) popup via `notifications::history_list` — no duplication.

use gpui::{Context, Render, Window, div, prelude::*, px};

use chronos_services::Service;
use chronos_ui::Theme;
use crate::state::{self, AppState};

use crate::notifications::history_list;

pub struct NotificationsTab;

impl NotificationsTab {
    pub fn new(cx: &mut Context<Self>) -> Self {
        // Repaint on every NotificationState change so the list stays live.
        let signal = AppState::notification(cx).subscribe();
        state::watch(cx, signal, |_this: &mut Self, _state, cx| {
            cx.notify();
        });
        Self
    }
}

impl Render for NotificationsTab {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = *Theme::global(cx);
        let bg = theme.bg.primary;

        // The right panel already provides the outer chrome (header, rail,
        // resize handle). Scroll fills the canvas — no MAX_LIST_H cap like
        // the popup. `overflow_hidden` on the content column would otherwise
        // clip a long history with no way to reach Clear all.
        div()
            .id("notifications-tab")
            .w_full()
            .h_full()
            .flex()
            .flex_col()
            // T266: full-size tab plate — follows surface alpha.
            .bg(theme.surface_color(bg))
            .child(
                div()
                    .id("notifications-tab-scroll")
                    .w_full()
                    .flex_1()
                    .min_h(px(0.))
                    .overflow_y_scroll()
                    .child(history_list::render_history_list(window, cx)),
            )
    }
}
