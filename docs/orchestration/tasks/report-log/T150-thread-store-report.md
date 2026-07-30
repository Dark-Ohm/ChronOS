# T150 — хранилище тредов: SQLite + сервис (acceptance retry)

**Приёмка:** original c688c11 архитектор отклонил с четырьмя правками.
Follow-up коммит `9429f7d` закрывает все четыре пункта + несколько
комментариев обзорщика о стиле.

**Дата:** 2026-07-29.

**HEAD:** `9429f7d` на ветке `measure/gpui-component`. История сверху —
`9d9015d docs : канон на конец 29.07...`, ниже — `4261e29 component : final
T157…`. Подробности того, почему follow-up вместо буквального
`--amend c688c11`, — в разделе «git хирургия».

**CodeReviewer verdict (последний из трёх):** «OK — коммит `9429f7d` лендит
ровно то, что было согласовано в ревью… 185/0/1 lib + 9/0 threads».

---

## Что было сделано

### Закрытые четыре acceptance-пункта (обзор 2026-07-29)

1. **`Cargo.lock` вернулся в дерево** (был утерян в c688c11). `cargo
   update -p rusqlite` зафиксировал новые версии: `rusqlite 0.32.1 →
   0.40.1`, `libsqlite3-sys 0.30.1 → 0.38.1`, `hashlink 0.9.1 → 0.12.1`.
   `rsqlite-vfs v0.1.1` и `sqlite-wasm-rs v0.5.5` появились как
   transitive-deps — см. раздел «Транзитивные зависимости».

2. **`rusqlite 0.32 → 0.40`** в `crates/services/Cargo.toml`. Пин
   `0.40.1` в lockfile, обоснование — на bleeding-edge проекте чужие
   пины не наследуются (см. docs/MEMORY.md); 8 мажоров назад — техдолг
   первого дня.

3. **`threads.rs:106`: `expect` → `ok_or_else`**: замена
   `.expect("just inserted")` на
   `.ok_or_else(|| anyhow::anyhow!("insert succeeded but row missing: {id}"))`.
   Panic → typed error path. Правило `expect_used = warn` соблюдено.

4. **ACP-команды на типизированных схемах.** Прав был обзорщик: типы
   `ListSessionsRequest`/`ListSessionsResponse` и
   `LoadSessionRequest`/`LoadSessionResponse` существуют — лежат в
   `agent_client_protocol::schema::v1`, реализованы через
   `impl_jsonrpc_request!` (`agent-client-protocol-2.0.0/src/schema/
   client_to_agent/requests.rs:15-16`). Дефолтный путь импорта через
   `schema::v1` — совпадает с уже существующими импортами
   `client_to_agent` (ContentBlock, SessionUpdate и др.).

   Локальный `pub struct SessionInfo` в `client.rs` удалён. Вместо него:
   ```rust
   pub use agent_client_protocol::schema::v1::SessionInfo;
   ```
   Полевое shape совпадает 1:1 (`session_id`, `cwd?`, `title?`,
   `updated_at?`), camelCase даёт upstream `#[serde(rename_all =
   "camelCase")]`.

### Дополнительные правки по комментариям обзорщика

- **`Command::LoadSession` обзавёлся `cwd: PathBuf`** — обязательное поле
  по `LoadSessionRequest::new(session_id, cwd)` (см. агент-схему v1
  `LoadSessionRequest::new(session_id: impl Into<SessionId>, cwd: impl
  Into<PathBuf>)`). Публичный `HermesClient::load_session(acp_session_id,
  cwd: &Path, event_tx)` теперь тоже несёт `cwd`. T151 ещё не зовёт этот
  метод напрямую; breaking допустимо.

- **Empty-cwd guard в публичном `HermesClient::load_session`** —
  отказываемся отправлять пустой путь в Hermes локально, до round-trip:
  ```rust
  if cwd.as_os_str().is_empty() {
      anyhow::bail!(
          "load_session: cwd is empty — refusing to send empty path \
           to ACP (thread {acp_session_id} has no cwd yet)"
      );
  }
  ```
  Новые треды в store имеют пустой `cwd` до первого захода; раньше это
  дошло бы до runtime-ошибки внутри Hermes.

- **`hermes_acp::SessionInfo`** теперь ре-экспортируется через
  `crates/services/src/hermes_acp/mod.rs` — T151 импортирует тип через
  публичный путь без знаний про schema-крейт.

- **Шесть `let _ = reply.send(...)` с однострочным комментом**:
  5 в `execute_command` + 1 зеркальный в `set_model_on_active`. Текст:
  `// Receiver may have dropped — silence intentional.` Плюс 7-строчный
  блок-комментарий над `match cmd { … }` в `execute_command` с длинным
  объяснением why (cross-reference через «see function header»).

- **Drop pre-existing `use std::path::PathBuf;` в
  `crates/services/src/hermes_acp/registry.rs:3`**. Это ворнинг, который
  всплыл во время нашей работы и был flag-нут обзорщиком как
  «drop-trivially adjacent». Чужих файлов в коммит не тяну.

