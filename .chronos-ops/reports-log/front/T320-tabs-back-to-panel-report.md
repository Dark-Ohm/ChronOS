# T320 — отчёт: вкладки вернулись в панель, попап-хост снят

## Что сделано

**Маршрут стал одним.** Клик по иконке рельса теперь всегда зовёт
`SidePanelRightView::on_tab_select` — тот же путь, что IPC `select-tab:<id>`
и виджеты бара. Развилка `is_popup_tab` удалена.

- `rail_view.rs:81-85` — `on_select` сведён к одному вызову
  `view.on_tab_select(tab, cx)`; ветка `control_center::is_popup_tab` /
  `toggle` / `close` убрана. Подсветка активной иконки (`rail_view.rs:54-57`)
  теперь читает только `content.active_tab()` — fallback на
  `control_center::active_tab` удалён.
- `rail.rs:44-63` — из `render_rail_button` вырезан захват живых bounds
  иконки (`canvas` + `Rc<Cell<Bounds>>`), который существовал исключительно
  для анкоринга попапа. Сигнатура `on_select` упрощена с
  `Fn(PanelTab, Bounds<Pixels>, &mut Window, &mut App)` до
  `Fn(PanelTab, &mut Window, &mut App)`; то же в `render_group` и
  `render_rail`. Импорты `Bounds`, `canvas`, `Cell` убраны.

**Попап-хост снесён.**

- `control_center.rs` (411 строк) — удалён целиком.
- `mod.rs` — сняты `pub(crate) mod control_center;`, хук
  `control_center::close(cx)` из обоих un-map путей (`close`, `close_this`),
  и `control_center::init(cx)` из `init()`.
- `window_root.rs` (`crates/ui/src/window_root.rs`) — записи под
  control-center нет, трогать нечего.

**Достижимость всех девяти.** Media и LauncherSettings были вне рельса
(popup-only); остальные семь уже сидели в рельсе. Добавлены в состав рельса
обоих режимов:

- `tabs.rs` — `PanelTab::Media` добавлен в `ALL` (21 → 22, строка 564);
  `for_mode(Developer)` и `for_mode(Gamer)` получили `Media` (после
  Notifications) и `LauncherSettings` (перед EditorSettings).
- `panels_config.rs` — `default_dev_top`/`default_gamer_top` получили
  `media`; `default_dev_bottom`/`default_gamer_bottom` получили
  `launcher_settings` (display → launcher_settings → editor_settings).

**Решение по Media:** оставлен и посажен в рельс. `PanelTab::Media` имеет
готовую ветку `TabContent::Media` (`tab/mod.rs:126`) и сущность `MediaTab`
(mpris-карточка) — выпиливать значило бы резать живой контент, а не сносить
мёртвый хост. Иконка `icons/play.svg` уже зарегистрирована в `assets.rs:46`.

**Тесты.** `tabs.rs`: `all_has_twenty_two_tabs_in_fixed_order` (22 элемента,
Media в хвосте), `developer_rail_is_eleven_product_tabs`,
`gamer_rail_is_eleven_product_tabs` — правлены под новый состав, не
ослаблены (по-прежнему фиксируют точный порядок и отсутствие cut-табов).
`panels_config.rs`: `resolve_grouped_uses_config_values`,
`sanitize_drops_unknown_and_deduplicates`, `move_within_top_group_swaps`,
`move_last_in_bottom_crosses_to_top` — обновлены под media/launcher_settings
в дефолтах.

## Верификация

### Сборка и тесты
```
cargo check -p chronos --lib --bins   → ok (только pre-existing warnings)
cargo build --release -p chronos      → ok
cargo test -p chronos --lib           → 605 passed; 0 failed
cargo test -p chronos --bins          → 797 passed; 0 failed
```

### Живой smoke (release, DP-1)

Шелл перезапущен на новом бинарнике (`setsid ... ./target/release/chronos`,
лог `/tmp/chronos-smoke.log`, `/tmp/chronos-smoke2.log`). Ни в одном замере
слоя `control_center` нет.

`hyprctl layers -j` после `toggle-side-panel-right` + `select-tab` (дословно
по namespace):
```
DP-1 side_panel_hover_strip
DP-1 side_panel_right_content  x1600 w920
DP-1 side_panel_right_rail     x2520 w40
```

Ширины всех девяти — из журнала (строки `switched tab → opened at per-tab
width`, каждая равна `preferred_content_width`):
```
tab="System"          width=400.0
tab="Media"           width=400.0
tab="Updates"         width=420.0
tab="Notifications"   width=420.0
tab="Display"         width=440.0
tab="System settings" width=410.0   (EditorSettings)
tab="Hyprland binds"  width=320.0
tab="ACP agents"      width=320.0
tab="Launcher"        width=410.0
```

Промер отрендеренного столбца (grim + ImageMagick, тёмная тема, сканлайн
y=1380; левый край тёмного столбца = 920 − visible_content_width):
```
media           edge 560 → visible 360 (400−40) ✓
hyprland_binds  edge 640 → visible 280 (320−40) ✓
display         edge 520 → visible 400 (440−40) ✓
updates         edge 540 → visible 380 (420−40) ✓
system          edge 560 → visible 360 (400−40) ✓ (чистый повтор, sleep 3)
```

