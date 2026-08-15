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
pub(crate) mod display;
pub(crate) mod project;
pub(crate) mod sessions;
pub(crate) mod shell;

// Re-export event enums + tab structs so callers reach them as
// `tabs::{SessionsEvent, ProjectEvent, SessionsTab, ProjectTab, ShellTab}`
// without naming the submodule.
pub(crate) use project::{ProjectEvent, ProjectTab};
pub(crate) use sessions::{SessionsEvent, SessionsTab};
pub(crate) use shell::ShellTab;
pub(crate) use display::DisplayTab;

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
    Display,
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
            Self::Display => 440.0,
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
            Self::Display => "Display",
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
            Self::Display => "icons/rail-display.svg",
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
    LeftTab::Display,
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
        (false, _, true) => (clicked, panel_width, true),
        _ => (clicked, width_for_open(clicked, remembered), false),
    }
}

/// T281 / Task 7 — the single reducer boundary the plan (§Task 7 Step 1)
/// requires: every keybind/IPC/rail entry point funnels through this pure
/// function so the whole state-machine table (`toggle` / `SelectTab` /
/// `ToggleDock` / `expand-left` / `compose-and-send`) is exercised as one
/// contract instead of five independently-plausible call sites drifting
/// apart. Internally it composes the existing pure helpers
/// (`tab_select_transition`, `dock_transition`, `width_for_open`) — no
/// duplicated policy, this is a documented composition, not a rewrite.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WorkspaceAction {
    Toggle,
    SelectTab(LeftTab),
    ToggleDock,
    ExpandComposer,
    ComposeAndSend,
}

/// Read-only snapshot the reducer needs. `open` is `rail_handle.is_some()`
/// at the call site — the reducer never inspects a `WindowHandle` itself.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WorkspaceSnapshot {
    pub open: bool,
    pub active_tab: LeftTab,
    pub panel_width: f32,
    pub dock_content: bool,
    pub remembered_widths: ResizableWidths,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WorkspaceTransition {
    pub open_rail: bool,
    pub open_content: bool,
    pub active_tab: LeftTab,
    pub dock_content: bool,
    pub panel_width: f32,
    pub focus_composer: bool,
}

/// Pure decision for every `WorkspaceAction`. No `App`/`Window`/`Context` —
/// callers translate `open_rail`/`open_content` into `open_pinned`/`close`,
/// write the remaining fields into `SidePanelLeftState_`, and call
/// `request_focus_composer` when `focus_composer` is set.
///
/// `ExpandComposer`/`ComposeAndSend` are identical at this layer (both
/// must land on Chat, docked, focused) — `compose_and_send`'s extra "submit
/// exactly once after readiness" behaviour is not representable here (it
/// needs the live `ChatTab`/ACP connection) and stays the caller's job,
/// same as before this reducer existed (T247).
pub fn workspace_transition(
    state: WorkspaceSnapshot,
    action: WorkspaceAction,
) -> WorkspaceTransition {
    match action {
        WorkspaceAction::Toggle => {
            if state.open {
                WorkspaceTransition {
                    open_rail: false,
                    open_content: false,
                    active_tab: state.active_tab,
                    dock_content: false,
                    panel_width: RAIL_WIDTH,
                    focus_composer: false,
                }
            } else {
                WorkspaceTransition {
                    open_rail: true,
                    open_content: true,
                    active_tab: state.active_tab,
                    dock_content: false,
                    panel_width: RAIL_WIDTH,
                    focus_composer: false,
                }
            }
        }
        WorkspaceAction::SelectTab(clicked) => {
            // No `!state.open` special case: the rail (source of
            // `SelectTab`) only exists as a window while the workspace is
            // already open, so that combination cannot occur in
            // production. `tab_select_transition` derives "rail-only" vs
            // "content open" from `panel_width` alone (via
            // `visible_content_width`), which is already correct for
            // every reachable case.
            let (next_tab, next_width, next_dock) = tab_select_transition(
                clicked,
                state.active_tab,
                state.panel_width,
                state.dock_content,
                &state.remembered_widths,
            );
            WorkspaceTransition {
                open_rail: true,
                open_content: true,
                active_tab: next_tab,
                dock_content: next_dock,
                panel_width: next_width,
                focus_composer: false,
            }
        }
        WorkspaceAction::ToggleDock => {
            // Same reasoning as `SelectTab`: the dock button only exists
            // on the already-open rail, so `!state.open` cannot occur in
            // production — `dock_transition` derives rail-only from
            // `panel_width` alone.
            let (next_width, next_dock) = dock_transition(
                state.panel_width,
                state.dock_content,
                state.active_tab,
                &state.remembered_widths,
            );
            WorkspaceTransition {
                open_rail: true,
                open_content: true,
                active_tab: state.active_tab,
                dock_content: next_dock,
                panel_width: next_width,
                focus_composer: false,
            }
        }
        WorkspaceAction::ExpandComposer | WorkspaceAction::ComposeAndSend => {
            // Spec: "обеспечить Chat, dock и focus composer" from EVERY
            // entry state (closed / rail-only / content-open / docked).
            // Force the tab regardless of what was active — the previous
            // implementation read `state.active_tab` instead of forcing
            // Chat, so calling expand-left/compose-and-send from a
            // non-Chat tab silently focused/wrote into an entity that
            // was never on screen (the render match only paints Chat
            // when `active_tab == Chat` — see `workspace_view.rs`).
            let width = width_for_open(LeftTab::Chat, &state.remembered_widths)
                .max(SOFT_OPEN_MIN_WIDTH);
            WorkspaceTransition {
                open_rail: true,
                open_content: true,
                active_tab: LeftTab::Chat,
                dock_content: true,
                panel_width: width,
                focus_composer: true,
            }
        }
    }
}

