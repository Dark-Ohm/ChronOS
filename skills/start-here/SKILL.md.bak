---
name: start-here
description: Use at the very start of a new session in the chronos repo, before any other action — establishes which project docs to read in order, how to avoid confusing this repo with its two same-named sibling projects, and which of this repo's skills to invoke for the task at hand.
---

# Start Here — Chronos Session Orientation

## Overview

This repo's skill set was pruned to be self-contained: every skill here either
covers this project specifically or is genuinely reusable and self-contained.
This skill is the router — it tells you what to read and which of the other
skills applies, so you don't rediscover the same facts every session or reach
for the wrong tool.

`skills/` is the project's self-contained knowledge graph — the "bible."
Each skill is a node; skills cross-reference and route to one another. When a
skill points you to another, follow the link instead of re-deriving the answer.
This skill is the entry node.

## Step 0 — Read Project Docs, In This Order

Do this before writing code, answering an architecture question, or claiming
anything is "done." Dialogue memory does not replace these — they win on
conflict.

1. **`AGENTS.md`** (repo root) — persona, house rules, working-space facts
   (stack, hardware), module scope. Read first, every session.
2. **`MEMORY.md`** (repo root) — durable cross-session facts and hard rules
   (e.g. no `Co-Authored-By: Claude` in commits, respond in Russian by
   default, `SESSION_REPORT.md` format). Check the "Rules" section
   specifically before doing anything that might touch git or session
   reporting.
3. **`ARCHITECTURE.md`** (repo root) — accepted architectural decisions and
   why. Canonical. If it disagrees with what you remember from a prior chat,
   `ARCHITECTURE.md` wins.
4. **`DECISIONS.log`** (repo root) — alternatives that were considered and
   rejected, and why. Read it **in full, not just grep** — a rejected
   alternative is easy to miss with a targeted search. Read before proposing
   something that sounds obvious — it may already be a rejected alternative
   with a documented reason (e.g. "why not tokio for the tick timer").
5. **`SESSION_REPORT.md` / `fixplan.md`** (repo root, if present) — the
   actual, fact-checked state of the last work session (not aspirational).
   These use a strict format defined in `MEMORY.md`'s Rules section — read
   it before writing a new one.
6. **`docs/superpowers/specs/` and `docs/superpowers/plans/`** — historical
   spec/plan records. `specs/` is not edited after the fact; corrections go
   in `ARCHITECTURE.md` instead. Check `plans/done/` vs `plans/` (active) to
   know what's still in flight.

**Completion criterion:** you can state, in one sentence each, what this repo
is, what the last session actually finished (not planned), and whether any
open decision in `DECISIONS.log` bears on your current task.

## Step 1 — Don't Confuse This Repo With Its Siblings

Three unrelated projects on this machine share the "chronos" name. Mixing
them up produces confidently-wrong answers — this has happened before (see
`chronos-shell` skill's own writeup of `hermes-gpui-ide` content that turned
out to belong to a different repo entirely).

| Repo | What it is | Dependency tell |
|---|---|---|
| `chronos` (this repo) | Hyprland/Niri desktop shell — bar/dock/launcher/notifications/osd | `gpui` + `gpui_platform` as **path** deps, `mlua`, `hyprland`, `niri-ipc`, `zbus`. No `gpui_component`. |
| `Chronos-IDE` | Hermes Agent GPUI IDE (`chronos-agent`, `vessel-core`, ACP protocol) | `tokio-tungstenite`, `html2text`, `rusqlite`, `chronos-codegraph` |
| `chronos-fm` | GPUI file manager | `gpui-component`, `gpui-component-macros` |

If a doc, memory, or skill mentions `chronos-agent`, `vessel-core`, ACP, MCP
tool-building, `gpui_component`/`h_flex()`/`v_flex()`, or `tokio-tungstenite`
— it is not about this repo. Treat it as evidence you're looking at
sibling-project content that leaked in.

**Completion criterion:** before citing any fact that smells generic-GPUI
("use `gpui_component::Button`", "the agent's tool registry does X"), confirm
it's grounded in *this* repo's `Cargo.lock`/`crates/`, not a sibling.

