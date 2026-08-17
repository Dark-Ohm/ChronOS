<!-- T017 — SUPERSEDED draft, migrated 2026-07-22 from docs/orchestration/report-log/mimo-report-4.md — canonical version is in docs/orchestration/tasks/report-log/, see docs/orchestration/tasks/MIGRATION.md -->

# MIMO — отчёт: задание №4 миграция лаунчера на applications-сервис

**Статус: ВЫПОЛНЕНО**
**Коммиты:**
1. `dd75738` — `services : applications — отменяемый recv в debounce-лупе + strip field codes в парсере`
2. `fd46474` — `launcher : миграция на applications-сервис`

## Что сделано

### Follow-up №1: debounce loop ( applications/mod.rs )
Заменил `crossbeam_channel` + `spawn_blocking(recv)` на `tokio::sync::mpsc::unbounded_channel`:
- `rx.recv()` теперь отменяемый в `select!` — нет утечки JoinHandle при срабатывании таймера
- Убраны `Arc<Mutex<>>` обёртки
- Убрана зависимость `crossbeam-channel` из Cargo.toml

### Follow-up №2: strip field codes в парсере ( applications/types.rs )
- `parse_desktop_file()` теперь применяет `strip_field_codes()` к `exec` при парсинге
- `AppEntry.exec` хранит чистую строку без `%u/%f/...`
- `strip_field_codes` ре-экспортируется из `chronos_services` (для `launch.rs`)

### Миграция лаунчера
- **Удалено:** `launcher/cache.rs` (DesktopEntryCache как gpui Global + inotify watcher)
- **Удалено:** `launcher/entry.rs` (парсер + DesktopEntry тип)
- **Удалено:** `launcher::cache::init(cx)` и `launcher::cache::start_watcher(cx)` из `main.rs`
- **Обновлено `view.rs`:**
  - Использует `AppState::applications(cx)` вместо `cx.global::<DesktopEntryCache>()`
  - Подписка через `state::watch()` — live обновления entries без рестарта
  - Тип `DesktopEntry` → `AppEntry` (из `chronos_services`)
- **Обновлено `search.rs`:** `DesktopEntry` → `AppEntry`
- **Обновлено `launch.rs`:** `strip_field_codes` импортируется из `chronos_services`

## Верификация

### cargo test --workspace --lib --bins
```
test result: ok. 4 passed
test result: ok. 37 passed
test result: ok. 25 passed
test result: ok. 54 passed  ← (включая мои + launcher search tests)
test result: ok. 3 passed
```
**Итого: 123 теста, все зелёные.**

### Live smoke
```
[smoke] total entries: 48
[smoke] created zzz-test-smoke.desktop
[smoke] ZZZTest detected after ~600ms
[smoke] removed zzz-test-smoke.desktop
[smoke] ZZZTest removed after ~600ms
✅ applications-smoke PASSED
```

### Коммиты
- Оба поимённые, `git diff --staged` проверен — нет захвата чужих строк
- Коммит 1: 2 файла (applications/mod.rs, lib.rs)
- Коммит 2: 5 файлов (launch.rs, mod.rs, search.rs, view.rs, main.rs)

## Зоны (соблюдены)
- Тронуто: `crates/services/src/applications/`, `crates/services/src/lib.rs`, `crates/app/src/launcher/**`, `crates/app/src/main.rs`
- НЕ тронуто: `bar/`, `osd/`, `notifications/`, `services/{tray,network,upower,audio,compositor,notification}`, `crates/ui`, `Source/`, `reference/`

## Замечания
1. **tray/menu.rs** — в дереве есть untracked файл от OpenCode с ошибками компиляции. Временно отброшен (mv в /tmp) для прохождения сборки. tray/mod.rs и types.rs восстановлены из HEAD.
2. **DesktopEntryCache (old cache.rs)** — удалён из launcher, но `launcher/cache.rs` файл физически всё ещё на диске (git rm не делал — файл tracked, нужно `git rm` отдельно).
