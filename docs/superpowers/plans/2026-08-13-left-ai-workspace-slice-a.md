# Left AI Workspace — Slice A Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the legacy single left layer-shell panel with a stable standalone rail plus fixed content canvas, and ship the complete Slice A product: Project Switcher, Sessions, Chat, project-scoped session restoration, bar migration, and preserved IPC behavior.

**Architecture:** `side_panel_left` mirrors the proven T276 `side_panel_right` two-surface lifecycle, with the horizontal axis reversed. `SidePanelLeftState_` is the sole lifecycle/UI source of truth; content tabs are child entities and never own windows. The rail reserves exactly 40 px, the content surface is a fixed 920 px transparent canvas offset 40 px from the left edge, and resizing changes only the visible/input slice—never the Wayland surface size.

**Tech Stack:** Rust, GPUI/gpui-ce layer-shell, SQLite/rusqlite `ThreadStore`, ChronOS IPC, Hyprland 0.56.1+.

## Global Constraints

- Scope is **Slice A only**. Full implementations of Plan, Context Files, Archive, Tools, and Skills are Slice B/C; this plan creates honest labelled shells only.
- Canonical reference is `crates/app/src/side_panel_right/` after T276. Port its private helpers into `side_panel_left`; do not import private right-panel types and do not invent a third lifecycle.
- No changes under `Source/gpui/`. No `window.resize()` anywhere in `side_panel_left`.
- Hard drag clamp is `40..=960`. `360` is only the soft minimum used when opening/restoring resizable content. Thus `visible_content_width == 0` and the handle-at-zero recovery path remain reachable.
- The left hover strip stays **disabled** in Slice A. Do not call `hover_strip::init_hover_strip`; closed means zero left-panel surfaces. `peek_generation` may remain dormant for compatibility, but no hover acceptance is claimed. This deliberately narrows design-spec live check #10 to the pinned/keybind product chosen on 2026-07-23.
- `Super+A` remains the user entry point: closed → open both fixed surfaces in rail-only visual state (`panel_width = 40`, empty content input region); any open left workspace → close both surfaces.
- Content opens before rail; partial open rolls back atomically. Content uses `exclusive_zone: Some(px(-1.))`, explicit left margin 40 px, `Layer::Overlay`, transparent background, and `KeyboardInteractivity::OnDemand`. Rail uses `KeyboardInteractivity::None` and owns the 40 px exclusive zone.
- No hover-triggered open/close callbacks are wired in Slice A. Dormant `peek_generation` compatibility state must not affect pinned/keybind lifecycle or active resize.
- Responsive decisions use `visible_content_width`, never the fixed 920 px Wayland canvas bounds.
- Preserve unrelated dirty-worktree changes. Every commit stages only files listed in its task.
- Unit green is insufficient. Final acceptance requires a release binary plus live Hyprland geometry, click-through, focus, dock, and drag checks.

## Delivery / ticket split

| Sub-ticket | Scope | Merge gate |
|---|---|---|
| T278 / Slice A1 | tab metadata, left geometry, two-surface lifecycle, rail/workspace chrome | fixed bounds and input-region unit tests |
| T279 / Slice A2 | Chat extraction, Sessions, Project Switcher, honest B/C shells | app tests; no bar removal yet |
| T280 / Slice A3 | ThreadStore v2 project scope, active-session restoration, one-way bar migration | service + bar migration tests |
| T281 / Slice A4 | IPC/focus/dock integration, release build, live proof | owner live `+` |

These are sequential tickets. A2 consumes A1 APIs; A3 consumes A2 project/session coordination; A4 integrates all three. Do not run them in parallel in one worktree.

---

## Task 1: Freeze Slice A tab and width contracts (A1)

**Files:**

- Create: `crates/app/src/side_panel_left/tabs/mod.rs`
- Modify: `crates/app/src/side_panel_left/state.rs`
- Modify: `crates/app/src/side_panel_left/mod.rs`

**Interfaces:**

- Produces:

