---
name: mcp-memory-workflow
description: "Use when starting, continuing, or ending a coding session. Save checkpoints and recover context across all 3 MCP memory layers: lean-ctx, engram-memory, codebase-tools."
version: 1.0.0
author: Hermes Agent
license: MIT
platforms: [linux, macos, windows]
metadata:
  hermes:
    tags: [mcp, memory, lean-ctx, engram, codebase-tools, checkpoints, workflow, sessions]
    related_skills: [writing-plans, executing-plans, start-here]
---

# MCP Memory Workflow

Universal skill: works with the 3 MCP memory layers in ANY project. The
project-specific conventions below are for `chronos`, but the workflow itself
is repo-agnostic — swap the `project` scope values for whatever repo you're
in. For session orientation and the full skill map of THIS repo, see
`start-here` first.

Three independent MCP memory systems exist. Each stores different data, persists differently, and is queried differently. Use them together — not interchangeably.

## System Map

| System | What it stores | Persistence | Primary tool prefix | Best for |
|---|---|---|---|---|
| **lean-ctx** (81 tools) | Session state, project knowledge, dependency graphs, checkpoints | Per-chat + cross-chat via `ctx_session` | `ctx_*` | Real-time session continuity, code structure, architecture |
| **engram-memory** (20 tools) | Observations, prompts, session summaries | Persistent across all sessions | `mem_*` | Facts, decisions, bug fixes, learning, user requests |
| **codebase-tools** (12 tools) | Repository graph (nodes=functions/classes/files, edges=calls/imports) | Persistent per-indexed repo | `index_*`, `search_*`, `trace_*` | Structural queries, call graphs, impact analysis |

**Key rule:** lean-ctx is for *workflow state*, engram is for *durable knowledge*, codebase-tools is for *code structure*. Don't mix them.

## Project-Specific Conventions (Chronos)

For this repo, engram `project` scopes are STRICTLY:
- `chronos` — desktop shell + shell-layer (bar/dock/launcher/notifications/osd), path `/home/neo/Projects/chronos`
- `Chronos-IDE` — ai-native IDE (separate repo, `/home/neo/Projects/Chronos-IDE`)
- `chronos-fm` — file manager (`/home/neo/Projects/chronos-fm`)
- `hyprland` — system-level Lua compositor config (`~/.config/hypr/`)

