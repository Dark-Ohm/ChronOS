# T305 — control-center popup: settings-вкладки уезжают в единый anchored-popup

**Роль: FRONTEND.** **Стартует только после приёмки T304**
(`tab/mod.rs` — общий файл, параллелить нельзя).

## Контекст

Брейншторм 2026-08-17/18, видео-референс владельца
(`/home/neo/Videos/soramane.mp4`, Noctalia-style desktop): плавающая
пилюля-рейл с иконками, клик по иконке открывает **один** anchored-
popup со slide-анимацией из-за края рейла; внутри popup — таб-бар
(Dashboard/Media/Performance/Workspaces на референсе), переключает
контент той же карточки.

**Закрытый поимённый список (сверено с `tabs.rs:14`, индексы
`PanelTab::ALL`) — не пересматривать без нового брейншторма:**

Едут в popup (8): `System`[0], `Updates`[1], `Notifications`[2],
`AcpSettings`[13], `EditorSettings`[17] (= entity `BarSettingsTab`,
UI-label **«System settings»** — НЕ редактор; на этой путанице уже
дважды горели, два коммита правили не тот вариант, см. память
`system-vs-editorsettings-tab-confusion` — называть в коде и
комментариях `EditorSettings`/`BarSettingsTab` явно, никогда просто
«system tab» без уточнения), `HyprlandBinds`[18], `Display`[19],
`LauncherSettings`[20] (реальная entity, `launcher.toml` — тот же риск
дублирования что у System/Display/Updates, та же логика применяется).

Остаются в рейле как есть (13): `Files`[3], `Editor`[4], `Terminal`[5],
`Preview`[6], `Inspector`[7], `Build`[8], `SourceControl`[9],
`Library`[10], `Scenes`[11], `Captures`[12], `McpSettings`[14],
`LspSettings`[15], `ApiProviders`[16]. Последние три — пустышки
(`_ => TabContent::Placeholder` в `tab/mod.rs`, никакой сервисной
подписки) — **не трогать**, двигать их сейчас работа без пользы;
пересмотреть отдельным решением, когда получат реальный контент.

## Архитектурные решения (зафиксированы, не пересматривать без нового брейншторма)

1. **Popup эксклюзивно владеет settings-вкладками.** Никакого
   шаринга Entity/реестра между рейлом и popup — один инстанс
   каждого таба, живёт только в popup. Settings-варианты уходят из
   `PanelTab::ALL` целиком (рейл их больше не показывает и не
   создаёт).
2. **`TabContent` остаётся одним enum-реестром** (T304 это уже
   обеспечило) — не резать на «рейловые»/«popup» варианты. Popup-хост
   вызывает тот же `TabContent::create(tab, cx: &mut App)`.
3. **Popup — наш существующий native anchored-popup паттерн**
   (`bar_settings`/`tray_menu`-стиль layer-shell window + `gpui_animation`
   slide-transition), НЕ отдельная input-модель, НЕ слияние окон.
4. **Media — новый тонкий таб**, оборачивает уже существующий
   `render_mpris_card(&MprisState, &App)` (`mpris_card.rs:190`, чистая
   функция) — бэкенд/`services::mpris` не трогать.

## Задача

1. **`render_footer` / `power_row.rs` — удалить целиком**, не
   оставлять мёртвым кодом. Проверено: не единственный путь к
   power/network — `start_menu/view.rs:73-80` (`rail_power_actions`,
   5 действий: Lock/Sleep/LogOut/Restart/Shutdown) + OSD Hibernate уже
   дают выключение системы; `network`/`battery` — builtin-виджеты бара
   (`bar/layout_config.rs` `BUILTIN_NAMES`), в дефолтном right-порядке.
   Функционал не теряется, просто перестаёт дублироваться в панели.
   Убрать `power_row.rs`, импорт/вызов `render_footer` в
   `side_panel_right/view.rs:648-657` (весь `TabContent::System =>`
   рукав переписывается без footer-хвоста — сам System переезжает в
   popup, эта ветка `view.rs` может исчезнуть целиком, если System
   больше не создаётся рейлом).
2. **`tabs.rs` тест `all_has_twenty_tabs_in_fixed_order`
   (tabs.rs:14-40ish, `PanelTab::ALL.len() == 21` + 21 позиционный
   `assert_eq!`) — переписать под новый состав, сохранив смысл**:
   фиксированный порядок рейла и уникальность id, а не просто новую
   длину. Не «подогнать число и один assert», не удалить тест как
   мешающий — переписать полностью под реальный новый список
   `PanelTab::ALL` (минус 7 settings-табов) с тем же уровнем строгости
   (каждая позиция проверяется явно).
3. **Персист активного таба.** Если где-то сохраняется последний
   открытый таб рейла и там мог оказаться один из уехавших settings-
   табов (например `System`) — после этого тикета такое значение не
   резолвится в `PanelTab::ALL`. Найти, где персистится (`grep` по
   `active_tab`/`PanelTab` в конфиге/state), добавить миграцию/фолбэк
   (невалидный persisted tab → первый валидный таб рейла, не паника и
   не пустой экран). **Обязательный пункт верификации**: холодный
   старт с `system` (или другим уехавшим табом) в персисте конфига →
   рейл открывается на валидной вкладке.
4. **Popup-хост** (новый модуль, например
   `side_panel_right/control_center.rs`): layer-shell popup window,
   `gpui_animation` slide-open, анкер — позиция иконки в рейле
   (**живые координаты**, не кэш — та же ловушка что с
   `window.bounds()` на центрированных окнах, см. память
   `chronos-launcher-pin-window-bounds-trap`). Внутри — таб-бар
   (Dashboard-эквивалент/System, Media, Performance-эквивалент/Updates
   или что реально осмысленно замапить, Workspaces-эквивалент если
   есть) переключает, какой `TabContent` вариант рендерится в теле
   popup — через `TabContent::create(tab, cx)` (T304).
5. **Un-map рейла закрывает popup.** Хук вешается на call site в
   `side_panel_right/mod.rs:464` — там уже вызывается
   `frame::set_rail_mapped(FrameSide::Right, false, cx)` на un-map;
   добавить туда же закрытие popup, если открыт. **`frame.rs` не
   трогать** (T303-зона, живёт отдельно) — вся правка живёт в
   `side_panel_right/mod.rs`. Не закрыть popup при un-map — призрак-
   окно, тот же класс бага что в ghost-window саге launcher/tray_menu
   2026-07-18. Открытие popup — rollback-паттерн как у
   `open_wrap_windows` (частичный open запрещён).
6. **Media tab** — новый `media_tab.rs` (или модуль внутри
   control_center), тонкая обёртка над `render_mpris_card`, подписка
   на `MprisState` как в существующих потребителях (`bar/widgets/mpris.rs`,
   старый `mpris_card.rs` usage) — паттерн копировать, не изобретать.
7. Живой прогон: `hyprctl layers` координаты popup, `grim` до/после
   open/close, unmapped-rail сценарий, resize.
8. `cargo check`/`cargo test --lib` — переписанный `tabs.rs` тест
   зелёный, остальной lib не регрессирует.

## Зона файлов

`crates/app/src/side_panel_right/**` (широко — `tabs.rs`, `view.rs`,
`power_row.rs` удаление, новый `control_center.rs`/`media_tab.rs`).
Не трогать `crates/app/src/side_panel_left/**` — не в скоупе. Не
трогать `frame.rs` (T303 — параллельная зона, тот же `frame.toml`, но
разные файлы/функции — конфликта нет, просто не лезть).

## Отчёт

`.chronos-ops/reports-fresh/T305-control-center-popup-host-report.md`
