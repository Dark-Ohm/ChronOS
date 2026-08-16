# T287-B — список сессий на кит `List`/`VirtualList` + вернуть pin/archive/rename

**Родитель:** `T287-left-chat-onto-gpui-component.md`
**Приоритет:** P1 — не только рендер-волна: T287-C убила `toggle_pin`/
`toggle_archive`/`delete_thread`/`begin_rename` как мёртвый код (их
единственный вызывающий, `ChatTab`'s inline sidebar, снесён), а
`SessionsTab` — заявленный canonical дом этих действий (см. её же
docstring, `tabs/sessions.rs:6-9`, ныне устаревший) — их никогда не
реализовывал. **Pin/archive/rename треда сейчас нигде не живут.**
Обнаружено архитектором при закладке этого тикета, не ловилось приёмкой
T287-C.
**Роль:** FRONTEND. `crates/app/src/side_panel_left/tabs/sessions.rs`
(целиком, 406 строк) + `sessions_list.rs` (row-хелперы, переиспользовать).
**После T287-A в git.** Не трогает `composer.rs`.

## Сейчас

`SessionsTab::render` (`sessions.rs:169`) рисует список сам: свои `div`
на `Vec<ThreadListItem>` (`threads` поле), свой текстовый `search: String`
фильтр (`visible()`, строка 148, ручной `.to_lowercase().contains`), клик
на строке эмитит `SessionsEvent::SelectThread`. Сортировка pinned-first
→ updated_at desc уже покрыта тестом (`sort_pins_first_then_recency`,
строка 263) — **сохранить эту сортировку и её тест**, меняется только
рендер и добавляются действия.

Нет: пометки pinned (только сортировка учитывает флаг, отметить в UI
нечем), нет архивации, нет rename, нет удаления треда. Устаревший
docstring файла обещает «rename/pin/archive/menu остаётся у ChatTab» —
это больше не так, поправить комментарий заодно.

## Кит

- Список: `../Source/gpui-component/crates/ui/src/list/` +
  `virtual_list.rs` — виртуализация для длинных списков тредов.
- Поиск: кит `Input` (уже в дереве — T275/T286), не свой `String`+ручной
  фильтр.
- Действия на строке (pin/archive/rename/delete): контекстное меню кита —
  `PopupMenu` уже в дереве (dock/tray/launcher pin, T265-A..D) — тот же
  паттерн, правым кликом или `⋯` на строке.

## Сделать

- `search: String` + ручной `visible()` фильтр → кит `Input` над списком,
  тот же фильтр-предикат (`display_title` contains, case-insensitive) как
  делегат/callback кита, не переписывать логику фильтра с нуля.
- Список строк → `List`/`VirtualList`. Row-контент — переиспользовать
  `sessions_list::ThreadListItem::display_title`/`short_title`/
  `has_cache`, не дублировать разметку.
- Клик по строке — тот же `SessionsEvent::SelectThread`, не менять
  контракт с коордиатором (`WorkspaceView`).
- **Вернуть действия**, реализовав их здесь (не в `ChatTab` — та
  поверхность окончательно снесена T287-C):
  - Pin/unpin — пишет `record.pinned` через `ThreadStore` (тот же слой,
    что читает `SessionsTab::new`/`with_active_project`), список
    пересортировывается (сортировка уже готова).
  - Archive — `record.archived`; решить видимость архивных (скрыть по
    умолчанию, фильтр «Show archived» — на усмотрение исполнителя,
    задокументировать выбор в отчёте).
  - Rename — inline edit title (`title_override`) или мини-попап, кит
    `Input` для поля ввода.
  - Delete — с подтверждением (кит модалка/попап, не голый `on_click`).
  - Точки входа — `⋯`/right-click на строке через кит `PopupMenu`.
- Обновить `//!`-комментарий файла (строки 6-9) — убрать ссылку на
  несуществующий ChatTab-сайдбар, отразить, что действия теперь здесь.

## Нельзя

- Трогать `composer.rs`/`chat.rs` (T287-A/T287-C зона, уже сделаны).
- Менять контракт `SessionsEvent` без нужды — `SelectThread`/
  `CreateThread` используются коордиатором, лишние варианты — только
  если нужны для pin/archive/rename и явно провести через `WorkspaceView`.
- Терять сортировку pinned-first/recency-desc или её тест
  (`sort_pins_first_then_recency`).

## Верификация

- `cargo check -p chronos`, `cargo test -p chronos --lib side_panel_left`
  (включая существующий сортировочный тест).
- Live grim: список рендерится, поиск фильтрует, pin меняет позицию в
  списке, archive прячет/показывает по флагу, rename меняет заголовок и
  переживает перезапуск (пишет в `ThreadStore`), delete с подтверждением
  реально убирает тред.

## Коммит

`fix(left-panel): Sessions tab uses gpui-component List and regains pin/archive/rename (T287-B)`
