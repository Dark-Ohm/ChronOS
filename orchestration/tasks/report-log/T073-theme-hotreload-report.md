<!-- T073 — migrated 2026-07-22 from orchestration/report-log/glm-report-2.md — see orchestration/tasks/MIGRATION.md -->

# GLM Report №2 — тема из конфига + hot-reload

**Задание:** `GLM.md` №2 (2026-07-20)
**Коммит:** pending (поимённый add + `git diff --staged`)
**Статус:** ВЫПОЛНЕНО, живой смок пройден

---

## Что сделано

### 1. `crates/app/src/theme_config.rs` — новый модуль (230 строк кода + 13 тестов)

**Конфиг:**
- `ThemeConfig { scheme: Option<String> }` — serde Deserialize/Serialize.
- `config_path()` → `~/.config/chronos/theme.toml` (dirs::config_dir, XDG-совместимый).
- `load_config()` — чтение + парсинг toml. Отсутствие файла → `ThemeConfig::default()` + `tracing::debug!`. Битый toml → `ThemeConfig::default()` + `tracing::warn!`. **Файл НЕ перезаписывается молча** (per brief).

**Приоритет (per brief):**
1. `CHRONOS_THEME` env — highest. Пустая/пробельная строка → falls through.
2. `theme.toml` `scheme = "<name>"` — case-insensitive (делегирует в `Theme::select_scheme`).
3. `Theme::default()` (тёмная Mocha).

**Hot-reload:**
- `inotify` на parent dir (не файл — inotify на несуществующий файл падает, parent dir ловит CREATE).
- WatchMask: CLOSE_WRITE + MOVED_TO + CREATE + DELETE + MODIFY. Фильтр по basename `theme.toml`.
- std-тред owns blocking `Inotify::read_events_blocking` (паттерн luau/watcher.rs).
- `tokio::sync::mpsc` → GPUI `cx.spawn` trailing debounce 300 мс.
- По debounce: `resolve_active_theme()` → `cx.set_global(theme)` + `cx.refresh_windows()`.
- Env перебивает файл и при hot-reload: `resolve_theme` проверяет env первым на каждый reload.

**Wire в main.rs:**
- `mod theme_config;` добавлен.
- `chronos_ui::Theme::init(cx);` → `theme_config::init(cx);` (Theme::init остаётся как building block для тестов).

### 2. Тесты (13 штук, все зелёные)

| Тест | Что проверяет |
|---|---|
| `resolve_env_wins_over_config` | env=Default перебивает config=Light |
| `resolve_env_case_insensitive_wins_over_config` | env=LiGhT case-insensitive |
| `resolve_config_when_env_unset` | config=Light при отсутствии env |
| `resolve_config_when_env_empty` | пустой env → config |
| `resolve_default_when_both_unset` | оба пусто → Theme::default |
| `resolve_env_garbage_falls_to_default_not_config` | env мусор → default (НЕ config) |
| `resolve_config_garbage_falls_to_default` | config мусор → default |
| `resolve_config_empty_scheme_falls_to_default` | config пустое → default |
| `parse_theme_toml_with_scheme_field` | toml парсинг |
| `parse_theme_toml_empty_file` | пустой toml → default, без паники |
| `parse_theme_toml_ignores_unknown_keys` | будущие ключи не ломают чтение |
| `parse_theme_toml_invalid_does_not_panic` | битый toml → error, не panic |
| `accent_is_same_across_schemes` | `#007acc` одинаков в обеих схемах |

**Workspace тесты:** 298 passed, 0 failed (4 crates + 11 ui).

---

## Живой смок (6 сценариев, все пройдены)

| # | Сценарий | Результат |
|---|---|---|
| 1 | `theme.toml{scheme="Light"}` → grim бара | `#eceefa` (Light bg.tertiary) ✅ |
| 2 | Hot-reload: файл → `scheme = "Default"` | `#181825` (Dark bg.tertiary) ✅ |
| 3 | Hot-reload: файл → `scheme = "Light"` обратно | `#eceefa` ✅ |
| 4 | Удаление theme.toml | `#181825` (default dark) ✅ |
| 5 | `CHRONOS_THEME=light` + config=Default | `#eceefa` (env wins) ✅ |
| 6 | Hot-reload файла при env=light | `#eceefa` (env sticky) ✅ |

Тёмный шелл возвращён пользователю (без env, без theme.toml).

---

## Архитектурные решения

**Почему `crates/app/src/theme_config.rs`, а не `crates/ui/src/theme/`:**
- crates/ui — чисто stateless определения (gpui+anyhow+tracing), без IO/file/deps.
- inotify + tokio mpsc + cx.spawn + dirs + toml + serde — app-level deps.
- Паттерн: `monitor.rs`, `dock/config.rs` — аналогичные single-config loaders в crates/app.
- ui crate сохраняет архитектурную чистоту; `Theme::init`/`select_scheme` остаются building blocks.

**`cx.set_global(theme)` вместо `Theme::set(theme, cx)`:**
- `Theme::set` = `*cx.global_mut::<Theme>()` — паникует если глобал ещё не создан.
- На cold-start `Theme::init` больше не вызывается → первый `apply` должен создать глобал.
- `cx.set_global` создаёт если нет, заменяет если есть.

---

## Hex-таблица (Light C → токен → роль)

Из отчёта №1, актуально:

| Hex | Токен | Роль |
|---|---|---|
| `#dde0f2` | bg.primary | pageBg (базовый фон) |
| `#e6e9fa` | bg.secondary | cardBg (поверхность карточки) |
| `#eceefa` | bg.tertiary | cardBase (подложка/бар) |
| `#e0e3f4` | bg.elevated | hoverBg |
| `#2c2e4a` | text.primary | основной индиго-текст |
| `#5a5d80` | text.secondary | приглушённый |
| `#7d80a6` | text.muted | третичный |
| `#c4c8e6` | border.default | cardBorder |
| `#007acc` | accent.primary | акцент (НЕ переопределяется) |
| `#d20f39` | status.error | Latte red |
| `#df8e1d` | status.warning | Latte peach/yellow |
| `#40a02b` | status.success | Latte green |
| `#1e66f5` | status.info | Latte blue |

---

## Додумано (не из мокапа)

Ничего нового — все хексы идентичны отчёту №1. Модуль theme_config не добавляет визуальных значений.

## Известные хвосты

- **Клик-попапы (volume/system/updates/tray/project) в светлой ещё не проверены** — тема только на баре. Grok №16 в поле.
- **`Theme::init` стал неиспользуемым из main.rs** — но остаётся как building block и используется в тестах schemes.rs.
- **inotify watch на parent dir** — при одновременном создании нескольких theme.toml в разных подкаталогах — не проблема (watch фильтрует basename).
