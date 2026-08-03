//! The ephemeral toast stack view — renders each live notification as an
//! independent card with icon, app name, close ✕, summary, body, action
//! buttons, and a bottom progress bar. Cards stack top-to-bottom, clipped
//! to `LIST_MAX_H`.
//!
//! T124: each card is self-contained (its own border + rounded corners +
//! shadow + progress track). No outer list border. Progress is driven by
//! a 100ms tick loop that expires when the view is dropped.
//!
//! The old `render_notification_card` (shared-card style with left-border
//! urgency strip) is preserved at the bottom for history-popup compat.

use std::collections::HashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use gpui::{
    Context, FontWeight, Hsla, InteractiveElement, Render, Window, div, prelude::*, px,
};

use chronos_services::{Notification, NotificationCommand, Urgency};

use crate::notifications::{LIST_MAX_H, NotificationPopupState};
use crate::state::AppState;

use chronos_ui::{Theme, WindowRootExt};

// ── Time helpers ─────────────────────────────────────────────────────

fn now_epoch_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

// ── Color helpers ────────────────────────────────────────────────────

fn toast_progress_color(urgency: Urgency) -> Hsla {
    match urgency {
        Urgency::Critical => Hsla::from(gpui::rgba(0xf38ba8ff)),
        Urgency::Normal | Urgency::Low => Hsla::from(gpui::rgba(0x89b4faff)),
    }
}

/// Progress fraction `[0, 1]` for a notification with known `expire_at`.
/// Returns `None` when `expire_at` is `None` (sticky / no TTL known).
fn progress_fraction(n: &Notification, first_seen: &HashMap<u32, u64>, now_ms: u64) -> Option<f32> {
    let expire = n.expire_at?;
    let seen = *first_seen.get(&n.id)?;
    if expire <= seen {
        return Some(0.0);
    }
    let total = (expire - seen) as f32;
    let remaining = expire.saturating_sub(now_ms) as f32;
    Some((remaining / total).clamp(0.0, 1.0))
}

// ── Toast card renderer ──────────────────────────────────────────────

