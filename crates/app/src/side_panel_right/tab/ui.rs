//! Shared visual primitives for the right-panel tabs — the T231 pattern
//! (bar settings redesign, spread to sibling tabs afterwards):
//!
//! - `elevated_card` — the theme's popup-elevation underlay on `bg.elevated`,
//!   so a tab reads as a product surface, not a flat debug list.
//! - `section_header` — accent tick + semibold title + muted mono subtitle,
//!   the section-vs-setting hierarchy introduced in T231.
//! - `setting_label` / `setting_row` — the label (primary) + mono path
//!   (muted) pair and its one-baseline row.
//! - `GRID_BREAKPOINT` / `is_wide` — the responsive switch the T231 grids use.
//!   It reads the visible slice of T276's fixed canvas, never Wayland bounds.
//! - `empty_state_hero` / `empty_state_note` — the T252 empty-state canon
//!   (DECISIONS.log 2026-08-13, materialized in T269): hero for a fully empty
//!   surface, note for one empty section inside a live tab.

use chronos_ui::{Theme, elevation_apply_light_chrome};
use gpui::{
    AnyElement, App, ClickEvent, Div, ElementId, FontWeight, InteractiveElement, IntoElement,
    ParentElement, SharedString, StatefulInteractiveElement, Styled, Window, div, px, svg,
};

/// Panel width at which grids switch from 1 column to 2+.
/// `DEFAULT_CONTENT_WIDTH` (560) stays single-column; `MAX_WIDTH` (960) is
/// comfortably above the breakpoint (T231 §1).
pub(crate) const GRID_BREAKPOINT: f32 = 720.0;

pub(crate) fn is_wide_content_width(visible_width: f32) -> bool {
    visible_width >= GRID_BREAKPOINT
}

/// `true` when the visible content slice is stretched past `GRID_BREAKPOINT`.
/// T276 keeps the Wayland content surface permanently at 920px, so
/// `window.bounds()` is not the responsive viewport anymore.
pub(crate) fn is_wide(cx: &App) -> bool {
    let visible_width = cx
        .try_global::<crate::side_panel_right::SidePanelRightState>()
        .map(|state| crate::side_panel_right::visible_content_width(state.width))
        .unwrap_or_else(|| {
            crate::side_panel_right::visible_content_width(
                crate::side_panel_right::DEFAULT_CONTENT_WIDTH,
            )
        });
    is_wide_content_width(visible_width)
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

// ── Empty-state pattern (T252 canon, T269) ──────────────────────────────────

/// Severity of an empty-state message (T252 matrix: empty ≠ error).
///
/// `Muted` for expected emptiness; `Error` only when the source is broken
/// (e.g. Terminal spawn failure, unreadable directory). The one sanctioned
/// Error-on-empty is HyprBinds — 0 binds provably means a broken config —
/// and copying that to other tabs needs the same level of justification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NoteSeverity {
    Muted,
    Error,
}

impl NoteSeverity {
    fn color(self, theme: &Theme) -> gpui::Hsla {
        match self {
            NoteSeverity::Muted => theme.text.muted,
            NoteSeverity::Error => theme.status.error,
        }
    }
}

