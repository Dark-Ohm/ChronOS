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
        let border_subtle = theme.border.subtle;

        if notifications.is_empty() {
            return div().into_any_element();
        }

        let cards: Vec<AnyElement> = notifications
            .iter()
            .map(|n: &Notification| {
                let id = n.id;
                let text_muted = theme.text.muted;
                let close_btn = div()
                    .text_color(text_muted)
                    .cursor_pointer()
                    .id(format!("notif-close-{id}"))
                    .on_click(move |_event, _window, cx| {
                        let svc = AppState::notification(cx).clone();
                        cx.background_spawn(async move {
                            let _ = svc.dispatch(NotificationCommand::Close(id)).await;
                        });
                    })
                    .child("✕")
                    .into_any_element();
                render_notification_card(n, &theme, Some(close_btn))
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
            .border_1()
            .border_color(border_subtle)
            .overflow_hidden()
            .children(cards)
            .into_any_element()
    }
}

/// Render a single notification card (header + summary + body + action buttons).
///
/// Extracted from [`NotificationsView::render`] so the history popup can reuse
/// the exact same card styling. `close_button` is an optional pre-built element
/// (the ✕ in the ephemeral popup); the history popup passes `None` because its
/// items live in the persistent log and are not individually dismissable.
///
/// All `on_click` callbacks capture `n.id` and use the runtime-provided `cx`, so
/// this function needs no external `Context` borrow — it composes freely inside
/// an iterator over a `cx.global()` snapshot.
pub(crate) fn render_notification_card(
    n: &Notification,
    theme: &Theme,
    close_button: Option<AnyElement>,
) -> AnyElement {
    let accent = match n.urgency {
        Urgency::Critical => theme.status.error,
        Urgency::Normal => theme.status.warning,
        Urgency::Low => theme.status.info,
    };

    let bg_elevated = theme.bg.elevated;
    let text_primary = theme.text.primary;
    let text_secondary = theme.text.secondary;
    let text_muted = theme.text.muted;
    let bg_secondary = theme.bg.secondary;
    let radius = theme.radius;
    let radius_lg = theme.radius_lg;

    // Header: app name (left) + optional close button (right).
    let mut header = div()
        .flex()
        .justify_between()
        .items_start()
        .child(div().text_color(text_secondary).text_xs().child(n.app_name.clone()));
    if let Some(btn) = close_button {
        header = header.child(btn);
    }

    let title = div()
        .text_color(text_primary)
        .font_weight(FontWeight::BOLD)
        .child(n.summary.clone());

    // Body: hard-clipped so a long body truncates instead of overflowing.
    let content = div()
        .max_h(px(BODY_MAX_H))
        .overflow_hidden()
        .text_color(text_muted)
        .child(n.body.clone());

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
    if !n.actions.is_empty() {
        let id = n.id;
        let buttons: Vec<AnyElement> = n
            .actions
            .iter()
            .cloned()
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
                                .dispatch(NotificationCommand::InvokeAction(id, action_key))
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
}