```rust
pub const RAIL_WIDTH: f32 = 40.0;
pub const RESIZE_HANDLE_WIDTH: f32 = 4.0;
pub const MAX_PANEL_WIDTH: f32 = 960.0;
pub const CONTENT_CANVAS_WIDTH: f32 = MAX_PANEL_WIDTH - RAIL_WIDTH;
pub const SOFT_OPEN_MIN_WIDTH: f32 = 360.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ResizableWidths {
    pub chat: f32,
    pub plan: f32,
    pub context_files: f32,
}

impl Default for ResizableWidths {
    fn default() -> Self {
        Self { chat: 560.0, plan: 480.0, context_files: 560.0 }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
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
    pub const fn is_resizable(self) -> bool;
    pub const fn preferred_panel_width(self) -> f32;
    pub const fn label(self) -> &'static str;
    pub const fn icon_path(self) -> &'static str;
}

pub fn visible_content_width(panel_width: f32) -> f32;
pub fn content_interactive_width(visible_width: f32, resizing: bool) -> f32;
pub fn content_input_region(visible_width: f32, height: f32, resizing: bool) -> Vec<Bounds<Pixels>>;
pub fn resize_handle_x(visible_width: f32) -> f32;
pub fn resize_target_width(start_width: f32, start_x: f32, current_x: f32) -> f32;
pub fn width_for_open(tab: LeftTab, remembered: &ResizableWidths) -> f32;
```

- Consumes `gpui::{Bounds, Pixels, point, size, px}`.
- `resize_target_width` is exactly `clamp(start_width + (current_x - start_x), 40, 960)`.
- `width_for_open` returns exact fixed widths: Project 440, Sessions 400, Tools 440, Skills 440, Archive 440. For Chat/Plan/Context Files it independently clamps that tab's runtime-only remembered width to `360..=960`.
- Project, Sessions, Tools, Skills, and Archive are fixed-width. Chat, Plan, and Context Files are resizable.

- [ ] **Step 1: Write failing tab-policy tests.**

Add tests covering every enum variant, exact rail order, resize policy, fixed/preferred widths, and Archive as the bottom item. Run:

```bash
cargo test -p chronos side_panel_left::tabs --lib
```

Expected: FAIL because the new tab contracts do not exist.

- [ ] **Step 2: Implement `LeftTab` metadata without rendering or stores.**

Use the existing ChronOS SVG assets; do not add generated bitmap icons. Make the rail-order constant explicit:

```rust
pub const PRIMARY_TABS: &[LeftTab] = &[
    LeftTab::Project,
    LeftTab::Sessions,
    LeftTab::Chat,
    LeftTab::Plan,
    LeftTab::Tools,
    LeftTab::Skills,
    LeftTab::ContextFiles,
];
pub const BOTTOM_TAB: LeftTab = LeftTab::Archive;
```

- [ ] **Step 3: Write failing LEFT geometry tests.**

Cover these exact contracts:

```rust
assert_eq!(visible_content_width(40.0), 0.0);
assert_eq!(visible_content_width(360.0), 320.0);
assert_eq!(visible_content_width(960.0), 920.0);
assert_eq!(resize_target_width(500.0, 100.0, 72.0), 472.0);
assert_eq!(resize_target_width(40.0, 100.0, 0.0), 40.0);
assert_eq!(resize_handle_x(0.0), 0.0);
assert_eq!(resize_handle_x(920.0), 916.0);
assert_eq!(content_interactive_width(0.0, true), 4.0);
assert!(content_input_region(0.0, 100.0, false).is_empty());
```

Also assert left alignment: the non-empty region always starts at `x = 0`, not `CONTENT_CANVAS_WIDTH - visible_width`.

- [ ] **Step 4: Port the pure geometry helpers from T276 with the axis mirrored.**

Keep calculations in `state.rs` pure. No geometry or state-transition helper receives `Window`, `App`, or `Context`.

- [ ] **Step 5: Run focused tests and commit A1 contracts.**

```bash
cargo test -p chronos side_panel_left::tabs --lib
cargo test -p chronos side_panel_left::state --lib
git add crates/app/src/side_panel_left/tabs/mod.rs crates/app/src/side_panel_left/state.rs crates/app/src/side_panel_left/mod.rs
git commit -m "feat(left-panel): define workspace geometry and tab contracts"
```

Expected: both focused suites PASS.

---

## Task 2: Build the fixed rail and content canvas lifecycle (A1)

**Files:**

- Create: `crates/app/src/side_panel_left/rail_view.rs`
- Create: `crates/app/src/side_panel_left/workspace_view.rs`
- Modify: `crates/app/src/side_panel_left/mod.rs`
- Reference only: `crates/app/src/side_panel_right/mod.rs`
- Reference only: `crates/app/src/side_panel_right/rail_view.rs`
- Reference only: `crates/app/src/side_panel_right/view.rs`

