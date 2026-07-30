# Left Agent Panel — Design Spec

**Date:** 2026-07-23
**Status:** APPROVED (brainstorming complete)
**Scope:** новый модуль `crates/app/src/side_panel_left/` + новый сервис `crates/services/src/hermes_acp/`
**Dependencies:** `agent-client-protocol` (ACP Rust SDK), `gpui-component` (UI components)

---

## 1. Цель

Left agent panel — surface для AI-ассистента (Hermes ACP). Два основных кейса:
1. **Чат** — пользователь отправляет промпт, агент отвечает текстом + tool calls
2. **Сессии** — несколько независимых диалогов, быстрое переключение

**Успех v1:**
- Hover у левого края показывает панель (peek), уход курсора — закрывает
- Хоткей/иконка в баре пинает панель
- Чат работает через Hermes ACP (JSON-RPC over stdio)
- Tool calls отображаются как collapsed cards с превью результата
- Композер: текстовое поле + Send + model/mode picker + attach button
- Sessions list в collapsible sidebar
- Статус в хедере (online/offline/thinking)
- Drag-resize ширины панели

**Вне v1:** @mentions, subagents, inline diffs, terminal view

**Добавлено в v1 (2026-07-23, DECISIONS.log):** свитчер агента в хедере —
клик по названию текущего агента открывает список ACP-совместимых
бэкендов (Hermes/Cline/OpenCode/...), выбор меняет активный бэкенд для
новой сессии. Визуал принят (`design/Agent Panel.dc.html`, кадр 1,
`showAgentMenu`). Архитектура — см. T108
(`orchestration/tasks/done/T107-left-agent-panel.md` НЕ переоткрывается,
T108 строится поверх).

---

## 2. Архитектура

### 2.1 Модули

```
crates/app/src/side_panel_left/
├── mod.rs           — WindowKind::LayerShell, main entry
├── state.rs         — Peek | Pinned | Resizing state machine
├── panel.rs         — render: header + body (sidebar + chat)
├── sessions_list.rs — collapsible session sidebar
├── chat_view.rs     — message stream (user/agent messages)
├── composer.rs      — TextInput + model/mode pickers + send
└── tool_card.rs     — collapsed tool call cards

crates/services/src/hermes_acp/
├── mod.rs           — Service trait implementation
├── client.rs        — agent-client-protocol Client trait impl
├── session.rs       — AcpSession wrapper (session lifecycle)
└── transport.rs     — stdio spawn (hermes acp command)
```

### 2.2 Зависимости

```toml
[dependencies]
agent-client-protocol = "0.5"      # Official ACP Rust SDK
agent-client-protocol-tokio = "0.5" # Tokio stdio transport
gpui-component = { path = "../Source/gpui-component" }  # UI components
```

---

## 3. GPUI Window — layer-shell overlay

```rust
WindowKind::LayerShell(LayerShellOptions {
    namespace: "side_panel_left".to_string(),
    layer: Layer::Overlay,
    anchor: Anchor::LEFT | Anchor::TOP | Anchor::BOTTOM,
    exclusive_zone: Some(px(0.)),   // не двигает рабочую область
    keyboard_interactivity: KeyboardInteractivity::OnDemand,
    ..Default::default()
})
```

**Width:** `352px` default (как правая панель), **resizable** через drag handle (новая capability — правая панель зафиксирована).

**Height:** во весь монитор (top..bottom anchor).

---

## 4. States: Peek | Pinned | Resizing

Как в правой панели (`side_panel_right`), но с добавлением Resizing:

```rust
enum PanelState {
    Peek,      // hover-peek, закрывается при уходе курсора
    Pinned,    // закреплена, закрывается по тоглу/Esc
    Resizing,  // пользователь тянет resize handle
}
```

**Hover-peek:** курсор у левого края экрана (zone ~4px) показывает панель. Уход курсора с панели — закрывает с debounce.

**Pin:** хоткей или клик по иконке в баре открывает/держит панель пинованной.

**Resizing:** drag handle справа от панели. Минимальная ширина: 280px, максимальная: 50% экрана.

---

## 5. Hermes ACP Service

### 5.1 Spawn

Один процесс `hermes acp` на ChronOS shell. Запускается при старте шелла (не по первому открытию панели). Multi-session через `session_id`.

### 5.2 Official ACP SDK

```rust
use agent_client_protocol::Client;
use agent_client_protocol_tokio::StdioTransport;

// Spawn hermes process
let transport = StdioTransport::spawn("hermes", ["acp"])?;

// Connect as client
Client::builder()
    .name("chronos-shell")
    .connect_with(transport, |cx| async move {
        // Initialize
        cx.send_request(InitializeRequest::new(ProtocolVersion::V1)).await?;

        // Create session
        let session = cx.build_session_cwd()?.run_until(|session| async move {
            session.send_prompt("Hello!")?;
            let response = session.read_to_string().await?;
            Ok(())
        }).await?;
    })
    .await;
```

