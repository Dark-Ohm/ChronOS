//! Shared rendering pieces for the Updates tab's pending-package list
//! (T294). The layout was extracted from the deleted updates popup
//! (`updates_popup/view.rs`) — same mockup geometry (`design/Updates
//! Popup.dc.html`), not re-derived by eye. The list now lives on the right
//! panel's Updates tab; there is exactly one consumer, so the markup lives
//! in exactly one place (`tab/updates.rs`) and these are the shared,
//! stateless bits: geometry constants, the pure cell builders, and the
//! fixed AUR hint text.
//!
//! T294 contract: apply is ALWAYS pacman (official repos only). AUR rows are
//! display-only — they are NOT selectable (hover reveals a `yay` hint; click
//! does nothing). Only Official rows enter the selection set that feeds
//! "Upgrade selected".

use chronos_services::{PackageUpdate, UpdateSource};
use gpui::{AnyElement, IntoElement, ParentElement, Styled, px};

// ── Geometry from mockup ────────────────────────────────────────────
pub(crate) const HEADER_PY: f32 = 12.;
pub(crate) const HEADER_PX: f32 = 14.;
pub(crate) const ROW_PY: f32 = 9.;
pub(crate) const ROW_PX: f32 = 14.;
pub(crate) const FOOTER_PY: f32 = 12.;
pub(crate) const FOOTER_PX: f32 = 14.;
pub(crate) const BTN_PY: f32 = 8.;

/// Width of the left gutter that carries the selection indicator (px).
/// Mockup-fixed: smaller reads as noise, larger eats the name column.
/// Task constraint: <= 18px (T119 §1).
pub(crate) const SELECTION_GUTTER: f32 = 16.;

/// Fixed AUR hint (EN, like the rest of the UI) shown on hover. Meaning is
/// fixed by T294 — do not reword.
pub(crate) const AUR_HINT_LINE1: &str = "AUR package — install updates in a terminal with yay.";
pub(crate) const AUR_HINT_LINE2: &str = "Example: yay -S <name>";

/// Whether this update is the AUR "display-only" source.
pub(crate) fn is_aur(update: &PackageUpdate) -> bool {
    matches!(update.source, UpdateSource::Aur)
}

// ── Pure cell builders (stateless — the tab wires interactions) ──────

/// 10px accent-filled square when selected; 10px outlined square when not
/// (same outer footprint in both states so the version column never shifts
/// left/right as you toggle). T119 §1 gutter <= 16px.
pub(crate) fn selection_indicator(
    is_selected: bool,
    accent: gpui::Hsla,
    text_muted: gpui::Hsla,
) -> AnyElement {
    gpui::div()
        .flex_none()
        .w(px(SELECTION_GUTTER))
        .flex()
        .items_center()
        .justify_center()
        .child(if is_selected {
            gpui::div()
                .w(px(10.))
                .h(px(10.))
                .rounded(px(2.))
                .bg(accent)
                .into_any_element()
        } else {
            gpui::div()
                .w(px(10.))
                .h(px(10.))
                .rounded(px(2.))
                .border_1()
                .border_color(text_muted)
                .into_any_element()
        })
        .into_any_element()
}

/// Package name, mono, ellipsized.
pub(crate) fn name_cell(update: &PackageUpdate, text_primary: gpui::Hsla, font_mono: &'static str) -> AnyElement {
    gpui::div()
        .flex_1()
        .min_w(px(0.))
        .text_color(text_primary)
        .font_family(font_mono)
        .text_size(px(12.5))
        .font_weight(gpui::FontWeight::MEDIUM)
        .whitespace_nowrap()
        .overflow_hidden()
        .text_ellipsis()
        .child(update.name.clone())
        .into_any_element()
}

/// Small lavender "AUR" chip; empty div for official rows (keeps the row's
/// horizontal layout identical across sources).
pub(crate) fn aur_badge(is_aur: bool, radius: gpui::Pixels, font_mono: &'static str) -> AnyElement {
    if is_aur {
        gpui::div()
            .flex_none()
            .rounded(radius)
            .px(px(5.))
            .py(px(1.))
            .border_1()
            .border_color(gpui::Hsla::from(gpui::rgba(0xcb_a6_f74d)))
            .bg(gpui::Hsla::from(gpui::rgba(0xcb_a6_f71f)))
            .text_color(gpui::Hsla::from(gpui::rgba(0xcb_a6_f7ff)))
            .font_family(font_mono)
            .text_size(px(9.5))
            .font_weight(gpui::FontWeight::SEMIBOLD)
            .child("AUR")
            .into_any_element()
    } else {
        gpui::div().into_any_element()
    }
}

/// Name column `old → new` (old muted, new secondary).
pub(crate) fn versions(
    update: &PackageUpdate,
    text_muted: gpui::Hsla,
    text_secondary: gpui::Hsla,
    font_mono: &'static str,
) -> AnyElement {
    gpui::div()
        .flex_none()
        .font_family(font_mono)
        .text_size(px(11.))
        .flex()
        .items_center()
        .gap(px(5.))
        .child(
            gpui::div()
                .text_color(text_muted)
                .child(update.old_version.clone()),
        )
        .child(gpui::div().text_color(text_muted).child("→"))
        .child(
            gpui::div()
                .text_color(text_secondary)
                .child(update.new_version.clone()),
        )
        .into_any_element()
}