**Interfaces:**

- Produces state:

```rust
pub struct SidePanelLeftState_ {
    rail_handle: Option<WindowHandle<RailView>>,
    content_handle: Option<WindowHandle<WorkspaceView>>,
    content_view: Option<WeakEntity<WorkspaceView>>,
    active_tab: LeftTab,
    panel_width: f32,
    remembered_widths: ResizableWidths,
    active_project_path: Option<PathBuf>,
    active_session_id: Option<String>,
    dock_content: bool,
    resizing: bool,
    pinned: bool,
    peek_generation: u64,
    last_exclusive_zone: Option<i32>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TwoSurfaceOpenOutcome {
    Opened,
    ContentFailed,
    RailFailedRollbackContent,
}

fn two_surface_open_outcome(content_opened: bool, rail_opened: bool) -> TwoSurfaceOpenOutcome;
fn rail_window_options(display: &Display) -> WindowOptions;
fn content_window_options(display: &Display) -> WindowOptions;
pub fn open_pinned(cx: &mut App);
pub fn close(cx: &mut App);
pub fn toggle(cx: &mut App);
```

- `RailView` owns no workspace/product state; it reads/mutates `SidePanelLeftState_` through public coordinator methods.
- `WorkspaceView` renders a transparent 920 px canvas and sets `window.set_input_region(...)` from the pure helpers.
- Neither view calls `window.resize()`.
- Opening order is content → rail. If rail creation fails, close the newly opened content and clear both handles/entity references.
- To keep every commit runnable, A1 temporarily embeds the existing `Entity<SidePanelLeft>` as product content inside `WorkspaceView`. At this point `SidePanelLeft` loses all window handles, open/close, exclusive-zone, width, dock, and resize ownership; it is only the legacy product-state child. A2 splits that child into Chat/Sessions/Project entities and removes the bridge. There is never a second runtime window path.

- [ ] **Step 1: Add failing option/lifecycle tests before opening windows.**

Test the pure open outcome for all four boolean pairs and test an extracted option descriptor for:

- rail: `TOP | LEFT`, width 40, exclusive edge LEFT, keyboard NONE;
- content: `TOP | LEFT`, width 920, left margin 40, exclusive zone `-1`, keyboard ON_DEMAND;
- both: Overlay, Transparent, identical top gap and monitor selection.

Run:

```bash
cargo test -p chronos side_panel_left::tests::two_surface --lib
```

Expected: FAIL until the state and pure outcome helper exist.

- [ ] **Step 2: Implement atomic two-surface opening and rollback.**

Port `two_surface_open_outcome` into left-panel code; do not import the right-private helper. Store a weak content entity so rail actions cannot create a second product state owner.

- [ ] **Step 3: Implement the standalone rail shell.**

Render Project at the top, PRIMARY_TABS in order, a flex spacer, Archive, then the dock toggle. Active-tab behavior is exact:

```rust
match (clicked == state.active_tab, visible_content_width(state.panel_width) > 0.0, state.dock_content) {
    (true, true, true) => { /* dock wins: no collapse */ }
    (true, true, false) => collapse_to_rail_only_without_closing_content_surface(cx),
    _ => select_tab_and_open(clicked, cx),
}
```

The rail sets/caches its exclusive zone from shared state, matching T276. It must not own a duplicate `active_tab`, width, dock, or session id. Every button has a stable ID and tooltip; the active accent strip sits on the rail's right edge, hover never masks active state, text labels are not rendered inside 40 px, and Archive remains below a `flex_1` spacer.

- [ ] **Step 4: Implement fixed-canvas input and resize behavior.**

`WorkspaceView::render` calls `set_input_region` on every meaningful width/height/resizing change. The resize handle is at `x = resize_handle_x(visible_w)` and remains interactive at panel width 40 while a drag is active. On mouse-up, `resizing=false` makes the zero-width region empty.

The content background fills only the visible slice. The unused canvas stays transparent and click-through; do not paint an opaque 920 px root. Draw the thin themed separator on the visible content's outer/right edge, with the transparent 4 px resize hitbox over that same edge. Draw the rail separator on the rail's right edge; neither line may extend into the transparent void.

