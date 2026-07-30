<!-- T031 — migrated 2026-07-22 from orchestration/report-log/mimo-report-5.md — see orchestration/tasks/MIGRATION.md -->

# MIMO — отчёт: задание №5 управление обоями: IPC + циклер папки

**Статус: ВЫПОЛНЕНО**
**Коммит: `e278a58`** — `wallpaper : IPC-управление + циклер папки`

## Что сделано

### wallpaper_ctl.rs (новый модуль)
- `scan_wallpapers()` — скан `~/Pictures/Wallpapers` для изображений (jpg/jpeg/png/webp), отсортировано по имени файла
- `next(cx)` — циклический переход к следующему обою от `WallpaperState.current` (если current не из папки/None — первая)
- `set(cx, path)` — прямая установка по абсолютному пути
- Все операции через `AppState::wallpaper(cx).dispatch(WallpaperCommand { transition: Some("fade") })`

### IPC payload'ы (messages.rs)
- `wallpaper-next` — константа + `is_wallpaper_next()` + `encode_wallpaper_next()`
- `wallpaper-set:<abs-path>` — `encode_wallpaper_set()` + `parse_wallpaper_set()` + `classify_wallpaper()`
- `WallpaperIpcCmd` enum (Next, Set)
- 7 юнит-тестов на encode/parse/classify

### IPC accept loop (mod.rs + service.rs)
- `start_listener()` теперь возвращает 3-й ресивер `IpcWallpaperReceiver`
- `accept_loop` классифицирует payload через `classify_wallpaper()` → шлёт `WallpaperIpcCmd`
- `mod.rs` обрабатывает wallpaper команды в `tokio::select!`

### Wiring
- `main.rs:8` — `mod wallpaper_ctl;`
- `lib.rs` — `is_image` и `IMAGE_EXTENSIONS` ре-экспортированы из wallpaper модуля

## Верификация

### cargo test --workspace --lib --bins
```
test result: ok. 4 passed
test result: ok. 46 passed  ← (включая 7 IPC тестов)
test result: ok. 25 passed
test result: ok. 68 passed
test result: ok. 3 passed
```
**Итого: 146 тестов, все зелёные.**

### Что подтверждено деревом
- `wallpaper_ctl.rs:27-39` — scan с sorted_by file_name
- `wallpaper_ctl.rs:44-68` — next с round-robin от current
- `messages.rs` — wallpaper-next/wallpaper-set payloads + classify
- `service.rs:46-63` — start_listener возвращает 3-й ресивер
- `service.rs:131-175` — accept_loop с classify_wallpaper в ветке
- `mod.rs:33-52` — wallpaper handler в tokio::select!

## Зоны (соблюдены)
- Тронуто: `crates/app/src/wallpaper_ctl.rs` (новый), `crates/app/src/main.rs` (+1 строка mod), `crates/app/src/ipc/**` (свои payload'ы/ветки), `crates/services/src/lib.rs` (ре-экспорт is_image)
- НЕ трогать: `services/**` (кроме lib.rs ре-экспорта), `launcher/`, `bar/`, `osd/`, `tray_menu/`, `Source/`

## Предложение для пользователя
```lua
-- hyprland.lua: SUPER+W → wallpaper-next
bind = SUPER, W, exec, printf 'wallpaper-next' | socat - UNIX-CONNECT:"$XDG_RUNTIME_DIR/chronos.sock"
```

## Замечание
Директория `~/Pictures/Wallpapers` не существовала — создана вручную для верификации. В продакшене `wallpaper_ctl::scan_wallpapers()` просто вернёт пустой Vec с `warn!`.
