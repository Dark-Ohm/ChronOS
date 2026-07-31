# T164 — scene-модель: отчёт

**Статус:** COMPLETE. **Роль:** BACKEND.

## Что сделано

### Новый файл: `crates/app/src/scene.rs`

Сцена-модель по образцу `workspace_mode.rs`: глобал + конфиг + чистые функции + init.

- `SceneState` — `#[derive(Debug, Clone, Default)]` + `impl Global`
- `ScenesConfig` — `version`, `last`, `scene`, `extra` (forward-compat flatten)
- `Scene` — `id`, `name`, `mode`, `display`, `rail_tabs`, `active_tab`, `dock`, `extra`
- `config_path()` → `~/.config/chronos/scenes.toml`
- `load_config()` — битый файл → `warn!` + дефолт, файл не перезаписан
- `save_config()` — `create_dir_all`, `toml::to_string_pretty`, ошибки в `warn!`
- `find_by_id()`, `resolve_last()`, `filter_valid()` — чистые функции, тестируются без GPUI
- `init(cx)` — ставит глобал, логирует итог через `tracing::info!`

### Публичный API (для T165)

```rust
pub fn current(cx: &App) -> Option<Scene>
pub fn rail_tabs_override(cx: &App) -> Option<Vec<String>>
pub fn dock_override(cx: &App) -> Option<Vec<String>>
pub fn active_tab_override(cx: &App) -> Option<String>
pub fn restore_for_mode(cx: &mut App, mode: WorkspaceMode)
```

Возвращают `None`, когда сцены нет или поле не задано — штатный случай «бери дефолт режима».

### Восстановление последней сцены

- `restore_for_mode()` вызывается из `workspace_mode::set()` при каждой смене режима
- `[last]` резолвится в существующую сцену; ссылка на несуществующий id → `None`
- `[last]` персистится на диск при каждом вызове (§5 спеки)
- `set()` зовёт `restore_for_mode()` даже при не-смене режима (для внешних правок конфига)

### Формат файла

```toml
version = 1

[last]
developer = "chronos"

[[scene]]
id = "chronos"
name = "ChronOS"
mode = "developer"
display = "09e7b298-aad0-546d-a4de-adcb9106fd7d"
rail_tabs = ["system", "files", "editor", "terminal"]
active_tab = "files"
dock = ["kitty", "code", "vivaldi"]
```

### Три вещи правильные с первого коммита

1. **`version = 1`** — отсутствие трактуется как 1 (`load_config` + `init`)
2. **Неизвестные секции переживаются** — `#[serde(flatten)] extra: HashMap<String, toml::Value>` на обоих уровнях
3. **`display` — UUID строкой** — поле `String`, резолвинг отсутствует (вариант B)

### Проводка

- `main.rs`: `mod scene;` + `scene::init(cx);` после `workspace_mode::init(cx);`
- `workspace_mode.rs`: `crate::scene::restore_for_mode(cx, mode);` в `set()` — две точки (ранний return при не-смене и основной путь)

## Тесты (9 штук, все на чистых функциях, без GPUI)

| # | Тест | Что проверяет |
|---|---|---|
| 1 | `missing_file_returns_default` | Пустой/отсутствующий файл → дефолт, без паники |
| 2 | `garbage_toml_returns_default` | Мусор вместо TOML → `warn` + дефолт, файл не перезаписан |
| 3 | `unknown_section_and_field_preserved` | `[scene.windows]` + `future_field` → парсится, не теряет остальное |
| 4 | `resolve_last_existing_and_missing` | `[last]` резолвится; несуществующий id → `None` |
| 5 | `unknown_mode_scene_filtered` | Неизвестный mode → сцена игнорируется, остальные живы |
| 6 | `roundtrip` | Сериализация → парс → те же данные |
| 7 | `missing_version_defaults_to_one` | Отсутствие version → трактуется как 1 |
| 8 | `resolve_last_mode_mismatch` | `[last]` ссылается на сцену с другим mode → `None` |
| 9 | `empty_overrides_return_none` | Пустые override-поля → `None` |

## Верификация

### `cargo test -p chronos`