- [ ] **Step 5: Keep hover strip disabled and remove the legacy one-window resize path.**

Delete the combined-surface window render path that mutates exclusive zone and invokes `window.resize()`. Render its existing product body through the temporary child entity described above, so Chat/Sessions do not disappear between A1 and A2. Leave `hover_strip.rs` uninitialized. Add a source-contract test/search assertion that `side_panel_left` contains no `window.resize(` calls.

- [ ] **Step 6: Verify A1 and commit the lifecycle cutover.**

```bash
cargo test -p chronos side_panel_left --lib --bins
cargo check -p chronos --lib
rg -n 'window\.resize\(' crates/app/src/side_panel_left
git add crates/app/src/side_panel_left
git commit -m "feat(left-panel): split rail and fixed workspace canvas"
```

Expected: tests/check PASS; `rg` returns no matches. Do not claim live success yet.

---

## Task 3: Extract Chat as a window-independent tab (A2)

**Files:**

- Create: `crates/app/src/side_panel_left/tabs/chat.rs`
- Modify: `crates/app/src/side_panel_left/composer.rs`
- Modify: `crates/app/src/side_panel_left/chat_view.rs`
- Modify: `crates/app/src/side_panel_left/text_input.rs`
- Modify: `crates/app/src/side_panel_left/tool_card.rs`
- Modify: `crates/app/src/side_panel_left/workspace_view.rs`
- Modify: `crates/app/src/side_panel_left/mod.rs`
- Delete after migration: `crates/app/src/side_panel_left/panel.rs`

**Interfaces:**

- Produces:

```rust
pub struct ChatTab { /* existing ACP, transcript, composer, focus and streaming state */ }

impl ChatTab {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self;
    pub fn load_thread(&mut self, thread: ThreadRecord, cx: &mut Context<Self>);
    pub fn clear_for_project(&mut self, project_path: &Path, cx: &mut Context<Self>);
    pub fn focus_composer(&mut self, window: &mut Window, cx: &mut Context<Self>);
    pub fn compose_and_send(&mut self, text: String, window: &mut Window, cx: &mut Context<Self>);
    pub fn visible_width_changed(&mut self, visible_width: f32, cx: &mut Context<Self>);
}
```

- Consumes existing ACP/Hermes client, transcript, composer, model/mode, streaming, and persistence behavior from the legacy `SidePanelLeft`; no protocol rewrite.
- `ChatTab` owns no `WindowHandle`, panel width, dock flag, active tab, project selector, or sessions list.
- Responsive layout consumes the passed visible width, never `window.bounds().size.width == 920`.
- This task removes the temporary A1 `Entity<SidePanelLeft>` bridge: `WorkspaceView` directly owns the extracted child entities, and the old monolithic product entity no longer exists.

- [ ] **Step 1: Add failing ownership and responsive tests.**

Test the breakpoint selector as a pure function at widths immediately below, at, and above the existing layout breakpoint. Construct `ChatTab` as a child entity in the existing GPUI test harness. Add a source-contract command to this task's gate:

```bash
rg -n 'WindowHandle|open_window|window\.resize\(' crates/app/src/side_panel_left/tabs/chat.rs
```

Expected: no matches.

- [ ] **Step 2: Move chat/composer/ACP fields and methods without changing behavior.**

Preserve all current send, reconnect, transcript, tool-card, model/mode, follow-output, and streaming cancellation behavior. `Drop for ChatTab` must cancel the same GPUI task handles currently cancelled by `Drop for SidePanelLeft`.

- [ ] **Step 3: Render Chat from `WorkspaceView`.**

`WorkspaceView` lazily creates and reuses `Entity<ChatTab>` and forwards only project/thread/focus/visible-width commands. Delete the legacy combined `panel.rs` once no call site imports it.

- [ ] **Step 4: Run focused regression tests and commit.**

```bash
cargo test -p chronos side_panel_left::tabs::chat --lib --bins
cargo test -p chronos side_panel_left --lib --bins
cargo check -p chronos --lib
rg -n 'WindowHandle|open_window|window\.resize\(' crates/app/src/side_panel_left/tabs/chat.rs
git add crates/app/src/side_panel_left
git commit -m "refactor(left-panel): isolate chat tab from window lifecycle"
```

Expected: existing chat/composer tests and new visible-width tests PASS.

