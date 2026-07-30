# T140 — ACP permission auto-approve (tools can run)

**Статус: OPEN. P0 после T137.**  
**Канон:** `docs/superpowers/specs/2026-07-26-acp-panel-revive-design.md`  
**Блокирует:** живые tool calls Hermes; T141 UI tools бессмысленен без этого.

| | |
|---|---|
| **Skills** | `chronos-shell`, `zed-acp-stdio` (patterns only) |
| **Зоны** | `crates/services/src/hermes_acp/transport.rs` (+ тонко client если нужно) |
| **Не трогать** | `side_panel_left` UI polish (T141), multi-agent (T138), bar |
| **Отчёт** | `docs/orchestration/tasks/report/T140-acp-permission-auto-approve-report.md` |
| **Коммит** | `acp : auto-approve session/request_permission (T140)` |

## Контекст (проверено 2026-07-26)

- T137 (`af54fb0`): chat round-trip + session reuse + `unstable_session_usage` — **чат жив**.
- Live user: Hermes tool `write_file` →  
  **`Edit approval denied by ACP client; file was not modified`**.
- YOLO UI mode (`dont_ask` / bypass) **не** шлёт ACP permission response — это
  session *mode*, не client permission handler.
- `Client.builder()` в `transport.rs` **не** регистрирует
  `on_receive_request(RequestPermissionRequest, …)`.
- Канон SDK:  
  `agent-client-protocol-0.11.1/examples/yolo_one_shot_client.rs`  
  (строки ~99–116): auto-approve = select option, prefer allow.

## Цель

Любой `session/request_permission` от агента получает **approve** (по
умолчанию для shell), пока нет UI-гейта. Tools пишут/читают файлы.

## Задачи

### Task 1 — Handler на builder (до `connect_with`)

```rust
// pattern from yolo_one_shot_client.rs
Client.builder()
    .name("chronos-shell")
    .on_receive_request(
        async move |request: RequestPermissionRequest, responder, _cx| {
            // Prefer AllowAlways, then AllowOnce, else first option
            // Log: tracing::info!(?request, "ACP permission auto-approved");
            responder.respond(RequestPermissionResponse::new(...));
        },
        agent_client_protocol::on_receive_request!(),
    )
    .connect_with(...)
```

- Imports: `RequestPermissionRequest`, `RequestPermissionResponse`,
  `RequestPermissionOutcome`, `SelectedPermissionOutcome`,
  `PermissionOptionKind` from schema.
- Если `options` пуст → `Cancelled` + warn (не silent).

### Task 2 — Политика v1 (зафиксировать в report)

| | |
|---|---|
| **v1 default** | Always auto-approve (AllowAlways > AllowOnce) |
| **Не в v1** | UI prompt per tool, per-path ACL, deny list |
| **Связь с YOLO button** | UI YOLO остаётся session mode; permission handler **независим** |

Опционально позже: `auto_approve_permissions: bool` в config — **не**
блокер T140.

### Task 3 — Verify

```bash
cargo build --release -p chronos
# restart shell, Super+A, prompt:
# "create file /tmp/chronos-acp-t140.txt with hello then delete it"
# Expect: no "Edit approval denied"; file created+deleted OR tool result in reply
# Log: "ACP permission auto-approved" (or your message)
```

Smoke unit optional: если нет hermes — manual only, честно в report.

## Accept

- [ ] Tool write/edit no longer fails with `Edit approval denied by ACP client`.
- [ ] Log line on each auto-approve.
- [ ] Empty options → Cancelled + warn, no panic.
- [ ] Report: before/after quote from live thread.

## Out of scope

- Rendering tool cards / reasoning (T141).
- Model dropdown (T142).
- Multi-agent (T138).
