---
name: zed-ai
description: >
  Use when studying or porting patterns from Zed's AI agent stack (agent panel,
  ACP, threads, tools) — route to crate-level zed-* skills; never open them
  without first confirming which crate you're in, or claiming "Zed does X"
  without checking the actual source.
---

# Zed AI — Orientation

## Overview

Zed's agent product is a **multi-crate GPUI stack** over the
[Agent Client Protocol](https://agentclientprotocol.com) (ACP). Local recon
checkout (sparse, 2026-07-22): `/home/neo/scratch/zed-agent-recon`
(`github.com/zed-industries/zed`, depth-1).

**License wall:** `agent_ui`, `acp_thread`, `agent`, `agent_servers` are
**GPL-3.0-or-later**. ChronOS is Apache-2.0 — **patterns and protocol only,
never copy source**.

## Crate map

| Crate | Role | ~LOC (src) | Skill |
|---|---|---|---|
| `agent_servers` | Spawn ACP child, JSON-RPC stdio | ~5k (`acp.rs`) | `zed-acp-stdio` |
| `acp_thread` | Session model, entries, stream | ~10k | `zed-acp-thread` |
| `agent` | Native agent tools + ThreadStore | tools/* | `zed-agent-tools` |
| `agent_ui` | All UI | panel 13k, CV 11k, editor 5k | panel / conversation / thread / message-editor / metadata |
| `agent-client-protocol` | Schema (workspace dep) | — | used by all above |

## UI tree (one glance)

```
AgentPanel                    # workspace dock Panel
├── toolbar
├── ConversationView × N      # retained_threads + draft
│   └── ServerState
│         Loading | LoadError | Connected
│         └── HashMap<SessionId, ThreadView>
└── ThreadView
    ├── ListState (entries)
    ├── MessageEditor → Editor
    └── model / mode / profile
```

## Route

| Question | Skill |
|---|---|
| Dock, width, open/close, retained threads shell | `zed-agent-panel` |
| Spawn process, initialize, JSON-RPC | `zed-acp-stdio` |
| Entries, ToolCall, ThreadStatus, stream buffer | `zed-acp-thread` |
| Loading/auth/multi-session under one connection | `zed-conversation-view` |
| Message list render, send path | `zed-thread-view` |
| Composer = full Editor | `zed-message-editor` |
| Sidebar list, draft vs session, archive | `zed-thread-metadata` |
| Native tools (edit/grep/terminal/…) | `zed-agent-tools` |
| What ChronOS may steal for left panel | `zed-ai-for-chronos` |

## Hard rules

1. **Do not** `include` / vendor GPL agent crates into ChronOS.
2. **Do not** confuse `Chronos-IDE/chronos-agent` (custom LLM+tools) with Zed ACP.
3. Evidence paths below are relative to the scratch checkout; re-clone if missing:
   `git clone --depth 1 --filter=blob:none --sparse …` then
   `sparse-checkout set crates/agent crates/agent_ui crates/acp_thread crates/agent_servers`.
4. Upstream moves fast — treat line numbers as **anchors**, re-grep names.

## Related ChronOS work

Left agent panel (planned): approach A — `side_panel_left` + Hermes ACP stdio,
mirror windowing of `side_panel_right`. Full design still in brainstorming.
