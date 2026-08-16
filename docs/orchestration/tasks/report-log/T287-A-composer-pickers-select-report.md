# T287-A — model/mode пикеры на кит `Select` — Report

**Date:** 2026-08-16
**Role:** FRONTEND.
**Zone:** `crates/app/src/side_panel_left/composer.rs` +
`crates/app/src/side_panel_left/tabs/chat.rs` (только 4 поля/инициализации).
**Precondition:** T287-C в git (`220a05e2`).

### Status

**DONE.** Коммит `25ac46a` `fix(left-panel): composer model/mode pickers use
gpui-component Select (T287-A)` — 2 файла, +269/−426.

### Что сделано

1. **`model_picker` переписан на кит `Select`** — `Select::new(&panel.composer_model_select)`,
   `.searchable(true)` (установлено на `SelectState` при создании),
   `.placeholder("Model")`, `.search_placeholder("Search models…")`,
   `.disabled(!has_data)`, `.with_size(KitSize::XSmall)`. Ручной
   `composer_model_search`/`.to_lowercase().contains`/`model-item-{i}`-цепочка
   и `composer_model_dropdown_open` убраны — кит фильтрует сам через
   `SearchableVec::perform_search` (case-insensitive substring по `title()`).
2. **`mode_picker` — тот же кит**, `.searchable(false)` (дешевый путь, без
   ручного второго пикера), `.placeholder("Mode")`, без поиска.
3. **`on_select`/`Confirm` → тот же путь, что и старый `on_click` 1:1**:
   `apply_model_select` (set `composer_selected_model`, focus composer input,
   `client.set_model` async, `cx.notify`) и `apply_mode_select` (set
   `composer_selected_mode`, focus, notify — без ACP-вызова, как и старой
   mode-click). Источник события изменился: `SelectEvent::Confirm` через
   `cx.subscribe_in` вместо hand-rolled `on_click`.
4. **Плейсхолдер «Model»/«Mode» при пустом списке сохранён** — кит показывает
   placeholder, когда selection пуста; `disabled(!has_data)` делает триггер
   неактивным и полупрозрачным, как старый muted-пилл. Логика показа
   (не скрывать, а показывать muted) не трогалась, только рендер.
5. **Убраны**: `composer_model_dropdown_open`, `composer_mode_dropdown_open`,
   `composer_model_search`, `model-item-{i}`/`mode-item-{i}` div-цепочки,
   `.on_action(InputEscape)`-ловушка на composer-Input (pickers нет),
   три ветки `handle_composer_key` (escape-закрытие, model-search typing,
   mode-close) — кит owns keyboard nav + search + Cancel/Confirm.
   `InputEscape` импорт убран. Осталась только `search_focused`-ветка
   (T287-B territory, не трогалась).
6. **`ChatTab`**: добавлены `composer_model_select` /
   `composer_mode_select` (`Entity<SelectState<SearchableVec<…>>`) +
   подписки `_composer_model_select_events` / `_composer_mode_select_events`.
   `SelectState` создаётся в `ChatTab::new(window, cx)` (тот же путь, что и
   composer `Input`); `.searchable(true)` только для модели.

### Верификация (воспроизводимо)

- `cargo check -p chronos --message-format short` — **0 warning** в
  `composer.rs` / `tabs/chat.rs`; `Finished`.
- `cargo test -p chronos side_panel_left` — **119 passed, 0 failed**.
- `cargo test -p chronos` (весь крэнт) — **761 passed, 0 failed**.
- Grep: `composer_model_dropdown_open`/`composer_mode_dropdown_open`/
  `composer_model_search`/`model-item-`/`mode-item-` в зоне — пусто.
- `composer-follow` (T287-C) на месте, `composer-pickers-row` не трогался.

### Что НЕ делал (честно)

- **Live grim не гонял** — это UX (открыть picker, набрать буквы —
  фильтрует, выбрать — меняется `composer_selected_model`, ACP получает
  `set_model`; плейсхолдер «Model» виден до первого списка моделей). Зона
  window/UX по AGENTS.md требует release-сборки + живого smoke — оставил
  Архитектору.
- `set_model`/ACP-вызовы по существу не менялись — только точка входа
  (kit `Confirm` вместо ручного `on_click`).
- `text_input`/`Input` (T286) и `composer-follow` (T287-C) не трогались.
- `dock`/`table`/`tree`/`lsp`/`chart` кита не тащил — только `select` +
  `searchable_list`.
- `handle_composer_key` `search_focused`-ветка нетронута (T287-B territory).
- Файлы вне зоны (launcher/, side_panel_right/, README, docs) не трогались.