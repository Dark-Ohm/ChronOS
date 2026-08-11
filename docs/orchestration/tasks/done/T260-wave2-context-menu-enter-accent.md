# T260-wave2 — Context Menu: enter-анимация + акцент-бар (по «повзрослевшему» форку)

**Приоритет:** P2 — визуальное доведение T260, независимая зона.
**Роль:** FRONTEND.
**Эталон:** `docs/design/Chronos-Context-Menu.dc (1).html` — CANON (тот же, что в T260).
**Второй заход** к T260 (`report/T260-context-menu-redraw-report.md`) —
добираем пункты, отложенные в первом заходе, и подтягиваем их под новое состояние
`Source/gpui`.

## Контекст: что уже сделано в T260 (не переделывать)

- Оба попапа переведены на общий elevation-shell (blur + `elev.shadows` + Light-C
  chrome), `bg.primary`, `radius_lg`, `border.subtle`, `scroll-guard`
  `.id().flex_1().min_h(0).overflow_y_scroll()`, маркеры ✓/○/◉ в 16px-гуттере с
  акцентом, disabled без hover, `ROW_PAD_X=10`.
- Файлы: `crates/app/src/tray_menu/view.rs`, `crates/app/src/dock/context_menu.rs`.

## Новости: что «повзрослело» в Source (проверено по коду, не по скиллам)

> **Важно:** скиллы (`chronos-gpui`, `gpui-layer-shell`, `vendored-gpui-animation`)
> под эти изменения **ещё не обновлены**. Кто берёт задачу — сначала читает сам
> `Source`, а не устаревшие утверждения из скиллов (например «в ядре нет
> переходов»). Нашёл drift — зафиксируй по `fork-api-drift`, не правь `Source`.

- **`easing`** в ядре: `Source/gpui/src/easing.rs` → `EasingCurve` с
  `CubicBezier(f32,f32,f32,f32)`, `EaseOutBack`, `EaseOutCubic`, `Elastic`, … .
  Это **точно easing эталона** `ctx-in .12s cubic-bezier(.2,.8,.2,1)`
  → `EasingCurve::CubicBezier(0.2, 0.8, 0.2, 1.0)`.
- **`spring`** в ядре: `Source/gpui/src/spring.rs` → `Spring` / `SpringValue` /
  `SpringPoint`, `SpringPreset::{Gentle,Wobbly,Stiff,Slow,Snappy}` (physics-based).
- **`scrollbar_width`**: `Source/gpui/src/styled.rs` (~стр. 68) — резервирует место
  под скроллбар при `Overflow::Scroll`. Релевантен тонкому скроллбару 6px из эталона.
- ChronOS **уже** ездит по `gpui::easing::EasingCurve` и держит готовый enter-мотор:
  `crates/app/src/motion.rs` (`arm_enter_progress`, `apply_enter_rise`, `ease_enter`
  = `EaseOutBack(1.5)`); живые пользователи enter — volume/system/updates popup.
  Т.е. enter-анимацию не изобретать, а переиспользовать.
- **Чего НЕТ и не появится через псевдо:** полей `pseudo_before/pseudo_after`
  нет (проверено в `style.rs`) — значит `.ci::before` **по-прежнему
  невозможно** сделать нативно, только stateful дочерним элементом.
  **Правка архитектора:** предыдущая версия этого пункта утверждала, что
  градиент-fill тоже недоступен — неверно, `Fill`/`BackgroundTag::
  LinearGradient` в `color.rs`/`style.rs:849` существует. Не влияет на
  задачу (акцент-бару градиент не нужен), но фиксирую — не додумывать по
  памяти там, где смотрели в код.

## Что нужно во втором заходе

1. **Акцент-бар (`::before`: 2px слева, inset по вертикали ~7px, radius `0 2 2 0`,
   bg `accent.primary`, появление через opacity+scaleY, 0.12s).** Через состояние
   (псевдо нет):
   - строка получает стабильный `id` (`tray-menu-item-{id}`; parent-строки тоже —
     сейчас у них нет `id`), `on_hover(|hovered| view.set_hovered(Some(id)); cx.notify())`;
   - внутри строки absolute-стрип `left(0)`, `top/bottom ~7px`, `w(2)`, `bg(accent.primary)`,
     скруглён только правый край, появление/уход — плавно через
     `easing::EasingCurve::CubicBezier(0.2, 0.8, 0.2, 1.0)` (или spring), не мгновенным swap;
   - disabled-строки бар не получают (дизайн `.ci.disabled::before{display:none}`).

