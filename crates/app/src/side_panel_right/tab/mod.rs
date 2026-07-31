//! Tab content registry — lazy, cached views for each PanelTab.
//!
//! Each tab is its own GPUI entity with its own Render. Views are created
//! lazily on first activation and cached — switching away and back preserves
//! scroll position and any live state (pty, tree expansion, etc.).
//!
//! Empty tabs render a common honest-empty-state component that describes
//! what will be there without promising a delivery date (§13).

pub(crate) mod system;

use gpui::{FontWeight, IntoElement, Render, Window, Context, div, prelude::*, px, svg};

use chronos_ui::Theme;
use crate::side_panel_right::tabs::PanelTab;

use system::SystemTab;

// ---------------------------------------------------------------------------
// Registry — one entity type per populated tab, EmptyTab for the rest
// ---------------------------------------------------------------------------

/// Owned handle to a tab's view entity, type-erased for storage in a
/// `HashMap<PanelTab, _>`. Each variant holds an `Entity<T>` whose `T: Render`
/// — cloning the entity yields an `impl IntoElement`.
#[derive(Clone)]
pub(crate) enum TabContent {
    System(gpui::Entity<SystemTab>),
    Placeholder(gpui::Entity<EmptyTab>),
}

impl TabContent {
    /// Create the view for `tab` in the given context. `PanelTab::System` has
    /// its own entity; every other tab uses the common `EmptyTab`.
    pub(crate) fn create(
        tab: PanelTab,
        cx: &mut Context<crate::side_panel_right::view::SidePanelRightView>,
    ) -> Self {
        tracing::info!(
            tab = tab.label(),
            "side_panel_right: lazy-create tab view"
        );
        match tab {
            PanelTab::System => TabContent::System(cx.new(|cx| SystemTab::new(cx))),
            _ => TabContent::Placeholder(cx.new(|cx| EmptyTab::new(tab, cx))),
        }
    }
}

// ---------------------------------------------------------------------------
// Empty tab — honest placeholder, no "coming soon", no delivery promises
// ---------------------------------------------------------------------------

pub struct EmptyTab {
    tab: PanelTab,
}

impl EmptyTab {
    pub fn new(tab: PanelTab, _cx: &mut Context<Self>) -> Self {
        Self { tab }
    }
}

impl Render for EmptyTab {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = *Theme::global(cx);
        let tab = self.tab;

        div()
            .size_full()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(px(12.))
            .child(
                svg()
                    .path(tab.icon_path())
                    .size(px(40.))
                    .text_color(theme.text.muted.opacity(0.55)),
            )
            .child(
                div()
                    .text_size(px(13.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(theme.text.primary)
                    .child(tab.label().to_string()),
            )
            .child(
                div()
                    .text_size(px(11.5))
                    .text_color(theme.text.muted)
                    .child(placeholder_description(tab).to_string()),
            )
    }
}

/// What this tab will contain once implemented. Must be unique per tab and
/// must not promise a delivery date, a status (like "in development"), or a
/// progress bar (§13).
pub fn placeholder_description(tab: PanelTab) -> &'static str {
    match tab {
        PanelTab::System => "Hardware monitor and system controls",
        PanelTab::Files => "Browse and manage files on disk",
        PanelTab::Editor => "Text and code editor with syntax highlighting",
        PanelTab::Terminal => "Integrated terminal emulator session",
        PanelTab::Preview => "Live preview of web and UI surfaces",
        PanelTab::Inspector => "UI hierarchy and design-token inspector",
        PanelTab::Build => "Build, test, task and run orchestration",
        PanelTab::SourceControl => "Version control: branches, commits, diffs",
        PanelTab::AcpSettings => "Configure the AI agent protocol connection",
        PanelTab::McpSettings => "Manage Model Context Protocol server endpoints",
        PanelTab::LspSettings => "Language server and diagnostics configuration",
        PanelTab::ApiProviders => "API provider credentials and rate limits",
        PanelTab::EditorSettings => "Editor font, theme, and keybinding preferences",
        PanelTab::HyprlandBinds => "View and search active Hyprland keybindings",
    }
}

