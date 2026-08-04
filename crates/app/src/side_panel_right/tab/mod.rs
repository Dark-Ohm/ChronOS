//! Tab content registry — lazy, cached views for each PanelTab.
//!
//! Each tab is its own GPUI entity with its own Render. Views are created
//! lazily on first activation and cached — switching away and back preserves
//! scroll position and any live state (pty, tree expansion, etc.).
//!
//! Empty tabs render a common honest-empty-state component that describes
//! what will be there without promising a delivery date (§13).

pub(crate) mod acp_settings;
pub(crate) mod bar_settings;
pub(crate) mod build;
pub(crate) mod files;
pub(crate) mod hypr_binds;
pub(crate) mod library;
pub(crate) mod preview;
pub(crate) mod system;
pub(crate) mod terminal;
pub(crate) mod ui;

use gpui::{FontWeight, IntoElement, Render, Window, Context, div, prelude::*, px, svg};

use chronos_ui::Theme;
use crate::side_panel_right::tabs::PanelTab;

use acp_settings::AcpSettingsTab;
use bar_settings::BarSettingsTab;
use build::BuildTab;
use files::FilesTab;
use hypr_binds::HyprBindsTab;
use library::LibraryTab;
use preview::PreviewTab;
use system::SystemTab;
use terminal::TerminalTab;

// ---------------------------------------------------------------------------
// Registry — one entity type per populated tab, EmptyTab for the rest
// ---------------------------------------------------------------------------

/// Owned handle to a tab's view entity, type-erased for storage in a
/// `HashMap<PanelTab, _>`. Each variant holds an `Entity<T>` whose `T: Render`
/// — cloning the entity yields an `impl IntoElement`.
#[derive(Clone)]
pub(crate) enum TabContent {
    System(gpui::Entity<SystemTab>),
    Files(gpui::Entity<FilesTab>),
    Terminal(gpui::Entity<TerminalTab>),
    Build(gpui::Entity<BuildTab>),
    Preview(gpui::Entity<PreviewTab>),
    Library(gpui::Entity<LibraryTab>),
    HyprBinds(gpui::Entity<HyprBindsTab>),
    // T202: System settings «Bar» page — appearance presets + controls.
    BarSettings(gpui::Entity<BarSettingsTab>),
    // T196: ACP agents — list, add, remove agent backends.
    AcpSettings(gpui::Entity<AcpSettingsTab>),
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
            PanelTab::Files => TabContent::Files(cx.new(|cx| FilesTab::new(cx))),
            // Lazy by construction: `create` runs on first activation, and
            // `TerminalTab::new` raises the PTY only then (no tab → no shell).
            PanelTab::Terminal => TabContent::Terminal(cx.new(|cx| TerminalTab::new(cx))),
            PanelTab::Build => TabContent::Build(cx.new(|cx| BuildTab::new(cx))),
            // Preview: `PreviewTab::new` only subscribes to PreviewTarget; no
            // I/O happens until the user actually clicks a file in Files.
            PanelTab::Preview => TabContent::Preview(cx.new(|cx| PreviewTab::new(cx))),
            // Gamer at-rest hub (§4.2): Library is a real entity (T188) that
            // lists/launches detected games. Scenes/Captures stay placeholder
            // until T189 / a capture backend (slice 6, §13 honest empty).
            PanelTab::Library => TabContent::Library(cx.new(|cx| LibraryTab::new(cx))),
            // Hyprland binds: read-only keybind list (T193). Loads from the
            // modular Lua config lazily on first activation.
            PanelTab::HyprlandBinds => TabContent::HyprBinds(cx.new(|cx| HyprBindsTab::new(cx))),
            // T202: System settings («System settings» label) hosts the Bar
            // page — appearance presets + live controls. Reads bar.toml on
            // first activation, writes through the watcher (T134).
            PanelTab::EditorSettings => TabContent::BarSettings(cx.new(|cx| BarSettingsTab::new(cx))),
            // T196: ACP agents — list/add/remove ACP-compatible backends.
            // Reads/writes ~/.config/chronos/agents.toml.
            PanelTab::AcpSettings => TabContent::AcpSettings(cx.new(|cx| AcpSettingsTab::new(cx))),
            PanelTab::Scenes
            | PanelTab::Captures => TabContent::Placeholder(cx.new(|cx| EmptyTab::new(tab, cx))),
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
        PanelTab::Library => "List, pin and launch detected games",
        PanelTab::Scenes => "Activate per-game scenes and profiles",
        PanelTab::Captures => "Unavailable - no capture backend",
        PanelTab::AcpSettings => "Add, remove, and configure ACP agent endpoints",
        PanelTab::McpSettings => "Manage Model Context Protocol server endpoints",
        PanelTab::LspSettings => "Language server and diagnostics configuration",
        PanelTab::ApiProviders => "API provider credentials and rate limits",
        PanelTab::EditorSettings => "Shell and OS settings: appearance, keybindings, integrations",
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

    // --- T171: per-tab width behavioral tests ---