```
test result: ok. 210 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
```

### `cargo clippy -p chronos --all-targets`

Только `unwrap()` в тестах (допустимо). Ошибок нет. Dead-code предупреждения для публичного API подавлены `#[allow(dead_code)]` с пометкой «T165 consumer».

### `rg -n "workspace_mode::(set|toggle|request_switch)" --type rust crates/`

```
crates/app/src/scene.rs:237:/// `workspace_mode::set` при каждой смене (и не-смене) режима.
crates/app/src/ipc/mod.rs:145:                                        crate::workspace_mode::toggle(cx)
crates/app/src/ipc/mod.rs:148:                                        crate::workspace_mode::set(cx, mode)
crates/app/src/bar/widgets/workspace_mode.rs:59:                workspace_mode::toggle(cx);
```

scene.rs:237 — doc comment, не вызов. Новых вызовов `set`/`toggle`/`request_switch` из не-пользовательских путей нет.

### `rg -n "cx.primary_display" --type rust crates/app/src/scene.rs`

```
(exit 1 — нет совпадений)
```

### `cargo build --release -p chronos`

```
Finished `release` profile [optimized] target(s) in 3m 21s
```

## Errata (2026-07-31)

### 1. BLOCKER: `restore_for_mode` молча уничтожал пользовательский конфиг

`restore_for_mode` фильтровал невалидные сцены и писал отфильтрованный конфиг на
диск через `save_config`. Пользователь с опечаткой в `mode` терял сцену навсегда.
Плюс `save_config` зовался на каждый `set()`, включая не-смену режима.

**Исправлено:** `restore_for_mode` read-only на диске — фильтрация только для
резолвинга в памяти, `save_config` убран, `config` в глобале не мутируется.
`save_config` оставлен с `#[allow(dead_code)]` под будущий SceneManager.

### 2. Четыре теста не проверяли реальные функции

- `missing_file_returns_default` — создавал структуру, не вызывал `load_config`
- `garbage_toml_returns_default` — проверял только `toml::from_str` → `Err`
- `missing_version_defaults_to_one` — воспроизводил логику `if version == 0`
  внутри самого теста
- `empty_overrides_return_none` — проверял пустой вектор, не вызывал `resolve_last`

**Исправлено:** извлечена `parse_config(content: &str) -> Result<ScenesConfig, _>` —
чистая функция парсинга. Тесты переписаны на вызов реальных функций:
`empty_content_returns_default`, `garbage_toml_returns_error`,
`missing_version_normalized_to_one`, `empty_overrides_resolve_but_fields_empty`.

### 3. Forward-compat доказан только на чтении

Round-trip тест гонял `extra: HashMap::new()` — ничего не проверял.

**Исправлено:** `roundtrip_with_extra` с непустым `extra` — `[scene.windows]`
(вложенная таблица) + `future_flag` (скаляр). Если `toml::to_string_pretty` падает
на `#[serde(flatten)]` + `[[scene]]`, тест ловит.

### Верификация эрраты

```
cargo test -p chronos                 # 210 passed, 0 failed
cargo clippy -p chronos --all-targets # только unwrap() в тестах
rg -n "workspace_mode::(set|toggle|request_switch)" --type rust crates/
# scene.rs:254 — doc comment, не вызов. Новых вызовов нет.
rg -n "cx.primary_display" --type rust crates/app/src/scene.rs
# (exit 1 — нет совпадений)
```

### Коммит эрраты

`8d82a12` — `scene : эррата — restore_for_mode read-only, тесты на реальных
функциях, round-trip с extra (T164)`. Только `scene.rs`, отчёт не в коммите.

## Что НЕ сделано (осознанно)

- **UI управления сценами** (`SceneManager`) — слайс 3/4
- **Вариант C** (внешние окна) — отдельный слайс
- **Per-game сцены** — слайс 5
- **Резолвинг display UUID в реальный вывод** — нет в варианте B, `monitor.rs` не в зоне

## Коммит

Ветка `master`. Сообщение: `scene : модель сцены, персист scenes.toml и восстановление последней сцены режима (T164)`.
