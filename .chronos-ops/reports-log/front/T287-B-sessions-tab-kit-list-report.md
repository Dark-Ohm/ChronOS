# T287-B — Sessions tab on kit `List` + search/pin/archive/rename/delete

## Что сделано

Полностью переписан `crates/app/src/side_panel_left/tabs/sessions.rs`
(было 406 строк, стало 1384; +1061/−84). Зона соблюдена: `chat.rs` и
`composer.rs` не тронуты (проверено `git diff --name-only` — их нет в
выводе). Рендер-волна + действия реализованы с нуля, как и требовал
бриф — зависшие методы `ChatTab` не реанимированы.

### По пунктам брифа

- **Поиск — кит `Input`.** Новый элемент над списком (существующего поля
  не было). Связан с уже существующим полем `search: String` через
  `InputState` + подписку на `InputEvent::Change`. Фильтр `visible()`
  **по-прежнему по `short_title()`**, источник не менял — есть тест
  `search_filters_short_title_not_full_display_title`, который это
  фиксирует (хвост после 30-символьного усечения не ищется).
- **Список — кит `v_virtual_list`.** Строки рисует кит-виртуальный список
  (`gpui_component::v_virtual_list`), фиксированная `ROW_HEIGHT = 44px`.
  Контент строки переиспользует `ThreadListItem::display_title`/
  `short_title`/`format_timestamp` — текстовая логика не продублирована.
  `sessions_list.rs` не трогал (там разметки и не было).
- **Клик по строке** — тот же `SessionsEvent::SelectThread`, контракт
  не менялся. Поле `selected` пишется на клике и читается в render для
  подсветки (`when(is_selected, …)`), т.е. подсветка реальная, не
  write-only — отдельный тест `selected_field_is_written_on_click_and_read_in_render`.
- **Действия — через `ThreadStore` напрямую**, без нового слоя:
  - Pin/unpin → `set_pinned`, список пересортировывается (`reload` →
    `sort`). Сортировка pinned-first → updated_at desc **сохранена**,
    тест `sort_pins_first_then_recency` не тронут и проходит.
  - Archive → `set_archived`. По умолчанию архивные скрыты; тумблер
    «Show archived» дёргает `show_archived` → `include_archived` в
    `list_for_project` (метод уже принимал этот флаг).
  - Rename → `update` с `title`/`title_override`, inline-edit через кит
    `Input`. Enter или blur (клик мимо) коммитит; пустая строка
    игнорируется. Пишется заново, не через `ChatTab::rename_thread`.
  - Delete → `delete`, с инлайн-модалкой подтверждения (Cancel/Delete).
- **Точки входа** — кит `PopupMenu` в anchored-popup окне (`⋯` на строке +
  right-click по строке). Движок тот же, что dock/tray/launcher:
  `gpui_component::Root`-обёртка, `grab: false` (T264), click-catcher с
  дырой в screen-space (закрытие по клику мимо), фолбэк на layer-shell
  при `PopupNotSupportedError`. `PopupMenu` пересобирается с нуля на
  каждый вызов (snapshot меняется от строки к строке).
- **Докстринг файла** поправлен: убрана ссылка на несуществующий
  ChatTab-сайдбар, отражено, что действия теперь здесь.

## Что верифицировал и как

- `cargo check -p chronos` — чисто по `sessions.rs` (единственный
  warning — `selected_thread is never used`, он **был и в HEAD**,
  `git show HEAD:…sessions.rs:126`, не введён этим тикетом).
- `cargo test -p chronos --lib sessions::tests` — **7/7 pass**:
  `sort_pins_first_then_recency`,
  `search_filters_short_title_not_full_display_title`,
  `selected_field_is_written_on_click_and_read_in_render`,
  `clear_for_project_resets_scope`,
  `new_without_project_loads_empty_scope`,
  `no_unscoped_list_in_sessions`,
  `pin_archive_delete_roundtrip_through_store` (новый — гоняет pin →
  resort, archive → hide, show-archived → reveal, delete → gone на
  реальном temp-`ThreadStore`).
- `cargo test -p chronos --lib side_panel_left` — **119 passed, 0 failed**.
- `cargo build --release -p chronos` — `Finished release profile`,
  exit 0.

## Что НЕ сделал

- **Live Wayland smoke test (grim) не прогонял** — в этой сессии нет
  живого композитора/дисплея. Пункты брифа «список рендерится, поиск
  фильтрует, pin меняет позицию, archive прячет/показывает, rename
  переживает рестарт, delete убирает» проверены только через unit-тесты
  и сборку, не глазами. Это честный пробел для acceptance-прогона
  владельца.
- Не трогал `chat.rs`/`composer.rs` (зона T287-A) и не чистил зависший
  хвост `ChatTab` (`rename_thread`/`commit_rename`/`cancel_rename`/
  `search_threads`) — отдельная эррата после A+B, вне зоны.
- Не трогал `sessions_list.rs` и мёртвые константы `SIDEBAR_*` (отдельная
  эррата по брифу).
- `selected_thread()` остаётся мёртвым в non-test коде — как и было до
  тикета; не чистил (вне зоны, изменение не запрашивалось).

## Коммит

`fix(left-panel): Sessions tab uses gpui-component List, gains search/pin/archive/rename (T287-B)`
