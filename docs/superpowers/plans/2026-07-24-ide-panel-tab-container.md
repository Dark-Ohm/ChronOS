# Shell-IDE Right Panel — Tab Container Foundation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.
>
> **ChronOS-specific note:** this repo's Architect does not dispatch Claude
> subagents — this plan is executed by a human-directed local minion via
> `orchestration/tasks/active/`, following `orchestration/agents/rules.md`
> (poимённый `git add`, no unrequested commits to master, worktree
> isolation). Treat "subagent" below as "the assigned minion."

**Goal:** Turn `side_panel_right` from a single fixed view into a 10-tab
IDE-style container — vertical icon-rail on the left, the existing System
Sidebar content preserved byte-for-byte as the first tab, nine other tabs
rendering an honest placeholder. This is the foundation that unblocks all
nine follow-up tabs (each is its own future plan/T-task, not part of this
one).

**Architecture:** New `PanelTab` enum + ordering (pure, unit-tested) drives
a new `IconRail` component (44px vertical strip, one button per tab) and a
`render_tab_content` dispatcher inside `SidePanelRightView`. Only the
`System` arm renders the current body (header/permission/scroll/footer,
untouched); the other nine arms render a shared placeholder view. Panel
width goes from 352px (fixed) to 560px (still fixed — drag-resize is
explicitly out of scope for this plan, see Global Constraints).

**Tech Stack:** GPUI (this fork, `../Source/gpui`), existing
`crates/app/src/side_panel_right/` module, `chronos_ui::Theme` for colors,
embedded SVG assets via `crates/app/src/assets.rs`.

## Global Constraints

- Design brief: `design.md` §"Shell-IDE правая панель (таб-контейнер)" —
  order of the 10 tabs is fixed, do not reorder.
- Accepted mockup: `design/shell-ide-panel.zip` (Next.js/Tailwind preview,
  `components/panel/theme.ts` has the literal canon hex values — this plan
  uses the same values via `Theme::global(cx)`, never hardcoded hex).
- Palette: Catppuccin Mocha roles only, via `chronos_ui::Theme::global(cx)`
  — `theme.bg.*`, `theme.text.*`, `theme.accent.*`. Never a raw hex that
  isn't already what `Theme` resolves to.
- **Drag-resize is OUT of scope for this plan.** Width is a fixed constant
  bump (352→560). The left-panel resize-handle pattern (`fbcadd6`) is a
  reasonable model for a *later* task, not this one — do not add it here.
- **The `System` tab's inner layout/behavior must not change.** It is the
  already-accepted `System Sidebar v2` — this plan only wraps it in a new
  tab-switch shell, touching zero lines inside `render_header`,
  `render_permission_card`, the scroll section, or `render_footer`.
- `unsafe_code = "deny"` at the workspace level (`Cargo.toml:29`) — nothing
  in this plan needs `unsafe`, do not introduce any.
- `clippy::unwrap_used` / `expect_used` are `warn` — do not add new
  `.unwrap()`/`.expect()` in this plan's code; all data here is either a
  compile-time constant or an infallible pure function.
- Icons: `crates/app/src/assets.rs` `icons!` macro embeds SVGs from
  `crates/app/assets/icons/*.svg` at compile time (`include_bytes!`). Any
  new icon needs a real `.svg` file in that directory AND a new entry in
  the macro list — both, or the build fails with an unresolved `match`.
- Existing pattern for rendering an icon: `svg().path("icons/name.svg").size(px(N)).text_color(color)`
  (see `crates/app/src/bar/widgets/battery.rs:80`).
- Live UX verification for window/layout code is release build + grim,
  not unit tests alone (`HANDOFF.md`, `verification-before-completion`
  skill) — Task 5 below is mandatory, not optional polish.

## Out of scope for this plan (separate future plans/T-tasks)

- Files tab (real file tree) — needs its own plan.
- Editor tab (Kate-style, persistent sessions) — needs its own plan; this
  is the single largest remaining piece and should be scoped down
  carefully when planned (full syntax-highlighting engine and LSP wiring
  are almost certainly not v1).
- Terminal tab — likely reuses `crates/app/src/desktop_terminal/` PTY/VT100
  view embedded instead of windowed; needs its own plan.
- ACP settings tab — wire to `crates/services/src/hermes_acp/registry.rs`
  (this module already exists, that's a real advantage for whoever plans
  it next).
