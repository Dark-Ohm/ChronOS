# T285 — restore треда: ACP `load_session`, не `create_session`

**Приоритет:** P1 — гейт 8 T281 живьём провален.
**Роль:** FRONTEND + ACP. Зона: `crates/app/src/side_panel_left/tabs/chat.rs`
(connect-спавн в `ChatTab::new` + хвост `select_session`). Не композер,
не `text_input.rs`, не T286.
**Зависимость:** `23bf89f` в git (SoT path + `restore_project_thread`).
**Не параллелить** с T286 (тот же `chat.rs`).

## Симптом (кадр владельца, 2026-08-15)

После рестарта шелла лента из SQLite на месте (`hi` / «баннан» / memory tool).
Hermes — **новая** сессия: не знает ход, прёт Hindsight (`HERMES.md`,
Chronos-AI-IDE). «Слово запомнил» и «новая сессия» — оба правды.

## Почему (file:line)

1. `ChatTab::new` (`tabs/chat.rs` ~308–344) спавнит `HermesClient::new`,
   затем **всегда** `client.create_session()`. Пишет новый
   `state.session_id`.
2. `restore_project_thread` (~638) → `load_thread` (~582) →
   `select_session` (~651). `select_session` зовёт
   `client.load_session` (~704) **только если** клиент уже в
   `self.clients`. На старте `HashMap` пуст → ветка
   `else { thread_loading = false }` (~848): кэш нарисован, ACP нет.
3. Потом спавн (1) приносит новый session_id. Гермес пустой.

`HermesClient::load_session` уже есть
(`crates/services/src/hermes_acp/client.rs:1091`) — `session/load`,
нужны `acp_session_id` + непустой `cwd` (из `ThreadRecord`).

`23bf89f` закрыл только scope проекта. Этот тикет — вторая половина гейта 8.

## Задача

После успешного `HermesClient::new` в спавне `ChatTab::new`:

- Если у **активного** восстановленного треда есть `acp_session_id` и
  непустой `cwd` → `load_session`, **не** `create_session`.
  Проставить `state.session_id` = этот acp id (modes/models — если
  `load_session` их не отдаёт, оставить как после connect / не выдумывать).
- Иначе (нет acp id, пустой cwd, нет активного треда) → как сейчас
  `create_session`.
- `load_session` падает (сессия умерла у Hermes) → warn в лог, ленту
  SQLite **не** стирать, `create_session` только как явный fallback с
  пометкой в логе `load_session failed, new session`. Не молча.

**Дубль ленты:** кэш уже нарисован `select_session`. Replay
`load_session` снова шлёт TextChunk/Thought — если просто склеить,
«баннан» будет дважды. Если `chat.messages` уже непустой — replay
события **не** пушить в ленту (только bind сессии). Пустая лента —
оставить нынешний replay из `select_session` (клик по Sessions).

Не плодить третий connect-путь. Либо очередь «pending load» на
`select_session` после insert клиента, либо ветка в существующем спавне
`new`. Предпочтительнее: спавн смотрит активный тред после insert
клиента и вызывает тот же `load_session`, что `select_session`, без
повторной отрисовки кэша.

## Нельзя

- Композер / `text_input.rs` / пикеры (T286 / T287-A).
- `create_session` «на всякий» поверх успешного load.
- `Cargo.lock`, `Source/gpui/`, `hermes_acp` протокол без нужды
  (API `load_session` уже есть).

## Тесты

Чистый хелпер, без живого `ChatTab::new` (ACP в TestApp не встаёт):

```rust
enum ConnectSessionAction { Load { acp_id, cwd }, Create }

fn connect_session_action(
    restored_acp_id: Option<&str>,
    cwd: &str,
) -> ConnectSessionAction
```

- есть id + cwd → Load
- нет id → Create
- id есть, cwd пустой → Create (как `load_session` bail в клиенте)
- пустые оба → Create

Прод-спавн вызывает этот хелпер. Не тестировать «вызвали хелпер и
сравнили с хелпером».

## Верификация

```
cargo test -p chronos --lib connect_session_action
cargo test -p chronos --lib side_panel_left
```

Live, release, рестарт шелла на проекте с живым тредом:

- В логе: `load_session` / replay complete, **нет** `create_session`
  на этом пути (кроме fallback).
- «что я просил запомнить?» → слово из **этой** ленты, не дамп Hindsight.
- Лента не дублируется.

Без живого прогона гейт 8 не закрыт. Отчёт:
`docs/orchestration/tasks/report/T285-acp-load-session-on-restore-report.md`.

## Коммит

`fix(left-panel): load ACP session on restore, do not create_session (T285)`