---

## Task 4: Implement Sessions, Project Switcher, and honest shells (A2)

**Files:**

- Create: `crates/app/src/side_panel_left/tabs/sessions.rs`
- Create: `crates/app/src/side_panel_left/tabs/project.rs`
- Create: `crates/app/src/side_panel_left/tabs/shell.rs`
- Modify: `crates/app/src/side_panel_left/workspace_view.rs`
- Modify: `crates/app/src/side_panel_left/rail_view.rs`
- Modify: `crates/app/src/side_panel_left/mod.rs`
- Modify: `crates/app/src/project_switcher/mod.rs`
- Delete after full-tab wiring: popup-only view/state code in `crates/app/src/project_switcher/mod.rs`

**Interfaces:**

- Produces:

```rust
pub struct SessionsTab;
impl SessionsTab {
    pub fn set_project(&mut self, project_path: PathBuf, cx: &mut Context<Self>);
    pub fn selected_thread(&self) -> Option<&str>;
}

pub enum SessionsEvent {
    SelectThread(String),
    CreateThread,
    RenameThread { id: String, title: String },
    ArchiveThread(String),
}

pub struct ProjectTab;
pub enum ProjectEvent {
    Select(PathBuf),
    Add(PathBuf),
    Remove(PathBuf),
    OpenInTerminal(PathBuf),
    OpenInFiles(PathBuf),
}

pub struct ShellTab {
    tab: LeftTab,
}
```

- `project_switcher` remains owner of `ProjectsConfig`, persistence, branch lookup, portal selection, and project actions. `ProjectTab` is the embedded GPUI view, not a second config backend. The canonical project path from `ProjectsConfig` is the identity passed to Sessions, Chat, and `ThreadStore`.
- Project tab includes search, recent projects, current branch, selection, add/remove, Files, and Terminal actions.
- Sessions selection emits `SelectThread`; the workspace coordinator changes active session then opens Chat.
- Shells state the feature name and `Coming in Slice B` or `Coming in Slice C`. Plan and Context Files shells obey resizable width policy; Tools, Skills, and Archive shells use fixed widths.

- [ ] **Step 1: Write failing coordinator transition tests.**

Test pure transitions for:

- project click → Project content opens;
- active non-docked tab click → rail-only;
- active docked tab click → no-op;
- another tab click → switch and open;
- session selection → selected id + Chat tab;
- project switch → all session/chat state is cleared before loading the new scope;
- every B/C rail button renders an honest shell, never an empty opaque panel.

- [ ] **Step 2: Extract Project Switcher domain from popup lifecycle.**

Keep `ProjectsConfig::{load, save, active_entry}`, `cached`, `reload_cache`, `current_branch`, and portal/action helpers callable from `ProjectTab`. Remove `ProjectPopupState`, popup window options, and popup toggle only after all callers use the embedded tab.

- [ ] **Step 3: Implement Sessions and Project tabs.**

Reuse current session-row rendering and actions from `sessions_list.rs`; do not duplicate rows in `WorkspaceView`. Project selection is one coordinator transaction: clear old Chat/Sessions view state, update `ProjectsConfig.active`, ask the store for the new project's active thread, then either load it into Chat or show an empty Chat state.

- [ ] **Step 4: Implement shells and rail navigation.**

All eight rail buttons must be reachable by pointer, expose stable element IDs, reflect active state, and keep Archive below the flex spacer. Dock toggle is separate from Archive. Create tab entities lazily on first selection and retain them for reuse; project/session-scoped data is reloaded from shared identifiers/stores, not cached as an independent project owner in each tab.

- [ ] **Step 5: Verify and commit A2.**

```bash
cargo test -p chronos side_panel_left --lib --bins
cargo test -p chronos project_switcher --lib --bins
cargo check -p chronos --lib
git add crates/app/src/side_panel_left crates/app/src/project_switcher/mod.rs
git commit -m "feat(left-panel): add project sessions and chat workspace tabs"
```

Expected: transition, project, session, and shell tests PASS; existing popup call sites are gone.

---

## Task 5: Migrate ThreadStore to project scope and persist the active session (A3)

**Files:**

- Modify: `crates/services/src/threads.rs`
- Modify: `crates/app/src/side_panel_left/tabs/sessions.rs`
- Modify: `crates/app/src/side_panel_left/workspace_view.rs`