## Step 2 — Route To The Right Skill

This skill is **routing only** — it tells you where to go, it does not contain
implementation guidance. Once you've named the skill for your task, open it and
follow it; don't stop here.

Skills in this repo are either project-specific (grounded in this codebase)
or intentionally project-agnostic. Use this table to pick one — per
`using-superpowers`, if a skill applies, invoke it, don't wing it.

| Your task | Skill |
|---|---|
| Anything touching `crates/app`, `crates/services`, `crates/luau`, `crates/plugins`, the bar/dock/launcher, `Service` trait, `CompositorSubscriber`, Lua plugin hot-reload | `chronos-shell` |
| A GPUI API question not specific to this repo's usage (Element trait, entities, focus, async, layout primitives) | `gpui` |
| A generic Rust idiom/ownership/error-handling/perf question | `rust-skills-master` |
| Starting any creative/feature work — before design or implementation | `brainstorming` |
| A bug, test failure, or unexpected behavior | `systematic-debugging` |
| Implementing any feature or bugfix — before writing implementation code | `test-driven-development` |
| You have a spec/requirements for multi-step work, before touching code | `writing-plans` |
| You have a written plan to execute across a session with review checkpoints | `executing-plans` |
| Executing a plan whose tasks are independent | `subagent-driven-development` |
| 2+ independent tasks with no shared state | `dispatching-parallel-agents` |
| Starting feature work that needs isolation from the current workspace | `using-git-worktrees` |
| Finished a task/feature, before merge — need review | `requesting-code-review` |
| Got review feedback and it's unclear or questionable | `receiving-code-review` |
| About to claim something is done, fixed, or passing | `verification-before-completion` |
| Implementation complete, tests pass, deciding how to merge/PR/cleanup | `finishing-a-development-branch` |
| Investigating/extracting technical documentation across a codebase | `documentation-investigation` |
| Writing, auditing, or rewriting docs (README/API/architecture/runbook) from code evidence | `philip` (dir: `philip-main`) |
| Starting/continuing/ending a session with MCP memory tools (lean-ctx / engram / codebase-tools) available | `mcp-memory-workflow` |
| Creating or editing a skill in this repo | `writing-skills` |
| Exploratory QA of a **web app** | `dogfood` — almost certainly not applicable here (this is a desktop shell, not a web app); confirm before invoking |

**Completion criterion:** you named the skill you're about to invoke (or
explicitly noted none applies) before starting the task, not after.

## Step 3 — Session-End Habits

- Before claiming anything works: `verification-before-completion` — run the
  actual command, show the output, don't assert from memory.
- If `MEMORY.md`'s `SESSION_REPORT.md` format applies to your task (it's
  strict — see `MEMORY.md`'s Rules section), produce it before ending.
- If MCP memory tools (lean-ctx/engram/codebase-tools) are available in this
  session, follow `mcp-memory-workflow`'s session-end protocol — but note its
  project-scope convention: engram `project` values are strictly `chronos`,
  `Chronos-IDE`, `chronos-fm`, or `hyprland`. `chronos-shell` is **not** a
  valid standalone scope — it's part of `chronos`.

## Common Pitfalls

1. **Trusting remembered architecture over `ARCHITECTURE.md`/`DECISIONS.log`.**
   Dialogue memory is not project documentation — this is stated explicitly
   in `AGENTS.md`. Re-check before asserting a design decision.
2. **Citing generic GPUI-ecosystem patterns (`gpui_component`, `h_flex()`) as
   if they apply here.** They don't — see Step 1.
3. **Skipping the skill-routing check because a task "seems simple."** Simple
   tasks are exactly where `using-superpowers`' rationalization list applies
   — check the table in Step 2 anyway.
5. **Skipping `AGENTS.md` because you "know the stack."** It carries
   working-space facts and house rules that change; read it first every
   session, per Step 0.
6. **Naming the routed skill but not invoking it.** The Step 2 completion
   criterion is meaningless if you then wing the task — open the skill and
   follow it.
