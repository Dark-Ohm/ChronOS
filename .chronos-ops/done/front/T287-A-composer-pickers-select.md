# T287-A — model/mode пикеры на кит `Select`

**Родитель:** `T287-left-chat-onto-gpui-component.md`
**Приоритет:** P2 — работает, но самописный dropdown дублирует кит.
**Роль:** FRONTEND. `crates/app/src/side_panel_left/composer.rs`
(`model_picker`, `mode_picker`, их состояние в `ChatTab`).
**После T287-C в git** (`220a05e2`) — тот же файл, тот же
`composer-pickers-row`, где T287-C уже посадила Follow. Параллелить с
любой другой волной, трогающей `composer.rs`, нельзя.

## Сейчас (самопис)

- `model_picker` (`composer.rs:268-…`): свой dropdown-div, свой поиск по
  `panel.composer_model_search` (ручной `.to_lowercase().contains`), свой
  список `model-item-{i}` с ручной подсветкой `is_active`/hover, свой
  open-флаг `panel.composer_model_dropdown_open`.
- `mode_picker` (`composer.rs:502`): тот же паттерн, без поиска.
- Оба рисуют строки руками (`div().id(...).px().py().rounded()...`), не
  используют кит вообще.

## Кит

`../Source/gpui-component/crates/ui/src/select.rs` — `SelectState` +
`SearchableListDelegate` (реэкспортнуто как `SelectDelegate`/`SelectItem`
для обратной совместимости, новый код — `SearchableListDelegate` /
`SearchableListItem` напрямую). Кит уже тянет поиск, keyboard nav
(`SelectUp`/`SelectDown`/`Confirm`/`Cancel`), anchored-дропдаун через
`anchored`/`deferred`. История применения кита в этом дереве — T275
(`launcher`, `Input`), T286 (`composer.rs`, `Input` многострочный).

## Сделать

- Model picker → `SelectState` с делегатом над `panel.available_models`
  (`id`, `name`) как источником, поиск через кит (свой
  `composer_model_search` фильтр выкинуть, кит фильтрует сам через
  `SearchableListDelegate`).
- Mode picker → тот же кит, без поиска (или с — если кит не умеет
  «без поиска» дешевле, чем городить два пути, спросить архитектора,
  не решать самому).
- `on_select`/`Confirm` дергает тот же путь, что сейчас: `set_model`/
  режим-эквивалент на ACP-клиенте (см. текущий `on_click` в
  `model_picker`, строки ~342-360 — сохранить сайд-эффект 1:1, просто
  сменить источник события).
- Плейсхолдер-состояние «Model» при пустых `available_models` (комментарий
  в коде объясняет: ACP-агент не всегда сразу отдаёт список) — сохранить.
  Это не костыль, это осознанное решение с обоснованием в коде, не трогать
  логику показа плейсхолдера, только рендер.
- Убрать: `composer_model_dropdown_open`, `composer_model_search`,
  `model-item-{i}` div-цепочку, эквивалентные поля/циклы для mode.

## Нельзя

- Менять `set_model`/ACP-вызовы по существу — только точку входа с
  ручного `on_click` на кит-`Confirm`.
- Трогать `text_input`/`Input`-поле композера (T286) или `composer-follow`
  (T287-C) в этом же файле — не их зона.
- Тащить `dock`/`table`/`tree`/`lsp`/`chart` компоненты кита.

## Верификация

- `cargo check -p chronos`, `cargo test -p chronos --lib side_panel_left`.
- Live grim: открыть model picker, набрать буквы — фильтрует; выбрать —
  `composer_selected_model` меняется, ACP получает `set_model` (лог
  `RUST_LOG=chronos=info`, строка `set_model` или ошибка). То же для mode.
  Плейсхолдер «Model» виден сразу после connect до первого списка моделей.

## Коммит

`fix(left-panel): composer model/mode pickers use gpui-component Select (T287-A)`

## Приёмка

2026-08-16, коммит `25ac46a` (+269/−426, `composer.rs`+`chat.rs`).
Сверено архитектором: старый хлам (`composer_model_search`,
`model-item-{i}`, `composer_model_dropdown_open`) grep-чист, кит
`SelectState`/`SearchableVec`/`apply_model_select`/`apply_mode_select`
на месте, `composer-follow` (T287-C) и `search_focused` (T287-B зона) не
задеты. Релизный билд чист (0 новых warning в зоне), `cargo test -p
chronos` — 761 passed (совпадает с отчётом). Живой смок владельца:
функция работает (фильтр, выбор, placeholder), **но попап и его контент
обрезаны** — заведён T298 отдельным тикетом, не блокер приёмки этой
волны. Принято.