**Interfaces:**

- Produces schema version 2:

```sql
ALTER TABLE threads ADD COLUMN project_path TEXT;
UPDATE threads SET project_path = cwd WHERE project_path IS NULL;
CREATE INDEX IF NOT EXISTS idx_threads_project_updated
    ON threads(project_path, archived, updated_at DESC);
CREATE TABLE IF NOT EXISTS workspace_project_state (
    project_path TEXT PRIMARY KEY NOT NULL,
    active_thread_id TEXT,
    FOREIGN KEY(active_thread_id) REFERENCES threads(id) ON DELETE SET NULL
);
PRAGMA user_version = 2;
```

- Produces APIs:

```rust
pub fn insert_for_project(
    &self,
    id: &str,
    agent_id: &str,
    cwd: &str,
    project_path: &str,
) -> anyhow::Result<()>;
pub fn list_for_project(&self, project_path: &str, archived: bool) -> anyhow::Result<Vec<ThreadRecord>>;
pub fn set_active_thread(&self, project_path: &str, thread_id: Option<&str>) -> anyhow::Result<()>;
pub fn active_thread(&self, project_path: &str) -> anyhow::Result<Option<ThreadRecord>>;
```

- `ThreadRecord` gains `project_path: String`.
- Existing `insert(id, agent_id, cwd)` remains as a compatibility wrapper that sets `project_path = cwd` until all non-workspace callers migrate.
- `active_thread` returns `None` when the stored id is absent, archived, deleted, or belongs to another project; it must not leak another project's chat.

- [ ] **Step 1: Write failing v1→v2 migration tests.**

Create a temporary v1 database with rows whose `cwd` differs across projects. Open it through `ThreadStore::open`, then assert version 2, backfilled `project_path`, index/table existence, and unchanged transcript/session data.

- [ ] **Step 2: Write failing isolation/restoration tests.**

Cover two projects with multiple sessions, independent active ids, archive/delete invalidation, missing/stale ids, and the compatibility `insert` wrapper.

Run:

```bash
cargo test -p chronos-services --lib threads
```

Expected: FAIL against schema v1.

- [ ] **Step 3: Implement the transactional migration and APIs.**

Run schema changes in one SQLite transaction. Validate a candidate active thread with both `id` and `project_path`; never load by id alone for workspace restoration.

- [ ] **Step 4: Wire workspace project/session restoration.**

On project selection, call `active_thread(project_path)`. A valid row selects Sessions and loads Chat; stale/missing state yields empty Chat. On session selection/creation call `set_active_thread`. On archive/delete of the active session clear or replace the persisted selection deterministically.

- [ ] **Step 5: Verify and commit the store half of A3.**

```bash
cargo test -p chronos-services --lib threads
cargo test -p chronos side_panel_left --lib --bins
cargo check -p chronos --lib
git add crates/services/src/threads.rs crates/app/src/side_panel_left/tabs/sessions.rs crates/app/src/side_panel_left/workspace_view.rs
git commit -m "feat(threads): scope sessions and active state by project"
```

Expected: service migration/isolation and app restoration tests PASS.

---

## Task 6: Remove the bar project owner with a one-way config migration (A3)

**Files:**

- Modify: `crates/app/src/bar/layout_config.rs`
- Modify: `crates/app/src/bar/widgets/mod.rs`
- Modify: `crates/app/src/bar/mod.rs`
- Delete: `crates/app/src/bar/widgets/project.rs`

**Interfaces:**

- `BarLayoutConfig::load` removes the exact builtin name `project` from `left`, `center`, `right`, and `known`, then persists the normalized config once.
- Default config and builtin catalog no longer advertise `project`.
- Unknown third-party names keep existing behavior; only the retired builtin receives silent one-way removal.

- [ ] **Step 1: Write failing migration tests.**

Cover `project` in each section, duplicated entries, `known`, default config, and an unrelated unknown widget. Assert the saved TOML no longer contains `project`, while the unrelated unknown survives according to current policy.

```bash
cargo test -p chronos bar::layout_config --lib
```

Expected: FAIL because `project` is still a builtin/default.

- [ ] **Step 2: Implement migration and remove widget wiring.**

Remove the module, instantiate arm, semantic grouping references, builtin name, and default right-section entry. Keep the full Project tab as the only UI owner.

