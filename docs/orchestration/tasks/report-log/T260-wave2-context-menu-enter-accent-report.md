# T260-wave2 — контекст-меню: enter-анимация + акцент-бар — отчёт

**Роль:** FRONTEND.
**Эталон:** `docs/design/Chronos-Context-Menu.dc (1).html` (CANON).
**Статус:** код готов и собран (debug + release), тесты зелёные, шелл живьём
поднят и отрисовал бар; **открытие меню и кадры/клипы — НЕ выполнены** (см.
«Что НЕ сделано»).

## Что сделано

### `crates/app/src/motion.rs`
- `MENU_ENTER_MS = 120` — длительность из эталона (`@keyframes ctx-in .12s`).
- `MenuEase` — `CubicBezier(0.2, 0.8, 0.2, 1)` как
  `gpui_animation::transition::Transition` (для `transition_when_else`;
  `ease_menu_enter` — plain fn, его в Transition не передать).
- `arm_enter_progress_with(cx, duration, ease, set_t)` — параметризованная
  версия мотора; `arm_enter_progress` делегирует в неё с прежними
  (ENTER_MS + EaseOutBack). Меню едут на `.2,.8,.2,1`, попапы не тронуты.
- Тест `menu_ease_endpoints_and_monotonic` (endpoints 0/1 + без overshoot +
  монотонность) — вместе с существующими 2 тестами.

### `crates/app/src/tray_menu/view.rs` (переписан рендер строк)
- **Акцент-бар 2px** (эталон `.ci::before`): всегда присутствует у enabled
  строк, скрытое состояние — в базовой цепочке стилей (opacity 0, инсеты
  12px = «полувысота», scaleY(.5)-заменитель), появление/уход через
  `transition_when_else` 120ms `MenuEase` (opacity + top/bottom grow; форк не
  умеет element-scale). Disabled — без бара.
- **Hover-wash** строки анимируется тем же `transition_when_else`
  (`bg transparent → interactive.hover`); rest в базе (transparent) — без
  лишней невидимой анимации на старте.
- **Починена сборка задела:** тип `gpui::ListenerCallback` в форке не
  существует; hover теперь через `cx.listener` (даёт `&mut Context<Self>` →
  `set_hovered`), фабрика-замыкание `make_hover` убрана. `on_hover` вешается
  на `AnimatedWrapper::on_hover` (в форке один hover-слот на элемент,
  debug_assert) — `.id()`+`.with_transition()` до, `.on_click()` после.
- **Кастомный overlay-скроллбар 6px** (эталон `::-webkit-scrollbar`):
  выяснено, что форк **вообще не рисует скроллбары** — `scrollbar_width`
  только резервирует место (ряды сжались бы впустую). Поэтому `scrollbar_width`
  убран, thumb рисуется поверх по правому краю из живого `ScrollHandle`
  (`offset`/`max_offset`/`bounds`): wheel-обработчик контейнера сам шлёт
  `cx.notify(current_view)` → render перезапускается со свежим офсетом.
  Цвет — `border.subtle` (ровно `--border` эталона `#45475a`), radius 3px,
  появляется только при переполнении. Ряды не сжимаются.
- Enter: `arm_enter_progress_with(MENU_ENTER_MS, ease_menu_enter)` +
  `apply_enter_menu` (opacity + rise 4px; scale(.985) недоступен — в форке
  нет element-scale, задокументировано).

### `crates/app/src/dock/context_menu.rs`
- Тот же анимированный бар + wash (`transition_when_else`), rest в базе.
- **Починен `on_hover`:** предыдущий код не компилировался (`&mut Context<Self>`
  в слоте `&mut App`); `window.current_view()` в колбэке — **паника**
  (`debug_assert_paint_or_prepaint` + `rendered_entity_stack.last().unwrap()`).
  Теперь: `set_global(DockMenuHoverSignal)` + `cx.refresh_windows()` — тот же
  примитив, которым gpui_animation-тик двигает пере-рендеры (прецедент
  volume_popup живьём).
- Enter — `arm_enter_progress_with` меню-кривой. `DockMenuHoverSignal` — как в
  заделе (`dock/signal.rs`).

### `crates/app/src/tray_menu/mod.rs`
- `estimate_menu_height(nodes, display_h)`: кап `min(rows*ROW_H, display_h − 16)`
  — эталон `max-height: calc(100vh - 16px)` через `monitor::pult_display_info`
  (`pult_display_height`); `MAX_MENU_H=480` остался фолбэком на недостижимый
  display. Dock-меню — фикс.высота (один пункт), по тикету не трогается.

