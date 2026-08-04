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

**And a third time (2026-08-04, T231).** A "no CSS grid" claim nearly
spread to canon — false. `.grid()`, `.grid_cols(N)`, `.grid_rows(N)`,
`col_start`/`col_end`/`col_span`, `row_*` all exist
(`Source/gpui/src/styled.rs`, Taffy-based), and the fork ships live
users: `examples/grid_layout.rs`, `examples/anchor.rs`. The negative came
from grepping only the ChronOS tree root — **the fork is a *sibling* of
the repo (`../Source/gpui`), outside the root**, so repo-root searches
structurally cannot see it. New rule: when hunting a fork API, search
`../Source` too, not just the project root.

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
| Validate proof links repo-wide (`file:line` resolve) | `skills/check-proofs.sh` — run after any SKILL.md/reference/eval edit |

## Fast answers to the questions that keep getting asked wrong

| Question | Answer | Evidence |
|---|---|---|
| Can a bare `div()` scroll? | No — `.id()` it first; `overflow_y_scroll` is on `StatefulInteractiveElement` | `div.rs:1429`, `:3752` |
| Can an `on_click` mutate the view's own state? | Yes — `cx.listener`, no `Global` needed. **But:** pre-build the listener as a variable before the render chain (pattern: `let l = cx.listener(...);` then `.on_click(l)`). Inline `cx.listener` inside `.when()`, for-loops, or nested `.child()` chains may fail with E0599 — the closure type doesn't resolve to `Stateful<Div>::on_click`'s expected signature in those contexts. | `context.rs:252`, `volume_popup/view.rs:199`, T235 session 2026-08-04, **T237 preview.rs (2026-08-04): inline `.on_click(cx.listener(...))` inside a 3-deep `.child()` chain failed E0599; hoisting to `let open_files = cx.listener(...);` then `.on_click(open_files)` fixed it** |
| How to switch to another right-panel tab from a different view (e.g. Preview → Files)? | `select_tab(PanelTab, &mut App)` in `side_panel_right/mod.rs` takes `&mut App`, NOT `Context<T>` — can't call it directly from another view's `cx`. Instead reach the panel through its global: `cx.global::<SidePanelRightState>().view.clone().and_then(|w| w.upgrade())` then `view.update(cx, |view, cx| view.on_tab_select(PanelTab::Files, cx))`. Same path `select_tab` itself uses internally. | `side_panel_right/mod.rs:353`, `tabs.rs` (`PanelTab`), T237 preview.rs 2026-08-04 |
| Does the fork have CSS grid? | **Yes** — `.grid()`, `.grid_cols(N)`, `.grid_rows(N)`, `col_start/end/span`, `row_*` (Taffy-based). Live users: `examples/grid_layout.rs`, `examples/anchor.rs`. T231 used it for 2-col responsive settings grids. Grep `../Source/gpui` too — the fork is *outside* the repo root. | `styled.rs:52` (`.grid`), `:752` (`grid_cols`), `:780` (`grid_rows`), `style.rs:302` (`grid_cols` style field), `examples/grid_layout.rs`, T231 2026-08-04 |
| `elevation_apply_light_chrome` then `.id()`? | Apply `.id()` **after** the elevation helper — it takes/returns a bare `Div`; `.id()` upgrades it to `Stateful<Div>`. Pattern: `tab/ui.rs::elevated_card`. | `crates/app/src/side_panel_right/tab/ui.rs`, T231 2026-08-04 |
| Responsive 2-col → 1-col for right-panel tabs? | Breakpoint pattern in `tab/ui.rs`: `GRID_BREAKPOINT = 720.0`, `is_wide(&Window)` via `window.bounds().size.width.as_f32()`. Default docked width (560) stays 1-col; 960 (`MAX_WIDTH`) → 2-col. | `tab/ui.rs`, T231 2026-08-04 |
| Does `.gap_1()` (etc.) exist? | **No.** The fork only has `gap(px(n))`. `gap_1`/`gap_xl` from upstream Zed are absent — using them breaks the whole builder chain with a cascade of "method not found" errors. | `Source/gpui/src/styled.rs` (only `flex_col`/`flex_wrap`/`gap(px)`), T237 session 2026-08-04 |
| Two `on_hover` on one element? | No — `debug_assert` panics; one slot | `div.rs:622-625`, `:1995` |
| Is there an interval timer? | No — one-shot `timer`, loop it yourself | `executor.rs:162` |
| Must Kael easing be ported? | Already ported | `easing.rs:1-71` |

## Related skills

| Need | Skill |
|---|---|
| Generic/upstream GPUI concepts | `gpui` |
| ChronOS shell code itself | `chronos-shell` |
| Layer-shell popup sizing recipes | `gpui-layer-shell` |
| **Changing the fork itself** (Source/) — entry map | `gpui-fork-start-here` (fork-internals layer) |
| Porting external gpui code / crates.io API drift | `fork-api-drift` |
| Fork's layer-shell / popup / blur / spring APIs | `layer-shell-windows`, `anchored-popups`, `backdrop-blur`, `easing-and-springs` |
| Renderer internals, vendoring policy, Wayland lifecycle | `wgpu-render-pipeline`, `workspace-vendoring`, `wayland-window-lifecycle` |

This skill is the **consumer-side** ground truth (using the fork from
ChronOS). The fork-internals layer above lives in the same `skills/` dir —
route there when the task is *modifying* `Source/`, not consuming it.
