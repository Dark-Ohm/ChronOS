**Принято архитектором 2026-08-15.** Live System-вкладка ок. Тумблер Gaming → T291-E (`refresh_windows` в apply/revert). Попап grim не требовали.

# T291 — System справа: power profile и Gaming Mode с бара

**Роль:** FRONTEND. **Статус:** done (на проверке архитектора — не принимал сам).
**Ветка:** `feat/t291-system-power-gaming` (worktree-изолированная, sibling of ChronOS).
**База:** `17afee6` (T289 accept).

## Что сделано

1. `power_profile_block` + `gaming_mode_block` (+ `toggle_switch`) вынесены из
   `system_popup/view.rs` в новый общий модуль `crates/app/src/power_controls.rs`
   как `render_power_profile_card` / `render_gaming_mode_card`. Поведение 1:1 —
   те же сервисы (`AppState::upower`, `GamingModeState`), тот же arm/клик, та же
   геометрия тумблера. Не копипаст в System, а общий модуль.
2. Обе карточки вставлены в `SystemTab::render` **над** спектрами (после
   MPRIS / wallpaper, до CPU) в стиле System — `surfaces::card` fill +
   `border.subtle` (как `mpris_card` / `wallpaper_card` / `disks`), не сырой
   попап 280px.
3. Попап `system_popup` теперь = шапка + яркость. Блоки power/gaming и их
   helper-ы удалены из `view.rs`; неиспользуемые импорты/переменные (`PowerProfile`,
   `gaming_mode`, `upower`, `gaming_active`) убраны. Пустым не оставлен.
4. `SystemTab` подписан на UPower (`state::watch`) — repaint при смене профиля.
   Gaming-карточка тоже оживает через этот сигнал (gaming mode крутит
   `set_power_profile` Performance/restore, так что round-trip будит перерисовку).
5. `mod power_controls;` добавлен в `lib.rs` и `main.rs` (свои строки).
   `surfaces` в `side_panel_right/mod.rs` поднят до `pub(crate)`, чтобы
   top-level модуль читал `surfaces::card` (без хардкода цвета карточки).

## Чем доказано

- `cargo check -p chronos` → exit 0 (изолированный `CARGO_TARGET_DIR`,
  чтобы не трогать артефакты основного дерева).
- `cargo test -p chronos --lib side_panel_right` → 195 passed.
- `cargo test -p chronos --lib system_popup` → 3 passed.
- `cargo test -p chronos --lib power_controls` → 1 passed (smoke: обе
  карточки строятся из default-состояния).
- `git diff --staged` — только мои файлы, чужого нет (worktree чист кроме T291).

## Что НЕ сделано

- **Грим попапа (`system_popup`) не снят.** Бар-виджет system открывает попап
  по `on_mouse_down(Left)`; без точных координат виджета клик через
  `ydotool` хрупок (риск попасть в volume/clock/tray). Попап код-верифицирован
  (см. § «Код-верификация попапа»), пользователь верифицирует клик-прогон
  сам. Не блокер.
- **`BASE_HEIGHT` попапа (274) не ужат.** Теперь в попапе только шапка +
  яркость, высота избыточна. Не трогал — яркость и размер попапа = зона T290,
  этот тикет яркость не трогает. Флаг для T290.
- Темы: код собирается и использует токены темы + `on_fill(accent)` для текста
  на акценте (верно в обеих схемах по контракту `on_fill`). Светлая схема
  живьём не подтверждена (запускал на текущей тёмной сессии).

## Live-проверка (release, grim)

- `cargo build --release -p chronos` (5m34s) → exit 0. Бинарь:
  `wt-t291-target/release/chronos`. 93 warnings — все pre-existing
  (`PanelTab::ALL`, `is_dnd`, `tray_menu.open_service`/`nodes`, `close_this`,
  `open_waytrogen_gallery_async` и т.п.), к T291 не относятся.
- `pkill -x chronos` (процесса не было — stale-сокет `/run/user/1000/chronos.sock`,
  перебинд без конфликта). Запуск: `RUST_LOG=info ./wt-t291-target/release/chronos`,
  мониторы резолвятся, 2 live displays.
- IPC: `toggle-side-panel-right` → панель открыта (лог: `opened both surfaces
  (pinned)`); `select-tab:system` принят (лог: `IPC select-tab received tab="system"`).
