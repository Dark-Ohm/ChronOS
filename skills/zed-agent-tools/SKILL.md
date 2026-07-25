---
name: zed-agent-tools
description: >
  Use when surveying Zed's native agent tool surface (edit_file, terminal,
  grep, spawn_agent, …), tool permissions, or deciding which tool results the
  ChronOS left panel must render versus what Hermes owns server-side.
---

# Zed native agent tools

**Source:** `crates/agent/src/tools/` + `tools.rs`  
**License:** GPL-3.0-or-later  
**Entry:** `zed-ai`

## Split of responsibility

| Side | Owns |
|---|---|
| **Agent process** (Claude/Codex/Gemini/… via ACP) | Decides to call tools; may run tools itself |
| **Zed `agent` crate** | Native tool implementations when Zed is the tool host |
| **`acp_thread` ToolCall entries** | UI-facing status + content (diff/terminal/markdown) |
| **`agent_ui`** | Cards, permission prompts, expand/collapse |

ChronOS left panel talks to **Hermes over ACP**. Hermes runs tools; ChronOS
**renders** tool_call updates. Do not reimplement Zed's edit_session stack
unless ChronOS becomes a tool host.

## Tool files (inventory)

```text
read_file, write_file, edit_file (+ edit_session streaming fuzzy)
delete_path, copy_path, move_path, create_directory, list_directory, find_path
grep, find_references, go_to_definition, get_code_actions, apply_code_action
rename, diagnostics, symbol_locator
terminal_tool
fetch, web_search
create_thread, spawn_agent, list_agents_and_models
skill_tool
context_server_registry (MCP-ish)
tool_permissions
```

## UI content types that matter

From `ToolCallContent` in `acp_thread`:

1. **ContentBlock** — markdown / text result (default card body)
2. **Diff** — inline diff view (`acp_thread/src/diff.rs`)
3. **Terminal** — streaming terminal (`acp_thread/src/terminal.rs`)

ChronOS waves (brainstorm):

| Wave | Render |
|---|---|
| v1 | collapsed card + status + short text |
| v1.1 | expand terminal output + read-only diff |
| v2 | subagent spawn_agent navigation |

## Permissions

`WaitingForConfirmation` on tool status + `PermissionOptions` — UI must
answer allow/deny or the turn stalls. Mirror with simple Allow / Deny /
Allow always in panel (right panel already has static permission mock —
different feature).

## Common mistakes

- Building ChronOS tools by copying `crates/agent` — GPL + Project/Buffer.
- Assuming every tool_call has a diff — most are text.
- Ignoring spawn_agent — creates **child sessions**, needs session list UX.
