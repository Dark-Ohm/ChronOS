## T107 — Task 8: Chat View (Message Stream)

**Commit:** `70b4d61` on `feat/left-agent-panel`

### What was done

Replaced stub `chat_view.rs` (1-line comment) with a fully functional chat message view.

### Files changed

| File | Change |
|---|---|
| `crates/app/src/side_panel_left/chat_view.rs` | Full implementation: `ChatView`, `ChatMessage`, `MessageRole`, `ToolCallPreview`, scrollable message list with auto-scroll |
| `crates/app/src/side_panel_left/mod.rs` | Added `chat: ChatView` field to `SidePanelLeft`, initialized in `new()` |
| `crates/app/src/side_panel_left/panel.rs` | Replaced `"Chat goes here"` placeholder with `panel.chat.render()` inside a flex column |

### Architecture

- **`ChatView`** owns a `Vec<ChatMessage>` and a `ScrollHandle`
- **`ChatMessage`** has `role` (User/Agent), `content` (plain text), and `tool_calls` (optional `Vec<ToolCallPreview>`)
- **User messages**: right-aligned, blue-tinted role label, darker bubble (`0x313244`)
- **Agent messages**: left-aligned, green-tinted role label, slightly lighter bubble (`0x1e1e30`)
- **Tool calls**: rendered as small status dots (yellow=running, green=done, red=error) under agent messages
- **Empty state**: centered "No messages yet" placeholder
- **Scroll**: `overflow_y_scroll()` with `.id()` + `.track_scroll()` on `ScrollHandle`
- **Auto-scroll**: `scroll_to_bottom()` sets offset to `f32::MAX`

### What's not wired yet

- `ChatView` is not connected to a real ACP session (no `push_message` calls from live data). That's expected — Task 9+ territory.
- No markdown rendering (plain text for now). gpui-component is not in ChronOS deps, so this stays plain until a markdown crate is vendored.
- `scroll_to_bottom()` is defined but not called automatically on push — the caller (future ACP integration) should call it after `push_message` + `cx.notify()`.

### Build

`cargo build --release -p chronos` — clean, 14 pre-existing warnings, 0 errors.