### 5.3 Session Management

- Hermes хранит сессии — ChronOS = UI только
- При переподключении ChronOS подтягивает список сессий
- Draft vs live: draft = незавершённый промпт, live = завершённый диалог

### 5.4 Error Handling

- **Offline:** Hermes недоступен → красный статус в хедере, composer заблокирован
- **Retry:** автоматический reconnect + кнопка retry
- **Timeout:** показать "Thinking..." → таймаут через 60s → "No response"

---

## 6. UI Components (gpui-component)

| Компонент | Использование |
|-----------|---------------|
| `Sidebar` | Sessions list, collapsible в иконки |
| `Resizable` | Panel width drag-resize |
| `TextInput` | Composer input field (auto-grow) |
| `Button` | Send, attach, close |
| `Markdown` | Message rendering (agent responses) |
| `VirtualList` | Session list (large datasets) |
| `Collapsible` | Tool call cards (expand/collapse) |

### 6.1 Layout

```
┌─────────────────────────────────────────────┐
│ [≡] Agent ● Online                    [×]  │  ← Header
├────────┬────────────────────────────────────┤
│ Session│  User: What is GPUI?               │  ← Chat
│ ▸ New  │                                     │
│        │  Agent: GPUI is a GPU-accelerated   │
│ Chat 1 │  UI framework...                   │
│ Chat 2 │                                     │
│        │  ┌─────────────────────────────┐   │  ← Tool Card
│        │  │ 🔧 search_web (collapsed)   │   │
│        │  └─────────────────────────────┘   │
│        │                                     │
│        │  Agent: Based on my research...    │
│        │                                     │
│        ├────────────────────────────────────┤
│        │ [📎] [Model ▾] [Mode ▾] [Send]   │  ← Composer
└────────┴────────────────────────────────────┘
```

---

## 7. Message Flow

```
User types prompt → composer.rs
  → hermes_acp::client.rs → session.send_prompt()
  → response stream → chat_view.rs
  → text chunks → markdown render
  → tool calls → tool_card.rs (collapsed by default)
  → tool result → back to agent
```

### 7.1 Entry Point

```rust
// crates/app/src/main.rs
// Side panel left — layer-shell overlay
app.spawn_window(side_panel_left::create_window)?;
```

### 7.2 Event Flow

1. **Prompt submitted** → `HermesService::send_prompt(session_id, text)`
2. **Response chunk** → `AgentEvent::TextChunk(session_id, chunk)`
3. **Tool call** → `AgentEvent::ToolCall(session_id, tool_call)`
4. **Tool result** → `HermesService::submit_tool_result(call_id, result)`
5. **Done** → `AgentEvent::Done(session_id)`

---

## 8. Error Handling / Offline

| State | UI | Action |
|-------|-----|--------|
| Hermes online | Green dot in header | Normal operation |
| Hermes offline | Red dot, "Offline" text | Composer disabled, retry button |
| Thinking | Yellow dot, "Thinking..." | Composer disabled, cancel button |
| Timeout (60s) | "No response" in chat | Retry or new prompt |
| Error | Inline error card | Retry button |

---

## 9. Phased Ship

**One shot:** SDK + gpui-component significantly reduce work. All features in single PR.

| Component | Est. Effort | Notes |
|-----------|-------------|-------|
| Layer-shell window + peek/pin | 1 day | Pattern from right panel |
| Hermes ACP service | 2 days | Official SDK, stdio transport |
| Sessions list (sidebar) | 1 day | gpui-component Sidebar |
| Chat view + message stream | 2 days | gpui-component Markdown |
| Composer + pickers | 1 day | gpui-component TextInput |
| Tool call cards | 0.5 day | gpui-component Collapsible |
| Drag-resize | 0.5 day | gpui-component Resizable |
| Error handling | 0.5 day | Status indicator + retry |
| **Total** | **~8.5 days** | |

---

## 10. References

- Right panel spec: `docs/superpowers/specs/2026-07-20-right-side-panel-design.md`
- Zed AI recon: `skills/zed-ai/` (10 skills)
- ACP Rust SDK: `agent-client-protocol` crate (Apache-2.0)
- gpui-component: `longbridge/gpui-component` (Apache-2.0, planned in ARCHITECTURE.md)
- ARCHITECTURE.md §4: layer-shell window patterns
- DECISIONS.log: gpui-component decision at L303
