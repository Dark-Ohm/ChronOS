//! T279 / Slice A2 — honest placeholder shells for Slice B/C tabs.
//!
//! Plan/Tools/Skills/ContextFiles/Archive each need a tab *body* rendered
//! inside the content canvas, but their full implementations are Slice B/C.
//! Rather than paint an empty opaque panel (the plan explicitly forbids
//! "empty opaque panel" — Step 1 test: "every B/C rail button renders an
//! honest shell, never an empty opaque panel"), each shell states the
//! feature name and its target slice. No stores, no IPC, no window — a
//! `ShellTab` is a pure render entity carrying which `LeftTab` it speaks for.
//!
//! Width policy still travels with the tab: Plan and Context Files obey the
//! resizable policy; Tools, Skills, and Archive use fixed widths. The rail
//! and `select_tab` already enforce that via `LeftTab::is_resizable` and
//! `width_for_open`, so the shell itself does not re-derive widths.

use gpui::{Context, IntoElement, Render, Window, div, prelude::*, px};

use chronos_ui::Theme;

use crate::side_panel_left::tabs::LeftTab;

/// A labelled placeholder for a tab whose implementation is Slice B or C.
///
/// `tab` is the `LeftTab` this shell stands in for; the render reads it for
/// the label and target slice so a single entity shape serves every
/// not-yet-built tab.
pub struct ShellTab {
    tab: LeftTab,
}

impl ShellTab {
    pub fn new(tab: LeftTab) -> Self {
        Self { tab }
    }

    /// Which `LeftTab` this shell represents.
    pub fn tab(&self) -> LeftTab {
        self.tab
    }

    /// Honest slice label — "Coming in Slice B" or "Coming in Slice C".
    /// Plan and Context Files are Slice B; Archive is Slice C (per the
    /// plan's delivery table: T279 A2 ships shells, T281 A4 live, Archive
    /// full scope is B/C). Tools and Skills are Slice C.
    fn coming_label(&self) -> &'static str {
        match self.tab {
            LeftTab::Plan | LeftTab::ContextFiles => "Coming in Slice B",
            LeftTab::Tools | LeftTab::Skills | LeftTab::Archive => "Coming in Slice C",
            // Project/Sessions/Chat are real tabs (Slice A2), not shells —
            // a ShellTab should never be constructed for them. Render a
            // diagnostic if it ever is, rather than a misleading label.
            _ => "unreachable: ShellTab for a real tab",
        }
    }
}

impl Render for ShellTab {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = Theme::global(cx);
        div()
            .id(("left-shell", self.tab as usize))
            .size_full()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(px(8.))
            .child(
                div()
                    .text_size(px(16.))
                    .text_color(theme.text.primary)
                    .child(self.tab.label()),
            )
            .child(
                div()
                    .text_size(theme.font_sizes.sm)
                    .text_color(theme.text.muted)
                    .child(self.coming_label()),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every B/C tab must report a real "Coming in Slice …" label, never
    /// the unreachable placeholder.
    #[test]
    fn shell_tabs_report_honest_slice() {
        for tab in [
            LeftTab::Plan,
            LeftTab::Tools,
            LeftTab::Skills,
            LeftTab::ContextFiles,
            LeftTab::Archive,
        ] {
            let shell = ShellTab::new(tab);
            let label = shell.coming_label();
            assert!(
                label.starts_with("Coming in Slice"),
                "{tab:?} label is {label:?} — must be an honest slice label"
            );
            assert_ne!(
                label, "unreachable: ShellTab for a real tab",
                "{tab:?} must not report the unreachable placeholder"
            );
        }
    }

    /// A ShellTab constructed for a real tab (Project/Sessions/Chat) reports
    /// the unreachable sentinel — this is the guard, not a valid state.
    #[test]
    fn real_tab_shell_reports_unreachable() {
        for tab in [LeftTab::Project, LeftTab::Sessions, LeftTab::Chat] {
            let shell = ShellTab::new(tab);
            assert_eq!(
                shell.coming_label(),
                "unreachable: ShellTab for a real tab",
                "{tab:?} should be a real tab, not a shell"
            );
        }
    }

    /// `tab()` round-trips the constructor — guards against a future field
    /// rename silently breaking the accessor.
    #[test]
    fn tab_accessor_roundtrips() {
        let shell = ShellTab::new(LeftTab::Plan);
        assert_eq!(shell.tab(), LeftTab::Plan);
    }
}