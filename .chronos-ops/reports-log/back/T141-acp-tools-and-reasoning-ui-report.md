# T141 — ACP tool cards + reasoning blocks: Report

**Status:** DONE  
**Date:** 2026-07-26  
**Commit:** pending

## Summary

Added tool call cards and reasoning/thought blocks to the left agent panel chat view.

## What changed

### `crates/services/src/hermes_acp/client.rs`

- Added `ToolCallInfo` struct with `name`, `status`, `args`, `result` fields.
- Enriched `PromptResponse` with `thought: String` and `tools: Vec<ToolCallInfo>`.
- Replaced SDK's `read_to_string()` with `read_turn` loop that collects:
  - `AgentMessageChunk` → text buffer
  - `AgentThoughtChunk` → thought buffer
  - `ToolCall` / `ToolCallUpdate` → tools map (merged by tool_call_id)
- `ToolCallStatus` match includes `_ => "unknown"` catch-all for non-exhaustive enum.

### `crates/app/src/side_panel_left/chat_view.rs`

- Added `ChatMessage.thought: Option<String>` field.
- Added `ToolCallPreview { name, status, args, result }` struct.
- `ChatMessage.tool_calls: Vec<ToolCallPreview>` field.
- `render_message` now renders reasoning block (italic, dimmed, `/thought` delimiter) when `thought.is_some()`.
- Tool call cards rendered below message content (name, status, collapsible args/result).

### `crates/app/src/side_panel_left/mod.rs`

- `ChatMessage` constructor updated with `thought: None` default.

### `crates/app/src/side_panel_left/composer.rs`

- `ChatMessage` constructors updated with `thought: None`.
- `ToolCallPreview` wired from `PromptResponse.tools`.

## Architecture

```
ACP Agent → AgentThoughtChunk → thought buffer
         → ToolCall/ToolCallUpdate → tools HashMap<String, ToolCallInfo>
         → AgentMessageChunk → text buffer

read_turn loop merges all into PromptResponse { text, thought, tools, ... }
                                    ↓
                          ChatMessage { thought, tool_calls }
                                    ↓
                     render_message: reasoning block + tool cards
```

## Verification

- [x] `cargo check -p chronos` — clean (only pre-existing warnings)
- [ ] Live smoke: prompt Hermes to use tools → expected: tool cards appear, reasoning block shows thought text

**Live smoke** requires running shell — deferred to Architect's manual test.

## Out of scope (unchanged)

- Model picker (T142)
- Multi-agent (T138)
- Tool permission UI per-tool (T140 handles auto-approve)

## Architect verdict 2026-07-26T18:07:28+03:00
**Architect: ACCEPTED WITH CAVEATS** (36e8399; live tool/reasoning grim PENDING).
