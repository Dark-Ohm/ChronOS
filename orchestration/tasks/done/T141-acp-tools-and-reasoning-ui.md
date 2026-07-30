# T141 — ACP tool cards + reasoning blocks in chat

**Статус: OPEN. После T140 (tools must actually run).**  
**Канон:** revive design + mockups `design/Agent Thread.dc.html`.  
**Зависит:** T140 permission auto-approve.

| | |
|---|---|
| **Skills** | `chronos-shell`, `zed-thread-view` (patterns only — 0 line copy) |
| **Зоны** | `crates/services/src/hermes_acp/client.rs`,  
| | `crates/app/src/side_panel_left/{chat_view,composer,tool_card}.rs` |
| **Отчёт** | `orchestration/tasks/report/T141-acp-tools-and-reasoning-ui-report.md` |
| **Коммит** | `acp : tool calls + thought chunks in left chat (T141)` |

## Контекст

- Сегодня `read_to_string` (SDK) **игнорирует** non-text updates  
  (`session.rs` ActiveSession::read_to_string — only `AgentMessageChunk` Text).
- UI уже умеет `ToolCallPreview` + `ToolCard` (`chat_view.rs`, `tool_card.rs`),
  но `composer` всегда пушит `tool_calls: Vec::new()`.
- Reasoning: `SessionUpdate::AgentThoughtChunk` — **не** собирается.
- User (2026-07-26 grim): chat text OK; tools/reasoning empty; YOLO mode
  label visible but irrelevant without stream parse.

## Цель

После send в thread видны:
1. **Thought / reasoning** (collapsible, muted).
2. **Tool cards** (name, status running/done/error, args, result) — reuse ToolCard.
3. Agent **text** as now.

## Задачи

### Task 1 — `PromptResponse` richer

В `client.rs` заменить/обернуть `read_to_string` своей `read_turn`:

```text
loop read_update until StopReason:
  AgentMessageChunk Text  → append text
  AgentThoughtChunk Text  → append thought
  ToolCall                → push/update tool map by tool_call_id
  ToolCallUpdate          → merge status/content/raw_output
  other (plan, usage, …)  → ignore or log debug
```

```rust
pub struct PromptResponse {
    pub text: String,
    pub thought: String,           // may be empty
    pub tools: Vec<ToolCallInfo>,  // ordered
    pub modes: Option<...>,
    pub models: Option<...>,
    pub session_id: String,
}

pub struct ToolCallInfo {
    pub id: String,
    pub name: String,      // title or kind
    pub status: String,    // "running" | "done" | "error" | "pending"
    pub args: Option<String>,
    pub result: Option<String>,
}
```

Map `ToolCallStatus::{Pending,InProgress,Completed,Failed}` → UI status
strings that `ToolCard` already understands (`running`/`done`/`error`).

Extract args: `raw_input` JSON pretty-trunc; result: `raw_output` or text
from `ToolCallContent::Content`.

### Task 2 — Wire chat UI

- `ChatMessage`: optional `thought: Option<String>` + fill `tool_calls`.
- `render_message`: if thought non-empty → muted block «Reasoning» (collapsed
  by default OK).
- Composer on Ok(response): push Agent message with text + thought + tools.
- Empty thought/tools → no extra chrome (no empty cards).

### Task 3 — Verify

```bash
# After T140:
# Prompt that triggers a tool (write /tmp/…)
# Expect: tool card(s) in thread; reasoning if model emits thought chunks
# Prompt pure chat → no empty tool section
cargo check -p chronos
```

## Accept

- [ ] Tool-using turn shows ≥1 ToolCard with name + status.
- [ ] Thought chunks appear when agent sends them (if model silent — note in report).
- [ ] Pure text turn unchanged (no blank reasoning box).
- [ ] No regression: session_id reuse, permission still approved (T140).

## Out of scope

- Live streaming mid-turn (token-by-token) — later.
- Model dropdown (T142).
- Diff viewer for ToolCallContent::Diff — show as text dump OK.
