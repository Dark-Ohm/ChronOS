# T258 — desktop-terminal: настоящий $HOME-шелл + тема из kitty.conf

**Статус:** DONE
**Дата:** 2026-08-05
**Коммит:** `services+ui : desktop-terminal real $HOME shell + kitty.conf theme (T258)`

## Что сделано

### 1. Убрана ZDOTDIR-песочница

`crates/services/src/terminal/mod.rs`:
- Удалена `const ZDOTDIR` (ранее L33)
- Удалена функция `ensure_empty_zdotdir` (ранее L41-46)
- Удалён вызов `cmd.env("ZDOTDIR", ZDOTDIR)` и `ensure_empty_zdotdir(ZDOTDIR)` в `launch()` (ранее L166-172)
- Удалён юнит-тест `ensure_zdotdir_creates_zshrc_idempotently` (ранее L531-543)
- Комментарий "No login shell (-l) so the p10k/oh-my-zsh prompt noise does not drown the grid" тоже убран — он больше не отражает реальность (теперь шелл запускается с нормальным промптом)

Шелл теперь наследует `$HOME`/`$ZDOTDIR` из родительского окружения. `portable_pty::CommandBuilder` по умолчанию не сбрасывает окружение, поэтому никаких дополнительных действий не требуется.

### 2. Kitty theme parser

Новый модуль `crates/services/src/terminal/kitty_theme.rs`:

- `KittyTheme` — GPUI-независимая структура с `Option`-полями (fallback на дефолты вью)
- `Rgba8` — 8-битный RGBA без зависимости от GPUI
- `load(path)` — построчный парсинг `key value`, пропуск `#`-комментариев и пустых строк
- `include <path>` — резолв относительно директории текущего файла, максимум 1 уровень рекурсии (`depth >= 1` → скип)
- Поддерживаемые ключи: `font_family`, `font_size`, `background_opacity`, `foreground`, `background`, `color0`..`color15`
- Цвета — только hex (`#rrggbb` / `#rrggbbaa`), именованные X11-цвета не парсятся (задокументировано как известное ограничение)
- `background_opacity` клиппится в `[0.0, 1.0]`
- Отсутствующий файл → `KittyTheme::default()` (все `None`)
- Декодер hex написан вручную (без `hex`-крэйта — зависимость на 6 строк не оправдана)

### 3. Применение темы в рендере

`crates/app/src/desktop_terminal/view.rs`:
- `KittyTheme` загружается один раз в `DesktopTerminalView::new()` (из `~/.config/kitty/kitty.conf`, если есть)
- `font_size` → `text_size()` вместо хардкод-константы
- `font_family` → `font_family()` вместо `theme.font_mono`
- `foreground`/`background` → заменяют `theme.text.primary`/`theme.bg.primary`
- `background_opacity` → `.opacity()` на контейнере грида
- `cell_w`/`cell_h` масштабируются пропорционально `font_size` (с сохранением соотношений дефолта 13.0→8.0/16.0)

### 4. Побочный фикс (T257-артефакт)

`crates/services/src/lib.rs:40-43`: исправлен путь импорта `TerminalHandle`/`TerminalRegistry` — они живут в `terminal::registry`, а не в `terminal` напрямую. Без этого сборка была невозможна.

## Верификация

- `cargo build --release -p chronos-services` — чисто (5 предупреждений, все от T257: dead_code `DummyMaster`/`DummyChild`, unused imports в других модулях)
- `cargo test -p chronos-services --lib -- terminal` — 20 passed, 0 failed
  - `kitty_theme::tests::load_parses_full_file` — OK
  - `kitty_theme::tests::load_resolves_include` — OK
  - `kitty_theme::tests::load_missing_file_returns_default` — OK
  - `kitty_theme::tests::load_ignores_unknown_keys` — OK
  - `kitty_theme::tests::named_colors_not_parsed` — OK
  - `kitty_theme::tests::opacity_clamped_to_0_1` — OK
  - Остальные 14 тестов терминального движка (compute_grid, VT parser, resize, dummy session, live spawn smoke) — OK

Живой прогон (реальный шелл + kitty-тема) — out of scope для текущей сессии, требует ручного запуска в Hyprland.

## Зона файлов

- `crates/services/src/terminal/mod.rs` — удаление ZDOTDIR-кода
- `crates/services/src/terminal/kitty_theme.rs` — новый модуль (+ `pub mod kitty_theme;` в mod.rs)
- `crates/app/src/desktop_terminal/view.rs` — применение темы к рендеру
- `crates/services/src/lib.rs` — фикс импорта TerminalHandle/TerminalRegistry

**Не трогал:** `desktop_terminal/{mod.rs,config.rs}` (T257), Edit Mode/drag/resize (T259), `side_panel_right/tab/terminal.rs` (получает эффект автоматически).
