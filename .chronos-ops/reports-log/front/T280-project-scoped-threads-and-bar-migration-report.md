# T280+T283 — ПРИНЯТ 2026-08-14

Архитектор: store v2, real v1 fixture, bar strip, Sessions empty-scope. Live = T281.

# T280 — project-scoped ThreadStore v2 + bar project retirement (закрыто T283)

**Закрывает:** T280 (store v2 + bar) + T283 (дыры Sessions scope) одним тикетом.
**База git:** `bd999a5`. Незакоммичено — коммит по слову архитектора.
**Live UX (реальный клик по сессии на Hyprland):** не проверялось, gate T281.

## ThreadStore v2 — schema / API diff

`SCHEMA_VERSION = 2` (`crates/services/src/threads.rs:37`). Миграция — одна
transaction, явные ветки, `user_version` штампуется **только после** того, как
ветка v2 реально выполнилась (`threads.rs:75–88`):

```sql
-- migrate_v1_to_v2 (threads.rs:116–129)
ALTER TABLE threads ADD COLUMN project_path TEXT;
UPDATE threads SET project_path = cwd WHERE project_path IS NULL;  -- backfill
CREATE INDEX idx_threads_project_updated
    ON threads(project_path, archived, updated_at DESC);
CREATE TABLE workspace_project_state (
    project_path TEXT PRIMARY KEY NOT NULL,
    active_thread_id TEXT,
    FOREIGN KEY(active_thread_id) REFERENCES threads(id) ON DELETE SET NULL
);
```

Новые API (`threads.rs:145–257`):

```rust
pub fn insert_for_project(&self, id, agent_id, cwd, project_path) -> Result<ThreadRecord>;
pub fn list_for_project(&self, project_path: &str, include_archived: bool) -> Result<Vec<ThreadRecord>>;
pub fn set_active_thread(&self, project_path: &str, thread_id: Option<&str>) -> Result<()>;
pub fn active_thread(&self, project_path: &str) -> Result<Option<ThreadRecord>>;
```

- `insert(id, agent, cwd)` — compatibility wrapper: `insert_for_project(..., cwd)` (`threads.rs:137`).
- `active_thread` валидирует **id И project_path** (`WHERE id = ?1 AND project_path = ?2 AND archived = 0`, `threads.rs:250–256`). Missing / archived / deleted / stale / cross-project id → `None` → пустой Chat, не чужой проект. Stale-кейс доказан физически вставленной ghost-строкой через `PRAGMA foreign_keys=OFF` (`threads.rs:498–546`).

## Доказательство real v1 fixture (не fresh DB)

`make_v1_fixture` (`threads.rs:359–391`) вручную строит БД голым rusqlite:
v1-схема (без `project_path`), две v1-строки, `PRAGMA user_version = 1`,
sanity-ассерт `SELECT project_path` **падает** («это правда v1»), connection
закрыт, и только потом `ThreadStore::open` на том же файле.

`migration_v1_to_v2_real_fixture` (`threads.rs:394–441`) доказывает:
- `user_version == 2` только после миграции;
- backfill: `v1-a → /home/neo/alpha`, `v1-b → /home/neo/beta` (из cwd);
- v1-данные переживают: pinned, archived, title, и transcript
  `["hello alpha"]` байт-в-байт;
- v2-объекты на месте: `idx_threads_project_updated` и `workspace_project_state`.
- `migration_is_idempotent` (`threads.rs:609–618`): повторный open — no-op,
  версия 2, данные читаются.

## Bar: retired `project` pill

- `project` убран из `BUILTIN_NAMES` (`layout_config.rs:21`) и default.
  Файл виджета `bar/widgets/project.rs` уже удалён в `25b1885`.
- `strip_retired_project` (`layout_config.rs:290–300`) вычищает `project`
  из left/center/right и `known`, возвращает `true`, чтобы `load` сохранил
  нормализованный конфиг один раз. Вызывается ДО `sanitized()` — retired имя
  не всплывает как warning-unknown.
