# Task 1 Review: Layer-Shell Window + Peek/Pin

**Reviewer:** Lead Architect Agent
**Commit:** `51ba643 feat(side_panel_left): layer-shell window with peek/pin state`
**Diff:** `522232e..51ba643`

---

## Spec Compliance
[VERDICT: PASS]

- **state.rs with PanelState:** Met. `PanelState { Peek, Pinned, Resizing }` enum with `#[derive(Clone, Copy, PartialEq)]`. `SidePanelLeftState` with `state`, `width`, `session_id` fields. `new()` returns correct defaults (Peek, 352.0, None). Exact match with brief code.
- **mod.rs with SidePanelLeft entity:** Met. Struct owns `state: SidePanelLeftState`. `Render` impl delegates to `panel::render_panel`. `SidePanelLeft::new()` constructs correctly.
- **Layer-shell window creation:** Met. `WindowKind::LayerShell(LayerShellOptions { ... })` with all required fields: `namespace`, `layer: Layer::Overlay`, `keyboard_interactivity: KeyboardInteractivity::OnDemand` (matches brief, NOT the right panel's None), `exclusive_zone`, `anchor`. Window opens via `cx.open_window` with correct `WindowOptions`.
- **Public API surface:** Met. `open_pinned`, `open_peek`, `close`, `toggle`, `init` — all present as `pub`.
- **Module registration in main.rs:** Met. `mod side_panel_left;` added alphabetically between `project_switcher` and `side_panel_right`. `side_panel_left::init(cx)` called after `side_panel_right::init(cx)`.
- **Builds:** `cargo check -p chronos` passes. 21 warnings (all pre-existing or expected dead-code from stubs).

### Deviations from brief (all documented in report)

| Brief | Implemented | Justified? |
|---|---|---|
| `gpui_platform::*` imports | `gpui::layer_shell::*` | Yes — matches right panel pattern |
| `Anchor::LEFT \| TOP \| BOTTOM` (stretch) | `Anchor::LEFT \| TOP` (fixed height) | Yes — gpui-layer-shell skill documents that TOP\|BOTTOM + exclusive_zone skews gaps on Hyprland. Fixed height = proven right-panel recipe mirrored to LEFT side |
| `exclusive_zone: Some(px(0.0))` | `exclusive_zone: None` | Yes — functionally equivalent (both = no exclusive zone). Matches right panel |
| `SidePanelLeftState` (global) | `SidePanelLeftState_` (trailing underscore) | Acceptable — avoids collision with per-view state. Naming is cosmetic |
| Brief `create_window` (associated fn) | `open_window` (private fn) + `open_pinned`/`open_peek` (pub fns) | Yes — matches right panel pattern. `init()` pattern also adapted from right panel (50ms deferred spawn, smoke env) |
| Brief had `use gpui::*` in state.rs | Removed (not needed) | Fine — no gpui types in state.rs |

All deviations are either required by the actual codebase (import paths), prevented by known Hyprland bugs (anchor geometry), or matched to existing patterns (right panel). None are gratuitous.

---

## Code Quality
[VERDICT: PASS]

- **Structure:** Clean separation. `state.rs` (22 lines) holds only data types. `mod.rs` (186 lines) handles lifecycle. `panel.rs` (11 lines) is a minimal render stub. Four stubs are single-line comments — correct forward declarations for later tasks.
- **Pattern adherence:** Window creation, global state management, open/close/toggle, init deferred spawn, and test structure are faithful mirrors of `side_panel_right/mod.rs`. This is exactly what the brief requested ("follow right panel patterns").
- **Commit scope:** Clean single commit. Only files specified in the brief (plus forward-declared stubs). `git add` is named, not wildcard.
- **Tests:** Two unit tests for `SidePanelLeftState` — correct defaults verified. Appropriate for a skeleton task.
- **Global state struct:** `SidePanelLeftState_` with `#[derive(Default)]` + `impl Global` is correct. The trailing underscore is ugly but not a bug.
- **`close_this` function:** Correct reentrancy guard pattern from right panel — checks handle match before taking, avoids double `remove_window`.
- **Missing `#[allow(dead_code)]`** on `close_this` — right panel has it, this one doesn't. Minor: dead_code warning will appear until a later task wires it up. Not a build blocker.

---

## Findings

- **Minor:** `SidePanelLeftState_` trailing underscore naming. Functional but cosmetic. Consider renaming the global to `SidePanelLeftGlobal` or `SidePanelLeftWindowTracker` to avoid the underscore convention, which doesn't exist anywhere else in the codebase. Not blocking.
- **Minor:** `close_this` lacks `#[allow(dead_code)]` annotation that the right panel has. Will produce a dead_code warning until Task 8+ wires it. Trivial.
- **Minor:** `state.rs` in the brief included `use gpui::*;` but the implementation correctly removed it (no gpui types needed in the pure-data state file). This is an improvement over the brief, not a deviation.

---

## Task Quality
[VERDICT: Approved]

The implementation delivers exactly what Task 1 promised: a layer-shell window skeleton with peek/pin state, correct module registration, and a pattern that faithfully mirrors the right panel. The anchor deviation from the brief is well-documented and prevented by a real Hyprland bug. Code is clean, builds green, and tests verify the state contract. Ready for Task 2 (sessions list).
