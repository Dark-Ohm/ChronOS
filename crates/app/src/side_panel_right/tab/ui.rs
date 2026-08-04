//! Shared visual primitives for the right-panel tabs — the T231 pattern
//! (bar settings redesign, spread to sibling tabs afterwards):
//!
//! - `elevated_card` — the theme's popup-elevation underlay on `bg.elevated`,
//!   so a tab reads as a product surface, not a flat debug list.
//! - `section_header` — accent tick + semibold title + muted mono subtitle,
//!   the section-vs-setting hierarchy introduced in T231.
//! - `setting_label` / `setting_row` — the label (primary) + mono path
//!   (muted) pair and its one-baseline row.
//! - `GRID_BREAKPOINT` / `is_wide` — the responsive switch the T231 grids use:
//!   default docked width stays 1 column, stretched panels go 2+.

use chronos_ui::{Theme, elevation_apply_light_chrome};
use gpui::{
    AnyElement, Div, FontWeight, IntoElement, ParentElement, SharedString, Styled, Window, div,
    px,
};

/// Panel width at which grids switch from 1 column to 2+.
/// `DEFAULT_CONTENT_WIDTH` (560) stays single-column; `MAX_WIDTH` (960) is
/// comfortably above the breakpoint (T231 §1).
pub(crate) const GRID_BREAKPOINT: f32 = 720.0;

/// `true` when the panel is stretched past `GRID_BREAKPOINT`.
pub(crate) fn is_wide(window: &Window) -> bool {
    window.bounds().size.width.as_f32() >= GRID_BREAKPOINT
}

/// Elevated card underlay wrapping a tab's scrollable content.
///
/// NOTE: `.id()` must be applied by the caller AFTER this helper — it upgrades
/// the returned bare `Div` into a `Stateful<Div>`, which is what lets the
/// scroll container track state across re-renders.
pub(crate) fn elevated_card(theme: Theme) -> Div {
    let elev = theme.elevation_popup();
    let card = div()
        .relative()
        .w_full()
        .flex()
        .flex_col()
        .gap(px(16.))
        .px(px(16.))
        .py(px(16.))
        .bg(theme.bg.elevated)
        .border_1()
        .border_color(theme.border.subtle)
        .rounded(elev.radius)
        .shadow(elev.shadows.to_vec());
    elevation_apply_light_chrome(&elev, card)
}

/// Section header: accent tick + semibold title + muted mono subtitle.
/// Distinct from setting labels (T231 §2 — visual hierarchy).
pub(crate) fn section_header(theme: Theme, title: &str, subtitle: &str) -> AnyElement {
    let title = SharedString::from(title);
    let subtitle = SharedString::from(subtitle);
    div()
        .w_full()
        .flex_col()
        .gap(px(4.))
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(6.))
                .child(
                    div()
                        .w(px(3.))
                        .h(px(12.))
                        .rounded(px(1.5))
                        .bg(theme.accent.primary.opacity(0.85)),
                )
                .child(
                    div()
                        .text_color(theme.text.primary)
                        .text_size(px(12.5))
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(title),
                ),
        )
        .child(
            div()
                .text_color(theme.text.muted)
                .text_xs()
                .font_family(theme.font_mono)
                .child(subtitle),
        )
        .into_any_element()
}

/// Setting label: primary label + muted mono path (the `*.toml` / `*.*` key).
pub(crate) fn setting_label(theme: Theme, label: &str, path: &str) -> AnyElement {
    let label = SharedString::from(label);
    let path = SharedString::from(path);
    div()
        .flex_col()
        .gap(px(1.))
        .child(
            div()
                .text_color(theme.text.primary)
                .text_size(px(11.))
                .font_weight(FontWeight::MEDIUM)
                .child(label),
        )
        .child(
            div()
                .text_color(theme.text.muted)
                .text_xs()
                .font_family(theme.font_mono)
                .child(path),
        )
        .into_any_element()
}

/// One grid/flex cell: label · control, laid out on one baseline row.
pub(crate) fn setting_row(label: AnyElement, control: AnyElement) -> AnyElement {
    div()
        .w_full()
        .flex()
        .items_center()
        .justify_between()
        .gap(px(10.))
        .child(label)
        .child(control)
        .into_any_element()
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn breakpoint_keeps_default_width_single_column() {
        assert!(
            GRID_BREAKPOINT > crate::side_panel_right::DEFAULT_CONTENT_WIDTH,
            "2-col grid would squeeze the default 560px panel"
        );
        assert!(
            GRID_BREAKPOINT <= 960.0,
            "breakpoint must be reachable at MAX_WIDTH"
        );
    }
}
