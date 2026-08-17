# T176 — отчёт: вкладка Files

**Исполнитель:** FRONTEND (Grok). **Коммит:** `1567065`.

## Выбор модуля листинга

`crates/services/src/files/` — сервисный слой без GPUI, рядом с
applications/udisks; UI только потребляет. Не `crates/app/src/files/`.

Ошибка: `std::io::Result` (без thiserror/anyhow-обёртки) — достаточно.

## Порт

| файл | источник |
|------|----------|
| `files/listing.rs` | Chronos-FM `fs/listing.rs` (+ поле `total`) |
| `files/entry.rs` | Chronos-FM `FileEntryDto` |
| `files/sort.rs` | Chronos-FM `explorer/entries.rs` |

`ops.rs` / `trash` — **не** переносились (§4.1).

Листинг: `cx.background_spawn` → apply на entity (generation guard).

## UI

- `tab/files.rs` — `FilesTab` как `SystemTab`
- `tab/mod.rs` — `TabContent::Files`
- 440 px, одна колонка, path text, `..`, reload
- **без VirtualList** — plain `overflow_y_scroll` (ChronOS repo listing
  плавный; 1000 записей — scroll DOM-элементов, не упёрлись)

## Тесты

- services: listing + sort (12)
- FilesTab gpui: settle, error path, truncated
- `cargo test -p chronos` → **268 passed**

## Живой прогон

- release, log `/tmp/chronos-t176-evidence/chronos.log`
- **0 panicked**
- panel `side_panel_right` **440×1410** at x=2120 (`hyprctl layers`)
- Files: cwd ChronOS, dirs first, sizes; nav `.cline` → `..` back
- кадры: `files-docked-zoom.png`, `files-nav2-zoom.png`, `files-up-zoom.png`
- `/root` UI: unit-тест `files_tab_error_on_missing_path` (Cannot read)

## view.rs

Минимальные match-arms `TabContent::Files` (иначе enum неexhaust).
Ширину/T174 не трогал.
