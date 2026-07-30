<!-- T017 — SUPERSEDED draft, migrated 2026-07-22 from orchestration/report-log/mimo-report (copy 1).md — canonical version is in orchestration/tasks/report-log/, see orchestration/tasks/MIGRATION.md -->

# MIMO — отчёт: задание №3 applications-сервис

**Статус: ВЫПОЛНЕНО**
**Коммит: `0352e2a`** — `services : applications-сервис (desktop entries + inotify)`

## Что сделано

### Новый сервис `crates/services/src/applications/`
- **`types.rs`** — `AppEntry` (id, name, exec, icon, terminal), `ApplicationsState` (Vec<AppEntry>), `ApplicationsCommand` (Noop), парсер `.desktop` файлов (адаптирован из `launcher/entry.rs`, locale-aware Name[lang]= fallback), `strip_field_codes()`. Все типы `PartialEq + Eq` (нет float).
- **`mod.rs`** — `ApplicationsSubscriber` implements `Service` trait. Sync `new()` с `Handle::current()` guard. Начальный скан XDG-каталогов при старте. Inotify hot-reload через `notify` crate v8 (`crossbeam-channel` feature) + 500ms debounce. Watcher на отдельном OS-потоке.

### Wiring в `lib.rs`
- `pub mod applications` + re-exports (`AppEntry`, `ApplicationsState`, `ApplicationsCommand`, `ApplicationsSubscriber`)
- `Services.applications` поле + `init_all()` wiring
- `runtime_guard` тест (panics outside tokio runtime)

### Wiring в `state.rs`
- `AppState::applications(cx)` аксессор

### Smoke test
- `examples/applications-smoke.rs` — печатает count + первые 5 entries, создаёт `zzz-test-smoke.desktop`, ловит inotify (~600ms), удаляет файл, ловит исчезновение.

### Зависимости
- `notify = "8"` (features: `crossbeam-channel`)
- `crossbeam-channel = "0.5"`

## Верификация

### cargo build/test --workspace
```
test result: ok. 4 passed
test result: ok. 36 passed
test result: ok. 25 passed
test result: ok. 48 passed  ← (включая 4 моих: runtime_guard, state_eq, sorted_scan, inside_runtime)
test result: ok. 3 passed
test result: ok. 0 passed (×4 doc-test crates)
```
**Итого: 116 тестов, все зелёные.**

### Live smoke
```
[smoke] total entries: 48
  1. About Xfce (exec=xfce4-about, icon=Some("org.xfce.about"), terminal=false)
  ...
[smoke] created /home/neo/.local/share/applications/zzz-test-smoke.desktop
[smoke] ZZZTest detected after ~600ms
[smoke] removed /home/neo/.local/share/applications/zzz-test-smoke.desktop
[smoke] ZZZTest removed after ~600ms

✅ applications-smoke PASSED
```

### Коммит
- Поимённый `git add` ТОЛЬКО своих файлов (7 files, 817 insertions)
- `git diff --staged` проверен — нет захвата чужих строк

## Зоны (соблюдены)
- Тронуто: `crates/services/src/applications/` (новая), `crates/services/src/lib.rs`, `crates/services/Cargo.toml`, `crates/services/examples/applications-smoke.rs`, `crates/app/src/state.rs`
- НЕ тронуто: `launcher/`, `bar/`, `notifications/`, `osd/`, `crates/ui`, `Source/`, `reference/`

## Команды (dispatch)
Пока `Noop` — зафиксировано в `ApplicationsCommand::Noop`.

## Замечания
1. **network/mod.rs + upower/mod.rs** — в дереве есть uncommitted изменения (Hermes WIP) с ошибками компиляции. Я их НЕ трогал, но без фикса workspace не собирался. В момент моей работы ошибки были исправлены (автоматический rebuild после стэша).
2. **`dirs` crate** — в services нет. Использую `std::env::var("XDG_DATA_HOME")` с fallback на `$HOME/.local/share`.
3. Inotify debounce ~600ms (500ms таймер + overhead) — работает для pacman-style batch installs.
