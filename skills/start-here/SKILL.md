---
name: start-here
description: >
  Use at the very start of a new session in the chronos repo, before any other
  action — establishes which project docs to read in order, how to avoid
  confusing this repo with its same-named siblings, and which skills to invoke.
---

# Start Here — Chronos Session Orientation

## Overview

`skills/` is the project's self-contained knowledge graph. This skill is the
**entry router** — what to read, which skill applies. When a skill points
elsewhere, follow the link; do not re-derive.

### Memory layers (order of authority)

1. Repo docs: `HANDOFF.md` / `ARCHITECTURE.md` / `DECISIONS.log` / `MEMORY.md` /
   `AGENTS.md` — **win on conflict**.
2. Self-hosted **Hindsight** (podman, bank `chronos-ecosystem`, REST `:8888`) —
   long-term supplementary memory. Live skill: **`hindsight-self-hosted`**
   (often under `~/.agents/skills/`). Skills `hindsight-local` /
   `hindsight-cloud` describing `uvx hindsight-embed` are **STALE** for this
   machine.
3. Dialogue memory — never overrides (1).

If MCP tools (lean-ctx / engram / codebase-tools) are available this session,
follow `mcp-memory-workflow` for session start/end. Engram `project` scopes:
`chronos` | `Chronos-IDE` | `chronos-fm` | `hyprland` — **not** `chronos-shell`
as a standalone project name.

---

## Step 0 — Read Project Docs, In This Order

Do this before coding, architecture answers, or "done" claims.

1. **`HANDOFF.md`** (repo root) — **first every Architect/minion session**: who
   is in the field, queue, blood technical facts, field rules (stash ban,
   worktree isolation, pkill -x, single-instance, release-only UX smokes).
2. **`AGENTS.md`** — persona, house rules, stack/hardware, module scope.
3. **`MEMORY.md`** — durable rules (no AI commit trailers, Russian by default,
   strict `SESSION_REPORT` / report format).
4. **`ARCHITECTURE.md`** — accepted decisions (canonical).
5. **`DECISIONS.log`** — rejected alternatives; read in full, not only grep.
6. **Minion file if you are one** — `GROK.md` / `CLINE.md` / … last section =
   current assignment; report → `<name>-report.md` **at repo root**.
7. **`docs/superpowers/specs/` + `plans/`** — historical; corrections go to
   `ARCHITECTURE.md`, not back into frozen specs.

**Completion criterion:** one sentence each — what this repo is, what the last
accepted wave finished, whether an open field rule or DECISIONS item bites your
task.

---

## Step 1 — Don't Confuse This Repo With Its Siblings

| Repo / skill | What it is | Dependency tell |
|---|---|---|
| **ChronOS** (this repo) | Hyprland/Niri desktop shell — bar/dock/launcher/notifications/osd/mpris/… | path `../Source/gpui`, `mlua`, `hyprland`, `niri-ipc`, `zbus`. **No** `gpui_component`. |
| `Chronos-IDE` | Hermes Agent GPUI IDE | `tokio-tungstenite`, ACP, `chronos-agent` |
| `chronos-fm` | GPUI file manager | `gpui-component`, `h_flex()` / `v_flex()` |
| `chronos-shell` (**skill**, not a repo) | Project skill for **this** tree (`skills/chronos-shell/`) | — |

If a doc mentions `chronos-agent`, ACP tool registries, or `gpui_component` —
wrong tree. Confirm against **this** `Cargo.lock` / `crates/`.

**Completion criterion:** no generic-GPUI claim without a path in this repo.

---

## Step 2 — Route To The Right Skill

Routing only — open the skill and follow it.

