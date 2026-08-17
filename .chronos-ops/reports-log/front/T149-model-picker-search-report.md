# T149 Model Picker Search — Report

## Commit
`503b339` — `panel : model picker search — filter by name/id, keyboard input, RTL base-direction detection`

## Files Changed
- `crates/app/src/side_panel_left/composer.rs` — search field, filter logic, keyboard routing
- `crates/app/src/side_panel_left/mod.rs` — `composer_model_search` field + `is_rtl_text` (T152 bundled)

## What Was Built
- 🔍 Search field above scrollable model list with counter "N of M"
- Case-insensitive substring filter on both `id` and `name` fields
- Keyboard input routing: when model dropdown open, all typing goes to search field (not composer)
- Backspace deletes last character from search
- Enter selects first filtered match, closes dropdown, calls `set_model`
- Escape closes dropdown and clears search
- Clicking anywhere closes dropdown and clears search
- "Ничего не найдено" message when filter returns empty

## Decisions
- `composer_model_search` stored on `SidePanelLeft` (not composer) — follows same pattern as `composer_model_dropdown_open`
- Search field placed outside `max_h(250).overflow_y_scroll()` container — stays visible when list scrolls
- Dropdown uses `let dropdown = if model_open { Some(...) } else { None }` + `.children(dropdown)` — avoids `'static` lifetime issues with `SharedString` captures that `.when()` caused
- T152 RTL detection (`is_rtl_text`) bundled into same commit — was pre-existing in working tree and required for build consistency (chat_view.rs imports it)

## Scope Note
RTL text detection (`is_rtl_text` in mod.rs, `.when(is_rtl_text(text))` in composer.rs) was pre-existing uncommitted work from T152, not part of T149. Included in this commit because leaving it uncommitted would break the build (chat_view.rs references the function).

---

## Приёмка архитектора (2026-07-28) — КОД ПРИНЯТ, живая часть PENDING

**Проверено по дереву (`503b339`):**

- `composer_model_search` в состоянии панели, рядом с
  `composer_model_dropdown_open` — как и просил бриф;
- фильтр регистронезависимый по `id` **и** `name` (`composer.rs:266-279`),
  счётчик «N of M»;
- поле поиска вне скроллящегося контейнера — при прокрутке списка
  остаётся видимым (требование брифа);
- маршрутизация клавиш разведена: при открытом дропдауне символы и
  `backspace` правят строку поиска, а не композер
  (`handle_composer_key`, ветка `if self.composer_model_dropdown_open`);
- `Enter` берёт первый отфильтрованный и зовёт **тот же** `set_model`, что
  и клик — логика переключения не продублирована;
- `Escape` закрывает и чистит запрос;
- пустой результат даёт строку «Ничего не найдено», а не пустую рамку.

`cargo build --release -p chronos` — зелёная.

**Живая часть — PENDING за архитектором.** Работающий шелл несёт сборку до
этого коммита, а перезапуск убил бы активный рабочий тред пользователя
(персистентности нет — это T150/T151). Кадры с фильтром, пустым
результатом и выбором из отфильтрованного списка снимутся при ближайшем
рестарте.

### Замечание: коммит несамодостаточен по задачам

В `503b339` вместе с поиском уехала правка T152 (`is_rtl_text` +
`chat_view.rs`). Отчёт это признаёт честно, и мотив верный — без функции
дерево не собирается. Но выход был другой: **два коммита подряд** —
сначала чужая правка своим сообщением, потом своя. Сейчас в истории один
коммит, который нельзя откатить по одной задаче, не задев вторую.

Это ровно тот класс, про который в `CLAUDE.md` написано «несамодостаточные
коммиты shared-файлов — эпидемия, 3 случая». Теперь четыре.

### Мелочь на будущее

Сборка даёт два новых `warning: unused variable` (`text_secondary`,
`border_subtle`) — почистить при следующем касании файла.
