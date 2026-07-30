<!-- T017 — migrated 2026-07-22 from docs/orchestration/report-log/mimo-report-4-rework.md — see docs/orchestration/tasks/MIGRATION.md -->

# MIMO — отчёт: доработка №4 — mpsc, strip, notify

**Статус: ВЫПОЛНЕНО**
**Коммит: `acad3b3`** — `applications/launcher : доработка №4 — mpsc, strip, notify`

## Что исправлено

### 1. Debounce loop: crossbeam → tokio::sync::mpsc (applications/mod.rs)
- Заменил `crossbeam_channel::unbounded` + `Arc<Mutex<>>` + `spawn_blocking(recv)` на `mpsc::unbounded_channel`
- `rx.recv()` теперь отменяемый в `select!` — нет утечки JoinHandle
- Убрана зависимость `crossbeam-channel` из Cargo.toml
- Убран `crossbeam-channel` feature из `notify`

### 2. strip_field_codes в парсере (applications/types.rs)
- `parse_desktop_file()` теперь применяет `strip_field_codes()` к `exec` при парсинге
- `AppEntry.exec` хранит чистую строку без `%u/%f/...`
- Добавлен тест `parse_strips_field_codes_from_exec`

### 3. cx.notify() в watch handler (launcher/view.rs)
- Watch handler теперь вызывает `cx.notify()` после `refresh_results()`
- UI перерисовывается при live-изменениях entries (hot-add при открытом лаунчере)

## Верификация

### cargo test --workspace --lib --bins
```
test result: ok. 4 passed
test result: ok. 37 passed
test result: ok. 25 passed
test result: ok. 58 passed  ← (включая новый тест strip_field_codes)
test result: ok. 3 passed
```
**Итого: 127 тестов, все зелёные.**

### Live smoke
```
[smoke] total entries: 48
  2. AnyDesk (exec=/usr/bin/anydesk, ...)  ← без %u — strip работает
[smoke] ZZZTest detected after ~600ms
[smoke] ZZZTest removed after ~600ms
✅ applications-smoke PASSED
```

### Что подтверждено деревом
- `applications/mod.rs:139` — `mpsc::unbounded_channel` (НЕ crossbeam)
- `applications/mod.rs:191` — `rx.recv()` напрямую (НЕ spawn_blocking)
- `applications/types.rs:122` — `strip_field_codes(&exec?)` (НЕ `exec?`)
- `Cargo.toml` — нет `crossbeam-channel`, `notify = "8"` без feature
- `view.rs:47` — `cx.notify()` в watch handler

## Зоны (соблюдены)
- Тронуто: `crates/services/src/applications/{mod,types}.rs`, `crates/services/Cargo.toml`, `crates/app/src/launcher/view.rs`
- НЕ тронуто: `launcher/cache.rs` (уже удалён), `launcher/entry.rs` (уже удалён), `bar/`, `osd/`, `notifications/`, `services/{tray,...}`, `crates/ui`, `Source/`