### Что НЕ менялось (по правилу «не переписывай существующие пути»)

- `UntypedMessage` в `set_model_on_active` — типизированного
  `SetSessionModelRequest` в `schema::v1` нет; upstream выпилил метод
  `session/set_model`. Старая пометка об этом сохранена дословно.

- D3 lock-only-for-handle паттерн в `load_session_command` — не тронут
  (нужно для D3-прохода через `stream_read_turn` после успешного
  `session/load`).

- `start_new_session`, `ensure_fresh_session`, `acp_session_meta`,
  `read_turn`, `stream_read_turn`, `send_prompt_on_active`,
  `send_prompt_streaming`, `cx.send_request(InitializeRequest::new(
  ProtocolVersion::V1)…` в `transport.rs` — не наши.

### Транзитивные зависимости

После `cargo update -p rusqlite` в `Cargo.lock` появились новые
транзитивные зависимости `rsqlite-vfs v0.1.1` и `sqlite-wasm-rs v0.5.5`.
Проверка (`cargo tree -p chronos-services -e=normal`):

```
$ cargo tree -p chronos-services -e=normal --depth=2 | grep -E 'rsqlite-vfs|sqlite-wasm-rs'
(нет вывода — оба пакета не на пути к non-wasm таргету)

$ cargo tree -p chronos-services -e=normal --invert rsqlite-vfs
warning: nothing to print.

$ cargo tree -p chronos-services -e=normal --invert sqlite-wasm-rs
warning: nothing to print.
```

То есть обе записи присутствуют в `Cargo.lock` только для
опциональных/wasm-веток rusqlite 0.40 и в non-wasm shell-билд не
линкуются. Решение обрезать их через
`rusqlite = { version = "0.40", default-features = false, features =
["bundled"] }` — отдельное; см. followups.

---

## git хирургия — почему follow-up, а не буквальный `--amend c688c11`

Архитектор просил «довесить `Cargo.lock` в тот же коммит
(`--amend`)». Буквальный `--amend c688c11` был недоступен: на момент
работы `c688c11` уже не HEAD, между ним и `9d9015d` лежат 5 коммитов
(gpui-component T156/T157 + документы, не относящиеся к T150):

```
$ git log --oneline c688c11..HEAD
9d9015d docs : канон на конец 29.07 — решение по компоненту, замер +1.74 MiB, …
4261e29 component : final T157 real Input consumer + smoke text + report
ee63c19 architect : gpui-component взят как инфраструктура IDE-панели, реверс …
14e270c component : real gpui-component Input consumer for T157 measurement
73a1793 docs : HANDOFF — состояние на вечер 29.07, новая база замера, ловушка …
```

Попытка autosquash-rebase (`git commit --fixup=c688c11` + `GIT_SEQUENCE_
EDITOR=true git rebase -i --autosquash c688c11~1`) **наткнулась на
конфликт в `Cargo.lock`** — промежуточные component-коммиты тоже правили
lock (вероятно под `gpui-component`). Попытка `git rebase --continue`
упала на отсутствии `vi` для редактора squash-сообщения
(`error: cannot run vi: No such file or directory`). Через
`git -c core.editor=true`/раздельные `GIT_EDITOR=true
GIT_SEQUENCE_EDITOR=true` можно было бы дожать — но это переписывает
историю всех 5 промежуточных коммитов ради одной правки, что хуже
прямого follow-up.

Финальный путь:

1. `git rebase --abort` — вычистил half-rebased state.
2. `git reset --mixed HEAD~1` — сбросил fixup-коммит (если бы он
   выжил), 6 файлов модифицированы.
3. `git add` поимённо для всех шести:
   - `Cargo.lock`
   - `crates/services/Cargo.toml`
   - `crates/services/src/hermes_acp/client.rs`
   - `crates/services/src/hermes_acp/mod.rs`
   - `crates/services/src/hermes_acp/registry.rs`
   - `crates/services/src/threads.rs`
4. `git diff --staged --stat` глазами:
   ```
   Cargo.lock                                 |  43 +++++--
   crates/services/Cargo.toml                 |   5 +-
   crates/services/src/hermes_acp/client.rs   | 187 ++++++++++++++++-------------
   crates/services/src/hermes_acp/mod.rs      |   2 +-
   crates/services/src/hermes_acp/registry.rs |   1 -
   crates/services/src/threads.rs             |  10 +-
   6 files changed, 151 insertions(+), 97 deletions(-)
   ```
5. `git commit -m "threads : acceptance retry — Cargo.lock + ACP typed
   requests"` (полный текст — в `git log -1`).

---

## Acceptance: команды + их вывод целиком

### `cargo build -p chronos-services`

```
$ cargo build -p chronos-services 2>&1 | tail -3

warning: `chronos-services` (lib) generated 3 warnings (run `cargo fix --lib -p chronos-services` to apply 3 suggestions)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.80s
```

3 warnings — все pre-existing в чужих модулях (`aur/mod.rs:37`,
`brightness/ddcutil.rs:15`, `mpris/mod.rs:19`); не моих правок.

