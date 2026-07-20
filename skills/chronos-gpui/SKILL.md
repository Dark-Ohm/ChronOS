---
name: chronos-gpui
description: Ground truth about OUR gpui fork ("gpui-ce chronos edition" in ../Source) — what the fork actually contains, which APIs exist and under which traits, what the 55 shipped examples prove, and where it diverges from upstream GPUI. Use before claiming "the fork can't do X", when picking a windowing/layout/async API for ChronOS, or when an API "doesn't resolve". Evidence-based: every claim carries a file:line from Source/ or a runnable example.
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

**It happened a second time (2026-07-20, same day).** A plan concluded
that mutating view state from an `on_click` requires a `Global`, because
every `on_click(move |…)` in the tree receives `&mut App`. True premise,
false conclusion: `Context::listener` (`context.rs:252`) is the adapter
built for exactly this, it is used by 15 fork examples, and ChronOS
itself already calls it at `volume_popup/view.rs:199`. The grep that
produced the wrong answer — `on_click(move |` — *structurally cannot*
match `on_click(cx.listener(..))`.

**The generalized rule:** when a grep's shape determines your conclusion,
the grep is a hypothesis, not evidence. Search for the *thing you'd
expect to exist if you were wrong* before writing the limitation down.

## Scope

`../Source/` — our own fork, 19 crates + `gpui-component`. Not upstream
Zed, not crates.io. Path-deps from ChronOS point here.

| Crate group | What |
|---|---|
| `gpui` | core: elements, styling, layout, app/entity/context, 42 examples |
| `gpui_platform`, `gpui_linux` | windowing, Wayland, **layer-shell**, input, displays |
| `gpui_macros` | style-macro generation (where `px_*`/`max_h`-style methods come from) |
| `gpui_scheduler`, `gpui_tokio` | executors, `Task` (`#[must_use]`, drop = cancel) |
| forked zed-internal | `gpui_collections`, `gpui_sum_tree`, `gpui_refineable`, … |
| `gpui-component` | separate workspace, 13 examples — NOT used by ChronOS today |

## Navigation

| Topic | File |
|---|---|
| Elements, styling, layout, scroll | [elements-styling-layout.md](references/elements-styling-layout.md) |
| Windowing, Wayland, layer-shell, input | [windowing-platform.md](references/windowing-platform.md) |
| App/Entity/Context, async, executors | [state-async-executors.md](references/state-async-executors.md) |
| Example corpus, full catalog | [examples-catalog.md](references/examples-catalog.md) |
| Examples grouped by topic (task → example) | [examples-by-topic.md](references/examples-by-topic.md) |
| Run/check any example | `scripts/run-example.sh --list` / `--check <name>` |
| Eval per reference (8-10 questions) | `evals/*.eval.md` |

## Fast answers to the questions that keep getting asked wrong

| Question | Answer | Evidence |
|---|---|---|
| Can a bare `div()` scroll? | No — `.id()` it first; `overflow_y_scroll` is on `StatefulInteractiveElement` | `div.rs:1429`, `:3752` |
| Can an `on_click` mutate the view's own state? | Yes — `cx.listener`, no `Global` needed | `context.rs:252`, `volume_popup/view.rs:199` |
| Two `on_hover` on one element? | No — `debug_assert` panics; one slot | `div.rs:622-625`, `:1995` |
| Is there an interval timer? | No — one-shot `timer`, loop it yourself | `executor.rs:162` |
| Must Kael easing be ported? | Already ported | `easing.rs:1-71` |

## Related skills

| Need | Skill |
|---|---|
| Generic/upstream GPUI concepts | `gpui` |
| ChronOS shell code itself | `chronos-shell` |
| Layer-shell popup sizing recipes | `gpui-layer-shell` |