- MCP / LSP / API-providers settings tabs — **no backend service exists
  for any of these three today** (`crates/services/src/` has no `mcp`,
  `lsp`, or `api_providers` module — confirmed by directory listing before
  writing this plan). Each needs a new `crates/services/src/<name>/`
  module planned from scratch, not just a view.
- Editor settings tab — local config form, needs its own plan.
- Hyprland binds tab (live, watches `hyprland.lua`) — needs a new
  `crates/services/src/hyprland_binds/` parser+watcher, its own plan.

---

### Task 1: `PanelTab` enum and tab ordering

**Files:**
- Create: `crates/app/src/side_panel_right/tabs.rs`
- Modify: `crates/app/src/side_panel_right/mod.rs:13-20` (add `mod tabs; pub use tabs::PanelTab;`)
- Test: inline `#[cfg(test)] mod tests` in `tabs.rs`

**Interfaces:**
- Consumes: nothing (pure, no dependencies on other tasks).
- Produces: `pub enum PanelTab { System, Files, Editor, Terminal,
  AcpSettings, McpSettings, LspSettings, ApiProviders, EditorSettings,
  HyprlandBinds }`, `PanelTab::ALL: [PanelTab; 10]`, `PanelTab::label(self)
  -> &'static str`, `PanelTab::icon_path(self) -> &'static str`. Task 3
  (`IconRail`) and Task 4 (`SidePanelRightView`) both consume these three
  items by exact name.

- [ ] **Step 1: Write the failing tests**

```rust
// crates/app/src/side_panel_right/tabs.rs (top of file, before the enum)

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_has_ten_tabs_in_fixed_order() {
        assert_eq!(PanelTab::ALL.len(), 10);
        assert_eq!(PanelTab::ALL[0], PanelTab::System);
        assert_eq!(PanelTab::ALL[1], PanelTab::Files);
        assert_eq!(PanelTab::ALL[9], PanelTab::HyprlandBinds);
    }

    #[test]
    fn every_tab_has_a_non_empty_label() {
        for tab in PanelTab::ALL {
            assert!(!tab.label().is_empty(), "{tab:?} has an empty label");
        }
    }

    #[test]
    fn every_tab_has_a_distinct_icon_path() {
        let paths: Vec<&str> = PanelTab::ALL.iter().map(|t| t.icon_path()).collect();
        let mut sorted = paths.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), paths.len(), "two tabs share an icon path");
    }

    #[test]
    fn system_is_the_default_active_tab() {
        assert_eq!(PanelTab::default(), PanelTab::System);
    }
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p chronos --lib side_panel_right::tabs -- --nocapture`
Expected: FAIL to compile — `PanelTab` does not exist yet.

- [ ] **Step 3: Write the minimal implementation**

```rust
// crates/app/src/side_panel_right/tabs.rs (below the test module, or above — module order doesn't matter in Rust)

//! Tab identity and fixed ordering for the IDE tab-container.
//!
//! Order is fixed by the design brief (`design.md` §"Shell-IDE правая
//! панель (таб-контейнер)") — do not reorder `ALL` without updating the
//! brief and the accepted mockup.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PanelTab {
    #[default]
    System,
    Files,
    Editor,
    Terminal,
    AcpSettings,
    McpSettings,
    LspSettings,
    ApiProviders,
    EditorSettings,
    HyprlandBinds,
}

impl PanelTab {
    pub const ALL: [PanelTab; 10] = [
        PanelTab::System,
        PanelTab::Files,
        PanelTab::Editor,
        PanelTab::Terminal,
        PanelTab::AcpSettings,
        PanelTab::McpSettings,
        PanelTab::LspSettings,
        PanelTab::ApiProviders,
        PanelTab::EditorSettings,
        PanelTab::HyprlandBinds,
    ];

    pub fn label(self) -> &'static str {
        match self {
            PanelTab::System => "System",
            PanelTab::Files => "Files",
            PanelTab::Editor => "Editor",
            PanelTab::Terminal => "Terminal",
            PanelTab::AcpSettings => "ACP settings",
            PanelTab::McpSettings => "MCP settings",
            PanelTab::LspSettings => "LSP settings",
            PanelTab::ApiProviders => "API providers",
            PanelTab::EditorSettings => "Editor settings",
            PanelTab::HyprlandBinds => "Hyprland binds",
        }
    }

    pub fn icon_path(self) -> &'static str {
        match self {
            PanelTab::System => "icons/rail-system.svg",
            PanelTab::Files => "icons/folder.svg",
            PanelTab::Editor => "icons/rail-editor.svg",
            PanelTab::Terminal => "icons/rail-terminal.svg",
            PanelTab::AcpSettings => "icons/rail-acp.svg",
            PanelTab::McpSettings => "icons/rail-mcp.svg",
            PanelTab::LspSettings => "icons/rail-lsp.svg",
            PanelTab::ApiProviders => "icons/rail-api.svg",
            PanelTab::EditorSettings => "icons/rail-editor-settings.svg",
            PanelTab::HyprlandBinds => "icons/rail-binds.svg",
        }
    }
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p chronos --lib side_panel_right::tabs -- --nocapture`
Expected: `test result: ok. 4 passed; 0 failed`