**Forbidden:** `chronos-shell` as standalone (it's part of `chronos`); `chronos-ide`/`Chronos-IDE`/`chronos` as interchangeable (they are DIFFERENT projects).

Every engram record MUST have `topic_key` in format `<project>/<topic>` (e.g. `chronos/bar-widget-contract`). This enables upsert — re-saving same key updates, not duplicates.

Full conventions: see `MEMORY-CONVENTIONS.md` (in /home/neo/Projects/chronos/skills/mcp-memory-workflow/references).

## Session Lifecycle Protocol

### 0. Read Attached Reference Docs (BEFORE anything else)

When the user attaches files (`@folder:`, `@file:`, inline docs), **read them before any other action** — before session setup, before coding, before saving. Attached docs define the actual API surface and tool parameters. Guessing from memory produces wrong results.

```
read_file(attached_file_path)  → understand actual APIs
THEN proceed to Session Start
```

### 1. Session Start

Do these in order, before any coding work:

```
lean-ctx:    ctx_session(action="load")     → restore previous session state
lean-ctx:    ctx_knowledge(action="recall") → load project-specific knowledge
engram:      mem_session_start(directory, id, project)  → register session
engram:      mem_search(query="<project name>")         → recall prior context
codebase:    index_status(project) → if stale, index_repository(project)
```

**When each fires:**
- `ctx_session load` — every session start. Restores task, findings, decisions from last chat.
- `ctx_knowledge recall` — when resuming a project you've worked on before.
- `mem_session_start` — every session. Gives engram a session ID to attach observations to.
- `mem_search` — when you need context from prior sessions (bugs found, patterns established).
- `index_status` — when you need to query code structure and the index may be stale.

### 2. After Successful Task Completion

```
engram:      mem_save(title, content)  → record what was done, why, and what was learned
lean-ctx:    ctx_session(action="task", value="<what was done>")  → update task progress
lean-ctx:    ctx_session(action="finding", value="<key discovery>")  → if anything non-obvious was learned
lean-ctx:    ctx_session(action="decision", value="<what was decided and why>")  → if a choice was made
```

**What qualifies as "save-worthy":**
- Bug fix → `mem_save` with title like "Fixed N+1 in user list", content = what was wrong + how it was fixed
- Architecture choice → `mem_save` with type="decision", content = what was chosen + alternatives
- New API pattern discovered → `mem_save` with type="learning", content = the pattern
- Config change → `mem_save` with type="config", content = what changed + why

**What does NOT need saving:**
- Trivial edits (typo fixes, formatting)
- Intermediate steps (only save the result)
- Anything the next agent can trivially re-discover

### 3. When You Learn Something Important

```
engram:      mem_save(title, content, topic_key="<stable-key>")
lean-ctx:    ctx_knowledge(action="remember", category="<cat>", key="<key>", value="<what>")
```

Use `topic_key` (engram) or `key` (lean-ctx knowledge) to group related observations. This lets future sessions update the same entry instead of creating duplicates.

**lean-ctx knowledge categories:**
- `architecture` — structural decisions, design patterns
- `patterns` — confirmed API patterns, idioms that work
- `config` — environment setup, tool quirks
- `pitfalls` — gotchas that tripped you up

**engram topic_key format:** `<project>/<topic>`, e.g. `hermes-gpui-ide/gpui-component-api`

### 4. Session End

```
lean-ctx:    ctx_session(action="save")     → persist session state
engram:      mem_session_summary(content)   → structured end-of-session summary
engram:      mem_session_end(id, summary)   → close session
codebase:    (auto-sync keeps graph fresh, no manual step needed)
```

**mem_session_summary format** (use this structure):
```
## Goal
[One sentence]
## Instructions
[User preferences, constraints discovered]
## Discoveries
- [Finding 1]
- [Finding 2]
## Accomplished
- ✅ [Task 1]
- 🔲 [Not yet done]
## Relevant Files
- path/to/file.ts — [what it does]
```

## Tool Reference: lean-ctx (81 tools)

### Session & State (use every session)

| Tool | When | Example |
|---|---|---|
| `ctx_session(action="load")` | Start | Restores task/findings/decisions |
| `ctx_session(action="save")` | End | Persists all session data |
| `ctx_session(action="task")` | After each task | `value="Phase 1 at 85%"` |
| `ctx_session(action="finding")` | When learning | `value="API X requires Y"` |
| `ctx_session(action="decision")` | When deciding | `value="Use A over B because Z"` |
| `ctx_session(action="status")` | Any time | Read current session state |
| `ctx_knowledge(action="remember")` | Important facts | `category="patterns", key="...", value="..."` |
| `ctx_knowledge(action="recall")` | Start/resume | `key="project/thing"` |

### Code Intelligence (use during work)

| Tool | When | Example |
|---|---|---|
| `ctx_read(mode="full")` | Need file content | `path="/abs/path/to/file.rs"` |
| `ctx_read(mode="anchored")` | About to edit | Returns line+hash anchors for ctx_patch |
| `ctx_patch` | Editing files | Uses anchors from ctx_read(mode="anchored") |
| `ctx_search(pattern)` | Find code | Regex search across project |
| `ctx_semantic_search(query)` | Find by meaning | "where is auth middleware" |
| `ctx_tree` | Project overview | Directory structure with file counts |
| `ctx_shell(command)` | Run commands | Compressed output (95+ patterns) |
| `ctx_graph(action="impact")` | Change impact | What breaks if file X changes |
| `ctx_compose(task)` | Complex reads | Ranked files + symbols for a task |
| `ctx_symbol(name)` | Find definition | Across 26 languages |
| `ctx_callgraph(name)` | Call relationships | Who calls this, what it calls |

### Checkpoints & Multi-Agent

| Tool | When | Example |
|---|---|---|
| `ctx_checkpoint` | Before big changes | Shadow git snapshot |
| `ctx_handoff` | Agent transfer | Push session to another agent |
| `ctx_agent` | Multi-agent | Register/post/sync |
| `ctx_compress` | Context too large | Compact checkpoint for continuation |

## Tool Reference: engram-memory (20 tools)

### Core Operations

| Tool | When | Example |
|---|---|---|
| `mem_save` | Record something durable | Bug fix, decision, pattern, config change |
| `mem_update` | Update existing observation | `id=N, content="corrected version"` |
| `mem_delete` | Remove wrong observation | `id=N` |
| `mem_search` | Find past observations | `query="auth middleware"`, `project="my-app"` |
| `mem_context` | Recent session context | `project="my-app"` |
| `mem_timeline` | Chronological context | Around a specific observation |
| `mem_get_observation` | Full content by ID | After mem_search found it |

### Session Lifecycle

| Tool | When | Example |
|---|---|---|
| `mem_session_start` | Every session | `id="unique-id", project="name"` |
| `mem_session_end` | Every session end | `id="same-id", summary="..."` |
| `mem_session_summary` | Before session end | Structured Goal/Instructions/Discoveries/Accomplished/Files |
| `mem_save_prompt` | Record user intent | What the user asked for this session |

### Utilities

| Tool | When | Example |
|---|---|---|
| `mem_stats` | Quick status check | See total sessions/observations |
| `mem_suggest_topic_key` | Before mem_save | Get stable key for upserts |
| `mem_review` | Periodic cleanup | Review and consolidate observations |

## Tool Reference: codebase-tools (12 tools)

### Indexing (do once per repo)

| Tool | When | Example |
|---|---|---|
| `index_repository` | First time / stale | Index repo into graph |
| `index_status` | Before queries | Check if index is fresh |
| `list_projects` | Overview | See all indexed repos |

### Querying (use during work)

| Tool | When | Example |
|---|---|---|
| `search_graph` | Find by label/name | `label="function", name_pattern="*auth*"` |
| `trace_path` | Call chain | "who calls `process_payment`" depth 1-5 |
| `detect_changes` | After git diff | Blast radius + risk classification |
| `get_architecture` | Project overview | Languages, packages, hotspots, ADR |
| `get_code_snippet` | Read by symbol | `qualified_name="module::function"` |
| `search_code` | Grep-like | Text search in indexed files |
| `query_graph` | Cypher-like | Complex graph queries |
| `manage_adr` | Architecture decisions | CRUD for ADR records |

## Quick Recipes

### "I just started a new session on project X"

```python
# 1. Lean-ctx: restore state
ctx_session(action="load")
ctx_knowledge(action="recall", key="X/checkpoint")

# 2. Engram: register session
mem_session_start(id="X-YYYY-MM-DD", project="X")
mem_search(query="X", limit=3)  # what did I do last time?

# 3. Codebase: check index
index_status(project="X")
# if stale → index_repository(project="X")
```

### "I just fixed a bug"

```python
# Engram: record the fix
mem_save(
    title="Fixed <bug name>",
    content="**What**: <what was wrong>\n**Why**: <root cause>\n**Where**: <file:line>\n**Learned**: <prevention>",
    project="X",
    topic_key="X/bugs/<slug>"
)

# Lean-ctx: update session
ctx_session(action="finding", value="Fixed <bug>: <one-liner>")
```

### "I just made an architectural decision"

```python
# Engram: record decision
mem_save(
    title="Decision: <what>",
    content="**What**: <choice>\n**Why**: <reasoning>\n**Alternatives**: <rejected options>\n**Where**: <files affected>",
    project="X",
    type="decision",
    topic_key="X/architecture/<topic>"
)

# Lean-ctx: record in session + knowledge
ctx_session(action="decision", value="<what> because <why>")
ctx_knowledge(action="remember", category="architecture", key="X/<topic>", value="<summary>")
```

### "I just discovered an important API pattern"

```python
# Engram: record pattern
mem_save(
    title="Pattern: <name>",
    content="**What**: <pattern>\n**Where**: <files>\n**Learned**: <gotchas>",
    project="X",
    type="pattern",
    topic_key="X/patterns/<name>"
)

# Lean-ctx: add to knowledge store
ctx_knowledge(action="remember", category="patterns", key="X/<name>", value="<summary>")
```

### "I'm ending this session"

```python
# 1. Lean-ctx: save session state
ctx_session(action="save")

# 2. Engram: structured summary
mem_session_summary(content="## Goal\n...\n## Discoveries\n- ...\n## Accomplished\n- ✅ ...\n## Relevant Files\n- path — role")
mem_session_end(id="X-YYYY-MM-DD", summary="one-liner")
```

### "I need to hand off to another agent"

```python
# Lean-ctx: handoff
ctx_handoff(target="agent-name", summary="...", files=["..."])

# Codebase: ensure index is fresh
index_status(project="X")

# Engram: summary stays accessible via mem_search
mem_session_summary(content="...")
```

## Common Pitfalls

1. **Saving to only one layer.** When recording something important, save to ALL available layers: lean-ctx (`ctx_session` + `ctx_knowledge`), engram (`mem_save` + `mem_session_start/end`), skill file, and plan file. Each layer serves a different consumer: lean-ctx for current chat continuity, engram for cross-session durability, skills for procedural memory, plans for task tracking.

2. **Using mem_save for ephemeral state.** Task progress, intermediate steps, temporary debug info → `ctx_session`. Durable facts, decisions, patterns → `mem_save`. Don't pollute engram with noise.

3. **Skipping session_start/session_end.** Without `mem_session_start`, observations have no session linkage. Without `mem_session_end`, the session appears abandoned and future agents can't tell it completed.

4. **Not using topic_key.** Without a stable `topic_key`, each `mem_save` creates a new observation. With it, you get upsert behavior — future saves update the same entry, keeping the knowledge store clean.

5. **Forgetting codebase-tools index.** `search_graph` and `trace_path` only work on indexed repos. Always check `index_status` before structural queries.

6. **Mixing up ctx_knowledge vs mem_save.** `ctx_knowledge` is project-scoped and lives in lean-ctx's system. `mem_save` is cross-project and lives in engram. Use `ctx_knowledge` for things you need in the same project's lean-ctx context. Use `mem_save` for things any project's agent should find.

7. **Not reading attached docs.** When users attach reference files (like `mcp-tools/`), read them FIRST. They contain the actual API surface and tool parameters.

8. **Saving too much or too little.** The right amount: save after task completion, after discoveries, and at session end. Don't save after every file edit — that's noise.

9. **Guessing tool APIs instead of reading docs.** When the user attaches reference files (like `mcp-tools/` with lean-ctx/engram/codebase-tools docs), READ THEM FIRST before any tool calls. The attached docs contain exact parameters, method signatures, and required fields. Guessing wastes turns and produces errors the docs explicitly prevent. This applies to any MCP tool system — lean-ctx, engram, codebase-tools, or custom servers.

10. **Parallel subagent documentation.** When creating a documentation suite (multiple related .md files), dispatch subagents in batches of 3 (the max concurrent). Each subagent writes one file with full context in the goal. This produces 6 docs in ~3 minutes vs sequential writing. Group related docs in the same batch for context coherence.
