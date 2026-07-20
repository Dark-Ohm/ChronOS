---
name: chronos-gpui
description: Ground truth about OUR gpui fork ("gpui-ce chronos edition" in ../Source) — what the fork actually contains, which APIs exist and under which traits, what the 60 shipped examples prove, and where it diverges from upstream GPUI. Use before claiming "the fork can't do X", when picking a windowing/layout/async API for ChronOS, or when an API "doesn't resolve". Evidence-based: every claim carries a file:line from Source/ or a runnable example.
---

# ChronOS GPUI fork — ground truth

**Why this skill exists.** On 2026-07-20 a documented "blood fact" —
*"`overflow_y_scroll()` does not resolve in this fork, scroll is
impossible"* — turned out to be **false**. The method exists
(`Source/gpui/src/elements/div.rs:1429`); it lives on
`StatefulInteractiveElement`, implemented only for `Stateful<E>`
(`:3752`), so calling it on a bare `div()` fails with *"no method"*.
A compile error was misread as a missing feature, and the wrong
constraint spread into 6 documents and 2 minion briefs before anyone
opened the fork. A working sample was shipped inside the fork the whole
time: `Source/gpui/examples/scrollable.rs`.

**Therefore the rule:** a claim that "the fork cannot do X" requires
evidence from the fork's sources or a runnable example — never a
retelling. `Source/*/examples/` is a ready-made proving ground; look
there *before* writing a limitation into canon.

## Scope

`../Source/` — our own fork, 19 crates + `gpui-component`. Not upstream
Zed, not crates.io. Path-deps from ChronOS point here.

| Crate group | What |
|---|---|
| `gpui` | core: elements, styling, layout, app/entity/context, 44 examples |
| `gpui_platform`, `gpui_linux` | windowing, Wayland, **layer-shell**, input, displays |
| `gpui_macros` | style-macro generation (where `px_*`/`max_h`-style methods come from) |
| `gpui_scheduler`, `gpui_tokio` | executors, `Task` (`#[must_use]`, drop = cancel) |
| forked zed-internal | `gpui_collections`, `gpui_sum_tree`, `gpui_refineable`, … |
| `gpui-component` | separate workspace, 14 examples — NOT used by ChronOS today |

## Navigation

| Topic | File |
|---|---|
| Elements, styling, layout, scroll | [elements-styling-layout.md](references/elements-styling-layout.md) |
| Windowing, Wayland, layer-shell, input | [windowing-platform.md](references/windowing-platform.md) |
| App/Entity/Context, async, executors | [state-async-executors.md](references/state-async-executors.md) |
| Example corpus (what each proves) | [examples-index.md](references/examples-index.md) |

## Related skills

| Need | Skill |
|---|---|
| Generic/upstream GPUI concepts | `gpui` |
| ChronOS shell code itself | `chronos-shell` |
| Layer-shell popup sizing recipes | `gpui-layer-shell` |
