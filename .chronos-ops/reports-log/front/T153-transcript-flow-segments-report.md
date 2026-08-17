# T153: Transcript flow — segments instead of three buffers

**Status:** committed `b4fd47c`  
**Files:** 4 changed, +328/−226

## What

Replaced `ChatMessage`'s three-buffer model (`content`, `thought`, `tool_calls`) with `Vec<Segment>` — a chronologically ordered list where each segment is one of `Thinking`, `ToolCall`, or `Response`.

## Changes

### `chat_view.rs`
- `Segment` enum with three variants.
- `ChatMessage`: fields → `segments: Vec<Segment>`.
- `collapsed_reasoning`: `HashSet<usize>` → `HashSet<(usize, usize)>` for seg-level granularity.
- New render functions: `render_segment_content`, `render_thinking_block`, `render_tool_card_segment`.
- `render_message` iterates `msg.segments`, dispatches to render functions, unwrapped via `.into_any_element()`.

### `composer.rs`
All 6 handler blocks updated:
- **TextChunk**: append to last `Response` segment, or create new one (switches segment type).
- **ThoughtChunk**: append to last `Thinking` segment, or create new + remove from collapsed_reasoning.
- **ToolCall**: search by `tool.id` across segments (rev), update or push new `ToolCall`.
- **Done**: guard response length vs `prompt_response.text`, insert `Response`/`Thinking` if missing from streaming, sync tool statuses, collapse all thinking.
- **Error**: replace empty `Response` or push new one, collapse thinking.
- **Timeout**: check for any non-empty `Response` segment, push timeout message, collapse thinking.
- **cancel_turn**: find last `Response` segment, append cancel marker (or push new), collapse thinking.
- **mark_pending_tools_stale**: iterate segments for `ToolCall` variants.

### `state.rs`
- Stripped `text_buffer`, `thought_buffer`, `tool_calls` from `StreamingState`.

### `mod.rs`
- Error-message `ChatMessage` constructor updated to `segments`.

## Verify
```
cargo build --release -p chronos   # 0 errors
```

## Holes (accepted before implementation)
- H1: Done finalization guards against lost streaming events (warns on length mismatch, inserts missing segments).
- H2: Timeout checks for non-empty Response segment (not `last_msg.content.is_empty()`).
- H3: All 5 `collapsed_reasoning` touchpoints use `(msg_idx, seg_idx)`.
