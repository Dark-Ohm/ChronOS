---
name: zed-acp-thread
description: >
  Use when reading Zed's AcpThread session model — AgentThreadEntry variants,
  ToolCall status machine, ThreadStatus, streaming text buffer, plan/elicitation,
  or designing ChronOS message/tool entry types for the left agent panel.
---

# Zed AcpThread model

**Source:** `crates/acp_thread/src/acp_thread.rs` (~10k)  
**License:** GPL-3.0-or-later  
**Entry:** `zed-ai`

## `AcpThread` (~L2078)

One ACP **session**. Holds:

| Field | Role |
|---|---|
| `session_id` | `acp::SessionId` |
| `parent_session_id` | subagent parent (optional) |
| `entries: Vec<AgentThreadEntry>` | timeline |
| `elicitations` | permission/forms store |
| `plan` | agent plan entries |
| `running_turn` | in-flight generation |
| `connection` | `Rc<dyn AgentConnection>` |
| `token_usage` / `cost` | metering |
| `prompt_capabilities` / `available_commands` | server ads |
| `terminals` | ACP terminal ids → entities |
| `draft_prompt` | unsent composer restore |
| `streaming_text_buffer` | smooth typewriter drain |

Tied to Zed `Project` + `ActionLog` — ChronOS will substitute thinner deps.

## Timeline entries (`AgentThreadEntry` ~L389)

```rust
UserMessage | AssistantMessage | ToolCall
| Elicitation | CompletedPlan | ContextCompaction
```

**v1 ChronOS minimum:** User / Assistant / ToolCall.  
Elicitation + plan + compaction = later waves.

## ToolCall (struct ~L851, status ~L1253, content ~L1797)

- `id: ToolCallId`, `label: Entity<Markdown>`, `kind: ToolKind`
- `content: Vec<ToolCallContent>` — `ContentBlock | Diff | Terminal`
- `status: ToolCallStatus` — Pending, WaitingForConfirmation { current_status,
  options, respond_tx, kind }, InProgress, Completed, Failed, Rejected, Canceled
- `locations` / `resolved_locations` — file positions for jump/diff UI
- `raw_input` / `raw_output` — JSON blobs for debug/expand
- updates via `acp::ToolCallUpdateFields` merge

Collapsed vs expanded is **UI state** (`ThreadView.expanded_tool_call_raw_inputs`),
not the model.

## `ThreadStatus` (~L2211)

```rust
Idle | Generating
```

UI status chrome is richer (errors, retry, auth) on `ThreadView` /
`ConversationView`, not this enum alone.

## Streaming

`StreamingTextBuffer` (~L2115): model chunks land in `pending`; timer reveals
bytes into `Markdown` entity for fluid typing instead of chunk snaps.

ChronOS can start with direct append; steal the buffer if stream feels choppy.

## Subagents

`parent_session_id` + separate `AcpThread` entities; UI navigates via
`ConversationView::navigate_to_thread`. Pattern: **sessions are first-class**,
not nested message blobs.

## Load errors (`LoadError` ~L2217)

Unsupported version | FailedToInstall | Exited { status, stderr } | Other — map
1:1 to ChronOS status line / retry UI.

## Common mistakes

- Flattening tool calls into assistant markdown — lose status/permissions.
- Ignoring WaitingForConfirmation — agent blocks forever.
- Assuming entries are pure text — Diff/Terminal content types matter for v1.1+.
