//! The notifications popup view: a vertical stack of notification cards
//! rendered on a transparent layer-shell surface.
//!
//! The view holds no state of its own — it reads the live snapshot from the
//! `NotificationPopupState` global and re-paints on every global change
//! (the `notifications::sync_window` driver calls `view_cx.notify()` on the
//! open window). All mutations go back to the daemon through
//! `NotificationCommand` dispatches.

use gpui::{
    AnyElement, App, Context, Div, FontWeight, InteractiveElement, Render, Window, div, prelude::*,
    px,
};

use chronos_services::{Notification, NotificationCommand, Urgency};

use crate::notifications::{BODY_MAX_H, LIST_MAX_H, NotificationPopupState};
use crate::state::AppState;

use chronos_ui::Theme;

/// Build a fresh, empty popup view.
impl NotificationsView {
    pub fn new(_cx: &mut App) -> Self {
        Self {}
    }
}

pub struct NotificationsView {}

impl Render for NotificationsView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // The popup re-paints via `view_cx.notify()` driven from
        // `notifications::sync_window` whenever the global snapshot changes,
        // so we just read the live snapshot straight from the global here.
        let notifications = &cx.global::<NotificationPopupState>().current.notifications;

        let theme = Theme::global(cx);

        // Urgency → left-border color (captured as plain `Hsla` values before
        // any closures so we don't hold a borrow of `cx`/`theme` inside the
        // click callbacks):
        //   * Critical → status.error  (red)    — can't be missed.
        //   * Normal   → status.warning (amber)  — common case, still "active".
        //   * Low      → status.info    (blue)   — quiet / background noise.
        // We avoid `status.success`/muted so a card is never visually "dead".
        let accent_for = |u: Urgency| -> gpui::Hsla {
            match u {
                Urgency::Critical => theme.status.error,
                Urgency::Normal => theme.status.warning,
                Urgency::Low => theme.status.info,
            }
        };

        if notifications.is_empty() {
            return div().into_any_element();
        }

        let cards: Vec<AnyElement> = notifications
            .iter()
            .map(|n: &Notification| {
                let id = n.id;
                let accent = accent_for(n.urgency);
                let app_name = n.app_name.clone();
                let summary = n.summary.clone();
                let body = n.body.clone();
                let actions: Vec<(String, String)> = n.actions.clone();

                let bg_elevated = theme.bg.elevated;
                let text_primary = theme.text.primary;
                let text_secondary = theme.text.secondary;
                let text_muted = theme.text.muted;
                let bg_secondary = theme.bg.secondary;
                let radius = theme.radius;
                let radius_lg = theme.radius_lg;

                // Header row: app name (left) + close (✕) button (right).
                let header = div()
                    .flex()
                    .justify_between()
                    .items_start()
                    .child(div().text_color(text_secondary).text_xs().child(app_name))
                    .child(
                        div()
                            .text_color(text_muted)
                            .cursor_pointer()
                            .id(format!("notif-close-{id}"))
                            .on_click(move |_event, _window, cx| {
                                let svc = AppState::notification(cx).clone();
                                cx.background_spawn(async move {
                                    let _ = svc.dispatch(NotificationCommand::Close(id)).await;
                                });
                            })
                            .child("✕"),
                    );

                // Summary (bold) + body (muted).
                let title = div()
                    .text_color(text_primary)
                    .font_weight(FontWeight::BOLD)
                    .child(summary);
                // Body: hard-clipped so a long body truncates instead of
                // overflowing the card and colliding with the next one.
                let content = div()
                    .max_h(px(BODY_MAX_H))
                    .overflow_hidden()
                    .text_color(text_muted)
                    .child(body);

                let mut card: Div = div()
                    .flex_col()
                    .gap(px(4.))
                    .p(px(12.))
                    .rounded(radius_lg)
                    .bg(bg_elevated)
                    .border_l_3()
                    .border_color(accent)
                    .child(header)
                    .child(title)
                    .child(content);

                // Action buttons — each dispatches InvokeAction(key).
                if !actions.is_empty() {
                    let buttons: Vec<AnyElement> = actions
                        .into_iter()
                        .map(|(key, label)| {
                            let id = id;
                            let key = key.clone();
                            div()
                                .px(px(8.))
                                .py(px(2.))
                                .rounded(radius)
                                .bg(bg_secondary)
                                .text_color(text_primary)
                                .cursor_pointer()
                                .id(format!("notif-action-{id}-{key}"))
                                .on_click(move |_event, _window, cx| {
                                    let svc = AppState::notification(cx).clone();
                                    let action_key = key.clone();
                                    cx.background_spawn(async move {
                                        let _ = svc
                                            .dispatch(NotificationCommand::InvokeAction(
                                                id, action_key,
                                            ))
                                            .await;
                                    });
                                })
                                .child(label)
                                .into_any_element()
                        })
                        .collect();

                    card = card.child(div().flex().flex_wrap().gap(px(6.)).children(buttons));
                }

                card.into_any_element()
            })
            .collect();

        // Card stack: hard-clipped to LIST_MAX_H so any number of
        // notifications can never grow the (fixed-cap) window taller and
        // push content past the surface edge — older cards are silently
        // clipped off the bottom and expire on their timer anyway.
        div()
            .flex_col()
            .gap(px(8.))
            .max_h(px(LIST_MAX_H))
            .overflow_hidden()
            .children(cards)
            .into_any_element()
    }
}
