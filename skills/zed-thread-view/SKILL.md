---
name: zed-thread-view
description: >
  Use when inspecting Zed ThreadView — message list (ListState), send/cancel
  path, tool-call expand, model/mode selectors, permission UI, or designing
  ChronOS chat transcript + tool cards layout.
---

# Zed ThreadView

**Source:** `crates/agent_ui/src/conversation_view/thread_view.rs` (~12k+)  
**License:** GPL-3.0-or-later  
**Entry:** `zed-ai`

## Role

**One ACP session's UI:** transcript + composer + chrome (title, model, mode,
permissions). Backed by `Entity<AcpThread>`.

## Struct highlights (~L566)

| Field | Role |
|---|---|
| `session_id` / `parent_session_id` | ACP ids |
| `thread: Entity<AcpThread>` | model |
| `list_state: ListState` | virtualized entry list |
| `message_editor` | composer |
| `model_selector` / `mode_selector` / `profile_selector` | chrome |
| `expanded_tool_call_raw_inputs` | which tools show raw I/O |
| `message_queue` | queue while Generating |
| `permission_selections` | allow/deny choices |
| `edits_expanded` / `plan_expanded` / `queue_expanded` | section toggles |
| `in_flight_prompt` | optimistic user bubble |

## Send path (simplified)

1. Editor event `MessageEditorEvent::Send` → `ThreadView::send`
2. Resolve contents (mentions → content blocks)
3. If already generating → **queue** or interrupt+send (`SendImmediately`)
4. Clear editor; push user entry; call into `AcpThread` / connection prompt
5. Stream updates mutate entries; `list_state` refreshes

Cancel → running turn cancel on thread + UI idle.

## Tool UI

- Default: **collapsed** card (name/status).
- Expand on NewDiff / NewTerminal events or user click.
- Permission wait: dropdown / allow once / allow always patterns
  (`PermissionOptions` from pending tool call).

## List

Uses GPUI `ListState` (not a naive full VStack of all messages) — required for
long threads. ChronOS should plan virtualization early if history grows.

## Selectors

Model / mode / profile are **optional entities** created when the server
advertises capabilities — hide chrome when absent.

## ChronOS v1 cut

Ship: list + stream text + collapsed tools + composer + model + status.  
Defer: queue UI polish, plan panel, feedback widgets, sandbox tooltips,
thread search bar.

## Common mistakes

- Re-render entire history every token without list diffing.
- No stop button while `ThreadStatus::Generating`.
- Tool cards only as plain text — status machine needs UI.
