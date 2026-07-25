---
name: gpui
description: GPUI framework knowledge covering actions/keybindings, async/background tasks, context management (App/Window/Context<T>/AsyncApp), custom elements (low-level Element trait), entity state management, event system, focus handling, global state, layout/styling (flexbox/CSS-like), and testing. Use when working with any GPUI framework concept, building GPUI applications, or needing guidance on GPUI-specific APIs and patterns.
---

## Navigation

Load the relevant reference file based on the task:

> Working in the `chronos` repo? For session orientation and which skill to
> invoke, see `start-here` first.

| Topic | File | When to load |
|-------|------|--------------|
| Actions & keybindings | [action.md](references/action.md) | `actions!`, `bind_keys`, `on_action`, `key_context` |
| Async & background tasks | [async.md](references/async.md) | `cx.spawn`, `background_spawn`, `Task`, async I/O |
| Context management | [context.md](references/context.md) | `App`, `Window`, `Context<T>`, `AsyncApp` |
| Custom elements (low-level) | [element.md](references/element.md) | `Element` trait, `request_layout`, `prepaint`, `paint` |
| Entity state | [entity.md](references/entity.md) | `Entity<T>`, `WeakEntity`, state management |
| Events & subscriptions | [event.md](references/event.md) | `cx.emit`, `cx.subscribe`, `cx.observe` |
| Focus & keyboard nav | [focus-handle.md](references/focus-handle.md) | `FocusHandle`, `track_focus`, Tab navigation |
| Global state | [global.md](references/global.md) | `Global` trait, `cx.set_global`, app-wide config |
| Layout & styling | [layout-style.md](references/layout-style.md) | `div()`, `h_flex()`, `v_flex()`, flexbox, overflow, positioning |
| ElementId | [element-id.md](references/element-id.md) | `ElementId`, `.id()`, uniqueness rules, stateful elements — **HARD RULE: interactive methods (`on_click`, `overflow_y_scroll`, `track_scroll`, `cursor_pointer`, …) need `.id()` FIRST; bare `div()` fails with E0599. Compile to verify, don't infer from the trait block.** |
| Testing | [test.md](references/test.md) | `#[gpui::test]`, `TestAppContext`, `VisualTestContext` |

## Extended References

For deep-dive topics, additional reference files are available:

**Element trait:**
- [element-api.md](references/element-api.md) — complete API, hitbox system, event handling
- [element-patterns.md](references/element-patterns.md) — text, interactive, container, composite patterns
- [element-examples.md](references/element-examples.md) — full examples: text, interactive, complex elements
- [element-best-practices.md](references/element-best-practices.md) — performance, state, common pitfalls
- [element-advanced.md](references/element-advanced.md) — masonry/circular layouts, async updates, virtual lists

**Entity management:**
- [entity-api.md](references/entity-api.md) — complete Entity API, methods, lifecycle
- [entity-patterns.md](references/entity-patterns.md) — model-view, cross-entity communication, observer
- [entity-best-practices.md](references/entity-best-practices.md) — memory, performance, lifecycle
- [entity-advanced.md](references/entity-advanced.md) — collections, registry, debounce, state machines

**Testing:**
- [test-examples.md](references/test-examples.md) — testing examples and patterns
- [test-reference.md](references/test-reference.md) — complete testing API reference

## Verification — compile a throwaway probe, don't infer from trait definitions

When you need to know whether a method/chain is valid on a fork (e.g. the ChronOS gpui-ce
fork, which DRIFTS from upstream zed `main`), **compile a one-file example and read the
rustc diagnostic — never conclude from reading the trait `impl` block.** Trait bounds in
GPUI are subtle: `Div` implements `InteractiveElement` but NOT `StatefulInteractiveElement`,
so a method that *looks* available from the `impl InteractiveElement for Div` block may be
missing on bare `Div` and only resolve on `Stateful<Div>` (returned by `.id()`).

Recipe (verified this session, caught a false skill "correction"):
1. Write `gpui/examples/_probe.rs` containing a `Render` impl that chains the method(s) under
   test on `div()` (and on `div().id("x")` for the stateful variant).
2. `cargo check --example _probe -p 'path+file:///<repo>/Source/gpui#0.2.2'` (or `-p gpui` in
   the app workspace).
3. Read the rustc output: E0599 `no method named X found for struct gpui::Div` = method is NOT
   on bare `Div` (needs `.id()` first); clean `Finished` = available.
4. `rm gpui/examples/_probe.rs` — never commit the probe.

This is the canonical way to settle "does the fork support X" disputes and to verify any
claim before writing it into a skill or report. Competed by reasoning, it produces confident
wrong answers.