| Your task | Skill |
|---|---|
| Studying / porting patterns from **Zed AI** (agent panel, ACP, threads, tools) — never copy GPL sources | **`zed-ai`** (router) → `zed-ai-for-chronos` for ChronOS left panel |
| `side_panel_left` agent thread canvas (thread header, chat flow, composer, YOLO, dark send) — visual implementation result | **`zed-ai-for-chronos`** §v0 — Thread canvas |
| `crates/app`, `crates/services`, `crates/luau`, `crates/ui`, bar/dock/launcher/osd/notifications/tray_menu, `Service` trait, subscribers, Lua hot-reload, wallpaper_ctl, IPC payloads | **`chronos-shell`** (+ `references/slow-service-dispatch.md` for audio/brightness drag) |
| Bar-anchored popups (volume/updates/system/history), blur, animation boot, **slider drag markers**, footer clip | **`chronos-gpui-popup`** |
| Visual polish: gradient fills / gradient "borders", drop + inset shadows, frosted glass, hover/enter motion, which style props interpolate | **`eye-candy`** |
| Layer-shell geometry: popup height / clip / `window.resize`, **or** full-height side panel under bar with equal top/bottom gaps, **or** `on_hover` + `gpui-animation` single-slot | **`gpui-layer-shell`** |
| `gpui-rsx` / `rsx!`, mockup HTML→panel chrome, `overflow_y_scroll`+`ScrollHandle`, hover E0631, rsx vs builder `div()` | **`gpui-rsx`** |
| What can OUR fork do / API "doesn't resolve" / before claiming "fork can't X" | **`chronos-gpui`** |
| **Modifying `../Source/` (the fork itself)** — crate map, layer-shell/popup/blur/spring APIs, renderer, vendoring, Wayland lifecycle | **`gpui-fork-start-here`** (routes the whole fork-internals layer) |
| Porting external gpui code into the fork / crates.io API drift (E0599 `.ok()`, E0063 `inset`, E0308 refineable) | `fork-api-drift` |
| Generic GPUI API (Element, entities, focus, layout) | `gpui` |
| Rust ownership / error / perf idioms | `rust-skills-master` |
| Creative feature before design | `brainstorming` |
| Bug / unexpected behavior | `systematic-debugging` |
| Feature or bugfix before implementation | `test-driven-development` |
| Spec → plan before code | `writing-plans` |
| Execute a written plan with checkpoints | `executing-plans` |
| Independent plan tasks | `subagent-driven-development` |
| 2+ independent tasks | `dispatching-parallel-agents` |
| Isolated workspace (ChronOS: **sibling of repo**, not `/tmp`) | `using-git-worktrees` + `chronos-shell` field rules |
| Pre-merge review | `requesting-code-review` |
| Review feedback unclear | `receiving-code-review` |
| About to claim done / green | **`verification-before-completion`** (+ release UX smoke per HANDOFF) |
| Merge/PR/cleanup choice | `finishing-a-development-branch` |
| Doc investigation | `documentation-investigation` |
| Write/audit docs from code evidence | `philip` (`philip-main`) |
| MCP memory session protocol | `mcp-memory-workflow` |
| Hindsight retain/recall on this machine | `hindsight-self-hosted` |
| Create/edit a skill here | `writing-skills` |
| Web-app exploratory QA | `dogfood` — almost never this repo |

**Completion criterion:** named skill before work starts, not after.

---

## Step 3 — Session-End Habits

- Claims of "works": run the command, show output (`verification-before-completion`).
- Window/UX work: **release binary** + grim / live log — unit green is not enough.
- Minion reports: root `<name>-report.md`, SESSION_REPORT sections from MEMORY.md.
- Architect archives accepted reports to `report-log/` with an explicit commit
  (uncommitted deletes resurrect under foreign git ops).
- MCP/Hindsight session-end if those tools were used.

---

## Common Pitfalls

1. **Remembered architecture over HANDOFF / ARCHITECTURE / DECISIONS.**
2. **`gpui_component` / sibling IDE patterns** as if they apply here.
3. Skipping skill routing because the task "seems small."
4. Skipping **HANDOFF** / AGENTS because you "know the stack."
5. Naming a skill without opening it.
6. Trusting stale hindsight-local/cloud skills (`uvx hindsight-embed`).
7. **`git stash` / checkout of foreign WIP** — forbidden; worktree sibling only.
8. **`pkill -f chronos`** — kills the controlling shell; use **`pkill -x chronos`**.
9. Second `chronos` without kill — single-instance exits; fake "restarts."
10. Worktree under `/tmp` — breaks `path = "../Source"`.
11. **Day-to-day shell:** `docs/dev-cli.md` — `chronos-rebuild` / `start` /
    `stop` (T122). Prefer CLI over ad-hoc pkill paths.
12. **`background_spawn` without `.detach()`** — Task cancelled; async
    notification Close/Clear never run (T120).
13. **Shared slider `DragMove` marker type** — one knob drives all listeners
    of that type (`chronos-gpui-popup`).
14. **Per-sample DDC/wpctl spawn + full re-read** — multi-minute brightness
    jumps / `available: false`; use latest-wins + debounce
    (`slow-service-dispatch.md`).

## Related entry points

- Deep code layout: **`chronos-shell`**
- Anchored popups / sliders / blur / animation boot: **`chronos-gpui-popup`**
- Fork internals (working on `../Source/` itself): **`gpui-fork-start-here`**
- Layer-shell placement / popup sizing / on_hover+animation: **`gpui-layer-shell`**
- `rsx!` / mockup→chrome / scroll+hover blood facts: **`gpui-rsx`**
- Operational queue: **`HANDOFF.md`**
- Dev CLI: **`docs/dev-cli.md`**
