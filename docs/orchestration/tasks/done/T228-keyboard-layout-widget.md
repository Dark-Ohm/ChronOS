# T228 — Виджет раскладки клавиатуры в баре

**Статус:** active, не начата
**Источник:** прямой запрос пользователя, 2026-08-03
**Приоритет:** P2 — маленький, изолированный, не блокирует другие ветки

## Контекст (уже готово — не переделывать)

Сервисный слой **полностью** готов, писать его не нужно:

- `crates/services/src/compositor/types.rs:66` — `CompositorState.keyboard_layout:
  String` уже есть как поле.
- `crates/services/src/compositor/hyprland.rs:239-242` — живой IPC-обработчик
  `add_layout_changed_handler` уже пишет `s.keyboard_layout = evt.layout_name`
  при каждой смене раскладки.
- `crates/services/src/compositor/hyprland.rs:156-170` — `fetch_full_state()`
  тоже заполняет `keyboard_layout` при старте (через `Devices::get()`), так что
  после запуска шелла поле не пустое даже до первого события.
- Раскладки в системе (`~/.config/hypr/modules/15-input.lua:7`):
  `kb_layout = "us,ru,il"`, переключение — `Alt+Shift`
  (`kb_options = "grp:alt_shift_toggle"`).

**Живой формат строки** (проверено `hyprctl devices -j` на этой машине,
2026-08-03): `"English (US)"`, `"Russian"` — полные XKB-имена, не короткие
коды. Виджет обязан их сокращать сам, сервис ничего не форматирует.

**Известное ограничение (не решать в этой задаче, просто знать):**
`keyboard_layout` — одно поле на весь `CompositorState`, а Hyprland хранит
раскладку **per-device**. На машине с несколькими клавиатурами это
last-writer-wins — поле показывает раскладку той клавиатуры, что последней
прислала событие. На однопользовательской машине с одной физической
клавиатурой (см. `hyprctl devices -j`: 7 записей, но только у одной реальной
клавиатуры раскладка меняется — остальные virtual/synthetic устройства) это
не проблема. Если завтра появится вторая физическая клавиатура — заводить
отдельную задачу, не блокировать T228 ей.

Итого: **вся работа T228 — это новый bar widget + одна новая команда
диспетчера**, по образцу уже существующих `workspace_mode` (простейший
текстовый виджет с кликом) и `workspaces` (чтение `AppState::compositor(cx)`).

## Требования

1. **Отображение.** Короткая метка (2 буквы, капс) текущей раскладки:
   `"English (US)"` → `"US"`, `"Russian"` → `"RU"`, `"Hebrew"` → `"IL"` (или
   `"HE"` — решить по месту, главное последовательность с остальными).
   Фолбэк для незнакомой строки — взять всё до `" ("` (если есть) или всю
   строку, первые 2 символа, `.to_uppercase()`. Не паниковать на пустой
   строке (`keyboard_layout` может быть `""` до первого события/фетча) —
   рендерить пустой виджет, как делает `workspaces.rs:35-37` для пустого
   списка воркспейсов.
2. **Клик = цикл раскладки.** По образцу `Alt+Shift` в системе — клик по
   виджету зовёт `hyprctl switchxkblayout all next` (диспетчер Hyprland,
   `all` — все клавиатуры разом, что заодно снимает multi-device рассинхрон
   из ограничения выше). Новая команда в
   `crates/services/src/compositor/types.rs::CompositorCommand`:
   `CycleKeyboardLayout` (без параметров, по образцу `NextWorkspace`/
   `PrevWorkspace`) → в `hyprland.rs::command_to_socket_line` рендерится в
   `"dispatch switchxkblayout all next"`.
3. **Виджет** — новый файл `crates/app/src/bar/widgets/keyboard_layout.rs`,
   по образцу `workspace_mode.rs` (иконка не обязательна — это текстовая
   пилюля, как workspace_mode без иконки, или с иконкой `⌨` из уже
   используемого SVG-набора, если такая есть в `assets/icons/` — проверить
   перед тем как выдумывать новую).
   - `name()` = `"keyboard_layout"`.
   - `section()` = `BarSection::Right` (рядом с `network`/`volume`/`clock` —
     системные индикаторы).
   - `render()` читает `AppState::compositor(cx).get().keyboard_layout`,
     сокращает, рисует pill с `on_click` → `CompositorCommand::CycleKeyboardLayout`.
   - Тултип (hover) — полное имя раскладки (`"Russian"`, не `"RU"`), если в
     дереве уже есть паттерн тултипов для bar-виджетов (проверить перед тем
     как изобретать — если нет, не заводить его ради одного виджета, просто
     полное имя в pill дешевле).
4. **Регистрация** — по образцу остальных виджетов:
   - `crates/app/src/bar/widgets/mod.rs:60` (там, где `match name`) — добавить
     `"keyboard_layout" => Box::new(keyboard_layout::KeyboardLayoutWidget)`.
   - `crates/app/src/bar/layout_config.rs` — добавить `"keyboard_layout"` в
     `BUILTIN_NAMES` (или как называется список `known`) и в `bar.toml`
     `known`/`right`, как это сделано для остальных виджетов (см. текущий
     `~/.config/chronos/bar.toml` — там уже есть полный список `known`).

## Зоны файлов

- `crates/services/src/compositor/types.rs` — новый вариант `CompositorCommand`.
- `crates/services/src/compositor/hyprland.rs` — `command_to_socket_line` матчит
  новый вариант.
- `crates/app/src/bar/widgets/keyboard_layout.rs` — новый файл, виджет.
- `crates/app/src/bar/widgets/mod.rs` — регистрация в `match name`.
- `crates/app/src/bar/layout_config.rs` — `BUILTIN_NAMES`/`known` список.

Конфликтов с активными ветками (T216/T218/T219/T221/T226/T227) нет — ни один
из них не трогает `bar/widgets/` или `compositor/`.

## Тесты

По образцу `workspace_mode.rs::tests` (имя+секция стабильны) и
`hyprland.rs::tests::command_to_socket_line_formats_every_variant`
(добавить кейс на `CycleKeyboardLayout`):

- `command_to_socket_line` рендерит `CycleKeyboardLayout` → правильную строку
  диспетчера.
- Функция сокращения имени раскладки (вынести в чистую `fn abbreviate(name:
  &str) -> String`, тестируемую без GPUI) — таблица случаев: `"English
  (US)"` → `"US"`, `"Russian"` → `"RU"`, `""` → `""`, незнакомая строка без
  скобки → первые 2 буквы капсом.
- `widget_name_and_section_are_stable` (копия паттерна workspace_mode).

## Приёмка

```bash
cargo test -p chronos --lib bar::widgets::keyboard_layout
cargo test -p chronos --lib compositor
cargo build --release -p chronos
```

**Live (обязателен — виджет живой, не статика):**

1. `bar.toml` `right` содержит `keyboard_layout` (дописать в конфиг, хот-релоад
   при сохранении).
2. `grim` бара — метка показывает текущую раскладку (сверить с реальной —
   набрать что-то в любом текстовом поле).
3. Клик по виджету → раскладка переключается (проверить набором текста после
   клика), метка в баре обновляется без задержки больше кадра.
4. `Alt+Shift` (штатное переключение) тоже обновляет метку — доказывает, что
   виджет слушает IPC-событие, а не только свой собственный клик.
5. Обе темы (`grim` Default + Light) — метка читаема, не сливается с фоном пилюли.

## Коммит

`bar : keyboard layout widget (T228)`
