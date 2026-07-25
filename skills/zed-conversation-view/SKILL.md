---
name: zed-conversation-view
description: >
  Use when studying Zed ConversationView — ServerState Loading/LoadError/Connected,
  multi-session HashMap under one AgentConnection, auth state, or mirroring
  connect lifecycle for ChronOS multi-session agent UI.
---

# Zed ConversationView

**Source:** `crates/agent_ui/src/conversation_view.rs` (~11k)  
**License:** GPL-3.0-or-later  
**Entry:** `zed-ai`

## Role

One **logical thread slot** in the panel (draft or retained). Owns connection
lifecycle to a chosen `AgentServer` and multiplexes ACP sessions into
`ThreadView`s.

## Struct (~L591)

```text
ConversationView {
  agent: Rc<dyn AgentServer>
  connection_store, connection_key: Agent
  thread_id: ThreadId              # panel-side id (metadata)
  root_session_id: Option<SessionId>
  server_state: ServerState
  …
}
```

## `ServerState` (~L725)

```text
Loading { connection?, load task, … }
LoadError { error }
Connected(ConnectedServerState)
```

### `ConnectedServerState` (~L739)

```text
auth_state: Ok | Unauthenticated { description, pending_method }
active_id: Option<SessionId>
threads: HashMap<SessionId, Entity<ThreadView>>
connection: Rc<dyn AgentConnection>
conversation: Entity<Conversation>
```

**Pattern:** one connection, many session views; `active_id` selects which
`ThreadView` is shown/focused.

## Auth

Unauthenticated is a **first-class UI state**, not a toast-only error. Methods
on view: `has_auth_methods`, `supports_logout`, `reauthenticate`, `logout`
(panel wires actions).

## Navigation

`navigate_to_thread(session_id)` (~L698): set `active_id`, focus child,
emit `ActiveThreadChanged`. Used for subagents and session switching inside
the same connection.

## Work dirs

`set_work_dirs(PathList)` pushes cwd set into conversation — agents scoped to
project roots. ChronOS: config/cwd or active project from project-switcher.

## Events

Emits `AcpThreadViewEvent` / server events upward so `AgentPanel` can refresh
metadata, notifications, serialize.

## ChronOS mapping

| Zed | ChronOS |
|---|---|
| ConversationView per retained thread | Entity or struct per UI session row |
| ServerState 3-way | same enum — forces honest loading UI |
| threads HashMap | multi-session list body |
| AgentServer trait object | single Hermes server + later pluggable |

## Common mistakes

- New OS process per session (Zed multiplexes; ChronOS decision: one Hermes).
- Skipping Loading state → blank panel while spawn runs.
- Dropping Connected on tab switch — lose in-flight stream (Zed retains).