2. **Enter-анимация попапа по `@keyframes ctx-in`** (fade + `translateY(-4px)` +
   `scale(.985)`, 0.12s, `cubic-bezier(.2,.8,.2,1)`) — обоим попапам.
   Использовать `motion::arm_enter_progress` (прецедент volume/system/updates).
   `motion::ease_enter` — сейчас `EaseOutBack(1.5)`; меню по эталону — `CubicBezier(.2,.8,.2,1)`.
   Добавить аддитивный хелпер в `motion.rs` (например `apply_enter_menu(el, delta)`:
   `opacity(d)`, `scale`, `top(4*(1-d))`), чтобы не плодить easing в двух вьюхах.
   Держаться view-driven пути (в motion.rs помечено: у anchored-попапов
   `with_animation` на живом Hyprland не доезжал → там нет view-driven).

3. **Тонкий скроллбар 6px** (дизайн `::-webkit-scrollbar { width:6px }`):
   на скролл-колонке `scrollbar_width(px(6.))`; выяснить, умеет ли форк красить свой
   скроллбар (`--border`) или рисует системный. Если не красится — кастомная
   overlay-полоса (canvas) по правому краю, 6px, `--border-subtle`; ряды не должны
   сжиматься от резервирования.

4. **`max-height: calc(100vh − 16px)`** (первый заход оставил фикс
   `MAX_MENU_H=480`): в `tray_menu/mod.rs` в `estimate_menu_height`/watcher взять
   высоту дисплея через `cx.find_display(...)` и капнуть `min(estimated, display.h − 16)`;
   dock — то же, если перестанет быть фикс.высоты.

5. Мелкая доводка первого захода: строка по эталону фиксированная `height:34px`
   (сейчас `py(6)`) — на фикс.высоте проще рисовать и transitions. Радиус строки 6px
   (`theme.radius`) и порядок по-прежнему из эталона.

## Зона файлов

- `crates/app/src/tray_menu/view.rs`, `crates/app/src/dock/context_menu.rs`
- `crates/app/src/motion.rs` (аддитивный enter-хелпер для меню)
- при необходимости `crates/app/src/tray_menu/mod.rs` (высота окна через display)
- НЕ трогать: `launcher/**`, `osd/**`, `crates/ui/**` (новых токенов не нужно),
  `Source/**` (fork — только чтение).

## Верификация

- `cargo build -p chronos` + `cargo test -p chronos motion` — зелёные.
- Живой прогон: tray-меню (короткое + submenu раскрыт), dock-контекст-меню, обе темы.
- **`grim -g`** по геометрии из `hyprctl layers` (не весь экран), 4 кадра:
  tray short, tray+submenu, dock, dock в светлой.
- **Enter-анимация и accent-переход — временна́я составляющая ⇒ по RULES короткий
  клип `wf-recorder`** на геометрии из `hyprctl layers` (`&` + `sleep` + `kill -INT`,
  не `timeout`): виден fade + подъём + scale по `.2,.8,.2,1` и 2px-акцент на hover.
- Сверить визуально с эталоном (`.dc (1).html` открыт рядом).

## Коммит

`context_menu : enter-anim + accent-бар по eased/spring форку`.
Без AI-трейлеров. `git add` поимённо; правила `docs/orchestration/agents/RULES.md`
(коммит обязан собираться сам по себе — не захватывать чужие незакоммиченные правки).

## Отчёт

`docs/orchestration/tasks/report/T260-wave2-context-menu-enter-accent-report.md`.
Отметить: что из нового форка реально использовано, что оказалось недоступно после
живой проверки, и не обновлённые скиллы (кандидат на отдельный долг по
`chronos-gpui`/`gpui-layer-shell`).