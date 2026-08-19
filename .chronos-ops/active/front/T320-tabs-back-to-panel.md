# T320 — вернуть вкладки в правую панель, снять попап-хост

**Роль:** FRONTEND. **Приоритет:** P1.
**Зоны:** `crates/app/src/side_panel_right/` — `control_center.rs`,
`view.rs`, `rail.rs`, `tabs.rs`, `panels_config.rs`, `mod.rs`,
`tab/media_tab.rs`; точечно `crates/app/src/bar/widgets/updates.rs` и
другие вызовы `select_tab`.
**Не трогать:** `frame.rs`, `aperture_ring` (принято T318), палитру,
левую панель.

## Решение владельца (2026-08-19)

> «у нас были всегда панели вкладки справа. не попап карточки»
> «вкладки вернуть короче»

Все вкладки живут в правой панели. Попап-карточек нет. Развилки по
режиму оболочки нет — сначала обсуждали «вкладки в `normal`, попапы в
`wrapped`», владелец эту идею свернул: одно поведение, всегда.

## Что сейчас

`control_center.rs::is_popup_tab` — захардкоженный список из девяти
имён: System, Media, Updates, Notifications, Display, EditorSettings,
HyprlandBinds, AcpSettings, LauncherSettings. Они уходят в анкорный
попап при любом `frame.style`.

**Это дрейф, а не изначальный замысел.** T294 специально УБРАЛ попап у
обновлений — виджет бара стал открывать `PanelTab::Updates` в правой
панели; тем же заходом `notifications/history_popup` заменили на
`PanelTab::Notifications`. Проект вытаскивал попапы В панель. Через два
дня T305 утащил девять вкладок ИЗ панели в попап. Архитектор принял
второй, не сверив с первым — признано при разборе.

**Рендер переписывать не надо:** у всех девяти уже есть ветки в
`tab/mod.rs::create` (System, Updates, Notifications, Display,
BarSettings, HyprBinds, AcpSettings, LauncherSettings, Media). Задача —
маршрут и снос хоста, не контент.

## Что сделать

### 1. Все вкладки — в панель

Убрать `is_popup_tab` и все ветвления по нему. Клик по иконке рельса,
виджеты бара и IPC `select-tab:<id>` открывают вкладку в панели —
одним путём, без вариантов.

Пройти грепом по `select_tab(` и `control_center::` и убедиться, что
не осталось второго пути.

### 2. Снять попап-хост

Ни один путь в `control_center` больше не ведёт — значит он удаляется,
а не остаётся мёртвым:

- `control_center.rs` (406 строк) — целиком;
- `tab/media_tab.rs` — если он существовал только ради попапа
  (проверить: `PanelTab::Media` имеет ветку `TabContent::Media`, она
  остаётся);
- `control_center::init` из `init()` (`mod.rs:806`);
- хуки `control_center::close` из обоих un-map путей
  (`mod.rs:527`, `mod.rs:595`);
- запись в `window_root.rs`, если есть.

Если понадобится вернуть — лежит в истории git, коммит T305 `f326fc7`.

### 3. Достижимость всех девяти

У каждой обязана быть иконка в рельсе и место в `panels_config.rs`
(`default_dev_top` / `default_gamer_top`) для ОБОИХ режимов workspace.

**`PanelTab::Media` — главный риск:** он был popup-only. Если у него нет
позиции в рельсе, после сноса попапа вкладка станет недостижимой.
Дать место, либо явно решить, что Media выпиливается — и записать в
отчёт, а не проглотить.

`PanelTab::ALL` — массив из 21 с тестом на фиксированный порядок
(`tabs.rs:533`). Меняется состав рельса — правится и тест, но **не
ослабляется**: он ловит пустые иконки и опечатки в id.

### 4. Ширины

У каждой вкладки своя `preferred_content_width` (`tabs.rs:773`):
System 400, Updates 420, Files/SourceControl 440, Editor/Terminal/
Preview 560, Build 640. Девять возвращаемых должны открываться со
своей шириной, а не с дефолтной. Проверить промером.

## Верификация — живая

```bash
cargo build --release --bin chronos
RUST_LOG=info ./target/release/chronos &
chronos-ipc toggle-side-panel-right
```

Приёмка:

1. Клик по каждой из девяти иконок рельса открывает вкладку в панели.
   Слоя `control_center` в `hyprctl layers` нет **ни разу**, ни в
   `hide`, ни в `wrap`. Кадры на каждую вкладку.
2. Ширина панели на каждой из девяти совпадает с её
   `preferred_content_width`. Дословный `hyprctl layers`.
3. Виджет обновлений в баре открывает вкладку Updates в панели.
4. IPC `select-tab:system` открывает System в панели.
5. Media достижим — или выпилен, с обоснованием в отчёте.
6. Переключение `frame.toml` между `hide` и `wrap` пять раз: поведение
   вкладок не меняется, осиротевших слоёв нет.
7. Обе темы.
8. `cargo test -p chronos --lib` зелёные. Тесты, завязанные на
   `is_popup_tab`, удаляются вместе с ним; тест на порядок `ALL`
   правится под новый состав, не ослабляется.

## Отчёт

`.chronos-ops/reports-fresh/T320-tabs-back-to-panel-report.md`.
Кадры по всем восьми критериям, дословный `hyprctl layers`, решение по
Media, список удалённых файлов и снятых хуков.

## Коммиты

```
panel : вкладки вернулись в правую панель
panel : control-center popup снят
```

Поимённый `git add`, `git diff --staged` глазами. Без AI-трейлеров.
