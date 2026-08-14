# T283 — закрыть дыры T280: Sessions scope + отчёт

**Статус:** ACCEPTED 2026-08-14 (дыры Sessions закрыты, сверка с деревом).
**Приоритет:** P1.
**Роль:** persistence wiring, не schema rewrite.
**Зависимость:** T280 код уже в working tree (не закоммичен). База git: `bd999a5`.
**Следующий тикет:** приёмка T280+T283 вместе; T281 не начинать.
**Параллелить нельзя** с T280 (та же зона).

Холодная сессия. Не продолжай «память» прошлого агента — он умер на лимите.
Сверяйся с деревом. Предыдущий агент **уже сделал** store v2 и bar strip;
твоя работа — два бага + отчёт, не переписать миграцию.

## Что уже в WD (архитектор сверил 2026-08-14, не переделывать)

Незакоммичено поверх `bd999a5`:

- `crates/services/src/threads.rs` — `SCHEMA_VERSION = 2`, одна `tx`,
  `migrate_v1` / `migrate_v1_to_v2`, stamp `user_version` после веток.
  `insert` = wrapper `project_path = cwd`.
  `insert_for_project` / `list_for_project` / `set_active_thread` /
  `active_thread` (фильтр `id AND project_path AND archived = 0`).
  Реальный v1 fixture (`make_v1_fixture` + close + `ThreadStore::open`).
  `cargo test -p chronos-services --lib threads` → **14/14**.
- `crates/app/src/side_panel_left/tabs/sessions.rs` — `set_project` зовёт
  `list_for_project`.
- `crates/app/src/side_panel_left/mod.rs` — `switch_project` делает
  `clear_for_project` + `restore_project_thread`.
- `crates/app/src/side_panel_left/tabs/chat.rs` — `insert_for_project`,
  `persist_active_thread`, `restore_project_thread`.
- `crates/app/src/bar/layout_config.rs` + `bar/mod.rs` — `project` нет в
  `BUILTIN_NAMES`/default; `strip_retired_project` на load; виджет
  `bar/widgets/project.rs` уже удалён в `25b1885`.
  Тесты бара живут в **`--bins`**, не `--lib`:
  `cargo test -p chronos --bins strip_retired` → 2/2.

`cargo test -p chronos side_panel_left --lib` → 90/90 на момент сверки.

## Две дыры — это весь объём кода

### 1. Remove активного проекта не сбрасывает список Sessions

`WorkspaceView::on_project_event(Remove)` зовёт
`sessions.clear_for_project`.

`SessionsTab::clear_for_project` (`tabs/sessions.rs` ~L124) только
`selected = None`. Комментарий в коде врёт: «T280 will also reload».
Список старого проекта остаётся на экране.

**Надо:** `clear_for_project` сбрасывает `project_path = None`,
`selected = None`, `threads.clear()`, `cx.notify()`.
Честный empty. Не `list()` всего store.

Тест на сам метод (без `ChatTab` / без окна): после
`clear_for_project` `threads` пуст и `selected_thread()` = `None`.
Конструируй `SessionsTab` через `WeakEntity` в `#[gpui::test]` **или**
вынеси чистый helper `fn empty_scope()` / сделай `clear_for_project`
тестируемым без store. Тест обязан звать прод-путь по имени
(`clear_for_project` или helper, который зовёт прод). Тавтология
«присвоил vec![] и проверил пусто» — отказ (урок T278).

### 2. `SessionsTab::new` без active project читает весь store

Сейчас (`sessions.rs` ~L65–67): нет `ProjectsConfig.active` →
`s.list(None, false, false)` — все треды всех проектов.
Комментарий у поля `project_path` говорит «empty is honest» — код врёт.

**Надо:** нет active project → пустой список, `project_path = None`.
Тот же helper, что в п.1. `list()` unscoped в Sessions больше не звать.

Тест: helper/new-path без project → 0 rows. Не через живой `ThreadStore`
на диске пользователя.

## Не трогать

- `migrate` / `migrate_v1` / `migrate_v1_to_v2` / fixture — уже приняты
  архитектором как форма.
- `select_session` / `chat_handle` / `load_thread` (T279) — кроме вызова
  уже существующего `restore_project_thread`.
- `bar/widgets/` — файла `project.rs` нет, не воскрешать.
- `Cargo.lock`, docs кроме отчёта T280, packaging, чужой dirty.
- T281.

Однострочный комментарий в `switch_project` («T280 will extend…») —
ложь, restore уже есть. Поправь заодно, не отдельным коммитом.

## Проверки (прогнать сам, вставить stdout в отчёт)

```bash
cargo test -p chronos-services --lib threads
cargo test -p chronos side_panel_left --lib --bins
cargo test -p chronos --bins strip_retired
cargo test -p chronos --bins workspace_mode
cargo check -p chronos --lib
rg -n 'list\(None, false, false\)' crates/app/src/side_panel_left/tabs/sessions.rs
```

Ожидается: threads 14+; left зелёный; strip_retired 2/2; workspace_mode
ассёрты на месте (не вырезаны); check 0 errors; `rg` пустой
(unscoped `list` в Sessions исчез).

`bar::layout_config` под `--lib` **не находится** — это модуль бинаря.
Не писать в отчёт «0 tests = зелёные». Команда: `--bins`.

## Коммит

Не коммить, пока архитектор не скажет. Когда скажет — **один** коммит
на весь T280+T283 (store+bar+дыры), поимённый `git add`:

```text
crates/services/src/threads.rs
crates/app/src/side_panel_left/mod.rs
crates/app/src/side_panel_left/tabs/chat.rs
crates/app/src/side_panel_left/tabs/sessions.rs
crates/app/src/side_panel_left/workspace_view.rs
crates/app/src/bar/layout_config.rs
crates/app/src/bar/mod.rs
```

Сообщение: `feat(threads): project-scoped store v2 and bar project retirement`

Не класть `Cargo.lock`, HANDOFF, DECISIONS, design HTML.
Перед коммитом `git diff --staged` глазами.

## Отчёт

`docs/orchestration/tasks/report/T280-project-scoped-threads-and-bar-migration-report.md`

(это закрытие T280; отдельный T283-report не нужен).

Зафиксировать: schema/API, proof v1 fixture (не fresh DB), bar strip +
workspace_mode fixtures, две дыры Sessions до/после, команды и exit-коды.
Не переносить в `report-log/`.

## Урок, который уже оплачен на T278/T279

Тест должен звать прод-функцию по имени. Зелёный helper, который прод
не вызывает — отказ без обсуждения. `ChatTab::new` в `TestAppContext`
не поднять (ACP). SessionsTab — можно, либо вынеси чистый scope-helper.