Кадры: `/tmp/t320-shots/{system,media,updates,notifications,display,
editor_settings,hyprland_binds,acp_settings,launcher_settings}.png` (все
девять, тёмная тема), `/tmp/t320-light-media.png`,
`/tmp/t320-light-launcher.png` (светлая тема). В светлой теме столбец белый
(#FFFFFF), контент Media/Launcher рендерится (синие/светлые карточки), слоя
`control_center` нет.

- IPC `select-tab:system` → журнал `IPC select-tab received tab="system"` +
  `switched tab → opened at per-tab width tab="System" width=400.0`.
- Media достижим: `select-tab:media` → `Media width=400`, рендерится в обеих
  темах.
- Виджет обновлений бара (`bar/widgets/updates.rs`) и колокольчик
  (`notification_bell.rs`) уже зовут `select_tab(PanelTab::Updates/
  Notifications)` (T294/T293) — путь не менялся, заходит в панель.
- Переключение `frame.toml` hide/wrap: слоя `control_center` нет ни разу;
  панель корректно закрывается/переоткрывается через `apply_frame_inset`
  (журнал: `rail closed` + `content closed` → `opened both surfaces
  (pinned)`). Осиротевших side_panel-слоёв нет.

## Что не сделано / оговорки

- **Pre-existing баг frame.rs, не мой.** При 5× быстром переключении
  `frame.toml` hide→wrap wrap-поверхность падает:
  `frame: wrap surface ExclBottom failed to open: Adapter "NVIDIA GeForce RTX
  3070" ... not compatible`, после чего в лог сыплются `Protocol error ... on
  object wl_surface` и шелл теряет поверхности. Это frame.rs (зона вне
  T320), воспроизводится быстрым рекреейтом matte; чистый рестарт шелла
  восстанавливает всё (проверено: frame_wrap_matte + bar +
  excl_left/right/bottom снова на месте, ошибок 0). Мои файлы frame.rs не
  трогают.
- Кадры-улики лежат в `/tmp` (не в репо). Ширины доказаны дважды — журналом
  (state.width) и пиксельным промером.
- Живой клик по иконке рельса мышью не делал (ydotool не поднимал) — клик
  рельса и `select-tab` теперь один и тот же код
  (`rail_view.rs::on_select` → `on_tab_select`), что покрыто промером
  `select-tab` и unit-тестами `on_tab_select`.

## Удалённые файлы / хуки

- Файл: `crates/app/src/side_panel_right/control_center.rs` (411 строк).
- Хуки: `mod.rs` — `mod control_center;`, `control_center::init`,
  `control_center::close` ×2.
- Мёртвый код: bounds-захват в `rail.rs::render_rail_button`.

---

# ПРИЁМКА АРХИТЕКТОРА — ПРИНЯТ С ПЕРВОГО ЗАХОДА (2026-08-19)

Код `c2379fcc` (9 файлов, `control_center.rs` удалён). Сверено по дереву
и на живом, не по отчёту.

## Что проверено самостоятельно

- `control_center` в `crates/` больше нет — единственное вхождение это
  исторический комментарий `frame.rs:848` (кровный факт T305 про
  `exclusive_zone: Some(px(-1.))`), он остаётся уместным;
- `cargo check -p chronos --lib --bins` чисто; `--lib` 605/605 и
  `--bins` 797/797 прогнаны архитектором, числа отчёта сошлись;
- `PanelTab::ALL` стал 22, `ALL[21] = Media`, Media и LauncherSettings
  сидят в рельсе обоих режимов (`tabs.rs:288,331`);
- живой прогон: панель открыта, шесть вкладок пройдены через IPC —
  слоя `control_center` в `hyprctl layers` **нет ни разу**.

**Ширина проверена пикселями, а не только журналом.** На
`launcher_settings` (410 по логу) левый край тёмного столбца ровно
`x=2150` = 2520 − (410 − 40). Совпадает с `preferred_content_width`.

## Что понравилось в работе

Ширины доказаны дважды и независимо — журналом (`state.width`) и
пиксельным промером. Это ровно то, чего не хватало в T311, где числа
отчёта не воспроизводились на собранном бинаре.

Оговорки честные: живого клика мышью не делали (ydotool не поднимали) —
и сразу объяснено, почему это не дыра: клик рельса и `select-tab` после
правки буквально один код (`rail_view.rs::on_select` → `on_tab_select`).

Мёртвый код вычищен без напоминания: захват живых bounds иконки
(`canvas` + `Rc<Cell<Bounds>>`) в `rail.rs::render_rail_button`
существовал только ради анкоринга попапа, сигнатура `on_select`
упрощена с четырёх параметров до трёх.

## Решение по Media — принято

Оставлен и посажен в рельс. Обоснование верное: у него готовая ветка
`TabContent::Media` и живая сущность `MediaTab` (mpris-карточка), иконка
`icons/play.svg` уже в `assets.rs`. Выпиливать значило бы резать живой
контент, а не сносить мёртвый хост.

## Чужая находка — заведён T321

Исполнитель поймал и честно вынес как чужую зону, чинить не полез:
при пятикратном быстром переключении `frame.style` wrap-поверхность
падает с `Adapter "NVIDIA GeForce RTX 3070" ... not compatible`, дальше
`Protocol error` и потеря слоёв; рестарт восстанавливает.

Архитектором **не воспроизведено**, принято со слов — первым шагом
T321 стоит воспроизведение с дословным логом. Класс тот же, что в T314
(`close + open` поверхностей даёт протокольные ошибки), и путь
пересоздания станет основным после T319, где толщины краёв крутятся
живьём. Поэтому P2, а не «когда-нибудь».

## Итог

Принято. Дрейф T294 → T305 → T320 закрыт: одна вкладка — одно место,
правая панель.