## Чем доказано

- `cargo check -p chronos` — чисто (ошибок нет; warnings в моих файлах
  убраны; остальные — предсуществующие чужие).
- `cargo build -p chronos` — green (debug). `./scripts/dev/chronos-rebuild` —
  релиз собран за 3m29s, `target/release/chronos` свежий.
- `cargo test -p chronos motion` — 3 passed (в т.ч. новый `MenuEase`),
  0 failed, в обоих бинарях (lib+bin).
- **Живой подъём шелла:** релиз-бинарь запущен на живом Hyprland 0.56.2 —
  бар отрисован (`hyprctl layers`: `namespace: bar, xywh: 0 0 2560 32, a: 1`),
  hover-strip на месте, tray-сервис зарегистрировал 4 иконки (easyeffects,
  :1.228, chromium, steam), dock-виджет загрузил pinned (kitty/thunar;
  firefox/code/vivaldi скипнуты «no AppEntry» — предсуществующее, не моё).
  Кадр бара: `/tmp/t260w2-bar.png`, кадр десктопа: `/tmp/t260w2-desktop.png`.
- Ключевые API сверены с `Source` (не по скиллам): `transition_when_else`
  анимирует оба направления (event NONE, без авто-отката), `State<StyleRefinement>`
  интерполирует opacity/top/bottom/left, первый кадр сидится из базового стиля
  (`with_state_default`) — rest обязан быть в базе, что и сделано.

## Что НЕ сделано — честно

- **Живые кадры меню и клипы НЕ сняты.** Причина: сессия пользователя активна
  (Vivaldi в фокусе, курсор двигается), а инъекция ввода нестабильна —
  `ydotool mousemove --absolute` маппит координаты не в экранное пространство
  (mousemove 21,16 → реальный курсор 48,24; x=2162 улетал за пределы мониторов),
  `hyprctl dispatch movecursor` в 0.56 требует другой синтаксис (Lua-dispatch
  ругается на пробел). Пара слепых кликов ушла в никуда/в область бара, после
  чего я остановил инъекции — риск попасть в окна пользователя перевесил.
  Шелл после проверки остановлен (до моей сессии он не был запущен).
- За архитектором (рецепт ниже): открыть tray-меню (правый клик по трей-иконке,
  трей в правой секции бара, до battery/volume/network) и dock-меню (правый
  клик по dock-иконке слева); `grim -g` по геометрии из `hyprctl layers`
  (4 кадра: tray short, tray+submenu, dock, dock light); `wf-recorder` клип
  (`&` + `sleep` + `kill -INT`) — enter fade+rise 120ms и accent-переход на
  hover. Сверить с `.dc (1).html` рядом.
- Скроллбар-thumb появляется без fade (bounds/max_offset известны только
  после первого layout) — незаметно на фоне enter-анимации, не блокер.

## Что из нового форка реально использовано

- `easing::EasingCurve::CubicBezier(.2,.8,.2,1)` — да (ease_menu_enter + MenuEase).
- `gpui_animation::transition_when_else` / `AnimatedWrapper` — да, оба меню.
- `ScrollHandle` (offset/max_offset/bounds) — да, живой thumb.
- `spring`/`SpringValue` — нет (не понадобилось; transition-путь проще и
  прецедент volume_popup).
- **Недоступно после живой проверки:** element `scale()` (в форке нет —
  scale(.985) и scaleY(.5) заменены rise/inset-морфами), `scrollbar_width`
  (резервирует место, ничего не рисует — кастомный overlay вместо),
  `window.current_view()` вне paint/prepaint (паника — `refresh_windows`).

## Скиллы устарели (долг)

`chronos-gpui` / `gpui-layer-shell` / `vendored-gpui-animation` утверждают
«в ядре нет переходов» / не описывают `transition_when_else`,
`CubicBezier`, `ScrollHandle.bounds()`, отсутствие отрисовки скроллбаров.
Кандидат на отдельную задачу по обновлению скиллов (в тикете предупреждено).

## Диапазон правок

- `crates/app/src/motion.rs`
- `crates/app/src/tray_menu/view.rs`
- `crates/app/src/tray_menu/mod.rs`
- `crates/app/src/dock/context_menu.rs`
- `crates/app/src/dock/signal.rs` (DockMenuHoverSignal — из задела)
- `docs/orchestration/tasks/report/T260-wave2-context-menu-enter-accent-report.md`

Не тронуты: `launcher/**`, `osd/**`, `crates/ui/**`, `Source/**`, чужие
незакоммиченные правки (Cargo.lock, DECISIONS.log, T252/T253, ticket T262).
