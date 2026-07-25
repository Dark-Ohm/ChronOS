---
name: zed-message-editor
description: >
  Use when studying Zed's agent composer (MessageEditor) — it wraps the full
  Zed Editor/MultiBuffer, not a simple TextInput; also when choosing ChronOS
  composer tech (gpui-component TextInput vs Editor port) or implementing
  send/cancel/slash/mention behavior.
---

# Zed MessageEditor

**Source:** `crates/agent_ui/src/message_editor.rs` (~5.7k)  
**License:** GPL-3.0-or-later  
**Entry:** `zed-ai`

## Critical fact

```text
MessageEditor { editor: Entity<Editor>, … }
```

Composer is the **full Zed `Editor`** (buffer, selections, soft wrap, IME,
keybindings, completions) — **not** a single-line input widget and **not**
Longbridge `TextInput`.

Construction (~L450+): `Editor::new(mode, buffer, …)` + `MessageEditorAddon`
+ completion provider for `@` mentions and `/` slash commands.

## Events (`MessageEditorEvent` ~L221)

| Event | Meaning |
|---|---|
| `Send` | user submit (queued if busy) |
| `SendImmediately` | interrupt + send |
| `Cancel` | stop generation |
| `Focus` / `LostFocus` | focus chrome |
| `SlashAutocompleteOpened` | `/` menu |
| `LocalCommandInvoked` | client-side slash |
| `Edited` / `InputAttempted` | draft persistence hooks |

`ThreadView` subscribes and routes Send → `send`, Cancel → cancel turn.

## Capabilities

`SessionCapabilities` / `SharedSessionCapabilities` gate what the editor
allows (images, etc.) from ACP prompt capabilities.

## Mentions

`mention_set.rs` + completion provider resolve `@file` / symbols into
`acp::ContentBlock`s at send time — heavy project index coupling.

## Expand

Panel action `ExpandMessageEditor` grows composer (min/max lines from
`AgentSettings`) — still the same Editor entity.

## ChronOS decision (2026-07-22 brainstorm)

User chose **`gpui-component` TextInput** for ChronOS composer — reverse of
prior "don't take gpui-component" for the right system panel. That gives
multiline + cursor without porting Zed `editor` crate (impossible/GPL/IDE).

| Need | Zed | ChronOS v1 |
|---|---|---|
| Multiline + enter-to-send | Editor | gpui-component input |
| @mentions | completion provider | v1.2 chips / path picker |
| Slash commands | completion | optional later |
| Draft persist | draft_prompt on thread | local state + optional disk |

## Common mistakes

- Assuming `gpui::Input` exists in core gpui like a DOM input — it does not.
- Porting `message_editor.rs` — pulls Editor, language, project, GPL.
- Enter key: decide send-vs-newline explicitly (Zed uses key context + actions).
