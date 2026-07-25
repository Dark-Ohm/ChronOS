---
name: zed-thread-metadata
description: >
  Use when studying Zed's thread sidebar persistence — ThreadMetadata,
  ThreadMetadataStore, draft vs session_id, archive, title_override, or
  designing ChronOS multi-session list storage for the left agent panel.
---

# Zed ThreadMetadataStore

**Source:** `crates/agent_ui/src/thread_metadata_store.rs` (~4k+)  
**License:** GPL-3.0-or-later  
**Entry:** `zed-ai`

## Role

**Sidebar index** of threads independent of heavy `ConversationView` /
`AcpThread` entities. Survives panel reloads; drives list UI and restore.

## `ThreadId`

Panel-local UUID wrapper (`ThreadId(uuid)`) — **not** the same as
`acp::SessionId`. A draft has `ThreadId` before any ACP session exists.

## `ThreadMetadata` (~L309)

| Field | Role |
|---|---|
| `thread_id` | stable UI key |
| `session_id: Option<…>` | None ⇒ **draft** (`is_draft()` ~L331) |
| `agent_id` | which server |
| `title` / `title_override` | agent title vs user rename (override wins) |
| `created_at` / `updated_at` / `interacted_at` | sort + activity |
| `worktree_paths` | project roots |
| `remote_connection` | remote project identity |
| `archived` | soft-delete from main list |

## Store behaviors

- List unarchived for sidebar; `archived_entries` for archive view
  (`threads_archive_view.rs`).
- Archive / unarchive toggles flag; may interact with git worktree cleanup
  (`ArchivedGitWorktree`).
- Updates from ACP title / session info must **not** clobber `title_override`.
- Draft cleanup: drafts without content can be dropped on panel serialize.

## Relation to panel retention

```text
ThreadMetadataStore  = durable index (many rows)
AgentPanel.retained_threads = hot Entity<ConversationView> (few, max idle 5)
```

Opening a cold thread: metadata hit → build fresh `ConversationView` →
`load_session` if `session_id` present.

## Terminal threads

Parallel store: `terminal_thread_metadata_store.rs` for terminal-agent tabs —
do not mix with ACP thread rows.

## ChronOS v1

Minimal table:

```text
id | acp_session_id? | title | updated_at | archived
```

Persist under `~/.config/chronos/` or state dir. Draft = no session yet.
Multi-session list UI reads this store, not live entity map alone.

## Common mistakes

- Using ACP session id as only key — drafts break.
- Letting agent title overwrite user rename.
- Keeping all ConversationViews forever — memory leak; pair with idle cap.