- [ ] **Step 5: Wire the module in**

```rust
// crates/app/src/side_panel_right/mod.rs — add near the other `mod` lines (currently lines 13-20)
mod tabs;
pub use tabs::PanelTab;
```

- [ ] **Step 6: Commit**

```bash
git add crates/app/src/side_panel_right/tabs.rs crates/app/src/side_panel_right/mod.rs
git commit -m "side_panel_right : PanelTab enum, fixed 10-tab order"
```

---

### Task 2: Rail icon assets

**Files:**
- Create: `crates/app/assets/icons/rail-system.svg`
- Create: `crates/app/assets/icons/rail-editor.svg`
- Create: `crates/app/assets/icons/rail-terminal.svg`
- Create: `crates/app/assets/icons/rail-acp.svg`
- Create: `crates/app/assets/icons/rail-mcp.svg`
- Create: `crates/app/assets/icons/rail-lsp.svg`
- Create: `crates/app/assets/icons/rail-api.svg`
- Create: `crates/app/assets/icons/rail-editor-settings.svg`
- Create: `crates/app/assets/icons/rail-binds.svg`
- Modify: `crates/app/src/assets.rs:25-48` (the `icons!(...)` macro call)

**Interfaces:**
- Consumes: the exact path strings from Task 1's `PanelTab::icon_path()`
  (the `"icons/rail-*.svg"` names must match these filenames exactly, minus
  the `icons/` prefix which the macro adds).
- Produces: nine loadable SVG assets. Task 3 (`IconRail`) renders them via
  `svg().path(tab.icon_path())`.

`folder.svg` already exists (reused for `Files` — no new file needed for
that one).

- [ ] **Step 1: Add the nine SVG files**

Same style as the existing icons (`viewBox 0 0 256 256`, `fill="currentColor"`,
single-color line-art) — see `crates/app/assets/icons/folder.svg` for the
exact convention. These are first-pass placeholder glyphs (simple
geometric shapes, not hand-tuned line-art) — refining them later is a
design task, not a blocker for this plan.

```xml
<!-- crates/app/assets/icons/rail-system.svg -->
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 256 256" fill="currentColor"><rect x="32" y="56" width="192" height="120" rx="12"/><rect x="104" y="192" width="48" height="16"/><rect x="80" y="208" width="96" height="12" rx="6"/></svg>
```

```xml
<!-- crates/app/assets/icons/rail-editor.svg -->
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 256 256" fill="currentColor"><path d="M56 32h96l48 48v144a8 8 0 01-8 8H56a8 8 0 01-8-8V40a8 8 0 018-8z"/><rect x="72" y="120" width="112" height="12" fill="black" opacity="0"/><rect x="72" y="120" width="80" height="10" fill="#000" fill-opacity="0"/><rect x="72" y="120" width="80" height="10" style="mix-blend-mode:destination-out"/><rect x="72" y="148" width="60" height="10" style="mix-blend-mode:destination-out"/></svg>
```

```xml
<!-- crates/app/assets/icons/rail-terminal.svg -->
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 256 256" fill="currentColor"><rect x="24" y="48" width="208" height="160" rx="12" style="mix-blend-mode:destination-out"/><path d="M24 60a12 12 0 0112-12h176a12 12 0 0112 12v136a12 12 0 01-12 12H36a12 12 0 01-12-12z" opacity="0"/><path d="M24 60a12 12 0 0112-12h176a12 12 0 0112 12v136a12 12 0 01-12 12H36a12 12 0 01-12-12V60z" fill="none" stroke="currentColor" stroke-width="12"/><path d="M56 96l32 28-32 28" fill="none" stroke="currentColor" stroke-width="14" stroke-linecap="round" stroke-linejoin="round"/><line x1="104" y1="152" x2="152" y2="152" stroke="currentColor" stroke-width="14" stroke-linecap="round"/></svg>
```

