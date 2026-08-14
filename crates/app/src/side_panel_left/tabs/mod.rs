//! T278 / Slice A1 — left workspace tab catalog and width policy.
//!
//! T279 / Slice A2: tab view modules live as submodules here.
//!
//! Pure metadata only. No rendering, no stores, no `Window`/`App`/`Context`.
//! Mirrors the shape of `side_panel_right::tabs::PanelTab` so the rail
//! composes the same way on both sides, but the catalog is product-specific
//! (AI workspace, not system shell): Project Switcher + Sessions + Chat +
//! Plan + Tools + Skills + Context Files + Archive.
//!
//! Width policy lives next to the variant so a future render call cannot
//! drift from "Chat is resizable, Archive is fixed at 440" — both pieces
//! of the rule travel together.

pub(crate) mod chat;
pub(crate) mod project;
pub(crate) mod sessions;
pub(crate) mod shell;

// Re-export event enums + tab structs so callers reach them as
// `tabs::{SessionsEvent, ProjectEvent, SessionsTab, ProjectTab, ShellTab}`
// without naming the submodule.
pub(crate) use project::{ProjectEvent, ProjectTab};
pub(crate) use sessions::{SessionsEvent, SessionsTab};
pub(crate) use shell::ShellTab;

use crate::side_panel_left::state::geometry;

/// Standalone rail width — pixel footprint of the `rail` layer-shell surface.
/// Identical to T276's `side_panel_right::RAIL_ONLY_WIDTH`.
pub const RAIL_WIDTH: f32 = 40.0;

/// Width of the transparent resize handle overlay. Does not consume any of
/// `RAIL_WIDTH`; the handle floats above the canvas's outer edge.
pub const RESIZE_HANDLE_WIDTH: f32 = 4.0;

/// Hard upper bound on the logical panel width — `MAX_PANEL_WIDTH = RAIL +
/// CONTENT_CANVAS_WIDTH`. Drag clamp and per-tab preferred widths both
/// anchor to this number.
pub const MAX_PANEL_WIDTH: f32 = 960.0;

/// Width of the fixed `content` Wayland canvas. Never resized after open —
/// the visible slice and input region only ever change inside it.
pub const CONTENT_CANVAS_WIDTH: f32 = MAX_PANEL_WIDTH - RAIL_WIDTH;

/// Soft floor used when opening/restoring resizable content. Below this the
/// tab still shows but the layout collapses to a single column without
/// side panels. The hard drag floor is `RAIL_WIDTH` (`40`) so the user can
/// still pull the panel back from rail-only with the same gesture.
pub const SOFT_OPEN_MIN_WIDTH: f32 = 360.0;

/// Per-resizable-tab runtime memory — width the user dragged the panel to
/// the last time this tab was active. Resets on process restart; never
/// persisted.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ResizableWidths {
    pub chat: f32,
    pub plan: f32,
    pub context_files: f32,
}

impl Default for ResizableWidths {
    fn default() -> Self {
        Self {
            chat: 560.0,
            plan: 480.0,
            context_files: 560.0,
        }
    }
}

impl ResizableWidths {
    /// Hard-clamp every runtime width into the resizable drag range
    /// `[SOFT_OPEN_MIN_WIDTH, MAX_PANEL_WIDTH]`. A stray negative or
    /// out-of-band value (e.g. an older memory layout restored from a
    /// downgrade) cannot survive this.
    pub fn sanitized(mut self) -> Self {
        self.chat = geometry::clamp_resizable(self.chat);
        self.plan = geometry::clamp_resizable(self.plan);
        self.context_files = geometry::clamp_resizable(self.context_files);
        self
    }

    /// Read the runtime width for a given resizable tab.
    pub fn get(&self, tab: LeftTab) -> Option<f32> {
        match tab {
            LeftTab::Chat => Some(self.chat),
            LeftTab::Plan => Some(self.plan),
            LeftTab::ContextFiles => Some(self.context_files),
            // Fixed-width tabs have no per-instance memory.
            _ => None,
        }
    }

