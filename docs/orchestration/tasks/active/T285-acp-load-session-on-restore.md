# T285 — cold `load_session` должен посадить `ActiveSession`, не `create_session`

**Статус:** STOP (тупик публичного API) — см. slice B в отчёте. Гейт 8 живьём
провален 2026-08-16; бэкенд-фикс невозможен без форка/бампа/v2-wire.
**Приоритет:** P1.
**Роль:** BACKEND. Калибр: 100b.
**Зона:** `crates/services/src/hermes_acp/client.rs`
(`load_session_command`, плюс только то, без чего bind не собрать).
**Не трогать:** `chat.rs` (slice A уже в git), композер / `text_input.rs` /
T286 / T287, `Cargo.lock`, `Source/gpui/`.

Slice A `f9cd9a2` уже зовёт `load_session` из `ChatTab::new` после
`HermesClient::new`. Это **не** дыра. UI путь холодного старта живой.
Чинить клиент.

Холодная сессия исполнителя. Не переписывай chat.rs «на всякий».

## Симптом (живой лог, не гипотеза)

Бинарь правильный: `target/release/chronos` 2026-08-16 01:55 =
`~/.local/bin/chronos` (symlink). В бинаре есть строки T285.

`~/.local/state/chronos/chronos.log` ~23:06 UTC 15 / 02:06+03 16:

```
ACP client connected, resuming session 65e1ce21-b5a6-48a3-9ca8-7e73ce573e32
Sending session/load session_id=65e1ce21-… cwd_log=…/ChronOS
session/load OK; consuming replay via stream_read_turn
load_session failed: no active session for load
load_session failed, new session
ACP session started session_id=362cd7c5-…
```

Hermes `session/load` принял. Наш клиент упал. Fallback в chat.rs
(`${create_session}`) сминтил **новую** сессию. Лента SQLite старая,
агент пустой. Владелец спросил «какое слово?» — агент не знает ход.

Строка `no active session for load` — **наша**, не ответ Hermes.

## Почему

`load_session_command` (`client.rs` ~862–898):

1. Шлёт `LoadSessionRequest` — ок.
2. `session.lock().take().context("no active session for load")?`
3. На холодном старте `SharedSession` = `None`. `ActiveSession`
   появляется только в `ensure_fresh_session` / `start_new_session`
   (`cx.build_session(cwd).start_session()` = **session/new**).
4. Этот `take()` имел смысл только если в **этом же** процессе уже
   был create (клик Sessions). После рестарта шелла процесса нет.

`attach_session` в `agent-client-protocol` **2.0.0** — `pub(crate)`.
Публичного `SessionBuilder::load_session` нет. `start_session` = new.

## Задача

После холодного `HermesClient::new` + `load_session(acp_id, cwd)`:

- `SharedSession` = `Some(ActiveSession)`, id = загруженный, не новый.
- Следующий `send_prompt` идёт в эту сессию, **без** `session/new`.
- `stream_read_turn` после load — если replay нужен; на старте UI
  кэш уже нарисован (`replay_into_chat=false`). Bind важнее реплея.
- Если Hermes реально отверг load (нет сессии у агента) — `Err` как
  сейчас, chat.rs сделает fallback. Не маскировать bind-баг под это.

`take()` пустого mutex — не стратегия bind.

Как собрать `ActiveSession` после `LoadSessionResponse` — твоя работа.
`add_dynamic_handler` и `ActiveSessionHandler::new` публичны.
Поля `ActiveSession` приватны, `attach_session` — нет.

Если публичного пути нет — **стоп**, в отчёт: что искал, какие API,
почему тупик. Не форкай SDK, не бампь крейт молча, не зови
`start_session` «чтобы заполнить слот».

## Нельзя

- `create_session` / `start_session` / `session/new` до или вместо
  успешного load «чтобы был ActiveSession».
- Переписывать `chat.rs` / хелпер `connect_session_action`.
- Считать гейт 8 закрытым по unit-тестам.

## Тесты

Чистый хелпер решения, без живого Hermes:

- холодный bind: `SharedSession == None` + успешный load-response
  → слот занят **тем же** id;
- load rejected агентом → `Err`, слот пуст, id не подменён;
- регресс T288: `start_new_session` по-прежнему `.build_session(cwd)`.

Не тестировать «вызвали хелпер и сравнили с хелпером».

```
cargo test -p chronos-services --lib load_session
cargo test -p chronos --lib connect_action
```

(фильтр брифа `connect_session_action` ловит 0 тестов — имена
`connect_action_*`.)

## Live (гейт 8 — закрывает архитектор)

Release, рестарт шелла на ChronOS с живым тредом. Hermes не рестартить
между ходом и рестартом шелла, если проверяешь persist сессии агента.

Лог:

- есть `resuming session <старый-id>`
- есть `session/load OK`
- **нет** `no active session for load`
- **нет** `load_session failed, new session`
- **нет** нового `ACP session started` на этом пути
- есть `load_session replay complete` (если реплей ещё идёт)

«что я просил запомнить?» → слово из **этой** ленты.
Лента не двоится.

## Отчёт

`docs/orchestration/tasks/report/T285-acp-load-session-on-restore-report.md`
(допиши сверху slice B, старый slice A не три).

## Коммит

`fix(acp): bind ActiveSession on cold load_session (T285)`