    #[gpui::test]
    async fn tab_select_applies_preferred_width(cx: &mut TestAppContext) {
        // T221 changes the precondition: rail-off-dock clicks on a different
        // tab apply preferred width (branch 4), rail-dock clicks only
        // switch active_tab. Use the off-dock rail-only path so widths are
        // assertable end-to-end — under dock, click → no width change (the
        // dock button ⊞/⊟ is the controlling knob for height-zone, and
        // pinning overrides per-tab width).
        use crate::side_panel_right::{RAIL_ONLY_WIDTH, SidePanelRightState};
        cx.update(|cx| {
            cx.set_global(SidePanelRightState::default());
        });
        let view = cx.new(|cx| SidePanelRightView::new(cx));
        cx.update_entity(&view, |this, cx| {
            this.on_tab_select(PanelTab::Files, cx);
        });
        cx.update(|cx| {
            let state = cx.global::<SidePanelRightState>();
            assert_eq!(
                state.width, 440.,
                "different-tab click off dock must apply Files preferred width (T221 branch 4)"
            );
            assert!(
                state.width > RAIL_ONLY_WIDTH,
                "must have expanded from rail-only"
            );
        });
        // Switch to Editor → width should be 560 (DEFAULT_CONTENT_WIDTH).
        cx.update_entity(&view, |this, cx| {
            this.on_tab_select(PanelTab::Editor, cx);
        });
        cx.update(|cx| {
            let state = cx.global::<SidePanelRightState>();
            assert_eq!(state.width, 560., "Editor tab preferred width must be applied");
        });
    }

    #[gpui::test]
    async fn first_rail_click_under_dock_off_opens_at_natural_width(cx: &mut TestAppContext) {
        // T221 deliberately inverted the prior invariant: rail icon is the
        // SINGLE affordance. Pre-T221: clicking a non-active icon while
        // the panel was rail-only + dock_content=false was a silent no-op
        // (active_tab changed but width stayed at RAIL_ONLY_WIDTH). T221
        // branch 4: a click on a *different* rail icon must open the panel
        // at that tab's natural width. The pre-T221 test was named
        // `dock_content_false_keeps_rail_only_width`; the renamed version
        // pins the new contract so a future revert is caught here.
        use crate::side_panel_right::{RAIL_ONLY_WIDTH, SidePanelRightState};
        cx.update(|cx| {
            cx.set_global(SidePanelRightState::default()); // dock_content=false, width=RAIL_ONLY_WIDTH
        });
        let view = cx.new(|cx| SidePanelRightView::new(cx));
        cx.update(|cx| {
            assert_eq!(
                cx.global::<SidePanelRightState>().width, RAIL_ONLY_WIDTH,
                "panel must start rail-only"
            );
        });

        // (1) Click Files (different from default active=System) → opens
        // at Files' natural width 440. Width is the user-visible signal; a
        // matching `active_tab` is implicit because branch 4 sets it before
        // applying width (we don't poke the private `active_tab` field here
        // — it lives in `view::`'s module).
        cx.update_entity(&view, |this, cx| {
            this.on_tab_select(PanelTab::Files, cx);
        });
        cx.update(|cx| {
            assert_eq!(
                cx.global::<SidePanelRightState>().width,
                PanelTab::Files.preferred_content_width(),
                "first rail-click under dock_content=false must open at natural width (T221 branch 4)"
            );
        });

        // (2) Click Files again (same active, content open) → collapses.
        cx.update_entity(&view, |this, cx| {
            this.on_tab_select(PanelTab::Files, cx);
        });
        cx.update(|cx| {
            assert_eq!(
                cx.global::<SidePanelRightState>().width, RAIL_ONLY_WIDTH,
                "click on active open tab must collapse (T221 branch 2)"
            );
        });

        // (3) Switch to Terminal (different active, content closed) →
        // opens at Terminal's natural width 560.
        cx.update_entity(&view, |this, cx| {
            this.on_tab_select(PanelTab::Terminal, cx);
        });
        cx.update(|cx| {
            assert_eq!(
                cx.global::<SidePanelRightState>().width,
                PanelTab::Terminal.preferred_content_width(),
                "different-tab click from collapsed must open at the new tab's natural width"
            );
        });
    }

