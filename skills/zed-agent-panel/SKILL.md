---
name: zed-agent-panel
description: >
  Use when inspecting Zed's AgentPanel shell — dock position, default/min
  width, retained threads, draft slot, toolbar, visible surface switching, or
  comparing ChronOS left-panel windowing to Zed's workspace dock Panel.
---

# Zed AgentPanel

**Source:** `crates/agent_ui/src/agent_panel.rs` (~13.4k LOC)  
**License:** GPL-3.0-or-later  
**Entry:** `zed-ai`

## Role

Top-level **workspace dock panel** (`impl Panel for AgentPanel`). Owns which
conversation/terminal is visible; does **not** own ACP wire protocol (that is
`ConversationView` → connection → `AcpThread`).

## Struct (fields that matter)

```text
AgentPanel {
  workspace, workspace_id, project, fs, …
  thread_store, connection_store, context_server_registry
  focus_handle
  draft_thread: Option<Entity<ConversationView>>
  retained_threads: HashMap<ThreadId, Entity<ConversationView>>
  terminals: HashMap<TerminalId, AgentTerminal>
  selected_agent: Agent
  is_active, zoomed, …
}
```

Anchor: `pub struct AgentPanel` ~L1153.

## Dock / size (`impl Panel`)

| Method | Behavior | ~line |
|---|---|---|
| `position` | settings → Left or Right (not Bottom-only) | ~4963 |
| `position_is_valid` | rejects `DockPosition::Bottom` | ~4967 |
| `default_size` | `AgentSettings.default_width` (L/R) or height | ~4985 |
| `min_size` | `MIN_PANEL_WIDTH = px(300.)` on L/R | ~106, ~4993 |
| `supports_flexible_size` | `true` | ~5000 |
| `set_active(true)` | `ensure_thread_initialized` | ~5017 |

Resize is **dock chrome**, not a custom drag handle inside the panel.

## Render hierarchy

`fn render` ~L6432:

```text
v_flex (panel_background, key_context, actions)
  ├── render_toolbar
  ├── optional onboarding
  └── match visible_surface:
        Uninitialized → empty / no-project
        AgentThread(cv) → ConversationView + drag target
        Terminal(tv) → terminal (+ search bar)
```

Font: optional `WithRemSize(agent_ui_font_size)`.

## Thread retention

- **`retained_threads`**: hot `ConversationView` entities kept alive.
- **`draft_thread`**: unsent new thread (`session_id` still `None` in metadata).
- **`MaxIdleRetainedThreads`**: GPUI global, default **5** (~L151–159).
- Serialize active thread / agent / draft id via KVP (`serialize` ~L1192).

## Surfaces

`VisibleSurface`: uninitialized | agent thread | embedded terminal agent tab.
Terminal agents are a **parallel product surface** (known CLI names list
~L111–129), not the same as ACP tool terminal output inside a thread.

## ChronOS mapping

| Zed | ChronOS left panel (planned) |
|---|---|
| `Panel` in workspace dock | layer-shell `side_panel_left` (like right) |
| dock drag width | **explicit drag-resize** on overlay |
| min 300 / settings width | min + default + persist |
| retained_threads | same idea, smaller cap OK |
| terminal tab in panel | **out of v1** unless scoped later |

## Common mistakes

- Treating AgentPanel as the ACP client — it is not; it hosts views.
- Assuming bottom dock is supported — explicitly invalid.
- Copying file into ChronOS — GPL.
