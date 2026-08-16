# T285 — restore треда: ACP `load_session`, не `create_session` — Report

---

## SLICE B — ACP backend: cold `load_session` → seat `ActiveSession` (STOP, dead-end)

**Date:** 2026-08-16
**Role:** BACKEND + ACP.
**Zone:** `crates/services/src/hermes_acp/client.rs`
(`load_session_command` / `load_session`), только то, что нужно для сборки.

### Status

**STOP (тупик публичного API), код не менён.** Сработал гейт из таска:
*«Если публичного пути нет — стоп, в отчёт: что искал, какие API, почему тупик.
Не форкай SDK, не бампь крейт молча, не зови start_session "чтобы заполнить слот".»*
Бэкенд-фикс в пределах таска невозможен; фронтенд slice A (ниже) живёт как есть —
bind на восстановленный тред уходит в fallback `create_session` и сессия не та,
но это единственное поведение, доступное без форка/бампа.

### Что искал (порядок проверки)

1. `agent-client-protocol` 2.0.0 — последняя опубликованная версия на crates.io
   (max_stable_version = newest = 2.0.0, не yanked). Бамп не из чего делать.
2. Публичный путь `LoadSessionResponse` → `ActiveSession` в 2.0.0:
   - `attach_session(NewSessionResponse)` — **`pub(crate)`** (`session.rs:80`). В
     `0.11.1` был `pub` (`0.11.1/src/session.rs:79`) — регресс при переходе на 2.0.0.
   - Поля `ActiveSession` (`session.rs:506-529`) — все приватные, публичного
     конструктора нет. Производители только `SessionBuilder::start_session` /
     `run_until` / `on_session_start` / `start_session_proxy` — все шлют `session/new`.
   - `ActiveSessionHandler` (`session.rs:730`) — **приватный struct** (без `pub`),
     его `pub fn new` (`session.rs:736`) наружу недостижим. Подсказка таска
     «ActiveSessionHandler::new публичен» для 2.0.0 неверна. В `lib.rs:135-136`
     только `pub use session::{ActiveSession, McpClient, McpServer, SessionMessage,...}`
     — хендлера там нет.
   - `ConnectionTo::add_dynamic_handler` — публичен (`jsonrpc.rs:3569`), но
     регистрирует только `HandleDispatchFrom`-хендлер и не умеет собрать
     `ActiveSession`. Сам по себе недостаточен.
   - v1 `SessionBuilder::load_session` / `resume_session` — **не существует**.
   - v2 `resume_session` — есть в `SessionBuilder` (за фичей `unstable_protocol_v2`),
     но: возвращает `V2Session`, не `ActiveSession`; требует v2-wire (переговоры
     v2 в `Client::v2`); Hermes говорит v1 (наш клиент шлёт `schema::v1::LoadSessionRequest`).
   - v1 `LoadSessionResponse` (`agent-client-protocol-schema-1.5.0/src/v1/agent.rs:1248`)
     несёт `modes` / `config_options` / `meta`, **без** `session_id` (id только в
     запросе). Даже если бы `attach_session` был публичным, конверсии
     `LoadSessionResponse` → `NewSessionResponse` нет.
3. git main rust-sdk (`zed-industries/agent-client-protocol`): `attach_session` всё ещё
   `pub(crate)`; публичного v1 load-билдера нет. Признаков будущего PR нет.

### Почему тупик

Единственный способ посадить `ActiveSession` в `SharedSession` — публично
сконструировать значение типа, у которого приватны все поля, а единственный
фабричный путь (`attach_session`) закрыт `pub(crate)`. Доступные альтернативы
нарушают ограничения таска: форк SDK, бамп крейта (некуда — 2.0.0 уже max),
`start_session` вместо load (даёт новый id — ровно баг, который чиним), v2-wire
(ломает совместимость с Hermes v1 и меняет тип слота).

### Следствие для пользователя

