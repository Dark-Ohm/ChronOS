# T228 — Отчёт: виджет раскладки клавиатуры в баре

**Дата:** 2026-08-03
**Статус:** Реализован, приёмочные тесты и релизная сборка зелёные; live-прогон не сделан

## Что сделано

Сервисный слой (задача: «уже готов») и файл виджета `keyboard_layout.rs` уже лежали
в дереве (предзаготовка). Доведено до рабочего состояния то, что оставалось — регистрация
виджета в баре и включение его в список известных имён:

- `crates/app/src/bar/widgets/mod.rs` — добавлен `mod keyboard_layout;` и
  `"keyboard_layout" => Box::new(keyboard_layout::KeyboardLayoutWidget)` в `instantiate`.
- `crates/app/src/bar/layout_config.rs`:
  - `"keyboard_layout"` добавлен в `BUILTIN_NAMES`;
  - добавлен в дефолтный `right` сразу после `"network"` (системные индикаторы, рядом
    с `volume`/`clock`);
  - обновлён тест `default_matches_historical_builtin_order` под новый состав `right`.
- `crates/services/src/compositor/types.rs` — вариант `CompositorCommand::CycleKeyboardLayout`
  (уже был в дереве, часть T228).
- `crates/services/src/compositor/hyprland.rs` — `command_to_socket_line` рендерит
  `CycleKeyboardLayout` в `"switchxkblayout all next"`; `execute_command` шлёт эту строку
  сырой (не через `/dispatch`, иначе Lua-Hyprland съедает её как Lua), добавлен `send_raw`
  (уже было в дереве, часть T228).

## Поведение виджета (из готового `keyboard_layout.rs`)

- `abbreviate(name)`: `"English (US)"`→`"US"`, `"Russian"`→`"RU"`, `"Hebrew"`→`"HE"`,
  `""`→`""`, незнакомая без скобок → первые 2 буквы капсом.
- `render` читает `AppState::compositor(cx).get().keyboard_layout`, рисует pill, клик →
  `CompositorCommand::CycleKeyboardLayout`.
- `name()` = `"keyboard_layout"`, `section()` = `BarSection::Right`.

## Верификация

Команды из задачи (`cargo test -p chronos --lib ...`) неприменимы напрямую: модуль `bar`
**bin-only** (`mod bar;` в `main.rs`, не в `lib.rs`), а `compositor` живёт в
`chronos-services`. Запущено по фактическим путям:

- `cargo test -p chronos --bin chronos bar::widgets::keyboard_layout` → **6 passed**
  (`widget_name_and_section_are_stable` + 5 `abbreviate_*`).
- `cargo test -p chronos-services compositor` → **3 passed**, в т.ч.
  `command_to_socket_line_formats_every_variant` (покрывает `CycleKeyboardLayout` →
  `"switchxkblayout all next"`).
- `cargo build --release -p chronos` → **ok** (только warnings).

Примечание: бин компилируется только после временной правки импорта `WindowRootExt` в
`bar/mod.rs` — это **T227** (файл `crates/ui/src/window_root.rs` незакоммичен, трейт не
подключён). Правку импорта я НЕ коммитил (чужой полуфикс, без `window_root.rs` ломает
сборку). T228 от `window_font` не зависит — виджет его не зовёт, и lib/service-уровни
собираются и тестятся независимо. Как только T227 ляжет, бин соберётся и T228 будет
живым.

## Live (не сделан — нет дисплея/реального Hyprland в окружении)

По задаче: `bar.toml` `right` содержит `keyboard_layout` (миграция добавит автоматически
на 2-м рестарте, либо сразу через дефолт), `grim` показывает метку, клик и `Alt+Shift`
переключают раскладку, обе темы читаемы.

## Коммит

`bar : keyboard layout widget (T228)`.

Файлы коммита (только T228): `bar/widgets/keyboard_layout.rs`,
`bar/widgets/mod.rs`, `bar/layout_config.rs`,
`compositor/types.rs`, `compositor/hyprland.rs`.