// T279 round 2: the coordinator transitions (`session_select_transition`,
// `project_switch_transition`) were deleted — unconditional helpers that
// returned their input/a constant are tautology bait (the T278 lesson).
// The real policy lives in the free-fn reducers in
// `crate::side_panel_left` (`select_session`, `switch_project`,
// `remove_project_scope`), exercised by name in `#[gpui::test]`s there.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn primary_tabs_in_fixed_order() {
        assert_eq!(
            PRIMARY_TABS,
            &[
                LeftTab::Project,
                LeftTab::Display,
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

    /// Different tab while docked → switch but keep docked width pinned.
    /// Plan line 611 / T281: "In dock mode ... tab switches keep that
    /// docked width pinned"; docked tab switch does NOT apply the tab's
    /// fixed/remembered policy. The `_` arm must not undock here.
    #[test]
    fn select_other_tab_docked_keeps_dock_and_width() {
        let r = tab_select_transition(
            LeftTab::Sessions,
            LeftTab::Chat,
            612.0,
            true,
            &ResizableWidths::default(),
        );
        assert_eq!(r, (LeftTab::Sessions, 612.0, true));
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

    /// Display is also a fixed-width tab (440) — selecting it from rail-only
    /// opens at 440 and ignores any remembered width, exactly like Project.
    #[test]
    fn select_display_uses_fixed_preferred_width() {
        let mut remembered = ResizableWidths::default();
        remembered.chat = 800.0; // irrelevant — Display ignores remembered.
        let r = tab_select_transition(
            LeftTab::Display,
            LeftTab::Chat,
            RAIL_WIDTH,
            false,
            &remembered,
        );
        assert_eq!(
            r,
            (LeftTab::Display, LeftTab::Display.preferred_panel_width(), false)
        );
        assert_eq!(r.1, 440.0);
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
        assert!(!LeftTab::Display.is_resizable());
        assert!(!LeftTab::Sessions.is_resizable());
        assert!(!LeftTab::Tools.is_resizable());
        assert!(!LeftTab::Skills.is_resizable());
        assert!(!LeftTab::Archive.is_resizable());
    }

    #[test]
    fn fixed_widths_match_spec() {
        // Spec §7: Project 440, Sessions 400, Tools 440, Skills 440, Archive 440.
        assert_eq!(LeftTab::Project.preferred_panel_width(), 440.0);
        assert_eq!(LeftTab::Display.preferred_panel_width(), 440.0);
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
            LeftTab::Display,
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
            LeftTab::Display,
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

    // ── T281 / Task 7 — unified `workspace_transition` reducer ──
    //
    // One test per table cell from the plan
    // (`docs/superpowers/plans/2026-08-13-left-ai-workspace-slice-a.md`
    // §Task 7). "Closed" = `open: false`; "Rail-only" = `open: true,
    // dock_content: false, panel_width: RAIL_WIDTH`; "Content open" =
    // `open: true, dock_content: false, panel_width > RAIL_WIDTH`;
    // "Docked" = `open: true, dock_content: true`.

    fn snap(open: bool, tab: LeftTab, width: f32, dock: bool) -> WorkspaceSnapshot {
        WorkspaceSnapshot {
            open,
            active_tab: tab,
            panel_width: width,
            dock_content: dock,
            remembered_widths: ResizableWidths::default(),
        }
    }

    // Toggle row.
    #[test]
    fn toggle_from_closed_opens_both_rail_only() {
        let t = workspace_transition(
            snap(false, LeftTab::Chat, RAIL_WIDTH, false),
            WorkspaceAction::Toggle,
        );
        assert!(t.open_rail && t.open_content);
        assert!(!t.dock_content);
        assert_eq!(t.panel_width, RAIL_WIDTH);
        assert!(!t.focus_composer);
    }

    #[test]
    fn toggle_from_rail_only_closes_both() {
        let t = workspace_transition(
            snap(true, LeftTab::Chat, RAIL_WIDTH, false),
            WorkspaceAction::Toggle,
        );
        assert!(!t.open_rail && !t.open_content);
        assert_eq!(t.panel_width, RAIL_WIDTH);
    }

    #[test]
    fn toggle_from_content_open_closes_both() {
        let t = workspace_transition(
            snap(true, LeftTab::Chat, 612.0, false),
            WorkspaceAction::Toggle,
        );
        assert!(!t.open_rail && !t.open_content);
        assert!(!t.dock_content);
    }

    #[test]
    fn toggle_from_docked_closes_both() {
        let t = workspace_transition(
            snap(true, LeftTab::Chat, 612.0, true),
            WorkspaceAction::Toggle,
        );
        assert!(!t.open_rail && !t.open_content);
        assert!(!t.dock_content);
    }

    // SelectTab row: "active tab click" — n/a closed, open tab, collapse
    // content, no-op docked. Covered together with `tab_select_transition`
    // above; here we prove the reducer wires `open_rail`/`open_content`
    // correctly around it.
    #[test]
    fn select_tab_from_closed_opens_both() {
        let t = workspace_transition(
            snap(false, LeftTab::Chat, RAIL_WIDTH, false),
            WorkspaceAction::SelectTab(LeftTab::Sessions),
        );
        assert!(t.open_rail && t.open_content);
        assert_eq!(t.active_tab, LeftTab::Sessions);
        assert_eq!(t.panel_width, LeftTab::Sessions.preferred_panel_width());
    }

    #[test]
    fn select_tab_active_docked_is_noop() {
        let t = workspace_transition(
            snap(true, LeftTab::Chat, 612.0, true),
            WorkspaceAction::SelectTab(LeftTab::Chat),
        );
        assert_eq!(t.active_tab, LeftTab::Chat);
        assert_eq!(t.panel_width, 612.0);
        assert!(t.dock_content, "dock wins over collapse");
    }

    #[test]
    fn select_tab_active_undocked_collapses() {
        let t = workspace_transition(
            snap(true, LeftTab::Sessions, 444.0, false),
            WorkspaceAction::SelectTab(LeftTab::Sessions),
        );
        assert_eq!(t.panel_width, RAIL_WIDTH);
        assert!(!t.dock_content);
    }

    #[test]
    fn select_tab_other_switches_and_opens() {
        let t = workspace_transition(
            snap(true, LeftTab::Chat, RAIL_WIDTH, false),
            WorkspaceAction::SelectTab(LeftTab::Project),
        );
        assert_eq!(t.active_tab, LeftTab::Project);
        assert_eq!(t.panel_width, 440.0);
    }

    // ToggleDock row.
    #[test]
    fn toggle_dock_from_closed_opens_docked() {
        let t = workspace_transition(
            snap(false, LeftTab::Chat, RAIL_WIDTH, false),
            WorkspaceAction::ToggleDock,
        );
        assert!(t.open_rail && t.open_content);
        assert!(t.dock_content);
        // Closed + Chat active + default remembered widths → opens at the
        // remembered Chat width (560), same as `width_for_open`.
        assert_eq!(t.panel_width, ResizableWidths::default().chat);
    }

    #[test]
    fn toggle_dock_from_rail_only_expands_and_docks() {
        let t = workspace_transition(
            snap(true, LeftTab::Chat, RAIL_WIDTH, false),
            WorkspaceAction::ToggleDock,
        );
        assert!(t.dock_content);
        assert_eq!(t.panel_width, ResizableWidths::default().chat);
    }

    #[test]
    fn toggle_dock_from_content_open_preserves_width_and_docks() {
        let t = workspace_transition(
            snap(true, LeftTab::Chat, 612.0, false),
            WorkspaceAction::ToggleDock,
        );
        assert!(t.dock_content);
        assert_eq!(t.panel_width, 612.0);
    }

    #[test]
    fn toggle_dock_from_docked_undocks_and_preserves_width() {
        let t = workspace_transition(
            snap(true, LeftTab::Chat, 612.0, true),
            WorkspaceAction::ToggleDock,
        );
        assert!(!t.dock_content);
        assert_eq!(t.panel_width, 612.0);
    }

    // ExpandComposer / ComposeAndSend row: identical at this layer, from
    // ALL four entry states — always Chat, docked, focused.
    #[test]
    fn expand_composer_from_every_state_lands_on_chat_docked_focused() {
        let cases = [
            snap(false, LeftTab::Project, RAIL_WIDTH, false), // closed
            snap(true, LeftTab::Sessions, RAIL_WIDTH, false), // rail-only
            snap(true, LeftTab::Project, 612.0, false),       // content open, wrong tab
            snap(true, LeftTab::Sessions, 612.0, true),       // docked, wrong tab
        ];
        for s in cases {
            for action in [WorkspaceAction::ExpandComposer, WorkspaceAction::ComposeAndSend] {
                let t = workspace_transition(s, action);
                assert!(t.open_rail && t.open_content);
                assert_eq!(t.active_tab, LeftTab::Chat, "{s:?} / {action:?} must land on Chat");
                assert!(t.dock_content, "{s:?} / {action:?} must dock");
                assert!(t.focus_composer, "{s:?} / {action:?} must focus composer");
                assert!(t.panel_width >= SOFT_OPEN_MIN_WIDTH);
            }
        }
    }

    // Hard-drag edge: 40 (rail-only) → dock on → 960 → dock off must not
    // clamp outside the drag range at any step (Task 7 gate: "remembered
    // width, hard drag to 40, drag back from 40, and dock/undock").
    #[test]
    fn drag_960_to_40_and_back_stays_in_range_across_dock_toggle() {
        let mut widths = ResizableWidths::default();
        widths.chat = MAX_PANEL_WIDTH; // simulate a drag all the way to 960
        let s = WorkspaceSnapshot {
            open: true,
            active_tab: LeftTab::Chat,
            panel_width: RAIL_WIDTH, // collapsed via drag to 40
            dock_content: false,
            remembered_widths: widths,
        };
        let t = workspace_transition(s, WorkspaceAction::ToggleDock);
        assert!(t.dock_content);
        assert_eq!(t.panel_width, MAX_PANEL_WIDTH, "dock-on from rail-only restores the remembered drag width, clamped in range");
        assert!(t.panel_width <= MAX_PANEL_WIDTH && t.panel_width >= RAIL_WIDTH);
    }
}