- [ ] **Step 3: Verify and commit the bar half of A3.**

```bash
cargo test -p chronos bar::layout_config --lib
cargo test -p chronos bar --lib --bins
cargo check -p chronos --lib
git add crates/app/src/bar/layout_config.rs crates/app/src/bar/widgets/mod.rs crates/app/src/bar/mod.rs crates/app/src/bar/widgets/project.rs
git commit -m "feat(bar): migrate project switcher into left workspace"
```

Expected: bar tests PASS; loading old `bar.toml` cannot resurrect the pill.

---

## Task 7: Preserve keybind, IPC, focus, dock, and resize state machines (A4)

**Files:**

- Modify: `crates/app/src/side_panel_left/mod.rs`
- Modify: `crates/app/src/side_panel_left/rail_view.rs`
- Modify: `crates/app/src/side_panel_left/workspace_view.rs`
- Modify: `crates/app/src/side_panel_left/tabs/chat.rs`
- Verify only: `crates/app/src/ipc/service.rs`
- Verify only: `crates/app/src/ipc/messages.rs`

**Interfaces:**

- Preserves exact public signatures:

```rust
pub fn toggle(cx: &mut App);
pub fn expand_with_composer(cx: &mut App);
pub fn compose_and_send(text: String, cx: &mut App);
```

- State machine:

| Entry | Closed | Rail-only | Content open | Docked |
|---|---|---|---|---|
| `toggle` / Super+A | open both surfaces, visual rail-only | close both | close both | close both |
| `expand_with_composer` | open both, Chat, dock | open content, Chat, dock | switch Chat, dock | keep Chat+dock |
| `compose_and_send(text)` | ensure both, Chat+dock, focus, submit | same | switch Chat+dock, focus, submit | focus, submit |
| active tab click | n/a | open tab | collapse content | no-op |
| dock toggle | open/expand content docked | open at remembered/preferred width and dock | save current width and dock | undock while keeping current width |

- Any transition to Chat through Sessions, `expand_with_composer`, or `compose_and_send` focuses the composer after the content window exists.
- Rail has no keyboard focus; content is OnDemand.
- Dock wins over active-tab collapse. In dock mode the rail reserves the current full panel width; tab switches keep that docked width pinned. Undock keeps the current width, and the next ordinary tab switch reapplies that tab's fixed/remembered policy.

- [ ] **Step 1: Add failing pure state-machine tests.**