    #[gpui::test]
    async fn same_tab_reclick_collapses_to_rail_under_t221(cx: &mut TestAppContext) {
        // T221 deliberately inverted this contract: rail icon is the SINGLE
        // affordance. Re-clicking the active tab while content is open now
        // collapses to rail-only instead of silently preserving the manual
        // width. Memory of the user's resize survives in
        // `tab_resize_memory` (T218), so a subsequent click restores it
        // when (and only when) the tab supports manual resize — covered by
        // `on_tab_select_collapse_preserves_editor_resize_memory` in
        // `view::tests` for resizable Editor/Settings tabs; Files here is a
        // fixed-width tab whose `tab_resize_memory` is intentionally ignored
        // by `active_tab_width` (T218 "fixed-width tabs ignore memory").
        use crate::side_panel_right::{RAIL_ONLY_WIDTH, SidePanelRightState};
        cx.update(|cx| {
            cx.set_global(SidePanelRightState::default());
        });
        let view = cx.new(|cx| SidePanelRightView::new(cx));
        // Select Files (not default System — re-click is a no-op).
        cx.update_entity(&view, |this, cx| {
            this.on_tab_select(PanelTab::Files, cx);
        });
        // The first click (from rail-only) opens Files at its natural
        // width 440 — T221 branch 4 always opens, no more silent-rail-only.
        cx.update(|cx| {
            assert_eq!(
                cx.global::<SidePanelRightState>().width,
                PanelTab::Files.preferred_content_width(),
                "first Files click must open content (T221 branch 4)"
            );
        });
        // Simulate a manual resize to 480.
        cx.update(|cx| {
            let state = cx.global_mut::<SidePanelRightState>();
            state.resize(480.);
        });
        // Re-click Files — T221 branch 2 collapses to rail.
        cx.update_entity(&view, |this, cx| {
            this.on_tab_select(PanelTab::Files, cx);
        });
        cx.update(|cx| {
            let state = cx.global::<SidePanelRightState>();
            assert_eq!(
                state.width,
                RAIL_ONLY_WIDTH,
                "re-clicking the active tab while open MUST collapse to rail (T221)"
            );
        });
        // Re-clicking once more re-opens Files at 440 (fixed-width tab:
        // memory is ignored, preferred wins; Editor+Settings use memory —
        // same path, different contract).
        cx.update_entity(&view, |this, cx| {
            this.on_tab_select(PanelTab::Files, cx);
        });
        cx.update(|cx| {
            assert_eq!(
                cx.global::<SidePanelRightState>().width,
                PanelTab::Files.preferred_content_width(),
                "third click (collapsed) re-opens Files at its natural width"
            );
        });
    }

    #[gpui::test]
    async fn switch_tab_restores_per_tab_resize_memory(cx: &mut TestAppContext) {
        use crate::side_panel_right::SidePanelRightState;
        cx.update(|cx| {
            cx.set_global(SidePanelRightState::default());
        });
        let view = cx.new(|cx| SidePanelRightView::new(cx));
        // T218: memory only applies to resizable tabs, so this exercises Preview
        // (the Editor surface) rather than Files, which is now fixed width.
        assert!(PanelTab::Preview.resizable());
        cx.update_entity(&view, |this, cx| {
            this.on_tab_select(PanelTab::Preview, cx);
            this.sim_resize(480., cx);
        });
        // Switch to Files — width becomes Files' natural 440.
        cx.update_entity(&view, |this, cx| {
            this.on_tab_select(PanelTab::Files, cx);
        });
        cx.update(|cx| {
            assert_eq!(
                cx.global::<SidePanelRightState>().width,
                PanelTab::Files.preferred_content_width()
            );
        });
        // Switch back to Preview — must restore 480, not its preferred 560.
        cx.update_entity(&view, |this, cx| {
            this.on_tab_select(PanelTab::Preview, cx);
        });
        cx.update(|cx| {
            let state = cx.global::<SidePanelRightState>();
            assert_eq!(
                state.width, 480.,
                "returning to Preview must restore its resized width"
            );
        });
    }

    #[gpui::test]
    async fn fixed_width_tab_keeps_its_natural_width(cx: &mut TestAppContext) {
        // T218: Files is laid out for its content, so a drag must not move it and
        // leaving/returning must land on exactly `preferred_content_width` —
        // otherwise a tab could stay stuck narrow enough to clip its own
        // controls. T221 changes the precondition: the off-dock rail-only
        // start is the natural way to drive branch 4 (different-tab opens);
        // under dock, click → no width change (pinned at the dock width).
        use crate::side_panel_right::SidePanelRightState;
        cx.update(|cx| {
            cx.set_global(SidePanelRightState::default()); // off dock, rail-only
        });
        let view = cx.new(|cx| SidePanelRightView::new(cx));
        assert!(!PanelTab::Files.resizable());

        // Click Files (different from default active System) → branch 4
        // opens at Files' natural width 440.
        cx.update_entity(&view, |this, cx| {
            this.on_tab_select(PanelTab::Files, cx);
            this.sim_resize(900., cx);
        });
        cx.update(|cx| {
            assert_eq!(
                cx.global::<SidePanelRightState>().width,
                PanelTab::Files.preferred_content_width(),
                "T218: drag on a fixed-width tab must not change its width"
            );
        });

        // Visit Preview (Editor) — branch 4 under non-dock: re-open at its
        // natural 560; then return to Files — branch 4 re-opens at 440.
        cx.update_entity(&view, |this, cx| {
            this.on_tab_select(PanelTab::Preview, cx);
            this.on_tab_select(PanelTab::Files, cx);
        });
        cx.update(|cx| {
            assert_eq!(
                cx.global::<SidePanelRightState>().width,
                PanelTab::Files.preferred_content_width(),
                "T218/T221: returning to a fixed-width tab must land on its natural width"
            );
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
        // System, Files and Terminal have real content; the rest use EmptyTab.
        for tab in PanelTab::ALL {
            if tab == PanelTab::System {
                continue;
            }
            assert!(!tab.label().is_empty(), "{tab:?} label empty");
            assert!(!placeholder_description(tab).is_empty(), "{tab:?} desc empty");
        }
    }
}