// ---------------------------------------------------------------------------
// tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    use crate::side_panel_right::view::SidePanelRightView;
    use crate::side_panel_right::SidePanelRightState;
    use gpui::TestAppContext;

    // --- laziness & cache (real entities, #[gpui::test]) ---

    #[gpui::test]
    async fn tab_views_starts_empty(cx: &mut TestAppContext) {
        cx.update(|cx| {
            cx.set_global(SidePanelRightState::default());
        });
        let view = cx.new(|cx| SidePanelRightView::new(cx));
        cx.update_entity(&view, |this, _cx| {
            assert_eq!(
                this.tab_count(),
                0,
                "tab views must start empty — nothing created at construction time"
            );
        });
    }

    #[gpui::test]
    async fn first_render_without_tab_select_creates_view(cx: &mut TestAppContext) {
        cx.update(|cx| {
            cx.set_global(SidePanelRightState::default());
        });
        let view = cx.new(|cx| SidePanelRightView::new(cx));
        // No on_tab_select — simulate the very first render path.
        // Use Files (not System) to avoid requiring service globals.
        cx.update_entity(&view, |this, cx| {
            assert_eq!(this.tab_count(), 0, "must start empty");
            this.ensure_tab_view(PanelTab::Files, cx);
        });
        cx.update_entity(&view, |this, _cx| {
            assert_eq!(
                this.tab_count(),
                1,
                "ensure_tab_view must create exactly one entry on first call"
            );
        });
    }

    #[gpui::test]
    async fn first_activation_creates_exactly_one_view(cx: &mut TestAppContext) {
        cx.update(|cx| {
            cx.set_global(SidePanelRightState::default());
        });
        let view = cx.new(|cx| SidePanelRightView::new(cx));
        // Activate a non-System tab (Files) — does not need service globals.
        cx.update_entity(&view, |this, cx| {
            this.on_tab_select(PanelTab::Files, cx);
        });
        cx.update_entity(&view, |this, _cx| {
            assert_eq!(
                this.tab_count(),
                1,
                "after first activation, exactly one tab view must be cached"
            );
        });
    }

    #[gpui::test]
    async fn cache_preserves_entity_across_switches(cx: &mut TestAppContext) {
        cx.update(|cx| {
            cx.set_global(SidePanelRightState::default());
        });
        let view = cx.new(|cx| SidePanelRightView::new(cx));

        // Activate Files, capture its entity id.
        cx.update_entity(&view, |this, cx| {
            this.on_tab_select(PanelTab::Files, cx);
        });
        let id_a = cx.update_entity(&view, |this, _cx| {
            this.tab_entity_id(PanelTab::Files)
                .expect("Files tab must have an entity after activation")
        });

        // Switch to Terminal.
        cx.update_entity(&view, |this, cx| {
            this.on_tab_select(PanelTab::Terminal, cx);
        });

        // Switch back to Files.
        cx.update_entity(&view, |this, cx| {
            this.on_tab_select(PanelTab::Files, cx);
        });
        let id_b = cx.update_entity(&view, |this, _cx| {
            this.tab_entity_id(PanelTab::Files)
                .expect("Files tab must still be cached after switch-back")
        });

        assert_eq!(
            id_a, id_b,
            "returning to Files must yield the same entity — cache must not recreate"
        );
        // Two tabs visited, but Files should still be the same entity.
        cx.update_entity(&view, |this, _cx| {
            assert_eq!(this.tab_count(), 2, "two distinct tabs visited");
        });
    }

    // --- placeholder descriptions ---

    #[test]
    fn every_tab_has_a_nonempty_placeholder_description() {
        for tab in PanelTab::ALL {
            let desc = placeholder_description(tab);
            assert!(
                !desc.is_empty(),
                "tab {tab:?} has an empty placeholder description"
            );
        }
    }

    #[test]
    fn placeholder_descriptions_are_unique() {
        let mut seen: Vec<&str> = PanelTab::ALL
            .iter()
            .map(|t| placeholder_description(*t))
            .collect();
        seen.sort_unstable();
        let orig_len = seen.len();
        seen.dedup();
        assert_eq!(
            seen.len(),
            orig_len,
            "two or more tabs share the same placeholder description"
        );
    }

    #[test]
    fn empty_tab_has_a_label() {
        // System is the only tab with real content; the other 9 use EmptyTab.
        for tab in PanelTab::ALL {
            if tab == PanelTab::System {
                continue;
            }
            assert!(!tab.label().is_empty(), "{tab:?} label empty");
            assert!(!placeholder_description(tab).is_empty(), "{tab:?} desc empty");
        }
    }
}