/// Hero empty state — the whole tab surface is empty (T252 canon, extracted
/// from `EmptyTab`, which now renders through this helper): 40px icon at
/// `text.muted.opacity(0.55)`, 13px semibold title in `text.primary`, 11.5px
/// centered hint, 12px gap, optional action link (muted → primary on hover,
/// the «Files» link in Preview's empty state is the sample).
///
/// `icon_path` arrives ready-made: the caller passes its tab's
/// `PanelTab::icon_path()` (so an empty Library and an unimplemented Scenes
/// read as one family) or a deliberately contextual icon (Preview's
/// `icons/folder.svg`, Terminal Failed's `icons/rail-terminal.svg` — the
/// architect-sanctioned variations). The helper never invents icons.
///
/// `hint_severity` colors the hint: `Error` is for a genuine refusal (hero +
/// `status.error` + recovery per the T252 matrix), never for an expected
/// empty result. The hint must answer "where does content here come from".
///
/// A hero without a title is meaningless — `debug_assert`ed.
pub(crate) fn empty_state_hero(
    theme: Theme,
    icon_path: &str,
    title: &str,
    hint: &str,
    hint_severity: NoteSeverity,
    action: Option<(
        SharedString,
        Box<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>,
    )>,
) -> AnyElement {
    debug_assert!(
        !title.is_empty(),
        "empty_state_hero: a hero without a title is meaningless"
    );
    let action = action.map(|(label, on_click)| {
        div()
            .id(ElementId::Name(
                format!("empty-state-action-{label}").into(),
            ))
            .cursor_pointer()
            .text_size(px(11.5))
            .text_color(theme.text.muted)
            .hover(|s| s.text_color(theme.text.primary))
            .on_click(on_click)
            .child(label)
    });
    let mut hero = div()
        .size_full()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap(px(12.))
        .child(
            svg()
                .path(icon_path)
                .size(px(40.))
                .text_color(theme.text.muted.opacity(0.55)),
        )
        .child(
            div()
                .text_size(px(13.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(theme.text.primary)
                .child(title.to_string()),
        )
        .child(
            div()
                .text_size(px(11.5))
                .text_color(hint_severity.color(&theme))
                .text_center()
                .child(hint.to_string()),
        );
    if let Some(link) = action {
        hero = hero.child(link);
    }
    hero.into_any_element()
}

/// Inline empty state — one empty section/list inside a live tab (T252 canon,
/// extracted from the Files/HyprBinds inline rows): `px(10)`/`py(16)`, 12px
/// text, color by severity. There is no bordered variant — BarSettings' old
/// bordered-xs was drift, not a third canon.
pub(crate) fn empty_state_note(
    theme: Theme,
    message: &str,
    severity: NoteSeverity,
) -> AnyElement {
    div()
        .px(px(10.))
        .py(px(16.))
        .text_size(px(12.))
        .text_color(severity.color(&theme))
        .child(message.to_string())
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

    #[test]
    fn breakpoint_uses_visible_slice_not_fixed_wayland_canvas() {
        // A narrow tab is judged by its VISIBLE slice (width − rail), not by
        // the fixed Wayland canvas every content window is allocated.
        let launcher_visible = crate::side_panel_right::visible_content_width(
            crate::side_panel_right::tabs::PanelTab::LauncherSettings.preferred_content_width(),
        );
        assert_eq!(launcher_visible, 370.0);
        assert!(!is_wide_content_width(launcher_visible));
        // System settings was widened to 800 on 2026-08-20 precisely so its
        // visible slice clears the breakpoint and the two-column grids on
        // that page (theme picker, Hypr modules) are reachable at all.
        let settings_visible = crate::side_panel_right::visible_content_width(
            crate::side_panel_right::tabs::PanelTab::EditorSettings.preferred_content_width(),
        );
        assert_eq!(settings_visible, 760.0);
        assert!(is_wide_content_width(settings_visible));
        assert!(is_wide_content_width(crate::side_panel_right::CONTENT_CANVAS_WIDTH));
    }

    // --- T269: empty-state helpers ---

    #[test]
    #[should_panic(expected = "empty_state_hero: a hero without a title")]
    fn hero_without_a_title_panics() {
        let _ = empty_state_hero(
            Theme::default(),
            "icons/rail-library.svg",
            "",
            "hint",
            NoteSeverity::Muted,
            None,
        );
    }

    #[test]
    fn hero_with_a_title_constructs() {
        // Smoke: the canonical call shape (tab icon + tab label + description,
        // no action) must build without panicking.
        let _ = empty_state_hero(
            Theme::default(),
            "icons/rail-library.svg",
            "Library",
            "List, pin and launch detected games",
            NoteSeverity::Muted,
            None,
        );
        let _ = empty_state_hero(
            Theme::default(),
            "icons/rail-terminal.svg",
            "Terminal is unavailable",
            "spawn failed: permission denied",
            NoteSeverity::Error,
            Some(("restart".into(), Box::new(|_, _, _| {}))),
        );
    }

    #[test]
    fn note_severity_maps_to_theme_tokens() {
        let theme = Theme::default();
        assert_eq!(NoteSeverity::Muted.color(&theme), theme.text.muted);
        assert_eq!(NoteSeverity::Error.color(&theme), theme.status.error);
        let _ = empty_state_note(theme, "Directory is empty", NoteSeverity::Muted);
    }
}
