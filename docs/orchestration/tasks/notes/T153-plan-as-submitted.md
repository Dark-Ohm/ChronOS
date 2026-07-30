# T153 — Plan: Segments Model

## Problem
One agent response = one `ChatMessage` with `content`, `thought`, `tool_calls` — three fixed buckets. Chronology lost: thinking → tool → thinking → answer → tool all collapses into three piles drawn in fixed order.

## Solution
Replace three fields with `Vec<Segment>` — an ordered timeline of events as they actually arrived.

---

## Step 1: New types (`chat_view.rs`)

```rust
#[derive(Clone, Debug)]
pub enum Segment {
    Thinking { content: String },
    ToolCall { tool: ToolCallPreview },
    Response { content: String },
}

#[derive(Clone, Debug)]
pub struct ChatMessage {
    pub role: MessageRole,
    pub segments: Vec<Segment>,
}
```

Remove: `content`, `thought`, `tool_calls` fields from `ChatMessage`.

## Step 2: `ChatView` state (`chat_view.rs`)

```rust
pub collapsed_reasoning: HashSet<(usize, usize)>,  // (msg_idx, seg_idx) — was HashSet<usize>
pub expanded_tool_calls: HashSet<(usize, usize)>,   // same key, meaning unchanged
```

`toggle_reasoning(msg_idx, seg_idx)` — two args now.

## Step 3: Simplify `StreamingState` (`state.rs`)

Remove `text_buffer`, `thought_buffer`, `tool_calls` — data lives in segments now. Keep only `active`, `receiver_task`, `acp_task`.

## Step 4: Streaming events (`composer.rs` ~L948-1014)

Core append logic per event type:

- **TextChunk(delta):** if last segment is `Response` → append. Else → push `Segment::Response`.
- **ThoughtChunk(delta):** if last segment is `Thinking` → append. Else → push `Segment::Thinking`. Remove from `collapsed_reasoning` for this (msg_idx, seg_idx).
- **ToolCall { id, ... }:** search segments backwards for `Segment::ToolCall` with matching `tool.id` → update if found. Else → push new `Segment::ToolCall`.

## Step 5: Finalization (`composer.rs` ~L824-872)

**On Done:** do NOT replace segments with PromptResponse (events are the source of truth per spec). Just:
1. `streaming.reset()`
2. `mark_pending_tools_stale()`
3. Update session_id, modes, models
4. Collapse ALL thinking segments in last message
5. `scroll_to_bottom()`

**On Error:** push `Segment::Response { content: format!("Error: {e}") }`.

**On Cancel:** find last Response segment → append "⏹ Turn cancelled by user." or create one.

## Step 6: Initial placeholder (`composer.rs` ~L806)

```rust
ChatMessage { role: MessageRole::Agent, segments: Vec::new() }
```

User message: `segments: vec![Segment::Response { content: text }]`.

## Step 7: Rendering (`chat_view.rs` `render_message`)

Replace fixed-order sections with segment iteration:

```rust
for (seg_idx, seg) in msg.segments.iter().enumerate() {
    match seg {
        Segment::Thinking { content } => { /* reasoning block, key (msg_idx, seg_idx) */ }
        Segment::ToolCall { tool } => { /* tool card, key (msg_idx, seg_idx) */ }
        Segment::Response { content } => { /* text content */ }
    }
}
```

User messages: single Response segment, right-aligned bubble (unchanged visually).

## Step 8: Collapse auto-behavior

- **During streaming:** last thinking segment in last message → auto-expand (`collapsed_reasoning` removal on ThoughtChunk).
- **On stream end:** collapse ALL thinking segments in last message.
- **`mark_pending_tools_stale`:** iterate segments, update ToolCall variants.
- **`cancel_streaming`:** find last Response segment or create one.

---

## Files to modify
1. `crates/app/src/side_panel_left/chat_view.rs` — types + rendering
2. `crates/app/src/side_panel_left/state.rs` — StreamingState simplification
3. `crates/app/src/side_panel_left/composer.rs` — event handling, finalization, cancel

## Verification
- `cargo build --release -p chronos` — 0 errors
- Live: agent prompt with mixed thinking/tools/answer → segments appear in chronological order
- Live: each thinking block collapses independently
- Live: during stream, last thinking block auto-expands
- Live: tool cards stay at their position in the stream
