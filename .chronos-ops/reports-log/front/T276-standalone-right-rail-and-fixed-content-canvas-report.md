# T276 — отчёт исполнителя (Sonnet 5)

**Вердикт Архитектора:** VERIFIED / CLOSED — 2026-08-13. Владелец дал
финальный `+`: resize работает идеально, separator принят.

## Правка Архитектора после live-проверки

После первой пересборки `hyprctl layers` показал новый вертикальный дефект:
content стоял в `y=0`, тогда как rail и bar-gap начинались с `y=32`.
Причина: `exclusive_zone: -1` отключает влияние всех чужих exclusive zones,
включая bar, а не только rail. В `content_window_options` добавлен явный
верхний отступ `panel_edge_gap()` одновременно с постоянным правым отступом
`RAIL_ONLY_WIDTH`. Контракт закреплён красно-зелёным тестом
`content_margin_restores_bar_gap_while_ignoring_exclusive_zones`.

Владелец затем заметил тонкую визуальную щель при геометрически смежных
поверхностях. Это оказался не compositor-gap, а ошибочно оставленный 4px
resize-handle внутри rail surface. Промежуточная попытка закрасить его лишь
превратила щель в выпирающую полосу и была отвергнута. Финальная архитектура:
standalone rail занимает цельные 40px и рисует separator на настоящем стыке;
прозрачный resize-handle живёт на левой, screen-inward кромке content.
Геометрия закреплена тестом `resize_handle_tracks_left_edge_of_visible_content`.

Следующая live-проверка обнаружила сломанный responsive layout вкладки
Appearance: она выбирала wide-grid по `window.bounds() == 920`, хотя при
`preferred_content_width == 410` видимая часть равна только 370px. Общий
`tab::ui::is_wide` переведён на `visible_content_width(state.width)`; тем же
исправлены потенциально затронутые ACP и Hyprland Binds. Регрессия закреплена
тестом `breakpoint_uses_visible_slice_not_fixed_wayland_canvas`.

При live drag до rail-only clamp обнаружился ещё один дефект: при
`visible_w=0` render удалял handle и обнулял input-region прямо во время
зажатой кнопки, поэтому панель нельзя было потянуть обратно тем же жестом.
Теперь `content_interactive_width` сохраняет последние прозрачные 4px и
handle до mouse-up; его x clamp'ится внутрь canvas. Контракт закреплён тестом
`active_drag_keeps_handle_interactive_at_rail_only_clamp`.

После подтверждения владельцем (`+`: resize работает идеально) на внешнюю
левую кромку content добавлен тонкий `border_l_1` цвета
`theme.border.subtle`, симметричный separator'ам bar и левой панели. Border
движется вместе с видимой content-колонкой; rail остаётся отдельной цельной
поверхностью.

## Правки после первого замечания Архитектора

Первый черновик этого отчёта содержал пять реальных проблем, все
исправлены в этом же коммите:

1. **`exclusive_zone: None` на content давал протокольный `0`, а не
   opt-out.** Hyprland по wlr-layer-shell сдвигает same-edge Overlay-
   поверхность без собственной exclusive zone на величину чужой
   (rail'а) — вместе с explicit `margin-right: 40` получался двойной
   отступ, растущий вместе с rail'ом в dock-режиме (до 920px лишних).
   Исправлено: `exclusive_zone: Some(px(-1.))` — задокументированный в
   протоколе (wayland.app/protocols/wlr-layer-shell-unstable-v1)
   способ полностью вывести поверхность из авто-сдвига чужими зонами.
2. **`RailView::render` не вызывал `window.set_exclusive_zone` вообще**
   — я написал это в отчёте, но забыл сам код. Добавлено: rail читает
   `state.exclusive_px()`, кэширует через общее глобальное
   `last_exclusive_zone`, зовёт `set_exclusive_edge`/`set_exclusive_zone`
   при изменении.
3. **81 строка отвергнутого T273 configure-driven кандидата** осталась
   в `Source/gpui_linux/src/linux/wayland/window.rs` (не в зоне T276).
   `git checkout -- gpui_linux/src/linux/wayland/window.rs` в
   `Source/` — весь diff был T273-экспериментом одним куском, чужих
   правок внутри не было (`git diff` перед откатом проверен построчно).
   `Source/gpui/src/scene.rs` и `Source/gpui/src/window.rs` были уже
   чистыми — там нечего было откатывать.
4. **Не было теста на контракт «оба handle как единое целое».**
   Реальный `open_pinned`/`close` через `cx.open_window` в
   `TestAppContext` форсирует синхронный первый paint, а дефолтный таб
   `System` жадно читает пять живых `AppState`-сервисов (mpris/
   system_resources/disks/wallpaper/compositor) в конструкторе — в
   этом крейте нет ни одного прецедента фейковать `AppState` в юнит-
   тесте, городить его специально под один инвариант непропорционально.
   Вместо этого рефакторинг: решение «коммитить оба / откатить content»
   вынесено в чистую функцию `two_surface_open_outcome`, которую
   `open_window` реально вызывает (не дублирует) — протестирована двумя
   тестами без единого окна. Живой лайфсайкл (ghost/orphan) остаётся
   пунктом 7 живого чек-листа задачи.
5. **Отчёт лежал в `tasks/report/`, тикет требует `tasks/report-log/`.**
   Перемещён (`docs/orchestration/` не под git — обычный `mv`, не `git
   mv`).

Полный прогон после исправлений: `cargo test -p chronos --lib` — **330
passed, 0 failed** (весь крейт, не только side_panel_right);
`cargo test -p chronos --bins` — 554 passed; `cargo build --release -p
chronos` — exit 0.

## Изменённые файлы и символы

- `crates/app/src/side_panel_right/mod.rs` (переписан):
  - `SidePanelRightState`: `handle`→`rail_handle`+`content_handle`,
    `view`→`content_view`; поля `pinned`/`peek_generation`/`width`/
    `dock_content`/`resizing`/`last_exclusive_zone` сохранены без изменений
    (единый источник состояния для обеих поверхностей).
  - Новые константы: `CONTENT_CANVAS_WIDTH = MAX_WIDTH - RAIL_ONLY_WIDTH`
    (920px).
  - Новые чистые функции: `visible_content_width`, `content_input_region`,
    `two_surface_open_outcome` (+ `TwoSurfaceOpen` enum).
  - `resize_target_width` — сигнатура сменилась с `(actual_w, current_x,
    grab)` на `(start_width, start_x, current_x)` — чистая дельта, без
    зависимости от `window.bounds()` (обе поверхности больше не
    ресайзятся).
  - `rail_window_options`/`content_window_options` заменили единый
    `window_options`. `content_window_options`: `exclusive_zone: Some(px(-1.))`
    + `margin: Some((0, RAIL_ONLY_WIDTH, 0, 0))` — фиксированный
    40px-отступ независимо от rail'овой зоны и явный верхний
    `panel_edge_gap()` после opt-out от bar (см. правку Архитектора выше).
    `rail_window_options`: `exclusive_zone: Some(px(exclusive_px()))` на
    момент создания; live-обновления — из `RailView::render`.
  - `open_window` открывает обе поверхности; после попытки открыть rail
    решение «коммитить/откатить» идёт через `two_surface_open_outcome`
    (реальный вызов, не тестовый дубликат).
  - `close`/`close_this`/`toggle`/`select_tab`/`preview_target` адаптированы
    под два handle; публичный API (`toggle`, `open_pinned`, `open_peek`,
    `select_tab`, `preview_target`, `RAIL_ONLY_WIDTH`, `MAX_WIDTH`,
    `DEFAULT_CONTENT_WIDTH`) не изменился — внешние вызовы
    (`ipc/mod.rs`, `side_panel_left/*`, `bar/agent_api.rs`) не тронуты.
  - Тесты: 26 юнит-тестов на чистую геометрию/лайфсайкл-логику (было 14
    до T276), включая `visible_width_*`, `input_region_*`,
    `drag_left_grows_width_by_exact_delta`, `two_surface_open_outcome`×2.

- `crates/app/src/side_panel_right/rail_view.rs` (новый файл):
  - `RailView` — тонкий Render-энтити окна `rail`. Владеет только
    `WeakEntity<SidePanelRightView>` + `Subscription` (`cx.observe`), сам
    не хранит бизнес-состояние. Рендерит только цельный 40px
    `rail::render_rail(...)`; resize hitbox и drag-обработчиков в rail нет.
    `on_select`/`on_dock_toggle` делегируют в content через weak entity.
  - **Реально** ставит `window.set_exclusive_zone`/`set_exclusive_edge` на
    основе `state.exclusive_px()`, кэш через глобальное
    `last_exclusive_zone` (пункт 2 выше — в первом черновике это было
    написано в отчёте, но отсутствовало в коде).
  - Округляет **только свой** правый верхний угол экрана
    (`panel_corner_radius(display_w)`).

- `crates/app/src/side_panel_right/view.rs` (переписан):
  - `SidePanelRightView` больше НЕ владеет resize-геометрией через
    `window.bounds()` — новые методы `start_resize`/`update_resize`/
    `end_resize`/`toggle_dock`/`active_tab()` стали `pub(crate)`;
    rail вызывает tab/dock методы через weak entity, а resize-методы вызывает
    прозрачный handle на левой кромке самого content.
  - Удалены поля/механика: `last_resized_width`, `pending_resize_width`,
    `t273_frame`, весь блок `cx.on_next_frame` + T243/T273-трейсинг в
    `render()` (async-resize-retry). Добавлено одно поле
    `last_visible_width: Option<f32>` — кэш для `set_input_region`.
  - `render()` больше не рисует rail и не трогает
    `window.set_exclusive_zone` (это исключительно дело rail). Он рисует
    прозрачный resize-handle поверх левой кромки видимой content-колонки.
    Layout: `[void-spacer (flex_1) | content
    column (flex_none, w=visible_w)]` внутри `.size_full()` фиксированного
    920px canvas.
  - `on_tab_select`/`toggle_dock`/`start_resize`/`update_resize`/
    `end_resize` теперь зовут `cx.refresh_windows()` вместо `cx.notify()`
    там, где меняется общее состояние — тот же идиом, что уже использует
    `workspace_mode::set`/`edit_mode::toggle`/`panels_config::apply` для
    кросс-оконного релейаута; нужен, потому что рейл — теперь отдельное
    окно и не подписан персонально на каждый мутатор.
  - `needs_width_resize` — оставлена (используется `side_panel_left/
    mod.rs:197`, не относится к T276), с явным комментарием почему.
  - Тесты: старые 6 T221-регрессий сохранены дословно (логика
    `on_tab_select` не менялась), добавлены 7 новых на
    `start_resize`/`update_resize`/`end_resize`/`toggle_dock`, восстановлен
    тест на `needs_width_resize`.

- `crates/app/src/side_panel_right/tab/preview.rs`: одна строка,
  `.view` → `.content_view` (поле переименовано в mod.rs).

- `crates/app/src/side_panel_right/rail.rs`, `hover_strip.rs`,
  `surfaces.rs`, `tabs.rs`, `panels_config.rs`, `tab/*` — **не менялись**.

- `Source/gpui_linux/src/linux/wayland/window.rs` — откачен
  (`git checkout --`) до состояния апстрима форка; 81 строка отвергнутого
  T273-кандидата удалена из чужого репозитория, как требовал тикет.

## Схема ownership двух handles и закрытия

```
SidePanelRightState (global)
├─ rail_handle:    Option<WindowHandle<Root>>   — окно "rail"
├─ content_handle: Option<WindowHandle<Root>>   — окно "content"
└─ content_view:   Option<WeakEntity<SidePanelRightView>>

open_window(pinned):
  1. открыть content-окно → SidePanelRightView (сильный Entity)
  2. если (1) упало → return, ничего не открыто
  3. открыть rail-окно → RailView::new(content_entity.clone(), cx)
     (RailView держит weak-копию content_entity)
  4. two_surface_open_outcome(rail_result.is_ok()):
     - RollbackContent → закрыть уже открытое content-окно, ничего не
       коммитить в state
     - CommitBoth → записать оба handle + content_view.downgrade() в
       global state одним блоком — снаружи либо оба Some, либо оба None

close():
  забирает оба handle одним `take()`, чистит content_view/pinned/
  resizing/last_exclusive_zone, затем закрывает rail (clear exclusive
  zone → remove_window) и content (remove_window) НЕЗАВИСИМО — ghost на
  любой стороне логируется отдельным warn, а не глотается `let _ =`.

close_this(window, cx): для вызова изнутри одной из двух поверхностей —
  определяет, какая это (rail или content), закрывает её напрямую (уже
  есть &mut Window), а другую — через её собственный handle.update (не
  реэнтерабельный вызов на том же id). Не подключён ни к одному live
  триггеру (как и раньше, #[allow(dead_code)] — задел под будущий
  click-away).
```

## Доказательство постоянных window bounds при drag

- `rail_window_options`/`content_window_options` задают `window_bounds`
  один раз при `cx.open_window(...)`; больше НИ ОДНА строка в
  `mod.rs`/`view.rs`/`rail_view.rs` не вызывает `window.resize()` —
  грепом подтверждено: `grep -rn "\.resize(" crates/app/src/side_panel_right/`
  даёт ноль совпадений.
- `update_resize` (view.rs) — единственное место, куда прилетает
  `current_x` из drag — оперирует только `SidePanelRightState.width`
  (`state.resize(new_w)`), окно не трогает вообще.
- Юнит-тесты `drag_left_grows_width_by_exact_delta`,
  `drag_right_shrinks_width_by_exact_delta`,
  `drag_is_deterministic_for_repeated_identical_input`,
  `drag_target_clamps_to_both_bounds` (mod.rs) доказывают чистую функцию;
  `update_resize_moves_width_by_the_drag_delta` (view.rs) доказывает, что
  метод класса даёт тот же результат end-to-end через `Context`.
- **Не проверено живьём**: сам факт, что Hyprland действительно не
  реконфигурирует ни одну Wayland-поверхность во время драга — это
  логическое следствие отсутствия `window.resize()` в коде, а не
  измерение. Требует `hyprctl layers` / `wf-recorder` по чек-листу задачи
  (пункты 2–4).

## Доказательство input region

- `content_input_region(canvas_w, canvas_h, visible_w)` (mod.rs) — чистая
  функция, три теста: пусто при `visible_w=0`, прямоугольник
  прижат к правому краю canvas при частичном открытии, полный canvas при
  `visible_w=CONTENT_CANVAS_WIDTH`.
- `view.rs::render` вызывает `window.set_input_region(Some(&regions))`
  ровно когда `last_visible_width` изменился (кэш, как и было для
  `last_exclusive_zone`); `Some(&[])` (regions пуст) — подтверждённая по
  исходнику форка семантика «вход запрещён везде»
  (`Source/gpui_linux/src/linux/wayland/window.rs:1931-1955`,
  прочитано перед реализацией, не угадано).
- **Не проверено живьём**: реальный клик в пустую часть canvas должен
  долетать до окна ПОД панелью (Hyprland/приложение позади) — чек-лист
  пункт 5.

## Точные команды и результаты тестов

```
$ cargo check -p chronos --lib
    Finished `dev` profile [unoptimized + debuginfo] target(s)
(0 ошибок)

$ cargo test -p chronos --lib
test result: ok. 330 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

$ cargo test -p chronos --bins
test result: ok. 554 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

$ cargo build --release -p chronos
    Finished `release` profile [optimized] target(s)
(exit code 0)

$ cd Source && git status --short
 (пусто по gpui_linux/window.rs — откат чистый; assets/ и
 gpui/examples/popup_grab_repro.rs — чужие untracked-файлы, не тронуты)
```

## Что НЕ проверено живьём (владелец должен прогнать чек-лист задачи)

Геометрия после правки Архитектора проверена на свежем release-бинарнике:
`content x=1600 y=32 w=920`, `rail x=2520 y=32 w=40`. Следовательно,
`content.right == rail.x == 2520`, обе поверхности начинаются под bar на
`y=32`; `grim` подтверждает, что content больше не закрывает правую часть bar.

1. Rail виден на правой кромке, запускает все вкладки — проверено владельцем.
2. Плавное сужение без мельканий — проверено владельцем, `+`.
3. Быстрые рывки в обе стороны — проверено владельцем, `+`.
4. Длинный drag до rail-only clamp и возврат тем же жестом — проверено
   владельцем, `+`.
5. Overlay не блокирует клики в прозрачной части canvas.
6. Геометрия dock-режима и его reserved width ещё не измерена.
7. Toggle/peek/pin/cursor-transition rail↔content без ghost/orphan
   surfaces (`hyprctl layers`).
8. `select-tab:preview` фокус/клавиатурный ввод.

## Commit

Создан после live-проверки, финального `+` и явного разрешения владельца:
`side_panel_right: split rail from fixed content canvas (T276)`.
Чужой dirty worktree не включался и не очищался.
