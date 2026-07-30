# T137 — ACP chat must work (Phase A)

**Статус: ACCEPTED 2026-07-26 (user live chat). Follow-ups T140+.**  
**Канон:** `docs/superpowers/specs/2026-07-26-acp-panel-revive-design.md`  
**Не:** multi-agent add (T138), visual identity pass (T139), Files tab, T135/T136.

| | |
|---|---|
| **Skills** | `chronos-shell`, `zed-acp-stdio`, `zed-thread-view` (patterns only), `wayland-window-lifecycle` if window churn |
| **Зоны** | `crates/services/src/hermes_acp/**`, `crates/app/src/side_panel_left/{mod,composer,chat_view,state,panel}.rs` |
| **Отчёт** | `orchestration/tasks/report/T137-acp-chat-must-work-report.md` |
| **Коммит** | `acp : stable session + send path for left panel (T137)` |

## Цель

Пользователь открывает left panel → видит composer → шлёт сообщение →
получает ответ в thread → **второе** сообщение в **той же** ACP-сессии.
Disconnect/errors видимы. Super+A не оставляет «пустой рельс без чата».

## Контекст (не переизобретать)

- Connect often works: log `side_panel_left: ACP client connected`.
- **Bug:** `HermesClient::send_prompt` creates **new** ACP session every
  call (`client.rs` “stateless”). UI sessions are local UUIDs only.
- **Bug:** `Hermes ACP command channel closed` after spawn in logs —
  send dies with `command channel closed`.
- Super+A opens `SIDEBAR_MIN_WIDTH` only — chat hidden until drag/dock.
- Composer path: `composer.rs` `send_composer` → `client.send_prompt`.

## Задачи

### Task 1 — Live repro + log contract

- Open panel, expand chat, send `"ping"`.
- Capture: `composer: send`, transport errors, hermes exit.
- Write 5-line root-cause in report **before** large refactors.

### Task 2 — One ACP session per UI thread

- Hold live session (or session_id + reconnect policy) on client after
  first `create_session` / connect.
- `send_prompt` **must not** always `start_session` from zero if a live
  session exists (unless agent requires — document if so).
- Map UI `SessionItem` ↔ ACP session: create ACP session on “New”,
  switch reuses held client sessions where possible.
- Clear chat buffer on new UI session; keep history per session in memory
  for v1 (persist later).

### Task 3 — Transport keep-alive + UI status

- Find why command loop exits; fix or respawn with status
  `Disconnected` + message in chat.
- Never leave Thinking forever on failure.
- Status dot reflects Connected / Thinking / Disconnected truthfully.

### Task 4 — Open UX

- Super+A / toggle pinned: open to width that shows chat column
  (default ~ last width or `~420` + sidebar) **or** auto `dock_chat`
  with exclusive — pick one, document.
- Focus composer when chat becomes visible.

### Task 5 — Verify + report

```bash
cargo check -p chronos
# live:
# Super+A → composer visible
# send → reply
# send again → same session (log session_id unchanged or agent remembers)
# kill hermes mid-flight → error in thread + Disconnected
```

## Accept

- [ ] Round-trip message works live (user or grim+log).
- [ ] Second message same session.
- [ ] Failure path shows Error in thread.
- [ ] Super+A shows chat without secret resize ritual.
- [ ] Report honest about remaining stream/tools gaps.

## Out of scope

Second agent registry (T138). Visual redesign (T139). Streaming tokens
optional stretch if round-trip green first.