fn render_toast_card(
    n: &Notification,
    _theme: &Theme,
    now_ms: u64,
    first_seen: &HashMap<u32, u64>,
) -> impl IntoElement {
    let id = n.id;
    let is_critical = n.urgency == Urgency::Critical;

    // ── Mockup colors (dark theme, "Catppuccin Mocha" palette) ────
    let c_border = if is_critical {
        Hsla::from(gpui::rgba(0xf38ba833))
    } else {
        Hsla::from(gpui::rgba(0x31_32_44ff))
    };
    let c_bg = Hsla::from(gpui::rgba(0x1e_1e_2eff));
    let c_app_name = if is_critical {
        Hsla::from(gpui::rgba(0xf38ba8ff))
    } else {
        Hsla::from(gpui::rgba(0x6c_70_86ff))
    };
    let c_summary = if is_critical {
        Hsla::from(gpui::rgba(0xf38ba8ff))
    } else {
        Hsla::from(gpui::rgba(0xcd_d6_f4ff))
    };
    let c_body = Hsla::from(gpui::rgba(0xa6_ad_c8ff));
    let c_close = Hsla::from(gpui::rgba(0x45_47_5aff));
    let c_close_bg_hover = Hsla::from(gpui::rgba(0x31_32_44ff));
    let c_close_icon_hover = Hsla::from(gpui::rgba(0xcd_d6_f4ff));
    let c_action_text = Hsla::from(gpui::rgba(0xa6_ad_c8ff));
    let c_action_border = Hsla::from(gpui::rgba(0x45_47_5aff));
    let c_action_hover = Hsla::from(gpui::rgba(0xcb_a6_f7ff));
    let c_icon_bg = Hsla::from(gpui::rgba(0x31_32_44ff));
    let c_progress_track = Hsla::from(gpui::rgba(0x25_25_3bff));

    let progress_color = toast_progress_color(n.urgency);
    let frac = progress_fraction(n, first_seen, now_ms);
    let initials = app_initials(&n.app_name);
    let icon_color = monogram_color(&n.app_name);

    // ── Icon 28×28 ────────────────────────────────────────────────
    let icon = div()
        .w(px(28.))
        .h(px(28.))
        .rounded(px(6.))
        .flex_none()
        .bg(c_icon_bg)
        .flex()
        .items_center()
        .justify_center()
        .child(
            div()
                .text_size(px(12.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(icon_color)
                .child(initials),
        );

    // ── Close ✕ ───────────────────────────────────────────────────
    let close_btn = div()
        .w(px(16.))
        .h(px(16.))
        .rounded(px(4.))
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .text_color(c_close)
        .cursor_pointer()
        .id(format!("toast-close-{id}"))
        .hover(move |s| s.bg(c_close_bg_hover).text_color(c_close_icon_hover))
        .child("✕")
        .on_click({
            let id = id;
            move |_event, _window, cx| {
                let svc = AppState::notification(cx).clone();
                cx.background_spawn(async move {
                    let _ = svc.dispatch(NotificationCommand::Close(id)).await;
                })
                .detach();
            }
        });

    // ── App name ──────────────────────────────────────────────────
    let app_name = div()
        .flex_1()
        .min_w(px(0.))
        .text_size(px(11.))
        .text_color(c_app_name)
        .whitespace_nowrap()
        .overflow_hidden()
        .text_ellipsis()
        .child(n.app_name.clone());

    // ── Header row (icon | app_name | close) ──────────────────────
    let header_row = div()
        .flex()
        .items_start()
        .gap(px(10.))
        .child(icon)
        .child(app_name)
        .child(close_btn);

    // ── Summary ───────────────────────────────────────────────────
    let summary = div()
        .ml(px(38.)) // align with text after icon
        .mt(px(3.))
        .text_size(px(12.5))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(c_summary)
        .child(n.summary.clone());

    // ── Body ──────────────────────────────────────────────────────
    let body = div()
        .ml(px(38.))
        .mt(px(2.))
        .text_size(px(11.))
        .text_color(c_body)
        .child(n.body.clone());

    let mut content_children: Vec<gpui::AnyElement> =
        vec![header_row.into_any_element(), summary.into_any_element(), body.into_any_element()];

    // ── Action buttons ────────────────────────────────────────────
    if !n.actions.is_empty() {
        let buttons: Vec<gpui::AnyElement> = n
            .actions
            .iter()
            .map(|(key, label)| {
                let id = id;
                let key = key.clone();
                let label = label.clone();
                div()
                    .px(px(10.))
                    .py(px(4.))
                    .rounded(px(5.))
                    .border_1()
                    .border_color(c_action_border)
                    .text_size(px(10.5))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(c_action_text)
                    .cursor_pointer()
                    .id(format!("toast-action-{id}-{key}"))
                    .hover(move |s| s.border_color(c_action_hover).text_color(c_action_hover))
                    .child(label)
                    .on_click(move |_event, _window, cx| {
                        let svc = AppState::notification(cx).clone();
                        let key = key.clone();
                        cx.background_spawn(async move {
                            let _ = svc
                                .dispatch(NotificationCommand::InvokeAction(id, key))
                                .await;
                        })
                        .detach();
                    })
                    .into_any_element()
            })
            .collect();

        content_children.push(
            div()
                .mt(px(6.))
                .ml(px(38.))
                .flex()
                .gap(px(6.))
                .children(buttons)
                .into_any_element(),
        );
    }

    // ── Content area (padded 10/12 per mockup) ────────────────────
    let content = div().flex_col().p(px(10.)).px(px(12.)).children(content_children);

    // ── Progress bar ──────────────────────────────────────────────
    let progress: gpui::AnyElement = if let Some(f) = frac {
        let fill_w = f * 340.0; // POPUP_WIDTH
        div()
            .h(px(2.))
            .bg(c_progress_track)
            .child(
                div()
                    .h(px(2.))
                    .w(px(fill_w))
                    .bg(progress_color)
                    .opacity(if is_critical { 0.6 } else { 0.5 }),
            )
            .into_any_element()
    } else {
        div().into_any_element()
    };

    // ── Card ──────────────────────────────────────────────────────
    div()
        .flex_col()
        .rounded(px(8.))
        .bg(c_bg)
        .border_1()
        .border_color(c_border)
        .overflow_hidden()
        .child(content)
        .child(progress)
}

// ── NotificationsView ────────────────────────────────────────────────

/// The ephemeral toast-stack view. Holds `first_seen` timestamps for
/// progress bar math; drives a 100ms tick loop while alive.
pub struct NotificationsView {
    /// Epoch ms (SystemTime) when each notification id was first rendered.
    first_seen: HashMap<u32, u64>,
}

impl NotificationsView {
    pub fn new(cx: &mut Context<Self>) -> Self {
        // Tick loop for smooth progress bar animation. Every ~100 ms we
        // repaint the view. The task dies when the entity is dropped (window
        // closed). Panic from a dropped entity is swallowed by the runtime.
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(100))
                    .await;
                let _ = this.update(cx, |_, cx| cx.notify());
            }
        })
        .detach();

        Self {
            first_seen: HashMap::new(),
        }
    }
}

