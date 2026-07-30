# T154 — композер: настоящее текстовое поле — отчёт

**Статус:** реализовано, ждёт живой приёмки. **Коммит:** `2587db3`.

## Что сделано

### text_input.rs (новый модуль, ~370 строк)

| Компонент | Что |
|---|---|
| `TextInputState` | Состояние: `content`, `selected_range`, `selection_reversed`, `cursor_visible`, `marked_range` (IME), `is_selecting`, `has_drop_hover` |
| Каретка | Позиция по символам (не байтам), `prev_char_boundary`/`next_char_boundary` с защитой `offset.min(content.len())` от паники |
| Выделение | `move_to`/`select_to`, `Shift+←/→`, `Shift+Home/End`, `Ctrl+A`, drag-выделение мышью |
| Слово | `prev_word_boundary`/`next_word_boundary` — Unicode-aware (`is_alphanumeric()` + `_` + `-` + glue), покрывает иврит/арабику/кириллицу |
| Буфер обмена | `copy_selection`/`cut_selection`/`paste` через `ClipboardItem` |
| Drag & drop | `on_drop` (ExternalPaths), `on_drag_move` для визуального фидбека |
| IME | `offset_to_utf16`/`offset_from_utf16`, `range_to_utf16`/`range_from_utf16`, `marked_range` |
| `TextInputElement` | Кастомный GPUI Element — prepaint (shape_line, selection quad, cursor quad), paint |

### composer.rs

| Компонент | Что |
|---|---|
| Мышь | `on_mouse_down`/`on_mouse_up`/`on_mouse_up_out`/`on_mouse_move` — позиционирование каретки, drag-выделение |
| Двойной клик | Выделение слова по `Instant` (500ms) + дистанции (5px) |
| Drag & drop | `on_drop` (файлы → пути через пробел), `on_drag_move<ExternalPaths>` (визуальный hover-фидбек) |
| Blink timer | `start_blink`/`stop_blink` — GPUI `cx.spawn` + `background_executor().timer()`, **не tokio** |
| Курсор | `.cursor(CursorStyle::IBeam)` на поле ввода |
| Плейсхолдер | `mouse_offset` проверяет `actual_content.is_empty()` — клик по плейсхолдеру позиционирует в 0, а не в байт плейсхолдера |

### mod.rs

- Новые поля: `composer_last_click: Option<(Instant, Point<Pixels>)>`, `composer_blink_task: Option<Task<()>>`
- Инициализация в `new()`

## Что НЕ сделано (известные ограничения)

- **Живая приёмка** — нет доступа к сеансу. Кадры `grim` — за архитектором.
- **Blink при keyboard-only focus** — timer стартует по клику и по нажатию клавиш. Если фокус приходит без клика (keybinding), каретка не начнёт мигать, пока не нажата клавиша. Фикс: добавить `start_blink` в `render()` при `composer_focused && blink_task.is_none()` — но `render_composer` принимает `&self`, нужен рефакторинг.
- **Drag hover-фидбек** — `on_drag_move<ExternalPaths>` устанавливает флаг, сброс — в `on_drop`. При уходе drag за пределы без drop флаг не сбрасывается до следующего ререндера. Приемлемо для v1.
- **Модуль в `crates/ui`** — по плану (вариант B), отдельным коммитом после живой приёмки.

## Приёмка

```text
$ cargo check -p chronos
Finished `dev` profile ... in 1.58s
```

Билд зелёный. Для живой приёмки архитектором:

1. Каретка видна и мигает при фокусе, гаснет при `Escape`
2. Выделение мышью и `Shift+стрелками` подсвечено; ввод заменяет выделенное
3. `Ctrl+C/V/X` работает через буфер обмена; `Ctrl+V` многострочного текста вставляет, не отправляет
4. Файл, брошенный на панель → путь в поле; hover-подсветка при наведении
5. Кириллица/иврит/эмодзи — набор, стрелки, backspace, выделение без паники

## Коммит

```
2587db3 panel : real text input — caret, selection, clipboard, drag-drop, blink
3 files changed, 779 insertions(+), 69 deletions(-)
create mode text_input.rs
```

Без AI-трейлеров, `git add` поимённо.
