# T110 — Hot-reload bake-off, Track A (hot-lib-reloader) — Отчёт

**Агент:** OpenCode
**Дата:** 2026-07-24
**Ветка:** `spike/hot-reload-track-a`
**Ворктри:** `../ChronOS-wt-hotreload-a`

## Что сделано

1. Новый крейт `crates/hotview` (`crate-type = ["cdylib", "rlib"]`) со своей таблицей линтов (workspace lints не наследуются — `unsafe_code = "deny"` не применяется).
2. Render-функция `network.rs` вынесена в `crates/hotview` как чистая функция: принимает `(&str, &str, Hsla, Hsla, &Theme)`, возвращает `AnyElement`. Без `cx.subscribe`/`cx.observe`/владения состоянием.
3. В `crates/app` подключено через `hot_lib_reloader::hot_module!` под `#[cfg(feature = "hot-reload")]`. В release-профиле хот-релоад путь не собирается.
4. Dev-цикл: `cargo watch --delay 0 -w crates/hotview` пересобирает крейт, `hot-lib-reloader` подхватывает `.so`.

## Конфигурация

- **hot-lib-reloader** v0.8.2
- **Cargo-фича** `hot-reload` в `crates/app/Cargo.toml` (опциональные зависимости: `hot-lib-reloader`, `chronos-hotview`)
- **lib_dir**: `concat!(env!("CARGO_MANIFEST_DIR"), "/../../target/debug")` — путь до workspace-level target
- **dylib name**: `chronos_hotview` (Rust конвертирует дефис в underscore)
- **Файл**: `hot_functions_from_file!("crates/hotview/src/lib.rs")` — путь от workspace root

## Протокол 10 правок

| # | Описание | Время (сек) | Краш | Примечание |
|---|----------|-------------|------|------------|
| 1 | gap 4→8 px | ~2 | нет | `cargo watch --delay 0` |
| 2 | `↓` → `DL` | ~2 | нет | |
| 3 | Добавлена ветка `high_speed` (жёлтый dot >1 MB/s) | ~2 | нет | Требовал пересборки основного бинарника (смена API) |
| 4 | Upload строка серая при нуле | ~2 | нет | Только hotview пересобран |
| 5 | Намеренная ошибка компиляции | — | нет | **Виджет не пал, не пустел**. Hot-lib-reloader держит последнюю рабочую .so |
| 6 | Исправление ошибки | ~2 | нет | Автоподхват без рестарта |
| 7 | Dot 6→10 px | ~2 | нет | |
| 8 | DL шрифт mono→UI | ~2 | нет | |
| 9 | Padding 4px | ~2 | нет | |
| 10 | Порядок строк (upload сверху) | ~2 | нет | |

**Итого:** 0 крашей за 10 правок. Среднее время «сохранил → перезагрузка»: **~2 сек** (`cargo watch --delay 0` + компиляция hotview ~1 сек + inotify). Дефолтный дебаунс cargo-watch (500ms) давал ~8 сек; `--delay 0` сократил до ~2 сек.

## Найденные проблемы

1. **Имя dylib**: `hot_module!(dylib = "chronos-hotview")` не работает — Rust создаёт `libchronos_hotview.so` (underscore). Нужно `dylib = "chronos_hotview"`.
2. **lib_dir**: по умолчанию hot-lib-reloader ищет в `$CARGO_MANIFEST_DIR/target/debug/`, что для crates/app = `crates/app/target/debug/`. Workspace-level target位于 `../../target/debug/`. Нужен явный `lib_dir`.
3. **hot_functions_from_file!**: путь от workspace root, не от файла с макросом.
4. **Смена API**: при добавлении параметра в render_network required rebuild основного бинарника (не только dylib). Hot-reload подхватывает только dylib.
5. **cargo-watch debounce**: дефолтный 500ms добавлял ~6 сек кreload. `--delay 0` решает проблему — итого ~2 сек.

## Итоговая оценка стабильности

**Высокая.** За 10 правок — 0 крашей, 0 зависаний, 0 некорректной отрисовки. Namеренная ошибка компиляции (п.5) корректно обработана — виджет остался на последней рабочей версии. Dev-цикл предсказуемый и стабильный.

## Коммиты

- `ea65be5` — scaffolding: hotview crate + hot-lib-reloader integration
- `d0075ff` — clean up after 10-edit protocol

## Рекомендация

Track A готов к промоуту в `master` как dev-инструмент под `#[cfg(feature = "hot-reload")]`. Критичные находки (имя dylib, lib_dir, путь в hot_functions_from_file) должны быть задокументированы в `skills/chronos-shell/`.