    /// Write the runtime width for a given resizable tab. Fixed tabs are
    /// silently ignored — the rail UI never calls this for them.
    pub fn set(&mut self, tab: LeftTab, width: f32) {
        let w = geometry::clamp_resizable(width);
        match tab {
            LeftTab::Chat => self.chat = w,
            LeftTab::Plan => self.plan = w,
            LeftTab::ContextFiles => self.context_files = w,
            _ => {}
        }
    }
}

/// Catalog of every tab the left workspace knows about. Slice A ships the
/// full Slice-A product set; Slice B/C add the implementation bodies for
/// Plan / Tools / Skills / Context Files / Archive.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum LeftTab {
    Project,
    Sessions,
    Chat,
    Plan,
    Tools,
    Skills,
    ContextFiles,
    Archive,
}

impl LeftTab {
    /// Whether the user can drag-resize the panel while this tab is active.
    /// Resizable tabs honour their `ResizableWidths` slot; fixed tabs are
    /// pinned to `preferred_panel_width`.
    pub const fn is_resizable(self) -> bool {
        matches!(self, Self::Chat | Self::Plan | Self::ContextFiles)
    }

    /// Full logical panel width this tab opens at when no runtime memory
    /// exists. Resizable tabs return `SOFT_OPEN_MIN_WIDTH` so the first
    /// open of a fresh session is at the soft floor, not at `MAX_PANEL_WIDTH`.
    pub const fn preferred_panel_width(self) -> f32 {
        match self {
            Self::Project => 440.0,
            Self::Sessions => 400.0,
            Self::Chat => SOFT_OPEN_MIN_WIDTH,
            Self::Plan => SOFT_OPEN_MIN_WIDTH,
            Self::Tools => 440.0,
            Self::Skills => 440.0,
            Self::ContextFiles => SOFT_OPEN_MIN_WIDTH,
            Self::Archive => 440.0,
        }
    }

    /// Human-readable label used in tooltips, accessibility text, and any
    /// future settings UI. English source strings — localisation is out of
    /// Slice A scope.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Project => "Project",
            Self::Sessions => "Sessions",
            Self::Chat => "Chat",
            Self::Plan => "Plan",
            Self::Tools => "Tools",
            Self::Skills => "Skills",
            Self::ContextFiles => "Context files",
            Self::Archive => "Archive",
        }
    }

    /// SVG icon path used by the rail renderer. Maps to existing ChronOS
    /// rail icon assets — no new bitmap icons in A1 (plan §Task 1 step 2).
    pub const fn icon_path(self) -> &'static str {
        match self {
            Self::Project => "icons/rail-system.svg",
            Self::Sessions => "icons/rail-source-control.svg",
            Self::Chat => "icons/rail-editor.svg",
            Self::Plan => "icons/rail-build.svg",
            Self::Tools => "icons/rail-terminal.svg",
            Self::Skills => "icons/rail-inspector.svg",
            Self::ContextFiles => "icons/rail-preview.svg",
            Self::Archive => "icons/rail-acp.svg",
        }
    }
}

/// Top-of-rail tab list, in fixed order. The rail renderer must iterate this
/// exactly — order is part of the contract, not a UI suggestion.
pub const PRIMARY_TABS: &[LeftTab] = &[
    LeftTab::Project,
    LeftTab::Sessions,
    LeftTab::Chat,
    LeftTab::Plan,
    LeftTab::Tools,
    LeftTab::Skills,
    LeftTab::ContextFiles,
];

/// Bottom-of-rail tab, pinned at the bottom of the column. There is
/// exactly one — `flex_1` spacer between `PRIMARY_TABS` and `BOTTOM_TAB`
/// guarantees its position regardless of primary-tab badge count.
pub const BOTTOM_TAB: LeftTab = LeftTab::Archive;

/// Width the panel should snap to when this tab becomes active.
/// Resizable tabs honour their runtime memory, clamped to the resizable
/// drag range; fixed tabs return their `preferred_panel_width` exactly.
/// Callers may further apply `apply_open_floor` to honour `SOFT_OPEN_MIN_WIDTH`.
pub fn width_for_open(tab: LeftTab, remembered: &ResizableWidths) -> f32 {
    let raw = if tab.is_resizable() {
        remembered
            .get(tab)
            .unwrap_or_else(|| tab.preferred_panel_width())
    } else {
        tab.preferred_panel_width()
    };
    // Resizable tabs: clamp into the drag range. Fixed tabs: clamp into
    // [RAIL_WIDTH, MAX_PANEL_WIDTH] defensively so a stale config file
    // cannot crash the layout.
    if tab.is_resizable() {
        geometry::clamp_resizable(raw)
    } else {
        geometry::clamp_panel(raw)
    }
}

