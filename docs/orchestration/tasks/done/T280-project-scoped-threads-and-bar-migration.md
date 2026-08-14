# T280 — project-scoped ThreadStore v2 и удаление project pill из bar

**Статус:** ACCEPTED 2026-08-14 вместе с T283. Код: `f083779`.
**Приоритет:** P1.
**Роль:** persistence migration + bar configuration.
**Зависимость:** принятый T279.
**Следующий тикет:** T281; параллельно не выполнять.

## Канон

Выполнить только Tasks 5–6 из
`docs/superpowers/plans/2026-08-13-left-ai-workspace-slice-a.md`.

## Цель

Сделать SQLite источником восстановления последней session каждого проекта и
удалить bar project pill вместе с одноразовой миграцией `bar.toml`.

## ThreadStore v2

Добавить `ThreadRecord.project_path`, index по project/archive/update и:

```sql
CREATE TABLE workspace_project_state (
    project_path TEXT PRIMARY KEY NOT NULL,
    active_thread_id TEXT,
    FOREIGN KEY(active_thread_id) REFERENCES threads(id) ON DELETE SET NULL
);
```

Требуемые API:

```rust
pub fn insert_for_project(
    &self,
    id: &str,
    agent_id: &str,
    cwd: &str,
    project_path: &str,
) -> anyhow::Result<ThreadRecord>;
pub fn list_for_project(&self, project_path: &str, archived: bool)
    -> anyhow::Result<Vec<ThreadRecord>>;
pub fn set_active_thread(&self, project_path: &str, thread_id: Option<&str>)
    -> anyhow::Result<()>;
pub fn active_thread(&self, project_path: &str)
    -> anyhow::Result<Option<ThreadRecord>>;
```

Существующий `insert(...) -> Result<ThreadRecord>` остаётся compatibility
wrapper с `project_path = cwd`.

### Миграция обязана быть настоящей

`SCHEMA_VERSION = 2` недостаточно. `migrate()` берёт один mutable connection,
создаёт одну transaction и выполняет явные ветки:

```rust
if version < 1 { Self::migrate_v1(&tx)?; }
if version < 2 { Self::migrate_v1_to_v2(&tx)?; }
tx.pragma_update(None, "user_version", 2)?;
tx.commit()?;
```

Schema, backfill `project_path = cwd`, state table/index и user_version
коммитятся атомарно. Нельзя проштамповать 2 до выполнения v2 migration.

Migration test вручную создаёт rusqlite v1 schema/rows, ставит
`PRAGMA user_version = 1`, закрывает connection и лишь затем вызывает
`ThreadStore::open` на том же файле. Fresh DB не считается v1 fixture.

Active session проверяется одновременно по id и project path. Missing,
archived, deleted, stale или cross-project id очищается и даёт empty Chat.

## Bar migration

Удалить `project` из default, `BUILTIN_NAMES`, instantiate/catalog/grouping и
удалить `bar/widgets/project.rs`. Load migration убирает точное имя `project`
из left/center/right/known и один раз сохраняет нормализованный config.
Прочие unknown widgets не трогать.

Критично: тесты `workspace_mode` примерно на строках 671–774 используют
`project` только как ordering fixture. Переписать fixture на surviving builtin
(`tray` или `clock`) и сохранить эквивалентные predecessor/default-position
assertions. Запрещено удалять или ослаблять эти ассерты ради зелёного теста.

## TDD и проверки

До кода: real v1→v2 fixture, сохранность transcript, idempotence, два проекта,
independent active sessions, stale/archive/delete/cross-project cases;
bar project во всех sections/known/duplicates и сохранение чужого unknown.

```bash
cargo test -p chronos-services --lib threads
cargo test -p chronos side_panel_left --lib --bins
cargo test -p chronos bar::layout_config --lib
cargo test -p chronos bar --lib --bins
cargo check -p chronos --lib
```

## Запрещено

- поднимать только константу schema version;
- создавать migration test через уже мигрирующий `ThreadStore::open`;
- возвращать `()` из `insert_for_project`;
- искать active thread только по id;
- удалять workspace_mode positioning tests;
- превращать retired project в warning-producing unknown;
- начинать T281 до принятия отчёта T280.

## Отчёт

Создать
`docs/orchestration/tasks/report/T280-project-scoped-threads-and-bar-migration-report.md`.

Приложить schema/API diff, доказательство real v1 fixture, bar migration cases,
точные команды/exits и hashes commits. Не переносить в `report-log/`.

