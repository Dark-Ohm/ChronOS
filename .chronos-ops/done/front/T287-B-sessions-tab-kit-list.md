# T287-B — список сессий на кит `List`/`VirtualList` + впервые дать pin/archive/rename

**Родитель:** `T287-left-chat-onto-gpui-component.md`
**Приоритет:** P1 — не только рендер-волна: T287-C убила `toggle_pin`/
`toggle_archive`/`delete_thread`/`begin_rename` как мёртвый код (их
единственный вызывающий, `ChatTab`'s inline sidebar, снесён). `chat.rs`
докстринг (`tabs/sessions.rs:6-9`) утверждал, что эти действия «остаются
у ChatTab sidebar» — это враньё после T287-C, сайдбар снесён целиком.
**В `SessionsTab` этих действий никогда не было** — только просмотр.
Обнаружено архитектором при закладке тикета, владелец сверил дерево лично
и подтвердил находку с тремя уточнениями (ниже).
**Роль:** FRONTEND. `crates/app/src/side_panel_left/tabs/sessions.rs`
(целиком, 406 строк). **Не трогать `chat.rs`/`composer.rs`** — это зона
T287-A, не пересекать.
**После T287-A в git.**

## Статус (2026-08-16, архитектор)

Первая попытка — Гермес/LongCat-2.0 (free, Nous) — снята. Причина: провайдер
`inference-api.nousresearch.com` три раза подряд (21:59, 22:19, 22:44) вешал
стрим на ровно 900с (`RemoteProtocolError`, peer closed без завершения тела),
в 22:19:03 там же поймали `401 — API key invalid, blocked or out of funds`.
Час висения — не единичный сбой, дохлый/квотированный free-тир, доверия
больше нет.

За этот час реальной работы почти не сделано: 19 добавленных / 4 удалённых
строки, только докстринг + список импортов, ноль функциональной реализации
(List/Input/PopupMenu не подключены). Импорты к тому же битые: `ListDelegate`
тащился из `gpui` (реально живёт в `gpui_component::list`), `Popupmenu` с
маленькой `m` (реального типа с таким именем нет — есть `PopupMenu`,
`crates/ui/src/menu/popup_menu.rs:282`) — не скомпилилось бы. Правки
откачены (`git checkout --`), `sessions.rs` в исходном состоянии из
`4baa6cf2`, TODO ниже актуален полностью с нуля.

~~Отдать другому исполнителю~~ — тот же процесс (не убит, только файл
откачен) дописал реализацию после нуджа, дошёл до конца сам. Диф
`+1061/-84`, вменяемый (реальные типы кита: `PopupMenu`, `Input`,
`v_virtual_list`, `ThreadStore` напрямую — не мусор первой попытки).
Отчёт: `docs/orchestration/tasks/report/T287-B-sessions-tab-kit-list-report.md`.

Архитектор проверил вживую (не на слово): `cargo check -p chronos` чисто;
`cargo test --lib sessions::` 7/7 (`sort_pins_first_then_recency`
нетронут); `cargo test --lib side_panel_left::` 118 passed (отчёт
заявлял 119 — на 1 меньше, другой скоуп фильтра, не критично);
`cargo build --release -p chronos` exit 0.

Живой grim-прогон закрыт владельцем: pin/delete/rename/archive — всё
работает. Закоммичено `8e84d3f`. **Принято.**

## Сейчас (сверено владельцем)

`SessionsTab::render` (`sessions.rs:169`) рисует список сам: свои `div`
на `Vec<ThreadListItem>` (`threads`). Поле `search: String` **объявлено,
но нигде не пишется** — в UI нет поля поиска вообще, только заголовок,
кнопка `+ New` и строки списка. Фильтр `visible()` (строка 148) фильтрует
по `short_title()`, **не** `display_title()` — источник фильтра именно
короткий заголовок, использовать его же при переезде на кит, не менять
на `display_title`.

`sessions_list.rs` — почти не разметка: `ThreadListItem::display_title`/
`short_title`/`has_cache` (текстовые хелперы) плюс мёртвые константы
`SIDEBAR_*` (ширины старого сайдбара, снесённого T287-C — не переиспользуемы,
не трогать, их чистка отдельная эррата). Сами `div`-ряды рисует
`SessionsTab::render`, не `sessions_list.rs`.

Сортировка pinned-first → updated_at desc уже покрыта тестом
(`sort_pins_first_then_recency`, строка 263) — **сохранить сортировку и
тест без изменений**, меняется только рендер и добавляются действия.

`ChatTab` (после T287-C) всё ещё содержит `rename_thread`/`commit_rename`/
`cancel_rename`/`search_threads` **без UI** — хвост C, мёртвый код без
вызывающей стороны в композере. **Не чистить в этом тикете** — не зона
B, отдельная эррата после A+B. Rename в B делается **заново**, с нуля, в
`SessionsTab`, не переиспользуя эти зависшие методы `ChatTab`.

## Стор — уже всё есть, новый слой не городить

`ThreadStore` (`crates/services/src/threads.rs`):
- `set_pinned(&self, id: &str, pinned: bool)`
- `set_archived(&self, id: &str, archived: bool)`
- `delete(&self, id: &str)`
- `update(...)` — для rename (title/title_override)
- `list(agent_id, pinned_only, include_archived)` /
  `list_for_project(...)` — оба уже принимают `include_archived: bool`,
  готовы к переключателю «Show archived».

## Кит

- Список: `../Source/gpui-component/crates/ui/src/list/` +
  `virtual_list.rs`.
- Поиск: кит `Input` (T275/T286) — **новый** элемент в этом UI, не замена
  существующего (существующего поля ввода нет).
- Действия на строке: кит `PopupMenu` (уже в дереве — dock/tray/launcher
  pin, T265-A..D), `⋯`/right-click на строке.

## Сделать

- Добавить кит `Input` для поиска над списком — **впервые**, не
  «заменить свой инпут» (его нет). Подключить к уже существующему полю
  `search: String` и фильтру `visible()` (фильтр по `short_title()`,
  не менять источник).
- Список строк → `List`/`VirtualList`. Row-контент — переиспользовать
  `ThreadListItem::display_title`/`short_title`/`has_cache`, не дублировать
  текстовую логику. Разметку строки написать заново под кит (в
  `sessions_list.rs` разметки для переиспользования нет).
- Клик по строке — тот же `SessionsEvent::SelectThread`, контракт не
  трогать.
- Реализовать действия через `ThreadStore` напрямую (методы выше есть,
  не изобретать новый слой):
  - Pin/unpin → `set_pinned`, список пересортировывается (сортировка уже
    работает).
  - Archive → `set_archived`. **По умолчанию скрывать архивные**,
    опциональный тумблер «Show archived» — дергает `include_archived` в
    `list`/`list_for_project`, оба уже это умеют.
  - Rename → `update` (title/title_override), inline edit через кит
    `Input`. Писать заново, не реанимировать зависшие
    `rename_thread`/`commit_rename` из `ChatTab`.
  - Delete → `delete`, с подтверждением (кит модалка/попап).
  - Точки входа — `⋯`/right-click через `PopupMenu`.
- Поправить `//!`-докстринг файла (`sessions.rs:6-9`) — убрать ссылку на
  несуществующий ChatTab-сайдбар, отразить, что действия теперь здесь.

## Нельзя

- Трогать `composer.rs`/`chat.rs` — зона T287-A, уже сделана. Хвост
  `rename_thread`/`commit_rename`/`cancel_rename`/`search_threads` в
  `ChatTab` не чистить — отдельная эррата после A+B.
- Городить новый слой поверх `ThreadStore` — методы уже есть.
- Терять сортировку pinned-first/recency-desc или её тест.
- Менять источник фильтра с `short_title()` на `display_title()`.
- Менять контракт `SessionsEvent::{SelectThread, CreateThread}`.

## Верификация

- `cargo check -p chronos`, `cargo test -p chronos --lib side_panel_left`
  (включая `sort_pins_first_then_recency`).
- Live grim: список рендерится, поле поиска (новое) фильтрует по
  short_title, pin меняет позицию, archive прячет/показывает по тумблеру,
  rename меняет заголовок и переживает перезапуск (пишет в `ThreadStore`),
  delete с подтверждением реально убирает тред.

## Коммит

`fix(left-panel): Sessions tab uses gpui-component List, gains search/pin/archive/rename (T287-B)`