/// T278 architect round 3: pure dock reducer. Given the current
/// `panel_width`, `dock_content`, `active_tab`, and `remembered_widths`,
/// returns the next `(panel_width, dock_content)` pair after a single
/// dock-toggle click. No `App`/`Window`/`Context` — testable directly.
///
/// Spec §4.1 dock reducer rules:
///
/// - **rail-only + dock on** (`!dock_content && visible_w == 0`):
///   expand to `width_for_open(active_tab, remembered)` so the panel
///   leaves the rail-only state on dock enable. Without this, the user
///   could land in a deadlock: dock=true, panel_width=40, content
///   invisible, every rail-click on the active tab a no-op (dock-wins),
///   and `Super+A`/`close` cycle reset to rail-only — stuck.
/// - **overlay + dock on** (`!dock_content && visible_w > 0`): preserve
///   the current width, just flip the flag. The dock flag changes the
///   rail's exclusive zone from `RAIL_WIDTH` to `panel_width`.
/// - **docked + dock off** (`dock_content`): preserve the current width,
///   just flip the flag. The dock flag changes the rail's exclusive zone
///   from `panel_width` back to `RAIL_WIDTH`; the visible slice stays open.
///
/// The next regular `on_rail_tab_select` (different tab) applies the new
/// tab's `width_for_open` policy — dock toggle does NOT pre-bake width for
/// the current active tab, it only expands on the rail-only edge case.
pub fn dock_transition(
    panel_width: f32,
    dock_content: bool,
    active_tab: LeftTab,
    remembered: &ResizableWidths,
) -> (f32, bool) {
    let next_dock = !dock_content;
    let next_width = if !dock_content && geometry::visible_content_width(panel_width) <= 0.0 {
        // rail-only → dock on: expand to active tab's preferred/remembered.
        width_for_open(active_tab, remembered)
    } else {
        // overlay ↔ docked transitions preserve the user's drag width.
        // Round 3 deliberately reverts the round 2 "always preserve" fix
        // back to the spec table — the rail-only edge case is the only
        // branch that must expand.
        panel_width
    };
    (next_width, next_dock)
}

// ─────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────