На холодном старте восстановленный тред не bind-ится к загруженной сессии:
`load_session_command` по-прежнему падает на `session.lock().take().context("no
active session for load")?`, fallback `create_session` даёт новый id
(`362cd7c5-…`), SQLite-лента остаётся, но агентная память пуста. Вопрос «что я
просил запомнить?» ответит Hindsight-дамп, не лента. Это блокер UX до тех пор,
пока SDK не откроет публичный load-путь (или пока не будет принято решение
форкнуть/завести upstream-issue).

### Рекомендация (вне рамок таска)

Завести upstream-issue в `agentclientprotocol/rust-sdk`: сделать
`attach_session` публичным (или добавить публичный `load_session`-билдер,
принимающий `LoadSessionResponse` + `session_id` из запроса). В 0.11.1 API был
публичным — регресс 2.0.0, просить вернуть.

---

## Slice A — FRONTEND (не трогать)

**Date:** 2026-08-16
**Role:** FRONTEND + ACP.
**Zone:** `crates/app/src/side_panel_left/tabs/chat.rs`
(`ChatTab::new` spawn + `run_load_session` extracted из `select_session`).
**Зависимость:** `23bf89f` (T288, cwd проекта) — на месте.

## Status

**Done (код).** Живой прогон гейта 8 — отдельный шаг (ниже). Спавн теперь
смотрит активный восстановленный тред и зовёт `load_session`, а не
`create_session`.

## Что сделано

1. **Чистый хелпер** `connect_session_action(restored_acp_id, cwd)`
   (`enum ConnectSessionAction { Load { acp_id, cwd }, Create }`).
   - id + непустой cwd → `Load`
   - нет id / пустой cwd / оба пусты → `Create`
2. **`ChatTab::new` спавн** после `HermesClient::new`:
   - смотрит `state.active_session_id` → активный тред → `acp_session_id` + `cwd`;
   - `Load` → `run_load_session(..., replay_into_chat=false, fallback_cwd=Some(session_cwd))`
     (кэш уже нарисован `restore_project_thread`, реплей не дублируем);
   - `Create` → как раньше `create_session` (cwd = проект).
3. **`run_load_session`** — вынесен из `select_session` общий путь реплея.
   - `replay_into_chat=false`: транскрипт не трогаем, только bind сессии
     (гасим mutation в streaming-task, не пушим placeholder).
   - `fallback_cwd=Some`: на `load_session` Err (сессия умерла у Hermes) →
     `warn "load_session failed, new session"` + `create_session(fallback)`,
     SQLite-ленту **не** стираем. `select_session` шлёт `None` → тихий Err.
4. **Дубль ленты закрыт:** в спавне `replay_into_chat = chat.messages.is_empty()`;
   на старте кэш уже в `chat.messages` → реплей не пушится.

## Verified (тесты, не со слов)

- `cargo test --lib -p chronos connect_action` → 4 ok
  (`load` / `create-no-id` / `create-empty-cwd` / `create-both-empty`).
- `cargo test --lib -p chronos side_panel_left` → 117 ok.
- `cargo build --release -p chronos` → без ошибок (только pre-existing
  `proc-macro-error2` future-incompat warning, не наш).

## Живой прогон (гейт 8 — ещё не закрыт)

Без рестарта шелла на проекте с живым тредом гейт не закрыт. Нужно:

- Лог: `load_session replay complete` / `ACP client connected, resuming session`,
  **нет** `create_session after connect` на этом пути (кроме fallback).
- «что я просил запомнить?» → слово из **этой** ленты, не дамп Hindsight.
- Лента не дублируется («баннан» один раз).

## Caveats (не блокер)

- `load_session` не отдаёт modes/models — оставляем как после connect
  (согласно брифу, не выдумываем). Композер-индикаторы на этом пути
  не обновляются из реплея; при первом промпте подтянутся.
- Тесты хелпера не гоняют «вызвали хелпер и сравнили с хелпером» — прод-спавн
  реально его дёргает.

## Commit

`fix(left-panel): load ACP session on restore, do not create_session (T285)`
