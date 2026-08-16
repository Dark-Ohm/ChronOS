//! Shared notification-history renderer — used by both the (transient)
//! history popup and the right-panel Notifications tab (T293).
//!
//! This module owns the card geometry, the urgency/monogram helpers, and the
//! list+footer renderer. The popup wraps it in a chrome panel (border, blur,
//! glow); the tab wraps it in the right panel's scroll viewport. Neither
//! side re-implements the card — the canon lives here.

use gpui::{AnyElement, App, IntoElement, Window, div, prelude::*, px};

use chronos_services::{Notification, NotificationCommand, Service, Urgency};

use crate::state::AppState;

// ── Mockup-faithful geometry (shared) ──────────────────────────────

pub const PADDING: f32 = 10.;
pub const FOOTER_PAD: f32 = 12.;
pub const FOOTER_BTN_PY: f32 = 8.;
pub const URGENCY_STRIP_W: f32 = 3.;
pub const URGENCY_STRIP_MY: f32 = 10.;
pub const URGENCY_STRIP_ML: f32 = 8.;
pub const MONOGRAM_SIZE: f32 = 16.;
pub const MONOGRAM_RADIUS: f32 = 4.;
pub const APP_NAME_FZ: f32 = 10.5;
pub const SUMMARY_FZ: f32 = 12.5;
pub const BODY_FZ: f32 = 11.5;
pub const ACTIONS_FZ: f32 = 11.;
pub const BTN_PAD_X: f32 = 11.;
pub const BTN_PAD_Y: f32 = 5.;
pub const BTN_GAP: f32 = 6.;
pub const ROW_DISMISS_BTN: f32 = 18.;
pub const ROW_DISMISS_RADIUS: f32 = 5.;
pub const FOOTER_BTN_RADIUS: f32 = 6.;
pub const FOOTER_BTN_PY_OUTER: f32 = 8.;
pub const EMPTY_PY: f32 = 36.;
pub const EMPTY_FZ: f32 = 12.;

/// Render the notification history list (newest-first) with a "Clear all"
/// footer when `len > 1`, or a centered "No notifications" empty state.
///
/// The list itself does not scroll — the caller wraps it in a scroll
/// container appropriate to its surface (popup chrome caps at MAX_LIST_H,
/// the right-panel tab scrolls on the full canvas height).
pub fn render_history_list(window: &mut Window, cx: &mut App) -> AnyElement {
    let state = AppState::notification(cx).get();
    let ordered: Vec<Notification> = state.history.iter().rev().cloned().collect();

    let theme = *chronos_ui::Theme::global(cx);
    let text_muted = theme.text.muted;
    let font_mono = theme.font_mono;

    // ── Body: scroll list OR empty state ─────────────────────────
    let body: AnyElement = if ordered.is_empty() {
        div()
            .w_full()
            .py(px(EMPTY_PY))
            .text_color(text_muted)
            .font_family(font_mono)
            .text_size(px(EMPTY_FZ))
            .flex()
            .items_center()
            .justify_center()
            .child("No notifications")
            .into_any_element()
    } else {
        let cards: Vec<AnyElement> = ordered
            .iter()
            .map(|n| render_history_card(n, window, cx))
            .collect();
        div()
            .id("notif-history-list")
            .w_full()
            .flex_col()
            .children(cards)
            .into_any_element()
    };

    // ── Footer: Clear all (mockup: `len > 1`) ────────────────────
    let footer: AnyElement = if ordered.len() > 1 {
        let text_secondary = theme.text.secondary;
        let accent = theme.accent.primary;
        div()
            .w_full()
            .px(px(FOOTER_PAD))
            .py(px(FOOTER_PAD))
            .child(
                div()
                    .id("notif-history-clear-all")
                    .w_full()
                    .flex()
                    .items_center()
                    .justify_center()
                    .cursor_pointer()
                    .py(px(FOOTER_BTN_PY_OUTER))
                    .rounded(px(FOOTER_BTN_RADIUS))
                    .border_1()
                    .border_color(accent)
                    .text_color(accent)
                    .font_family(font_mono)
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_size(px(SUMMARY_FZ))
                    .hover(|s| s.border_color(accent).text_color(accent))
                    .child("Clear all")
                    .on_click({
                        move |_event, _window, cx: &mut App| {
                            let svc = AppState::notification(cx).clone();
                            cx.background_spawn(async move {
                                let _ = svc.dispatch(NotificationCommand::ClearHistory).await;
                            })
                            .detach();
                        }
                    }),
            )
            .into_any_element()
    } else {
        div().into_any_element()
    };

    // The outer column is NOT scrollable — the caller (popup chrome or
    // tab) wraps this in a scroll container so the list can fill the
    // available space without imposing its own scroll policy.
    div()
        .w_full()
        .flex_col()
        .child(body)
        .child(footer)
        .into_any_element()
}