impl Render for NotificationsView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let notifications = &cx.global::<NotificationPopupState>().current.notifications;
        let theme = Theme::global(cx);

        if notifications.is_empty() {
            return div().into_any_element();
        }

        let now_ms = now_epoch_ms();

        // Prune stale entries — only keep ids still in the active set,
        // then record first-seen for new arrivals.
        let active: std::collections::HashSet<u32> =
            notifications.iter().map(|n| n.id).collect();
        self.first_seen.retain(|id, _| active.contains(id));
        for n in notifications {
            self.first_seen.entry(n.id).or_insert(now_ms);
        }

        let cards: Vec<gpui::AnyElement> = notifications
            .iter()
            .map(|n| render_toast_card(n, &theme, now_ms, &self.first_seen).into_any_element())
            .collect();

        // Card stack — no outer border (mockup: each card is independent).
        div()
            .window_font(theme)
            .flex_col()
            .gap(px(8.))
            .max_h(px(LIST_MAX_H))
            .overflow_hidden()
            .children(cards)
            .into_any_element()
    }
}

// ── Color / string helpers ───────────────────────────────────────────

/// Monogram icon bg color — hash `app_name`'s first byte into a stable
/// palette (identical to `history_popup/view.rs` copy).
fn monogram_color(app_name: &str) -> Hsla {
    const PALETTE: [u32; 8] = [
        0x89b4faff, 0xa6e3a1ff, 0xf38ba8ff, 0xcba6f7ff, 0x89dcebff, 0xfab387ff, 0xa6e3a1ff,
        0x45475aff,
    ];
    if app_name.is_empty() {
        return Hsla::from(gpui::rgba(0x45475aff));
    }
    let first = app_name.as_bytes()[0] as usize;
    Hsla::from(gpui::rgba(PALETTE[first % PALETTE.len()]))
}

/// One- or two-letter initials from `app_name` (mockup style "Z", "M").
fn app_initials(app_name: &str) -> String {
    let trimmed = app_name.trim();
    if trimmed.is_empty() {
        return "?".to_string();
    }
    let mut up = trimmed.chars().filter(|c| c.is_alphabetic());
    match (up.next(), up.next()) {
        (Some(a), Some(b)) if a.is_uppercase() && b.is_uppercase() => format!("{a}{b}"),
        (Some(a), _) => a.to_uppercase().to_string(),
        (None, _) => trimmed.chars().next().unwrap_or('?').to_string(),
    }
}

// ── Shared card renderer (legacy, for history-popup compat) ──────────

/// Render a single notification card (header + summary + body + action buttons).
///
/// Maintained for backward compatibility (the history popup may reference
/// this via `crate::notifications::view::render_notification_card`). The
/// ephemeral toast stack uses `render_toast_card` instead.
///
/// All `on_click` callbacks capture `n.id` and use the runtime-provided `cx`, so
/// this function needs no external `Context` borrow — it composes freely inside
/// an iterator over a `cx.global()` snapshot.
pub(crate) fn render_notification_card(
    n: &Notification,
    theme: &Theme,
    close_button: Option<gpui::AnyElement>,
) -> gpui::AnyElement {
    let accent = match n.urgency {
        Urgency::Critical => theme.status.error,
        Urgency::Normal => theme.status.warning,
        Urgency::Low => theme.status.info,
    };

    let bg_primary = theme.bg.primary;
    let text_primary = theme.text.primary;
    let text_secondary = theme.text.secondary;
    let text_muted = theme.text.muted;
    let bg_secondary = theme.bg.secondary;
    let radius = theme.radius;
    let radius_lg = theme.radius_lg;

    // Header: app name (left) + optional close button (right).
    let mut header = div().flex().justify_between().items_start().child(
        div()
            .text_color(text_secondary)
            .text_xs()
            .child(n.app_name.clone()),
    );
    if let Some(btn) = close_button {
        header = header.child(btn);
    }

    let title = div()
        .text_color(text_primary)
        .font_weight(FontWeight::BOLD)
        .child(n.summary.clone());

    // Body: hard-clipped so a long body truncates instead of overflowing.
    let content = div()
        .max_h(px(crate::notifications::BODY_MAX_H))
        .overflow_hidden()
        .text_color(text_muted)
        .child(n.body.clone());

    let mut card: gpui::Div = div()
        .flex_col()
        .gap(px(4.))
        .p(px(12.))
        .rounded(radius_lg)
        .bg(bg_primary)
        .border_l_3()
        .border_color(accent)
        .child(header)
        .child(title)
        .child(content);

    // Action buttons — each dispatches InvokeAction(key).
    if !n.actions.is_empty() {
        let id = n.id;
        let buttons: Vec<gpui::AnyElement> = n
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
                        })
                        .detach();
                    })
                    .child(label)
                    .into_any_element()
            })
            .collect();

        card = card.child(div().flex().flex_wrap().gap(px(6.)).children(buttons));
    }

    card.into_any_element()
}