- Снят `grim` полного экрана 4480×1440, кроп правой панели
  (x=1965..2570, y=55..1155) — `t291-system-tab.png` — карточки рендерятся
  корректно, сегмент «Quiet» активен (реальный `UPowerData.power_profile =
  PowerSaver` с машины — карточка читает сервис, не хардкод), Gaming-тумблер
  OFF (default `GamingModeState`), effect-строка на месте.

## Код-верификация попапа (`system_popup`)

После T291 `system_popup/view.rs` рендерит **только** header + brightness-слайдер
(`sed -i '444,603d'` в worktree удалил `power_profile_block`, `gaming_mode_block`,
`toggle_switch` и их helper-ы). Импорты `PowerProfile`, `Service`, `UPowerData`,
`gaming_mode` убраны, переменные `upower` / `gaming_active` — тоже. Brightness
— зона T290, не тронута. `mod.rs` `BASE_HEIGHT = 274` оставлен (избыточен
для header+яркость, но это зона T290 — флаг передан).

Клик-прогон попапа (нажать виджет system на баре → увидеть только яркость;
клики по профилю и Gaming теперь во вкладке System, а не в попапе) — за
архитектором.

## Границы

- Не трогал: яркость, `wallpaper_card`, left rail, `Source/gpui/`, `Cargo.lock`,
  новый UPower/Hyprland путь, удаление `system_popup`.
- Задание не двигал в `done/`, отчёт не клал в `report-log/`, коммит не помечал
  «принята».

## Приёмка

- **Принято архитектором 2026-08-15.** Не перенесено в `done/` (per RULES —
  сам не закрываю). Коммит `84f25bf` слит на `master` fast-forward.
- T246 (disabled-состояние на профиле) в исходнике попапа тоже отсутствовал —
  поведение 1:1, ок.
- `BASE_HEIGHT` попапа (274) оставлен как есть — яркость/размер попапа = зона
  T290, этот тикет не трогает.

## Errata — ручка Gaming-тумблера едет с задержкой (не новый тикет)

**Симптом (ожидаемый, классифицирован как errata, НЕ новый тикет):**
клик по тумблеру Gaming в System-вкладке переключает состояние, но ручка
визуально доезжает не сразу — с задержкой (порядка D-Bus round-trip UPower,
десятки мс; дольше, если UPower медленный/недоступен).

**Корень:** `crates/app/src/system_popup/gaming_mode.rs` —
`apply()` (строки 102-108) и `revert()` (133-138) после переворота глобала
`GamingModeState` вызывают только `repaint_popup()` (красит **попап**). После
T291 попап больше не содержит блок Gaming (только шапка + яркость), поэтому
`repaint_popup` — no-op визуально. `SystemTab` читает
`GamingModeState::is_active(cx)` в `render` (через `render_gaming_mode_card`),
но у вкладки **нет** watch на `GamingModeState`: это простой GPUI-global,
мутируемый через `global_mut` (не `set_global`), так что `observe_global`
не срабатывает. Вкладка репейнтится только когда приходит сигнал UPower —
`apply`/`revert` дёргают `upower.set_power_profile(Performance)/restore`, и
watch UPower в `SystemTab::new` (добавлен в T291) будит `cx.notify()`.

То есть: клик → глобал перевернут, попап (пустой по Gaming) перекрашен,
вкладка ждёт UPower → только по round-trip перерисовка → ручка доезжает.

**Параллель с оригиналом:** в попапе ручка ехала сразу именно потому, что
`repaint_popup` красил тот самый попап, где тумблер и жил. После переезда в
вкладку immediate-repaint-цели нет. 1:1 по логике, но индикация в новом
месте ждёт сервис.

**Предлагаемый фикс (НЕ применён — вне зоны T291, на усмотрение архитектора):**
вариант А — в `apply`/`revert` вместо `cx.global_mut::<GamingModeState>()`
делать `cx.set_global(new_state)` (будит `observe_global`) + в
`SystemTab::new` добавить `cx.observe_global::<GamingModeState>(|this, cx| cx.notify())`.
Вариант Б (локальнее) — хранить handle `SystemTab` в глобале по аналогии с
`SystemPopupState::handle` и дёргать `notify` из `repaint_popup`/`repaint_tab`.
Оба без изменения сервисов/поведения клика.

**Статус live-проверки эрраты:** задержка — факт уровня кода (см. выше);
одиночный кадр `grim` её не покажет (ловит уже перерисованное состояние).
Подтверждение тайминга — в клик-прогоне архитектора («Если ручка едет с
задержкой — эррата»).