/// Render a single history card (urgency strip + monogram + app name +
/// summary + body + actions + row dismiss).
pub fn render_history_card(
    n: &Notification,
    _window: &mut Window,
    cx: &mut App,
) -> AnyElement {
    let theme = *chronos_ui::Theme::global(cx);
    let text_primary = theme.text.primary;
    let text_muted = theme.text.muted;
    let text_secondary = theme.text.secondary;
    let hover = theme.interactive.hover;
    let accent = theme.accent.primary;
    let border_subtle = theme.border.subtle;
    let font_mono = theme.font_mono;
    let _radius = theme.radius;

    let urgency_color = urgency_hsla(n.urgency);
    let icon_color = monogram_color(&n.app_name);
    let initials = app_initials(&n.app_name);
    let app_id = n.id;

    // ── Urgency strip (3px, accent per urgency) ────────────────────
    let strip = div()
        .flex_none()
        .w(px(URGENCY_STRIP_W))
        .my(px(URGENCY_STRIP_MY))
        .ml(px(URGENCY_STRIP_ML))
        .rounded(px(URGENCY_STRIP_W))
        .bg(urgency_color);

    // ── Monogram (16x16, first letter(s), colored bg) ─────────────
    let monogram = div()
        .flex_none()
        .w(px(MONOGRAM_SIZE))
        .h(px(MONOGRAM_SIZE))
        .rounded(px(MONOGRAM_RADIUS))
        .bg(icon_color)
        .flex()
        .items_center()
        .justify_center()
        .text_color(gpui::Hsla::from(gpui::rgba(0x11111_bff)))
        .font_family(font_mono)
        .font_weight(gpui::FontWeight::SEMIBOLD)
        .text_size(px(9.))
        .child(initials);

    let app_name = div()
        .flex_1()
        .min_w(px(0.))
        .text_color(text_muted)
        .font_family(font_mono)
        .text_size(px(APP_NAME_FZ))
        .whitespace_nowrap()
        .overflow_hidden()
        .text_ellipsis()
        .child(n.app_name.clone());

    // ── Row dismiss ✕ (history-only delete) ───────────────────────
    let dismiss_btn = div()
        .id(format!("notif-history-row-dismiss-{app_id}"))
        .flex_none()
        .w(px(ROW_DISMISS_BTN))
        .h(px(ROW_DISMISS_BTN))
        .rounded(px(ROW_DISMISS_RADIUS))
        .flex()
        .items_center()
        .justify_center()
        .text_color(text_muted)
        .cursor_pointer()
        .hover(|s| s.bg(hover).text_color(text_primary))
        .child("✕")
        .on_click(move |_event, _window, cx: &mut App| {
            let svc = AppState::notification(cx).clone();
            cx.background_spawn(async move {
                let _ = svc
                    .dispatch(NotificationCommand::RemoveFromHistory(app_id))
                    .await;
            })
            .detach();
        });

    let header_row = div()
        .w_full()
        .flex()
        .items_center()
        .gap(px(6.))
        .child(monogram)
        .child(app_name)
        .child(dismiss_btn);

    let summary = div()
        .mt(px(5.))
        .text_color(text_primary)
        .font_family(font_mono)
        .font_weight(gpui::FontWeight::SEMIBOLD)
        .text_size(px(SUMMARY_FZ))
        .child(n.summary.clone());

    let mut card_body: Vec<AnyElement> =
        vec![header_row.into_any_element(), summary.into_any_element()];

    if !n.body.is_empty() {
        card_body.push(
            div()
                .mt(px(3.))
                .text_color(text_secondary)
                .font_family(font_mono)
                .text_size(px(BODY_FZ))
                .line_height(px(BODY_FZ * 1.45))
                .max_h(px(BODY_FZ * 1.45 * 4.))
                .overflow_hidden()
                .child(n.body.clone())
                .into_any_element(),
        );
    }

    if !n.actions.is_empty() {
        let action_buttons: Vec<AnyElement> = n
            .actions
            .iter()
            .map(|(key, label)| {
                let app_id_c = app_id;
                let key_c = key.clone();
                div()
                    .id(format!("notif-history-action-{app_id_c}-{key_c}"))
                    .cursor_pointer()
                    .px(px(BTN_PAD_X))
                    .py(px(BTN_PAD_Y))
                    .rounded(px(MONOGRAM_RADIUS))
                    .border_1()
                    .border_color(border_subtle)
                    .text_color(text_secondary)
                    .font_family(font_mono)
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_size(px(ACTIONS_FZ))
                    .hover(|s| s.border_color(accent).text_color(accent))
                    .child(label.clone())
                    .on_click(move |_event, _window, cx: &mut App| {
                        let svc = AppState::notification(cx).clone();
                        let key = key_c.clone();
                        cx.background_spawn(async move {
                            let _ = svc
                                .dispatch(NotificationCommand::InvokeAction(app_id_c, key))
                                .await;
                        })
                        .detach();
                    })
                    .into_any_element()
            })
            .collect();
        card_body.push(
            div()
                .mt(px(9.))
                .flex()
                .gap(px(BTN_GAP))
                .children(action_buttons)
                .into_any_element(),
        );
    }

    let card_inner = div().w_full().flex().child(strip).child(
        div()
            .flex_1()
            .min_w(px(0.))
            .py(px(PADDING))
            .pr(px(PADDING))
            .pl(px(PADDING))
            .flex_col()
            .children(card_body),
    );

    div()
        .w_full()
        .flex()
        .border_b_1()
        .border_color(border_subtle)
        .child(card_inner)
        .into_any_element()
}

