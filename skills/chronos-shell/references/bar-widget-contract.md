# Bar widget contract + multi-agent-tree isolation verify

Verified against `crates/app/src/bar/widgets/*` (clock.rs = reference impl,
workspaces.rs = known-good new widget) on 2026-07-17.

## Registration contract (the LIVE one — Cline's pattern)

`widgets/mod.rs` owns the registry bootstrap. Each widget lives in its own
module that exports `pub fn register(cx: &mut App)`:

```rust
// crates/app/src/bar/widgets/mod.rs
mod clock;
mod workspaces;          // ← your one added line

use gpui::App;
use chronos_luau::bar::BarWidgetRegistry;

pub fn register_builtin(cx: &mut App) {
    clock::register(cx);
    workspaces::register(cx);   // ← your one added call
    // battery::register(cx);   // other agents append below, one mod + one call
    // network::register(cx);
    // tray::register(cx);
}
```

Each widget module's `register` does the boxing:

```rust
// crates/app/src/bar/widgets/workspaces.rs
pub fn register(cx: &mut App) {
    cx.global_mut::<BarWidgetRegistry>()
        .register(Box::new(WorkspacesWidget));
}
```

`bar/mod.rs` calls `widgets::register_builtin(cx)` from `init` after building
`Bar::new`. `BarWidgetRegistry` is a `Global` (no `new()` — use
`cx.global_mut()`).

**NEVER rewrite `widgets/mod.rs` wholesale.** Other agents append their own
`mod X;` + `X::register(cx);` lines. Add exactly the required lines. IMPORTANT
pre-edit check (2026-07-17 lesson): the file may already be in `HEAD`, so run
`git show HEAD:crates/app/src/bar/widgets/mod.rs` and `git diff HEAD -- …` before
editing. In this session the file was ALREADY committed by Cline with `// TEMP`
placeholder slots left for other agents:
```rust
// mod workspaces;            // ← TEMP slot for you
// workspaces::register(cx);  // ← TEMP slot for you
```
The correct edit was to **uncomment those 2 lines** (remove the `// ` prefix) —
NOT rewrite the file. A prior agent panicked, believed it had "overwritten
Cline's file," and rewrote it, creating a checkout-war risk. If the committed
file has slots for your widget, just uncomment/add exactly those lines and
**preserve Cline's shape, comments, and his own `mod clock;`/`clock::register`**
lines. Do NOT additionally strip his unused imports or re-sort — keep your diff
to exactly the required lines (the task said "ровно 2 строки / не переформатируй").
If the file is genuinely missing (Cline never merged it), coordinate via the
Architect; do not silently invent a replacement.

## `BarWidget` trait shape

```rust
impl BarWidget for WorkspacesWidget {
    fn name(&self) -> &str { "workspaces" }
    fn section(&self) -> BarSection { BarSection::Left }   // Left | Center | Right
    fn render(&self, _window: &mut Window, cx: &App) -> AnyElement { /* … */ }
}
```

`render` takes `(&self, &mut Window, &App)` → `AnyElement`. Build with raw
`gpui::div()`.

## Required imports (gpui-fork traps — these bite)

```rust
use gpui::{div, prelude::*, px, AnyElement, App, InteractiveElement, Window, rgba};
use chronos_luau::bar::{BarSection, BarWidget, BarWidgetRegistry};
use chronos_services::{CompositorCommand, Service};   // Service trait MUST be in scope
use chronos_ui::Theme;
use crate::state::AppState;
```

- **`rgba` is NOT in `prelude`** — `use gpui::rgba` explicitly. `rgba(0xffffffff)`
  is BE `0xRRGGBBAA` (here = opaque white).
- **`Service` trait must be imported** for `.get()` / `.dispatch()` / `.subscribe()`
  to resolve. `CompositorSubscriber` implements `Service`; calling `.get()`
  without `use chronos_services::Service;` is `E0599 no method named get`.
  (`dispatch` is an inherent method but the trait import is the safe default.)

## Compositor data + switching workspaces

```rust
let compositor = AppState::compositor(cx);   // → &CompositorSubscriber
let state = compositor.get();                // → CompositorState (needs Service in scope)
// state.workspaces: Vec<Workspace { id: i32, name: String, active: bool }>
// state.active_id: i32

// click handler — closure is Fn(&ClickEvent, &mut Window, &mut App):
.on_click(move |_event, _window, cx: &mut App| {
    let _ = AppState::compositor(cx)
        .dispatch(CompositorCommand::FocusWorkspace(id));
})
```

`CompositorCommand::FocusWorkspace(i32)` already exists in
`crates/services/src/compositor/mod.rs` and is wired through
`hyprland::execute_command` — **do NOT touch `crates/services`** to add a switch
command. The task's "add switch-workspace to dispatch" step is already done.

## Theme fields used for badges

`Theme::global(cx)`: `accent.primary`, `bg.secondary`, `text.muted`,
`text.primary`, `border.focused`, `radius` (for `.rounded()`), `font_sizes.sm`.
Active badge → `accent.primary` bg + opaque `rgba(0xffffffff)` fg; idle →
`bg.secondary` + `text.muted`. Conditional border: `.when(ws.active, |el|
el.border_l_2().border_color(theme.border.focused))`.

## Multi-agent shared-tree isolation verify (ad-hoc, not suite-green)

When the workspace tree is edited by several agents at once, the build is often
broken by **peers'** WIP, not yours (e.g. a peer's `clock.rs` calling chrono
APIs that don't match the locked version; an untracked `tray/` service breaking
`chronos-services`; a duplicate `register_builtin(cx)` call from a merge
artifact). To verify ONLY your slice:

1. **Back up** the files you'll temporarily touch (your `widgets/mod.rs` and,
   if a peer service is the blocker, `crates/services/src/lib.rs`).
2. In **your** `widgets/mod.rs`, comment out peers' `mod X;` + `X::register(cx);`
   lines (do NOT edit their widget files).
3. If a peer's service crate is the blocker, in `crates/services/src/lib.rs`
   temporarily disable `pub mod tray;`, its `pub use tray::{…}`, the
   `pub tray: TraySubscriber,` struct field, and `tray: TraySubscriber::new(),`
   constructor arg.
4. `cargo build -p chronos 2>&1 | grep -E "error|Finished"` — confirm `Finished`
   with no `error[` lines in YOUR modules.
   **The isolation build catches YOUR OWN real errors too** — in this session it
   surfaced two genuine bugs in `workspaces.rs` (missing `use
   chronos_services::Service;` for `.get()`, and `rgba` not in the gpui import
   → `E0599`/`E0425`). Fix those; they are yours, not peer noise. Don't dismiss
   all errors as "peer WIP" — read each one.
5. **Restore the exact originals** (a `trap cleanup EXIT` in a `hermes-verify-*.sh`
   under `/tmp` is the safe pattern) so your committed deliverable is only your
   2 lines.
6. **Re-run the full `cargo build --workspace` at the very end** before reporting.
   Peers' concurrent WIP often self-resolves: mid-session this tree was red from
   peers' `clock.rs`/`tray` WIP, but by the final build other agents had
   finished their slices and `cargo build --workspace` went green with NO changes
   from you. A green full build may be achievable without touching peers — try it
   before declaring a hard blocker.
7. Report the peer-side blocker to coordinate — **do NOT silently fix peers'
   files** to make the build green. The task explicitly says "don't wait
   silently — coordinate via the Architect."

This is explicitly **ad-hoc** verification: it proves your code compiles in
isolation, it does NOT mean the full workspace is green. Say so in the report.
