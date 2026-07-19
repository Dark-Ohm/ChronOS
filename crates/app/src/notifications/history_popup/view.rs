//! The notification-history popup view: a header + scrollable stack of every
//! notification that has appeared this session (`NotificationState::history`).
//!
//! Holds no state of its own — reads the live snapshot from the
//! `HistoryPopupState`/notification global and re-paints on every change (the
//! watcher calls `view_cx.notify()`). Cards reuse `render_notification_card`
//! from `crate::notifications::view`, so a history item looks exactly like a
//! live notification — it just passes `None` for the close button (history
//! entries are not individually dismissable; the log is cleared on restart).

use gpui::{
    AnyElement, App, Context, Div, FontWeight, Render, Window, div, prelude::*, px,
};

use chronos_services::{Notification, Service};

use crate::notifications::view::render_notification_card;
use crate::state::AppState;

use chronos_ui::Theme;

/// Build a fresh, empty history-popup view.
impl HistoryPopupView {
    pub fn new(_cx: &mut App) -> Self {
        Self {}
    }
}

pub struct HistoryPopupView {}

impl Render for HistoryPopupView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let state = AppState::notification(cx).get();
        let history: &[Notification] = &state.history;

        let theme = Theme::global(cx);
        let border_subtle = theme.border.subtle;
        let text_primary = theme.text.primary;
        let text_muted = theme.text.muted;
        let bg_elevated = theme.bg.elevated;
        let radius_lg = theme.radius_lg;

        // Close (✕) button — sits in the header, closes this popup.
        let close_btn = div()
            .text_color(text_muted)
            .cursor_pointer()
            .id("notif-history-close")
            .on_click(|_event, window, cx: &mut App| {
                crate::notifications::history_popup::close_this(window, cx);
            })
            .child("✕");

        let header = div()
            .flex()
            .justify_between()
            .items_center()
            .child(
                div()
                    .text_color(text_primary)
                    .font_weight(FontWeight::BOLD)
                    .child("Notifications"),
            )
            .child(close_btn);

        let body: Div = if history.is_empty() {
            div()
                .flex()
                .items_center()
                .justify_center()
                .h(px(80.))
                .text_color(text_muted)
                .child("No notifications yet")
        } else {
            // Newest first: render reversed. Hard-clipped so a long history
            // never grows the window past its fixed cap.
            let cards: Vec<AnyElement> = history
                .iter()
                .rev()
                .map(|n: &Notification| render_notification_card(n, &theme, None))
                .collect();
            div()
                .flex_col()
                .gap(px(8.))
                .max_h(px(380.))
                .overflow_hidden()
                .children(cards)
        };

        div()
            .flex_col()
            .gap(px(8.))
            .p(px(12.))
            .w(px(360.))
            .rounded(radius_lg)
            .bg(bg_elevated)
            .border_1()
            .border_color(border_subtle)
            .child(header)
            .child(body)
            .into_any_element()
    }
}