// ── Color / string helpers ──────────────────────────────────────────

/// Urgency strip color from the mockup URGENCY_COLORS map.
pub fn urgency_hsla(u: Urgency) -> gpui::Hsla {
    match u {
        Urgency::Low => gpui::Hsla::from(gpui::rgba(0x6c7086ff)),
        Urgency::Normal => gpui::Hsla::from(gpui::rgba(0xf9e2_afff)),
        Urgency::Critical => gpui::Hsla::from(gpui::rgba(0xf38ba8ff)),
    }
}

/// Monogram bg color — hash `app_name`'s first byte into the mockup's
/// palette so a per-app color is stable across renders without carrying
/// palette state. Falls back to accent if empty.
pub fn monogram_color(app_name: &str) -> gpui::Hsla {
    const PALETTE: [u32; 8] = [
        0x89b4faff, 0xa6e3a1ff, 0xf38ba8ff, 0xcba6f7ff, 0x89dcebff, 0xfab387ff, 0xa6e3a1ff,
        0x45475aff,
    ];
    if app_name.is_empty() {
        return gpui::Hsla::from(gpui::rgba(0x45475aff));
    }
    let first = app_name.as_bytes()[0] as usize;
    gpui::Hsla::from(gpui::rgba(PALETTE[first % PALETTE.len()]))
}

/// One- or two-letter initials from `app_name` (mockup style "Z", "M").
pub fn app_initials(app_name: &str) -> String {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn urgency_colors_match_mockup() {
        let low = urgency_hsla(Urgency::Low);
        let normal = urgency_hsla(Urgency::Normal);
        let crit = urgency_hsla(Urgency::Critical);
        assert_ne!(low, normal);
        assert_ne!(normal, crit);
        assert_ne!(low, crit);
    }

    #[test]
    fn initials_single_word() {
        assert_eq!(app_initials("Zed"), "Z");
        assert_eq!(app_initials("Mail"), "M");
        assert_eq!(app_initials("System"), "S");
    }

    #[test]
    fn initials_acronym_two_uppercase() {
        assert_eq!(app_initials("OS"), "OS");
        assert_eq!(app_initials("UI"), "UI");
    }

    #[test]
    fn initials_empty_safe() {
        assert_eq!(app_initials(""), "?");
        assert_eq!(app_initials("   "), "?");
    }

    #[test]
    fn monogram_color_stable_per_name() {
        let a = monogram_color("Zed");
        let b = monogram_color("Zed");
        assert_eq!(a, b, "same name → same color");
        let c = monogram_color("Mail");
        assert_ne!(a, c, "different names usually differ");
    }
}