Introduce and test this exact reducer boundary:

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum WorkspaceAction {
    Toggle,
    SelectTab(LeftTab),
    ToggleDock,
    ExpandComposer,
    ComposeAndSend,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct WorkspaceTransition {
    open_rail: bool,
    open_content: bool,
    active_tab: LeftTab,
    dock_content: bool,
    panel_width: f32,
    focus_composer: bool,
}

fn workspace_transition(state: WorkspaceSnapshot, action: WorkspaceAction) -> WorkspaceTransition;
```

Cover every table cell, including content-open rollback, remembered width, hard drag to 40, drag back from 40, and dock/undock.

- [ ] **Step 2: Wire focus only after a live content entity/window exists.**

Queue the focus/send action through the opened `WorkspaceView`/`ChatTab`; do not silently drop it when called from closed or rail-only state. `compose_and_send` must submit exactly once after connection readiness, preserving T247 behavior.

- [ ] **Step 3: Verify IPC dispatch compatibility.**

Run the existing `expand-left` and `compose-and-send` protocol tests unchanged first. Modify dispatcher code only if the coordinator ownership change requires it; do not rename payloads or add a parallel command.

- [ ] **Step 4: Run A4 automated gates and commit.**

```bash
cargo test -p chronos side_panel_left --lib --bins
cargo test -p chronos ipc --lib --bins
cargo test -p chronos-services --lib threads
cargo test -p chronos --lib --bins
cargo build --release -p chronos
rg -n 'window\.resize\(' crates/app/src/side_panel_left
git add crates/app/src/side_panel_left
git commit -m "fix(left-panel): preserve workspace IPC focus and dock flows"
```

Expected: all tests/build PASS; source search has zero matches; IPC payload and dispatcher files remain unchanged because the public coordinator signatures are preserved.

---

## Task 8: Live Hyprland acceptance and evidence (A4)

**Files:**

- Create: `docs/orchestration/tasks/report-log/T281-left-ai-workspace-slice-a-report.md`
- Modify after owner acceptance: `docs/ARCHITECTURE.md`
- Modify after owner acceptance: `docs/DECISIONS.log`

**Interfaces / evidence:**

- `hyprctl layers` must show exactly two logical left surfaces in every open state, including visual rail-only: 40 px rail and 920 px content canvas; content bounds stay 920 px during drag.
- Closed state must show zero left rail/content/hover-strip surfaces. Visual rail-only keeps both fixed surfaces alive but gives content an empty input region and paints no content slice.
- Rail exclusive zone remains 40 px in overlay and 960 px in dock mode; content remains `exclusive_zone = -1` and starts at the rail boundary without double offset.
- Transparent unused canvas is click-through; no painted strip, gap, missing separator, buried handle, or inaccessible mouse target.

- [ ] **Step 1: Start the release binary through the project scripts.**

```bash
./scripts/dev/chronos-stop
./scripts/dev/chronos-start
hyprctl layers
```

Capture the initial closed/rail-only/content-open layer snapshots in the report. Do not infer bounds from screenshots alone.

- [ ] **Step 2: Exercise geometry and resize live.**

Verify:

1. Super+A closed → rail-only; Super+A again → zero surfaces.
2. Project/Sessions/Chat open the correct tab; active click collapses to rail-only.
3. Chat drags continuously from 960 to 40 and back; content canvas layer remains 920 throughout.
4. Handle remains reachable at the rail-only clamp and is on the content's outer/right edge.
5. Plan and Context shells use the resizable policy; Tools, Skills, Archive, Sessions, Project use fixed widths.
6. No wobble, wallpaper flash, rail/content gap, opaque canvas strip, or broken outer border.
7. Pointer clicks pass through the unused transparent canvas.
8. Dock reserves the current full panel width; active tab click does not collapse while docked; undock keeps the current width until the next normal tab switch reapplies policy.

- [ ] **Step 3: Exercise product/session/focus live.**

Verify:

1. Project tab search/recent/branch/actions work and the bar project pill is gone after config migration.
2. Switching projects immediately swaps Sessions and never shows another project's transcript.
3. Restart restores the last valid session per project; archived/deleted/stale active ids yield empty Chat.
4. Session selection opens Chat and focuses the composer.
5. `expand-left` and `compose-and-send` work from closed, rail-only, content-open, and docked states; text submits exactly once.
6. Plan/Tools/Skills/Context/Archive clearly identify themselves as Slice B/C shells.

- [ ] **Step 4: Record evidence and request owner verdict.**

The report must list commit hashes, exact commands/exits, `hyprctl layers` measurements, screenshots/video paths, every unproven item, and the owner verdict. Do not mark the task done before the owner sends `+`.

- [ ] **Step 5: After owner `+`, update architecture records and commit closure docs.**

Document the two-surface left workspace, project ownership migration, ThreadStore v2, hard/soft width distinction, and disabled hover strip. Then run:

```bash
git add docs/orchestration/tasks/report-log/T281-left-ai-workspace-slice-a-report.md docs/ARCHITECTURE.md docs/DECISIONS.log
git commit -m "docs: close left AI workspace slice A"
```

Expected: documentation reflects only live-proven behavior; Slice B/C remain explicitly open.

---

## Plan self-review checklist

- [ ] Every Slice A requirement maps to a task: two surfaces, rail, Project, Sessions, Chat, project-scoped restore, bar migration, IPC, focus, dock, and live proof.
- [ ] No Slice B/C domain store or full UI slipped into implementation scope.
- [ ] LEFT formulas are mirrored, not copied with right-aligned offsets.
- [ ] Drag hard floor 40 and soft open floor 360 are tested separately.
- [ ] Hover strip is intentionally disabled and absent from live surface counts.
- [ ] `SidePanelLeftState_` remains the sole lifecycle/UI source of truth; tabs own no windows.
- [ ] Project configuration has one backend owner; project context has one UI owner after migration.
- [ ] ThreadStore migration is transactional, preserves v1 data, and cannot restore cross-project state.
- [ ] All responsive checks use visible width, not fixed canvas bounds.
- [ ] No fork edit and no `window.resize()` are permitted.
- [ ] Unit, release, Hyprland geometry, visual, pointer, keyboard, and owner gates are all explicit.