### `cargo test -p chronos-services --lib` (полный прогон)

```
$ cargo test -p chronos-services --lib 2>&1 | tail -3

test result: ok. 185 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 0.56s
```

185 passed, 0 failed, 1 ignored (smoke `client_smoke.rs` —
`#[ignore = "live hermes acp; set CHRONOS_SMOKE_HERMES_ACP=1"]`).

### `cargo test -p chronos-services --lib -- threads` (целевые)

```
$ cargo test -p chronos-services --lib -- threads 2>&1 | tail -3

test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 177 filtered out; finished in 0.00s
```

9 thread tests прошли (все требования спека покрыты: «schema version is
one on fresh db», «insert get roundtrip», «update fields», «pin archive
toggle», «list filters», «delete and get none», «cache and read
transcript», «search by title», «migration is idempotent»).

### `cargo update -p rusqlite`

```
$ cargo update -p rusqlite 2>&1 | tail -6
    Locking 5 packages to latest Rust 1.97.1 compatible versions
    Updating hashlink v0.9.1 -> v0.12.1
    Updating libsqlite3-sys v0.30.1 -> v0.38.1
      Adding rsqlite-vfs v0.1.1
    Updating rusqlite v0.32.1 -> v0.40.1
      Adding sqlite-wasm-rs v0.5.5
```

### `sqlite3 ~/.local/share/chronos/threads/threads.db ".schema"`

Нечего смотреть: путь не существует (`ThreadStore::open_default` ни разу
не запускался — шелл после правок ещё не поднимался на этой машине).
Схема проверяется через `cargo test --lib -- threads::tests`
(запускают `ThreadStore::open` на временных файлах) — все тесты
прошли, миграция `user_version=1` на месте, индексы
`idx_threads_agent`/`idx_threads_pinned`/`idx_threads_updated`
создаются, как заявлено.

Когда шелл впервые поднимется и позовёт `thread_store_init`, файл
`~/.local/share/chronos/threads/threads.db` появится. Полная схема
(для отчёта архитектора):

```sql
CREATE TABLE IF NOT EXISTS threads (
    id              TEXT PRIMARY KEY,
    agent_id        TEXT NOT NULL,
    acp_session_id  TEXT,
    title           TEXT NOT NULL DEFAULT '',
    title_override  TEXT,
    cwd             TEXT NOT NULL DEFAULT '',
    last_model      TEXT,
    pinned          INTEGER NOT NULL DEFAULT 0,
    archived        INTEGER NOT NULL DEFAULT 0,
    created_at      TEXT NOT NULL,
    updated_at      TEXT NOT NULL,
    transcript_json TEXT
);
CREATE INDEX IF NOT EXISTS idx_threads_agent    ON threads(agent_id);
CREATE INDEX IF NOT EXISTS idx_threads_pinned   ON threads(pinned)  WHERE pinned = 1;
CREATE INDEX IF NOT EXISTS idx_threads_updated  ON threads(updated_at DESC);
```

С `PRAGMA user_version = 1` (`ThreadStore::SCHEMA_VERSION`).

### Live smoke (ACP-команды под живым Hermes)

**Не проверял, за архитектором** — как и для оригинального c688c11.
Принято по спеке: «Не прогонял — пиши «не проверял, за архитектором»:
принимается. Галочка на непроверенном — нет.» Запуск потребует
`CHRONOS_SMOKE_HERMES_ACP=1 cargo test -p chronos-services --
--ignored` плюс живого `hermes` на машине.

---

## Code review (три прохода)

| Проход | Когда | Что сделано | Вердикт |
|---|---|---|---|
| 1 | после первой серии str_replace | build ругнулся на `%cwd` без Display + unused `SessionInfo` import + E0277 Path→Display | 2 фикса + затем критичное замечание про `LoadSessionRequest::new` ownership |
| 2 | после borrow-фикса + пустых комментариев | 5 review items: info! стиль, comment placement, mod.rs re-export, empty-cwd guard, set_model comment | «OK to --amend» с 2 non-blocking notes (test evidence + optional `_intercepted_models` comment) |
| 3 | после drop registry.rs:3 + final commit | post-commit verification, cargo tree analysis | **OK** с non-blocking note про `rsqlite-vfs`/`sqlite-wasm-rs` (см. раздел «Транзитивные зависимости») |

---

## Verdict

Все четыре acceptance-пункта из обзора архитектора **закрыты** +
комментарии обзорщика по стилю учтены + drop-trivially adjacent fix
сделан. `cargo build` зелёный, `cargo test --lib` 185/0/1, threads
9/0. Логический коммит `9429f7d` = `c688c11` + Cargo.lock + четыре
фикса.

**Live ACP smoke — за архитектором** (как и было для c688c11 —
обзорщик явно сказал «принимаю, это за мной»). T151 (UI-сторона
с `list_sessions`/`load_session`) — естественный следующий шаг, чтобы
нагрузочный тест прошёл в живую.