```xml
<!-- crates/app/assets/icons/rail-acp.svg -->
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 256 256" fill="currentColor"><circle cx="64" cy="72" r="28"/><circle cx="192" cy="72" r="28"/><circle cx="128" cy="192" r="28"/><line x1="64" y1="72" x2="128" y2="192" stroke="currentColor" stroke-width="14"/><line x1="192" y1="72" x2="128" y2="192" stroke="currentColor" stroke-width="14"/></svg>
```

```xml
<!-- crates/app/assets/icons/rail-mcp.svg -->
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 256 256" fill="currentColor"><rect x="88" y="96" width="80" height="80" rx="10"/><rect x="104" y="48" width="16" height="48" rx="6"/><rect x="136" y="48" width="16" height="48" rx="6"/><rect x="112" y="176" width="32" height="40" rx="6"/></svg>
```

```xml
<!-- crates/app/assets/icons/rail-lsp.svg -->
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 256 256" fill="currentColor"><rect x="72" y="72" width="112" height="112" rx="10"/><rect x="112" y="32" width="12" height="32"/><rect x="132" y="32" width="12" height="32"/><rect x="112" y="192" width="12" height="32"/><rect x="132" y="192" width="12" height="32"/><rect x="32" y="112" width="32" height="12"/><rect x="32" y="132" width="32" height="12"/><rect x="192" y="112" width="32" height="12"/><rect x="192" y="132" width="32" height="12"/></svg>
```

```xml
<!-- crates/app/assets/icons/rail-api.svg -->
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 256 256" fill="currentColor"><circle cx="88" cy="88" r="40"/><circle cx="88" cy="88" r="16" style="mix-blend-mode:destination-out"/><rect x="120" y="120" width="96" height="20" transform="rotate(45 120 120)"/><rect x="150" y="150" width="16" height="24" transform="rotate(45 150 150)"/><rect x="176" y="176" width="16" height="24" transform="rotate(45 176 176)"/></svg>
```

```xml
<!-- crates/app/assets/icons/rail-editor-settings.svg -->
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 256 256" fill="currentColor"><rect x="32" y="72" width="192" height="10" rx="5"/><circle cx="96" cy="77" r="18"/><rect x="32" y="123" width="192" height="10" rx="5"/><circle cx="168" cy="128" r="18"/><rect x="32" y="174" width="192" height="10" rx="5"/><circle cx="120" cy="179" r="18"/></svg>
```

```xml
<!-- crates/app/assets/icons/rail-binds.svg -->
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 256 256" fill="currentColor"><rect x="24" y="64" width="208" height="128" rx="14"/><rect x="48" y="88" width="20" height="20" style="mix-blend-mode:destination-out"/><rect x="80" y="88" width="20" height="20" style="mix-blend-mode:destination-out"/><rect x="112" y="88" width="20" height="20" style="mix-blend-mode:destination-out"/><rect x="144" y="88" width="20" height="20" style="mix-blend-mode:destination-out"/><rect x="176" y="88" width="20" height="20" style="mix-blend-mode:destination-out"/><rect x="64" y="152" width="128" height="18" rx="6" style="mix-blend-mode:destination-out"/></svg>
```

Note: `style="mix-blend-mode:destination-out"` may not be honored by the
SVG rasterizer this fork uses — if the "cutout" holes don't render (icon
looks like a solid blob instead of a keyboard/editor with gaps), simplify
to plain solid shapes without cutouts for this first pass; visual polish
is a follow-up design task, not a blocker. Verify visually in Task 5.

- [ ] **Step 2: Register the new icons in the macro**

```rust
// crates/app/src/assets.rs — extend the icons!(...) call (currently lines 25-48)
icons!(
    "arrow-up.svg",
    "arrows-clockwise.svg",
    "battery.svg",
    "battery-charging.svg",
    "bell.svg",
    "bolt.svg",
    "chevron-down.svg",
    "folder.svg",
    "hexagon-core.svg",
    "hexagon-sigil.svg",
    "pause.svg",
    "play.svg",
    "power.svg",
    "rail-acp.svg",
    "rail-api.svg",
    "rail-binds.svg",
    "rail-editor.svg",
    "rail-editor-settings.svg",
    "rail-lsp.svg",
    "rail-mcp.svg",
    "rail-system.svg",
    "rail-terminal.svg",
    "sign-out.svg",
    "skip-back.svg",
    "skip-forward.svg",
    "speaker-high.svg",
    "speaker-low.svg",
    "speaker-mute.svg",
    "speaker-none.svg",
    "users.svg",
    "x.svg",
);
```