/// T279 / Task 4 — pure rail-tab-select transition. The 3-action policy
/// from `on_rail_tab_select` (rail_view.rs), hoisted to a pure function
/// so a unit test exercises every branch without instantiating
/// `WorkspaceView` (which needs `ChatTab`, which spawns an async ACP
/// connect requiring a live Tokio runtime — unconstructable in
/// `TestAppContext`). Mirrors the T278 `dock_transition` carve.
///
/// Inputs are read-only snapshot fields from `SidePanelLeftState_`:
/// `panel_width`, `dock_content`, `active_tab`, `remembered_widths`.
/// Returns `(next_active, next_width, next_dock)`. The reducer
/// `select_tab` writes these into the global SoT; the rail view
/// delegates to the reducer.
///
/// Branches (mirror `on_rail_tab_select` word-for-word):
/// 1. Same tab, content open, dock on → no-op (`Some(active, w, dock)`).
/// 2. Same tab, content open, dock off → collapse to rail-only
///    (`Some(active, RAIL_WIDTH, false)`, remember the width).
/// 3. Else (same tab closed, *or* different tab) → select and open
///    (`Some(clicked, width_for_open(clicked), false)`).
///
/// The collapsing branch (#2) returns the *new* width/dock; the
/// reducer applies `remembered_widths.set(active, panel_width)` before
/// overwriting — this pure helper does not mutate `remembered` (it's
/// `&`), so the remember-step is the reducer's job. The return width
/// is `RAIL_WIDTH` on collapse.
pub fn tab_select_transition(
    clicked: LeftTab,
    active: LeftTab,
    panel_width: f32,
    dock_content: bool,
    remembered: &ResizableWidths,
) -> (LeftTab, f32, bool) {
    let visible_w = geometry::visible_content_width(panel_width);
    let content_open = dock_content || visible_w > 1.0;

    match (clicked == active, content_open, dock_content) {
        (true, true, true) => (active, panel_width, dock_content),
        (true, true, false) => (active, RAIL_WIDTH, false),
        _ => (clicked, width_for_open(clicked, remembered), false),
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Coordinator transitions — pure policy on workspace snapshot fields.
// ─────────────────────────────────────────────────────────────────────────

/// T279 / Task 4 — session-selection transition. When the user picks a thread
/// from SessionsTab, the workspace must (a) record the selected id and (b)
/// switch to Chat so the transcript opens. Both effects are pure here; the
/// reducer `select_session` writes them into the global SoT. Mirrors the
/// T278 carve: no `App`/`Window`/`Context`, testable directly.
///
/// Inputs are the *read-only* snapshot fields the policy depends on:
/// `active_tab` (to know whether we're already on Chat) and
/// `active_session_id` (to detect a no-op reselect). Returns
/// `(next_active_tab, next_active_session_id)`.
///
/// Contract (plan Task 4 Step 1: "session selection → selected id + Chat tab"):
/// - always sets `active_session_id = Some(thread_id)`; the store later
///   validates it against the project scope (T280: `active_thread`), but the
///   transition itself trusts the id the list emitted — the list only holds
///   ids the store returned.
/// - always switches `active_tab = Chat` (even if already there — the
///   reducer's `include_chat_width` open is idempotent via `ensure_content_width`).
pub fn session_select_transition(
    thread_id: &str,
    _active_tab: LeftTab,
    _active_session_id: Option<&str>,
) -> (LeftTab, String) {
    // T278 lesson: every policy reads at least one input field so the test
    // path covers the function. Here `active_tab` and `active_session_id`
    // are snapshot-only (no branch on them since the policy is unconditional),
    // but they stay in the signature so the reducer signature matches and a
    // future "reselect no-op" optimization threads through cleanly.
    (LeftTab::Chat, thread_id.to_string())
}

/// T279 / Task 4 — project-switch transition. When the user picks a project
/// from ProjectTab, the workspace must (a) set the active project path and
/// (b) **clear** the active session id — the old project's thread must not
/// leak into the new scope (plan Step 3: "clear old Chat/Sessions view state,
/// update `ProjectsConfig.active`, ask the store for the new project's active
/// thread"). The *loading* of whatever the store reports as the new
/// project's active thread is a separate store call (T280), not a transition;
/// this helper only encodes the clear-then-set-scope contract.
///
/// Inputs are read-only snapshot fields: the old `active_project_path` and
/// `active_session_id` (kept in the signature so the reducer mirrors the
/// surface, and so a future "same project reselect" optimization lands
/// cleanly). Returns `(next_active_project_path, next_active_session_id)`.
///
/// Contract (plan Task 4 Step 1: "project switch → all session/chat state is
/// cleared before loading the new scope"):
/// - `next_active_session_id` is always `None` — clear before load, the
///   store's `active_thread(project_path)` reinstates a valid one in the
///   reducer.
/// - `next_active_project_path` is always `Some(new_path.clone())`.
pub fn project_switch_transition(
    new_project_path: &std::path::Path,
    _old_project_path: Option<&std::path::Path>,
    _old_active_session_id: Option<&str>,
) -> (Option<std::path::PathBuf>, Option<String>) {
    (Some(new_project_path.to_path_buf()), None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn primary_tabs_in_fixed_order() {
        assert_eq!(
            PRIMARY_TABS,
            &[
                LeftTab::Project,
                LeftTab::Sessions,
                LeftTab::Chat,
                LeftTab::Plan,
                LeftTab::Tools,
                LeftTab::Skills,
                LeftTab::ContextFiles,
            ]
        );
    }

    #[test]
    fn bottom_tab_is_archive() {
        assert_eq!(BOTTOM_TAB, LeftTab::Archive);
    }

    // ── T279 / Task 4 — tab-select transitions (pure helper) ──

    /// Same tab, docked, content open → no-op (dock wins).
    /// Plan Task 4 Step 1: project click → content opens; here the
    /// symmetric "active docked tab click → no-op" branch.
    #[test]
    fn select_active_docked_tab_is_noop() {
        let r = tab_select_transition(
            LeftTab::Chat,
            LeftTab::Chat,
            612.0,
            true,
            &ResizableWidths::default(),
        );
        assert_eq!(r, (LeftTab::Chat, 612.0, true));
    }

    /// Same tab, undocked, content open → collapse to rail-only.
    /// Plan Task 4 Step 1: "active non-docked tab click → rail-only".
    #[test]
    fn select_active_undocked_open_collapses_to_rail_only() {
        let r = tab_select_transition(
            LeftTab::Sessions,
            LeftTab::Sessions,
            444.0,
            false,
            &ResizableWidths::default(),
        );
        assert_eq!(r, (LeftTab::Sessions, RAIL_WIDTH, false));
    }

    /// Different tab → switch + open at width_for_open(clicked).
    /// Plan Task 4 Step 1: "another tab click → switch and open".
    #[test]
    fn select_other_tab_switches_and_opens() {
        let r = tab_select_transition(
            LeftTab::Project,
            LeftTab::Chat,
            RAIL_WIDTH,
            false,
            &ResizableWidths::default(),
        );
        assert_eq!(r.0, LeftTab::Project);
        assert!(r.1 >= SOFT_OPEN_MIN_WIDTH, "Project opens at fixed width {}", r.1);
        assert!(!r.2);
    }

    /// Same tab, closed (rail-only) → re-open at width_for_open.
    /// The `_` arm of the match: clicked == active but content_open is
    /// false, so it falls to the select-and-open branch.
    #[test]
    fn select_active_closed_reopens() {
        let r = tab_select_transition(
            LeftTab::Chat,
            LeftTab::Chat,
            RAIL_WIDTH,
            false,
            &ResizableWidths::default(),
        );
        // Chat default remembered width = 560 (ResizableWidths::default).
        assert_eq!(r, (LeftTab::Chat, 560.0, false));
    }

    /// Project is a fixed-width tab (440) — selecting it from rail-only
    /// opens at 440, not at the Chat remembered width.
    #[test]
    fn select_project_uses_fixed_preferred_width() {
        let mut remembered = ResizableWidths::default();
        remembered.chat = 800.0; // irrelevant — Project ignores remembered.
        let r = tab_select_transition(
            LeftTab::Project,
            LeftTab::Chat,
            RAIL_WIDTH,
            false,
            &remembered,
        );
        assert_eq!(r, (LeftTab::Project, LeftTab::Project.preferred_panel_width(), false));
        assert_eq!(r.1, 440.0);
    }

    // ── T279 / Task 4 — coordinator transitions (pure helpers) ──

    /// Session selection → Chat tab + active_session_id set.
    /// Plan Step 1: "session selection → selected id + Chat tab".
    #[test]
    fn session_select_switches_to_chat_and_records_id() {
        let (tab, id) = session_select_transition("thread-42", LeftTab::Sessions, None);
        assert_eq!(tab, LeftTab::Chat);
        assert_eq!(id, "thread-42");
    }

    /// Session selection from already-on-Chat is still a no-op-on-tab field
    /// but the id updates — the store backs the new thread's transcript.
    #[test]
    fn session_select_from_chat_still_switches_and_records() {
        let (tab, id) =
            session_select_transition("thread-7", LeftTab::Chat, Some("thread-1"));
        assert_eq!(tab, LeftTab::Chat);
        assert_eq!(id, "thread-7");
    }

    /// Project switch → active_project_path set, active_session_id CLEARED.
    /// Plan Step 1: "project switch → all session/chat state is cleared
    /// before loading the new scope". The old session id must not leak.
    #[test]
    fn project_switch_clears_session_and_sets_path() {
        let old_path = std::path::PathBuf::from("/home/neo/old-proj");
        let new_path = std::path::PathBuf::from("/home/neo/new-proj");
        let (next_path, next_session) = project_switch_transition(
            &new_path,
            Some(&old_path),
            Some("old-thread"),
        );
        assert_eq!(next_path.as_deref(), Some(new_path.as_path()));
        assert_eq!(next_session, None, "old session id must be cleared");
    }

    /// Project switch to the same project still clears the session id —
    /// the store's `active_thread` reinstates it after. The transition is
    /// unconditional; it does not optimize away the clear.
    #[test]
    fn project_switch_same_path_still_clears_session() {
        let path = std::path::PathBuf::from("/home/neo/same-proj");
        let (next_path, next_session) =
            project_switch_transition(&path, Some(&path), Some("thread-99"));
        assert_eq!(next_path.as_deref(), Some(path.as_path()));
        assert_eq!(next_session, None);
    }

    #[test]
    fn archive_is_exactly_one_bottom_tab() {
        let mut bottom = Vec::new();
        for tab in PRIMARY_TABS {
            assert_ne!(*tab, BOTTOM_TAB, "Archive must not appear in PRIMARY_TABS");
        }
        assert!(bottom.is_empty());
        bottom.push(BOTTOM_TAB);
        assert_eq!(bottom.len(), 1);
    }

    #[test]
    fn resize_policy_matches_spec() {
        // Spec §7: only Chat, Plan, Context Files are resizable.
        assert!(LeftTab::Chat.is_resizable());
        assert!(LeftTab::Plan.is_resizable());
        assert!(LeftTab::ContextFiles.is_resizable());

        assert!(!LeftTab::Project.is_resizable());
        assert!(!LeftTab::Sessions.is_resizable());
        assert!(!LeftTab::Tools.is_resizable());
        assert!(!LeftTab::Skills.is_resizable());
        assert!(!LeftTab::Archive.is_resizable());
    }

    #[test]
    fn fixed_widths_match_spec() {
        // Spec §7: Project 440, Sessions 400, Tools 440, Skills 440, Archive 440.
        assert_eq!(LeftTab::Project.preferred_panel_width(), 440.0);
        assert_eq!(LeftTab::Sessions.preferred_panel_width(), 400.0);
        assert_eq!(LeftTab::Tools.preferred_panel_width(), 440.0);
        assert_eq!(LeftTab::Skills.preferred_panel_width(), 440.0);
        assert_eq!(LeftTab::Archive.preferred_panel_width(), 440.0);
    }

    #[test]
    fn resizable_preferred_widths_are_soft_floor() {
        // Spec §7: resizable tabs open at SOFT_OPEN_MIN_WIDTH when no
        // remembered width exists (not at MAX_PANEL_WIDTH — that would
        // make the first open always a full-canvas blast).
        assert_eq!(
            LeftTab::Chat.preferred_panel_width(),
            SOFT_OPEN_MIN_WIDTH
        );
        assert_eq!(LeftTab::Plan.preferred_panel_width(), SOFT_OPEN_MIN_WIDTH);
        assert_eq!(
            LeftTab::ContextFiles.preferred_panel_width(),
            SOFT_OPEN_MIN_WIDTH
        );
    }

    #[test]
    fn labels_are_unique_and_nonempty() {
        let mut seen = std::collections::HashSet::new();
        for tab in all_tabs() {
            let label = tab.label();
            assert!(!label.is_empty(), "{tab:?} label is empty");
            assert!(seen.insert(label), "{tab:?} label {label:?} collides");
        }
    }

    #[test]
    fn icon_paths_are_distinct_and_under_icons() {
        let mut paths: Vec<&str> = all_tabs().iter().map(|t| t.icon_path()).collect();
        paths.sort_unstable();
        paths.dedup();
        assert_eq!(paths.len(), all_tabs().len(), "two tabs share an icon path");
        for p in paths {
            assert!(
                p.starts_with("icons/") && p.ends_with(".svg"),
                "icon path must be icons/*.svg: {p}"
            );
        }
    }

    #[test]
    fn resizable_widths_default_matches_spec() {
        // Spec §7: initial widths Chat 560, Plan 480, Context Files 560.
        let w = ResizableWidths::default();
        assert_eq!(w.chat, 560.0);
        assert_eq!(w.plan, 480.0);
        assert_eq!(w.context_files, 560.0);
    }

    #[test]
    fn resizable_widths_sanitized_clamps_out_of_band() {
        let w = ResizableWidths {
            chat: 100.0,            // below soft floor → SOFT_OPEN_MIN_WIDTH
            plan: 1200.0,           // above MAX_PANEL_WIDTH → MAX_PANEL_WIDTH
            context_files: 560.0,   // in range, unchanged
        };
        let s = w.sanitized();
        assert_eq!(s.chat, SOFT_OPEN_MIN_WIDTH);
        assert_eq!(s.plan, MAX_PANEL_WIDTH);
        assert_eq!(s.context_files, 560.0);
    }

    #[test]
    fn resizable_widths_get_set_roundtrip() {
        let mut w = ResizableWidths::default();
        w.set(LeftTab::Chat, 612.0);
        assert_eq!(w.get(LeftTab::Chat), Some(612.0));
        w.set(LeftTab::Plan, 470.0);
        assert_eq!(w.get(LeftTab::Plan), Some(470.0));
        // Fixed tabs have no slot.
        assert_eq!(w.get(LeftTab::Project), None);
        // set() on a fixed tab is silently ignored.
        w.set(LeftTab::Project, 999.0);
        assert_eq!(w.get(LeftTab::Project), None);
    }

    #[test]
    fn width_for_open_fixed_tabs_use_preferred_exactly() {
        // Spec §7: fixed tabs ignore any remembered width entirely.
        let remembered = ResizableWidths {
            chat: 100.0,
            plan: 100.0,
            context_files: 100.0,
        };
        for tab in [
            LeftTab::Project,
            LeftTab::Sessions,
            LeftTab::Tools,
            LeftTab::Skills,
            LeftTab::Archive,
        ] {
            assert_eq!(
                width_for_open(tab, &remembered),
                tab.preferred_panel_width(),
                "{tab:?} must use preferred width"
            );
        }
    }

    #[test]
    fn width_for_open_resizable_uses_remembered_when_in_range() {
        let remembered = ResizableWidths {
            chat: 612.0,
            plan: 470.0,
            context_files: 500.0,
        };
        assert_eq!(width_for_open(LeftTab::Chat, &remembered), 612.0);
        assert_eq!(width_for_open(LeftTab::Plan, &remembered), 470.0);
        assert_eq!(
            width_for_open(LeftTab::ContextFiles, &remembered),
            500.0
        );
    }

    #[test]
    fn width_for_open_resizable_clamps_out_of_range_remembered() {
        // A width saved by an older build (or a stray config restore) must
        // not survive — clamped into the drag range so a render-side
        // computation can never panic on `visible_content_width`.
        let remembered = ResizableWidths {
            chat: 1200.0,          // above MAX_PANEL_WIDTH
            plan: 50.0,            // below SOFT_OPEN_MIN_WIDTH
            context_files: f32::NAN,
        };
        assert_eq!(width_for_open(LeftTab::Chat, &remembered), MAX_PANEL_WIDTH);
        assert_eq!(width_for_open(LeftTab::Plan, &remembered), SOFT_OPEN_MIN_WIDTH);
        // NaN must not leak through clamp_resizable.
        assert_eq!(
            width_for_open(LeftTab::ContextFiles, &remembered),
            SOFT_OPEN_MIN_WIDTH
        );
    }

    #[test]
    fn width_for_open_resizable_falls_back_to_preferred_on_default() {
        let remembered = ResizableWidths::default();
        // Default Chat=560 is above SOFT_OPEN_MIN_WIDTH so unchanged.
        assert_eq!(width_for_open(LeftTab::Chat, &remembered), 560.0);
        // Default Plan=480 in range.
        assert_eq!(width_for_open(LeftTab::Plan, &remembered), 480.0);
    }

    #[test]
    fn all_tabs_inventory_is_complete() {
        // Defensive: every variant must appear in PRIMARY_TABS or BOTTOM_TAB
        // — nothing leaks into the runtime that the rail can't render.
        let mut in_rail: std::collections::HashSet<LeftTab> =
            PRIMARY_TABS.iter().copied().collect();
        in_rail.insert(BOTTOM_TAB);
        for tab in [
            LeftTab::Project,
            LeftTab::Sessions,
            LeftTab::Chat,
            LeftTab::Plan,
            LeftTab::Tools,
            LeftTab::Skills,
            LeftTab::ContextFiles,
            LeftTab::Archive,
        ] {
            assert!(in_rail.contains(&tab), "{tab:?} missing from rail inventory");
        }
    }

    fn all_tabs() -> Vec<LeftTab> {
        let mut v: Vec<LeftTab> = PRIMARY_TABS.to_vec();
        v.push(BOTTOM_TAB);
        v
    }
}