- Тесты (в **`--bins`**, модуль бинаря):
  - `strip_retired_project_removes_from_all_sections_and_known` — все секции +
    known + дубликат `project` в right;
  - `strip_retired_project_nops_when_absent_and_keeps_unknown` — второй
    проход no-op, чужой `custom_plugin` цел;
  - `retired_project_never_reaches_sanitized_whitelist`.
- workspace_mode positioning fixtures переведены с retired `project` на
  выжившие builtin `tray`/`workspace_mode`, ассерты predecessor-якоря и
  default-позиции сохранены (не ослаблены): `migration_anchors_on_predecessor_not_successor`
  (`layout_config.rs:700`), `migration_adds_new_widget_at_default_pos`
  (`layout_config.rs:778`), `migration_reaches_phase_two_on_second_pass`.

## Две дыры Sessions — до/после

### Дыра 1: Remove активного проекта не сбрасывал список

**До:** `SessionsTab::clear_for_project` (`tabs/sessions.rs`) делал только
`selected = None`; комментарий врал «T280 will also reload». Список старого
проекта оставался на экране. `WorkspaceView::on_project_event(Remove)` звал
`clear_for_project`, но тот не чистил ни `threads`, ни `project_path`.

**После:**

```rust
pub fn clear_for_project(&mut self, cx: &mut Context<Self>) {
    self.empty_scope();
    cx.notify();
}
fn empty_scope(&mut self) {
    self.project_path = None;
    self.selected = None;
    self.threads.clear();
}
```

Честный empty, без `list()` всего стора. `set_project` перезагружает для
следующего проекта.

### Дыра 2: `SessionsTab::new` без active project читал весь store

**До:** нет `ProjectsConfig.active` → `s.list(None, false, false)` — все треды
всех проектов; комментарий у поля `project_path` («empty is honest») врал.

**После:** `new` делегирует в тестируемый `with_active_project(coordinator,
active)` (`tabs/sessions.rs:71`). `None` → пустой scope, store **не открывается**.
Unscoped `list(None, false, false)` из Sessions исчез (см. `rg` ниже).

### Заодно

Однострочная ложь в `switch_project` («T280 will extend…») поправлена —
`restore_project_thread` уже вызван ниже по коду (`side_panel_left/mod.rs:693`).

### Тесты дыр (зовут прод-путь по имени, урок T278)

- `clear_for_project_resets_scope` — `#[gpui::test]`, живой entity через
  `WeakEntity::<WorkspaceView>::new_invalid()`, зовёт настоящий
  `clear_for_project`, ассертит пустые `threads` + `selected_thread() == None`
  + `project_path == None`.
- `new_without_project_loads_empty_scope` — `with_active_project(_, None)` →
  0 rows, без живого `ThreadStore` на диске пользователя.
- `no_unscoped_list_in_sessions` — source-контракт: `list(None, false, false)`
  не вернётся в файл, `new` обязан делегировать в `with_active_project`.

## Команды и exit-коды

```text
cargo check -p chronos --lib                                      → 0 errors
cargo test -p chronos-services --lib threads                      → 14/14 ok
cargo test -p chronos side_panel_left --lib --bins                → 93 lib ok (90 до T283 + 3 новых)
cargo test -p chronos --bins strip_retired                        → 2/2 ok
cargo test -p chronos --bins workspace_mode                       → 15/15 ok (ассерты на месте)
rg -n 'list\(None, false, false\)' crates/app/src/side_panel_left/tabs/sessions.rs → пусто (exit 1)
```

Замечание: `bar::layout_config` под `--lib` не находится — это модуль бинаря,
поэтому компиляция/тесты бара идут через `--bins`.

## Не проверялось живьём

- Реальный клик по сессии на живом Hyprland грузит транскрипт в Chat.
- Смена/удаление проекта на живом экране: Sessions пустеет, highlight гаснет.
- Bar hot-reload после strip `project` из конфига на диске.

## Коммит

Один коммит на T280+T283 по слову архитектора, поимённый `git add`
(store + app side_panel + bar), без `Cargo.lock`/HANDOFF/DECISIONS/design HTML.
