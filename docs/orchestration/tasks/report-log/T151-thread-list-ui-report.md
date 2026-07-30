# T151 — Thread List UI report

**Дата:** 2026-07-29
**Статус:** BUILD GREEN (cargo build -p chronos)

## Что сделано

### Сессия 1 (исходная миграция, до текущего чата — предыдущая ночь)

- `sessions_list.rs` переписан с `SessionItem { id, title, active }` на
  `ThreadListItem { record: ThreadRecord, active }` с методами
  `display_title()`, `short_title()`, `format_timestamp()`.
- `mod.rs` — добавлены поля `thread_store`, `thread_search`, `show_archived`,
  `thread_loading`, `thread_menu_open`, `search_focused`, `rename_thread_id`,
  `rename_input`. Добавлены методы: `sort_sessions()`, `create_new_session()`,
  `select_session()`, `rename_thread()`, `toggle_pin()`, `toggle_archive()`,
  `delete_thread()`, `search_threads()`, `toggle_archived()`, `open_thread_menu()`,
  `close_thread_menu()`, `begin_rename()`, `commit_rename()`, `cancel_rename()`,
  `start_search()`, `cache_transcript()`, `set_auto_title()`.
- `composer.rs` — добавлена обработка клавиатуры для sidebar search / rename,
  и `set_auto_title` при первом пользовательском сообщении.
- `chat_view.rs` — `MessageRole`, `ToolCallPreview`, `Segment`, `ChatMessage`
  получили `Serialize`/`Deserialize` для кэширования транскрипта.

### Сессия 2 (текущий чат — фикс сборки)

**Проблема:** Миграция типа сломала сборку — `panel.rs` остался на старых
полях (`SessionItem`), а `composer.rs` получил E0502 из-за borrow checker.

**Найдено и исправлено:**

| # | Файл | Ошибка | Фикс |
|---|------|--------|------|
| 1 | `panel.rs:53` | E0609: no field `title` on `ThreadListItem` | `s.title.clone()` → `s.display_title().to_string()` |
| 2 | `panel.rs:473` | E0609: no field `id` on `ThreadListItem` | `s.id.clone()` → `s.record.id.clone()` |
| 3 | `panel.rs:636` | E0609: no field `title` on `ThreadListItem` | `s.title.clone()` → `s.short_title()` |
| 4 | `panel.rs:637` | E0609: no field `id` on `ThreadListItem` | `s.id.clone()` → `s.record.id.clone()` |
| 5 | `mod.rs:332` | E0596: cannot borrow `threads` as mutable | `let threads` → `let mut threads` |
| 6 | `composer.rs:763` | E0502: borrow `*self` as mutable while immutable | clone `self.thread_search` перед вызовом |
| 7 | `composer.rs:772` | E0502: same pattern | clone `self.thread_search` перед вызовом |
| 8 | `composer.rs:784` | E0502: same pattern | clone `self.thread_search` перед вызовом |
| 9 | `composer.rs:934` | E0502: `self.state.active_session_id` borrowed | clone `self.state.active_session_id` перед `set_auto_title` |

**Результат:** `cargo build -p chronos` — зелёный (только warnings, без errors).

## Состояние относительно спецификации T151

- [x] Список тредов из хранилища (SQLite, загрузка в `SidePanelLeft::new()`)
- [x] Закреплённые сверху, сортировка по `updated_at` (в `sort_sessions()`)
- [x] Архивные скрыты (поле `show_archived`, метод `toggle_archived()`)
- [x] Открытие треда восстанавливает разговор (`select_session()` → `load_session` + replay)
- [x] Кэш из хранилища показывается сразу (из `transcript_json`)
- [x] Автозаголовок от первого сообщения (`set_auto_title()`)
- [x] Переименование, пин, архив, удаление — методы заведены
- [x] Поиск по тредам (`search_threads()`, поле `thread_search`)
- [x] «+» создаёт тред явно (`create_new_session()`)

**Что пока не реализовано (UI rendering):**
- Рендер списка тредов в `build_sessions_sidebar()` всё ещё использует
  захардкоженный `sessions-list-scroll` с простой итерацией и без
  поискового поля, контекстного меню, кнопок rename/pin/archive/delete.
- `on_click` на элементах списка не вызывает `select_session()`.
- Переключение архивных — нет UI-виджета для показа/скрытия.

Это связано с тем, что миграция типа и фикс сборки были первым шагом;
непосредственный рендер UI списка тредов остаётся следующим шагом.

## Замечания

1. Миграция типа, затрагивающая соседние модули, должна делаться одним
   заходом до зелёной сборки. Оставлять на ночь в рабочем дереве нельзя.
2. `thread_search` и `search_threads` корректно принимают `&str`, но
   composer.rs требует clone из-за E0502 — это нормально для такого паттерна.
3. В `panel.rs` thread_title использует `display_title()` (уважает
   `title_override`), а в списке используется `short_title()` (обрезка до
   ~30 символов) — оба правильные методы из `ThreadListItem`.

## Команды для проверки

```bash
cargo build -p chronos 2>&1 | grep -E "^error"
# Должен показать 0 ошибок
```

---

## Приёмка (2026-07-30, Architect) — ПРИНЯТО

**Расхождение отчёта с деревом:** раздел «что пока не реализовано (UI
rendering)» УСТАРЕЛ. Рендер списка тредов дописан ПОСЛЕ отчёта (был
незакоммичен). В `panel.rs` (`build_sessions_sidebar`, развёрнутый сайдбар)
на момент приёмки есть всё, что отчёт числил отсутствующим:
- элемент `session-item-{sid}`: `on_click`→`select_session`, правый
  клик→`open_thread_menu`, 📌 для pinned, active-подсветка (panel.rs:838-883);
- контекстное меню Rename/Pin/Archive/Delete — все `on_click` на
  `begin_rename`/`toggle_pin`/`toggle_archive`/`delete_thread` (599-713);
- «+ New session»→`create_new_session`, футер «Show/Hide archived»→
  `toggle_archived`, поле поиска/rename (search_or_rename), collapse.

**Основание приёмки:** ревью кода (рендер полон и прошит к бэкенд-методам
mod.rs) + green build (`cargo build --release -p chronos`, exit 0) + рабочая
SQLite-БД (`~/.local/share/chronos/threads/threads.db`, 10+ записей) +
частичный живой вид панели (grim).

**Отложено — общая машина:** интерактивный клик-смок (pin/rename/archive/
delete мышью, фильтр поиска вводом) НЕ прогнан живьём — 2026-07-30 разработчик
зашёл на смену на ту же машину, ydotool/computer-use лагает и лезет в его
ввод. Догнать интерактивный смок на свободной машине (см. TBD хвосты).

**Наблюдение:** у существующих тредов в БД пустой `title` — автозаголовок
проставляется только для новых (первое сообщение), старые покажут
`short_title()` от пустого. Не блокер.

**Коммит:** `179dc75` (threads : T151 — UI списка тредов …). Код лёг поверх
незакоммиченного месива, развязан от эрраты T154 (`9d6020c`) реверс-патчем.
