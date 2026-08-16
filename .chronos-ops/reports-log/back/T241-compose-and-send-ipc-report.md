# T241 — compose-and-send IPC: отчёт

**Дата:** 2026-08-04
**Статус:** implemented, ready for live smoke

## Что сделано

### IPC-плюмбинг (паттерн `toggle-launcher`)

| Файл | Что |
|------|-----|
| `crates/app/src/ipc/messages.rs` | `COMPOSE_AND_SEND_PREFIX`, `encode_compose_and_send()`, `parse_compose_and_send()` + 4 теста |
| `crates/app/src/ipc/service.rs` | `IpcComposeAndSendReceiver`, канал в `start_listener` + диспатч в `accept_loop` |
| `crates/app/src/ipc/mod.rs` | `tokio::select!` arm, без дебаунса (explicit команда, не toggle) |
| `crates/app/src/side_panel_left/mod.rs` | `pub fn compose_and_send(text, cx)` — открывает панель, пишет текст в composer, вызывает `send_composer()` |

### Как работает

```bash
chronos-ipc compose-and-send:"hello, what can you help with?"
```

1. `open_pinned(cx)` — панель открывается (если уже открыта — ранний return, не создаёт новое окно)
2. `handle.update` — `dock_chat = true`, `ensure_chat_width()`, `composer_input.content = text`, `send_composer(window, cx)`
3. Агент получает промпт через ACP, ответ стримится в UI

### Фикс T242 (попутно)

Та же функция `expand_with_composer` и новая `compose_and_send` теперь сбрасывают `last_resized_width = None` перед `ensure_chat_width()`. Это гарантирует, что render resize-guard всегда сделает `window.resize()`, даже если state.width уже равен цели (устраняет интермиттентный desync из T242).

### Что не сделано

- Текст сообщения логируется только на `debug!` уровне (hygiene)
- Живой smoke не проведён (нужен рестарт ChronOS + IPC-команда + grim кадра)

## Верификация

```bash
cargo check -p chronos  # зелёный
cargo test -p chronos -- ipc::messages  # 35/35 passed (включая 4 новых)
```

## Коммит

`services+ui : compose-and-send IPC for programmatic composer testing (T241)`