- [ ] **Step 3: Verify the build picks up the new assets**

Run: `cargo check -p chronos`
Expected: no "unresolved" errors from `assets.rs` (a missing file would fail
`include_bytes!` at compile time — a clean `cargo check` is the proof the
nine files exist and are named correctly).

- [ ] **Step 4: Commit**

```bash
git add crates/app/assets/icons/rail-*.svg crates/app/src/assets.rs
git commit -m "assets : add 9 IDE-panel rail icons"
```

---

### Task 3: `IconRail` component

**Files:**
- Create: `crates/app/src/side_panel_right/rail.rs`
- Modify: `crates/app/src/side_panel_right/mod.rs` (add `mod rail;`)
- Test: inline `#[cfg(test)] mod tests` in `rail.rs`

**Interfaces:**
- Consumes: `PanelTab`, `PanelTab::ALL`, `PanelTab::label()`,
  `PanelTab::icon_path()` from Task 1.
- Produces: `pub fn render_rail(active: PanelTab, on_select: impl Fn(PanelTab, &mut Window, &mut App) + 'static) -> impl IntoElement`
  and the pure helper `pub fn rail_button_bg(is_active: bool, theme: &Theme) -> Hsla`
  (exact name/signature — Task 4 does not call `rail_button_bg` directly,
  but its test below documents the exact active/inactive contract so a
  future visual tweak can't silently invert it).

- [ ] **Step 1: Write the failing test for the pure active/inactive color rule**

```rust
// crates/app/src/side_panel_right/rail.rs (test module)

#[cfg(test)]
mod tests {
    use super::*;
    use chronos_ui::Theme;

    #[test]
    fn active_tab_uses_interactive_hover_fill_inactive_is_transparent() {
        let theme = Theme::default();
        assert_eq!(rail_button_bg(true, &theme), theme.interactive.hover);
        assert_eq!(rail_button_bg(false, &theme), gpui::transparent_black());
    }
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test -p chronos --lib side_panel_right::rail -- --nocapture`
Expected: FAIL to compile — `rail_button_bg` / module `rail` doesn't exist.

- [ ] **Step 3: Write the minimal implementation**

```rust
// crates/app/src/side_panel_right/rail.rs
//! Vertical icon-rail — switches the active tab of the IDE panel.
//!
//! One `on_hover`-free button per `PanelTab::ALL`; active tab gets an
//! `accent.primary` bar on its left edge + `interactive.hover` fill.
//! Design brief: `design.md` §"Shell-IDE правая панель (таб-контейнер)".

use gpui::{div, prelude::*, px, svg, App, Hsla, IntoElement, Window};

use chronos_ui::Theme;

use crate::side_panel_right::tabs::PanelTab;

const RAIL_WIDTH: f32 = 44.;
const BUTTON_SIZE: f32 = 36.;

pub fn rail_button_bg(is_active: bool, theme: &Theme) -> Hsla {
    if is_active {
        theme.interactive.hover
    } else {
        gpui::transparent_black()
    }
}

pub fn render_rail(
    active: PanelTab,
    on_select: impl Fn(PanelTab, &mut Window, &mut App) + 'static + Clone,
) -> impl IntoElement {
    let theme = Theme::global_static();
    div()
        .id("side-panel-right-rail")
        .flex()
        .flex_col()
        .items_center()
        .gap(px(4.))
        .py(px(8.))
        .w(px(RAIL_WIDTH))
        .h_full()
        .bg(theme.bg.tertiary)
        .border_r_1()
        .border_color(theme.border.default)
        .children(PanelTab::ALL.into_iter().map(|tab| {
            let is_active = tab == active;
            let on_select = on_select.clone();
            div()
                .id(("rail-tab", tab as usize))
                .relative()
                .flex()
                .items_center()
                .justify_center()
                .size(px(BUTTON_SIZE))
                .rounded(theme.radius)
                .bg(rail_button_bg(is_active, theme))
                .on_click(move |_, window, cx| on_select(tab, window, cx))
                .child(
                    svg()
                        .path(tab.icon_path())
                        .size(px(20.))
                        .text_color(if is_active {
                            theme.text.primary
                        } else {
                            theme.text.muted
                        }),
                )
                .when(is_active, |el| {
                    el.child(
                        div()
                            .absolute()
                            .left(px(-8.))
                            .top(px(BUTTON_SIZE / 2. - 10.))
                            .w(px(3.))
                            .h(px(20.))
                            .rounded(px(2.))
                            .bg(theme.accent.primary),
                    )
                })
        }))
}
```

**Note on `Theme::global_static()`:** check the exact accessor this fork's
`chronos_ui::Theme` exposes for a context-free reference before writing
this — every existing caller in this codebase has `cx`/`window` in scope
and calls `Theme::global(cx)` (see `battery.rs:61`). `render_rail` as
signed above takes no `cx` — that's a real gap. Fix before Step 4: either
(a) add a `cx: &App` parameter to `render_rail` and call
`Theme::global(cx)` normally (simplest, matches every other file in this
module), or (b) confirm a truly global accessor exists. Prefer (a) — it's
one extra parameter, zero new risk, and consistent with `render_header`/
`render_footer` in this same directory. Adjust the signature and the two
call sites (this file's doc example above and Task 4's call) accordingly
before running Step 4.

- [ ] **Step 4: Run the test to verify it passes**

Run: `cargo test -p chronos --lib side_panel_right::rail -- --nocapture`
Expected: `test result: ok. 1 passed; 0 failed`

- [ ] **Step 5: Wire the module in**

```rust
// crates/app/src/side_panel_right/mod.rs
mod rail;
```

- [ ] **Step 6: Commit**

```bash
git add crates/app/src/side_panel_right/rail.rs crates/app/src/side_panel_right/mod.rs
git commit -m "side_panel_right : IconRail component"
```

---

### Task 4: Wire the tab container into `SidePanelRightView`

**Files:**
- Modify: `crates/app/src/side_panel_right/view.rs:39-53` (struct fields)
- Modify: `crates/app/src/side_panel_right/view.rs:55-118` (`new`)
- Modify: `crates/app/src/side_panel_right/view.rs:180-298` (`Render` impl)
- Modify: `crates/app/src/side_panel_right/mod.rs:31` (`PANEL_WIDTH`)
- Test: inline `#[cfg(test)] mod tests` addition in `view.rs` (existing
  test module is elsewhere in the file — add to it, don't create a second
  one)

**Interfaces:**
- Consumes: `PanelTab` (Task 1), `render_rail` (Task 3).
- Produces: `SidePanelRightView` now has a working `active_tab: PanelTab`
  field and switches content on rail click. Nothing outside this module
  consumes this directly (window lifecycle in `mod.rs` is unaffected).

- [ ] **Step 1: Write the failing test for the pure tab-switch logic**

```rust
// crates/app/src/side_panel_right/view.rs — add inside the existing
// `#[cfg(test)] mod tests` block, if `mod.rs` doesn't already have one for
// this file; this repo's convention (see `mod.rs:236-253`) is a small
// `#[cfg(test)] mod tests` at the bottom of the file that owns the logic.

#[test]
fn switching_tabs_replaces_active_tab() {
    assert_eq!(next_active_tab(PanelTab::System, PanelTab::Files), PanelTab::Files);
    assert_eq!(next_active_tab(PanelTab::Files, PanelTab::Files), PanelTab::Files);
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test -p chronos --lib side_panel_right::view -- --nocapture`
Expected: FAIL to compile — `next_active_tab` doesn't exist.

- [ ] **Step 3: Write the minimal implementation**

```rust
// crates/app/src/side_panel_right/view.rs — free function, module level
// (place it near `format_bytes_per_sec` at the bottom of the file)

/// Pure: clicking a rail button always makes that tab active — no toggle,
/// no special-case for re-clicking the already-active tab.
fn next_active_tab(_current: PanelTab, clicked: PanelTab) -> PanelTab {
    clicked
}
```

Add the field to the struct (near `scroll: ScrollHandle,` at line 52):

```rust
pub struct SidePanelRightView {
    // ... existing fields unchanged ...
    scroll: ScrollHandle,
    active_tab: PanelTab,
}
```

Initialize it in `new()` (in the `Self { ... }` block, alongside `scroll: ScrollHandle::new(),`):

```rust
            scroll: ScrollHandle::new(),
            active_tab: PanelTab::default(),
```

Add the click handler as a method on `SidePanelRightView` (near
`on_power_click`):

```rust
    pub(crate) fn on_tab_select(&mut self, tab: PanelTab, cx: &mut Context<Self>) {
        self.active_tab = next_active_tab(self.active_tab, tab);
        cx.notify();
    }
```

Restructure the `Render` impl's body div: wrap the *existing* header/
permission/scroll/footer children (currently direct children of
`#side-panel-body`, lines ~221-295) into one branch, add the rail as a
sibling, and add a placeholder branch for the other nine tabs. The System
branch's inner content is moved, not rewritten:

```rust
        // side-panel-body becomes a row: rail + content column.
        div()
            .id("side-panel-right-root")
            .size_full()
            .on_hover(|hovered, _window, cx| {
                if *hovered {
                    crate::side_panel_right::hold_peek(cx);
                } else {
                    crate::side_panel_right::schedule_release_peek(cx);
                }
            })
            .child(
                div()
                    .id("side-panel-body")
                    .with_transition("side-panel-body")
                    .size_full()
                    .bg(rgb(0x18_18_25))
                    .border_l_1()
                    .border_color(rgb(0x31_32_44))
                    .flex()
                    .flex_row() // was flex_col — rail sits left of content now
                    .overflow_hidden()
                    .opacity(if revealed { 1.0 } else { 0.0 })
                    .transition_when(
                        revealed,
                        Duration::from_millis(REVEAL_MS),
                        Linear,
                        |s| s.opacity(1.0),
                    )
                    .child({
                        let active = self.active_tab;
                        crate::side_panel_right::rail::render_rail(
                            active,
                            cx.listener(|this, tab: &PanelTab, _window, cx| {
                                this.on_tab_select(*tab, cx);
                            }),
                        )
                    })
                    .child(
                        div()
                            .id("side-panel-content-column")
                            .flex_1()
                            .min_w(px(0.))
                            .flex()
                            .flex_col()
                            .overflow_hidden()
                            .when(self.active_tab == PanelTab::System, |col| {
                                col
                                    // 1. Header (flex:none) — rsx
                                    .child(render_header())
                                    // 2. Permission card (flex:none) — rsx
                                    .child(render_permission_card())
                                    // 3. Scrollable middle — UNCHANGED body
                                    .child(
                                        div()
                                            .id("side-panel-scroll")
                                            .flex_1()
                                            .min_h(px(0.))
                                            .overflow_y_scroll()
                                            .track_scroll(&self.scroll)
                                            .flex()
                                            .flex_col()
                                            .gap(px(14.))
                                            .p(px(14.))
                                            .child(render_mpris_card(&self.mpris, cx))
                                            .child(
                                                div()
                                                    .flex()
                                                    .flex_col()
                                                    .gap(px(10.))
                                                    .child(render_spectrum_row(
                                                        "CPU",
                                                        &self.cpu_history,
                                                        &format!("{:.0}%", self.system.cpu_percent),
                                                        color_cpu(),
                                                        color_cpu(),
                                                        H_CPU,
                                                    ))
                                                    .child(render_spectrum_row(
                                                        "RAM",
                                                        &self.ram_history,
                                                        &format!("{:.0}%", self.system.ram_percent),
                                                        color_ram(),
                                                        color_ram(),
                                                        H_RAM,
                                                    ))
                                                    .when_some(gpu, |d, gpu_pct| {
                                                        d.child(render_spectrum_row(
                                                            "GPU",
                                                            &self.gpu_history,
                                                            &format!("{gpu_pct:.0}%"),
                                                            color_gpu(),
                                                            color_gpu(),
                                                            H_GPU,
                                                        ))
                                                    }),
                                            )
                                            .child(
                                                div()
                                                    .flex()
                                                    .flex_col()
                                                    .gap(px(10.))
                                                    .child(render_spectrum_row(
                                                        "↓ down",
                                                        &self.net_dl_history,
                                                        &dl,
                                                        color_net(),
                                                        color_value_default(),
                                                        H_NET,
                                                    ))
                                                    .child(render_spectrum_row(
                                                        "↑ up",
                                                        &self.net_ul_history,
                                                        &ul,
                                                        color_net(),
                                                        color_value_default(),
                                                        H_NET,
                                                    )),
                                            )
                                            .child(render_disks_section(&self.disks, cx)),
                                    )
                                    // 4. Footer (flex:none)
                                    .child(render_footer(&net_summary, power_arm, cx))
                            })
                            .when(self.active_tab != PanelTab::System, |col| {
                                col.child(
                                    div()
                                        .size_full()
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .child(
                                            div()
                                                .text_color(rgb(0x6c_70_86))
                                                .child(format!("{} — coming soon", self.active_tab.label())),
                                        ),
                                )
                            }),
                    ),
            )
```

Bump the width constant in `mod.rs`:

```rust
// crates/app/src/side_panel_right/mod.rs:31
/// Mockup width (`design/shell-ide-panel.zip` — tab container, 10 tabs).
const PANEL_WIDTH: f32 = 560.;
```

- [ ] **Step 4: Run the test to verify it passes**

Run: `cargo test -p chronos --lib side_panel_right -- --nocapture`
Expected: all `side_panel_right::*` tests pass, including the new
`switching_tabs_replaces_active_tab`.

- [ ] **Step 5: Full workspace build check**

Run: `cargo check -p chronos`
Expected: clean, no errors. `cx.listener` closures and the `PanelTab: Copy`
derive from Task 1 should make the borrow-checker happy — if you hit an
RPIT-lifetime-capture error (E0502/E0499) on the closure passed to
`render_rail`, this repo has hit this class of bug three times already
(`HANDOFF.md` 2026-07-23) — the fix is building the rail element before
any other `cx.listener(...)` call in this render, not after.

- [ ] **Step 6: Commit**

```bash
git add crates/app/src/side_panel_right/view.rs crates/app/src/side_panel_right/mod.rs
git commit -m "side_panel_right : wire tab container, System preserved, 9 placeholders"
```

---

### Task 5: Live verification (mandatory, not optional)

**Files:** none (verification only).

- [ ] **Step 1: Release build**

Run: `cargo build --release`
Expected: clean build, no warnings introduced by this plan's code.

- [ ] **Step 2: Live smoke — rail renders, System tab unchanged**

```bash
pkill -x chronos 2>/dev/null
RUST_LOG=info ./target/release/chronos &
sleep 2
# open the panel however this session's bind/IPC opens it (toggle-side-panel-right
# or whatever the current bar-click/hotkey target is — check mod.rs::toggle callers)
grim -g "$(hyprctl clients -j | jq -r '.[] | select(.class=="chronos-side-panel-right") | "\(.at[0]),\(.at[1]) \(.size[0])x\(.size[1])"')" /tmp/ide-panel-rail-system.png
```

Expected: panel is now 560px wide, rail visible on the left with 10
buttons, `System` tab active by default and pixel-identical to the
pre-change System Sidebar (spectrum rows, mpris card, disks, footer all
present and unchanged).

- [ ] **Step 3: Live smoke — click through all 10 rail buttons**

Click (or dispatch the equivalent action) on each of the other 9 rail
buttons in turn; grim each. Expected: each shows `"<Label> — coming soon"`
centered, rail's active-tab accent bar moves to the clicked icon, no
crash, no panic in `RUST_LOG=info` output.

- [ ] **Step 4: Record the result**

If everything in Steps 2-3 holds, this plan is done. If the SVG cutout
holes from Task 2 didn't render (solid blobs instead of keyboard/editor
icons with gaps), note it — it's a cosmetic follow-up, not a re-open of
this plan.

---

## Self-review

**Spec coverage:** design brief's tab order, icon-rail-not-horizontal-tabs,
and the 560px width assumption are all implemented (Tasks 1, 3, 4). System
tab preservation is explicit (Task 4, byte-for-byte moved content). Drag-
resize and the other nine tabs' real content are explicitly out of scope
(Global Constraints + "Out of scope" section) — each is a gap by design,
not an oversight, and each has a one-line pointer to what it needs before
it can be planned (especially MCP/LSP/API-providers, which need new
services from scratch).

**Placeholder scan:** no TBD/TODO in code steps; the one open item
(`Theme::global_static()` in Task 3) is flagged explicitly as "verify
before Step 4," with both a concrete diagnosis and a concrete preferred
fix — not a vague "handle appropriately."

**Type consistency:** `PanelTab` (Task 1) is `Copy`, used identically in
`render_rail(active: PanelTab, ...)` (Task 3) and
`self.active_tab: PanelTab` / `next_active_tab(PanelTab, PanelTab) ->
PanelTab` (Task 4) — same type, same derive, no mismatch.
