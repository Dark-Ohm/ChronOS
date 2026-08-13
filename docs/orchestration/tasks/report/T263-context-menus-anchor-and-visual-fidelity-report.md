# T263 — контекст-меню tray/dock: промежуточный отчёт

**Дата:** 2026-08-12  
**Статус:** НЕ ПРИНЯТО / статические P1 исправлены, live-smoke заблокирован  
**Канон:** `docs/design/Chronos-Context-Menu.dc (1).html`

## Результат на момент остановки

### Коррекция после независимого review (2026-08-12)

Первоначальная диагностика ниже была неполной: пустой Steam popup объяснялся
не только гонкой первого fetch. `GetLayout(0, …)` возвращает synthetic root,
а сервис отдавал его view как обычный узел. View превращал пустой root с
children в submenu `…`; движение мыши лишь раскрывало настоящие пункты.

Исправлено и покрыто red→green тестами:

- synthetic root разворачивается в top-level children;
- integer `toggle-state` DBusMenu (`0/1`) корректно даёт checked state;
- compositor `PopupDone` очищает stale window/service state tray и dock;
- смена dock-иконки пересоздаёт popup ради нового creation-time anchor;
- checked custom rows сохраняют checkmark, shortcut сохраняет все комбинации;
- submenu children больше не раздувают высоту root surface.

Открытый блокер: `gpui-component::PopupMenu` размещает submenu как anchored
element внутри 300px host-window. Каноническое side-by-side submenu за
границей root-карточки туда физически не помещается; без отдельной popup
surface получится clipping либо наложение. Тикет одновременно требует
использовать component renderer и повторить HTML `positionSub`; текущий Linux
fallback API эти требования не совмещает. Коммитить/принимать работу до
решения этого конфликта и live-smoke нельзя.

Во время автоматического live-smoke синтетический right-click через ydotool
сломал обработку физических кликов Hyprland. Запущенный тестовый ChronOS
остановлен, все его surfaces исчезли, все кнопки uinput принудительно
отпущены, `ydotoold` перезапущен; клики не восстановились, пользователь
решил перезагрузить систему позже. До reboot любые новые GUI-прогоны и
синтетические клики запрещены.

В рабочем дереве реализованы anchor-aware окна для tray и dock,
`gpui-component::PopupMenu` внутри отдельного `gpui_component::Root`, поля
DBusMenu `icon-name`/`shortcut`, общий резолвер иконок и рендер checkbox/radio,
shortcut и submenu. Дополнительно исправлена гонка, которую архитектор поймал
на Steam: popup сначала показывал `label empty`, а после движения мыши внезапно
получал содержимое.

Задача не закрыта. Обязательные живые кадры двух tray-иконок и dock после
последней правки не получены. После всех review-фиксов повторно прошли полный
lib-suite, `cargo check` и release-сборка; бинарь намеренно не запускался.
Коммита нет.

## Почему визуал T260-wave2 не проявлялся живьём

Это был не другой `Render` и не старая release-сборка. Живой бинарь входил в
тот же `TrayMenuView`, который проверялся в T260-wave2. Расхождение сложилось
из четырёх причин:

1. Старое окно было fixed-corner layer-shell с `Anchor::TOP | Anchor::RIGHT`.
   Курсор после правого клика оставался на tray-иконке, а меню появлялось в
   другом месте. Hover-состояние строки не возникало, поэтому wash и 2px
   accent bar с rest-opacity `0` закономерно не были видны.
2. Кастомный scrollbar рисовался только при переполнении. Короткие меню Steam,
   Vivaldi и EasyEffects его не активировали.
3. Sticky header не имел полноценного источника/реализации в T260-wave2.
   Тот отчёт сам фиксировал отсутствие данных заголовка; живой кадр лишь
   подтвердил долг.
4. Карточка занимала почти точные границы отдельного окна. Внешняя часть тени
   обрезалась границей surface, поэтому карточка выглядела заметно площе
   HTML-канона.

T263 заменяет ручные строки на `gpui-component::PopupMenu`, как требует
тикет. Поэтому утверждения T260-wave2 о собственном accent bar и overlay
scrollbar больше не описывают текущий renderer; окончательная визуальная
оценка должна делаться по новому живому кадру.

## Гонка пустого Steam popup

Наблюдение архитектора: popup Steam открывался с `label empty`; движение мыши
показывало настоящее меню.

Последовательность была такой:

1. `open()` отправлял `TrayCommand::FetchMenu` до создания и регистрации
   `WindowHandle<Root>` и `WeakEntity<TrayMenuView>`.
2. Первый render видел пустой snapshot. Старое сравнение `Vec::new()` с
   `Vec::new()` считало дерево неизменившимся и не строило даже placeholder
   `PopupMenu`.
3. Ответ сервиса мог прийти, пока watcher ещё не имел handle/view. Данные в
   `TrayMenuState.nodes` обновлялись, но notify терялся.
4. Hover вызывал новый render; только тогда view видел готовые nodes и строил
   содержимое.

Исправление в `crates/app/src/tray_menu/view.rs` и
`crates/app/src/tray_menu/mod.rs`:

- `last_nodes` теперь `Option<Vec<MenuNode>>`; первый snapshot, включая
  пустой, всегда строит меню;
- после сохранения нового handle/view выполняется явный notify, закрывающий
  окно гонки `FetchMenu`/`open_window`;
- watcher уведомляет сам `TrayMenuView`, а не контекст корневого `Root`.

Есть unit-проверки первого пустого snapshot и неизменившегося snapshot.

## Эталон vs текущий код

| Ось | HTML-эталон | Текущий код в worktree | Живая проверка |
| --- | --- | --- | --- |
| Корневой anchor | Точка клика с clamp 8px | `WindowKind::AnchoredPopup`, bounds trigger-виджета, slide/flip по X/Y; fixed-corner layer-shell оставлен только fallback | Не подтверждено после последней правки |
| Две tray-иконки | Каждая открывает меню у собственной позиции | При смене service старый xdg-popup теперь закрывается и создаётся заново: anchor является creation-time state | Покрыто unit-решением, живого кадра нет |
| Dock anchor | У trigger/dock icon | Bounds иконки передаются в отдельный anchored popup | Живого кадра нет |
| Контейнер меню | 230–300px, radius, border, shadow | Окно 300px; внутри `gpui_component::Root` и `PopupMenu` | Требует grim-сверки |
| Hover/selection | Wash, 2px accent bar | Hover/selection/focus принадлежат `PopupMenu`; старый ручной T260 renderer удалён | Требует живого hover-кадра |
| Separator | Hairline | `PopupMenuItem::separator()` | Статически проверено |
| Disabled | Приглушённая строка без action | `.disabled()` | Статически проверено; live нет |
| Checkbox/radio | Разные glyph | Check использует `.checked()`; radio рендерит `◉`/`○` в custom row | Unit/compile, live нет |
| Submenu | У родительской строки, flip у края | Рекурсивный `PopupMenu::submenu()` | Собирается; Linux clipping живьём не проверен |
| Иконка строки | `ci-ic` слева | DBusMenu `icon-name` + общий freedesktop resolver + custom row | Парсер покрыт тестом; live нет |
| Shortcut | `ci-short` справа, mono | Сырой DBusMenu `Vec<Vec<String>>` хранится в service и форматируется в view | Парсер/formatter покрыты тестами; live нет |
| Header | Sticky, icon + title, border-bottom | Title берётся из `TrayItem.title`; icon source переиспользует данные tray item | Требует live-сверки |
| Keyboard | Arrow/Enter/Escape | `PopupMenu` + `KeyboardInteractivity::OnDemand` | Live не проверено |

## Контент-модель

`crates/services/src/tray/menu.rs` теперь запрашивает `icon-name` и
`shortcut`. `MenuNode` хранит имя иконки и сырой DBusMenu shortcut как
`Option<Vec<Vec<String>>>`; преобразование в `⌃`/`⌥`/`⇧`/`◆` выполняется
только в app/view. Variant wrappers разворачиваются рекурсивно: для `av` и
`a{sv}` встречаются вложенные `Value::Value`, и одноуровневый unwrap терял
shortcut.

`icon-data` намеренно не добавлялся: тикет требует `icon-name`; новый канал
inline-pixmap без отдельной модели и лимитов был бы расширением scope.

## Проверки

Свежий статический прогон после review-фиксов:

```text
cargo test -p chronos-services tray::menu --lib: 10 passed
cargo test -p chronos tray_menu:: --lib --bins: 16 passed
cargo test -p chronos dock::context_menu:: --lib --bins: 2 passed
cargo test -p chronos --lib: 299 passed
cargo check -p chronos: ok
cargo build --release -p chronos: ok (финальный повтор 2m24s)
git diff --check: ok
```

Release после этого не запускался. Live-верификация остаётся заблокирована
до reboot и безопасного ручного прогона без ydotool.

Выполнено:

```text
cargo test -p chronos-services tray::menu --lib
8 passed

cargo test -p chronos tray_menu:: --lib --bins
13 passed, 0 failed

cargo check -p chronos
ok

cargo build --release -p chronos
ok, но до последней правки fresh-anchor при смене service
```

Ранний полный прогон дал `298 passed, 1 failed` на существовавшем flaky-тесте
`side_panel_right::view::tests::move_tab_helper_persists_reorder_and_updates_cache`.
Свежий повтор после review-фиксов прошёл полностью: `299 passed, 0 failed`;
файлы `side_panel_right` в T263 не менялись.

`cargo fmt --all -- --check` не является чистым baseline: он показывает
массовый существующий формат-дрейф по workspace. `git diff --check` для
текущего diff проходит.

## Live smoke и кадры

Release-бинарь после исправления гонки был собран и запущен foreground; bar
поднялся на DP-1 как layer-shell `2560x32`. Скриншот правой части бара сохранён
в `/tmp/t263-current-right.png`.

Автоматический `hyprctl cursor.move + ydotool right-click` попал в Steam icon,
но не открыл контекстное меню, поэтому `/tmp/t263-steam-rest-fixed.png` не
является доказательством popup и в приёмку не включается. Ручной кадр после
race-fix до остановки работы получить не успели.

Обязательные кадры из тикета отсутствуют:

- ДО: архитекторские исходные grim-кадры описаны в тикете, но их файлов нет в
  рабочем дереве и `/tmp` текущего сеанса;
- ПОСЛЕ: две разные tray-иконки в разных X, rest/hover/disabled;
- ПОСЛЕ: dock rest/hover/disabled.

Без этих кадров визуальная часть T263 не принята. Подменять наблюдение
описанием кода здесь было бы ровно той ошибкой, из-за которой появился тикет.

## Дополнение 2026-08-13 — мапинг палитры gpui-component закрыт статически

Живой дефект из вступления тикета (меню трея стоковой палитрой gpui-component)
— исправлен в `theme_config.rs::sync_gpui_component_theme`: popup-токены
компонента мапятся из shell-темы (`popover` ← `bg.elevated`,
`popover_foreground` ← `text.primary`, `accent` ← `interactive.hover` — у
компонента `accent` это hover-фон MenuItem/ListItem, не наш акцент,
`accent_foreground` ← `text.primary`, `border` ← `border.subtle`,
`muted_foreground` ← `text.muted`, `selection` ← `bg.selection`). Документ
функции обновлён (прежняя фраза «We don't map tokens 1:1» была корнем
дефекта). Тесты: `sync_maps_shell_tokens_into_component_theme_dark/light`.
Полный прогон после правки: lib 306/306, bins 515/515, check чистый.

Файл `crates/app/src/theme_config.rs` добавляется к границам diff T263 ниже.
Визуальный вердикт (меню рядом с попапом обновлений читается как одна
система) — только живым кадром, в той же приёмке.

## Что осталось

0. **Submenu widest-reserve — единственный нереализованный пункт скоупа.**
   Решение архитектора в тикете (основной путь: считать размер popup-surface
   на открытии по всему полученному дереву меню с запасом под самое широкое
   submenu; surface прозрачный вне карточки, клик по пустой зоне закрывает
   меню своим обработчиком) в дереве НЕ применён: `MENU_WIDTH` фиксирован,
   submenu рендерится anchored-элементом внутри 300px host-window (clipping-
   риск из п.5 стоит). Реализовать до живой приёмки — иначе кадры submenu
   заведомо покажут клиппинг.
1. После reboot запустить уже пересобранный release foreground вручную.
2. Ручным правым кликом открыть Steam, не двигать мышь и подтвердить, что
   содержимое видно в первом кадре.
3. Снять rest/hover/disabled для Steam и второй tray-иконки; доказать разные X.
4. Снять dock menu и проверить направление gravity/flip у нижнего края.
5. Проверить submenu на Linux: fallback renderer живёт внутри 300px host
   window, поэтому clipping остаётся реальным риском.
6. Сверить radius/shadow/header/mono shortcut с HTML бок-о-бок.
7. Выполнить финальные check/test/release, оформить task commit без trailers.

## Границы diff

Целевые файлы T263:

- `crates/app/src/bar/widgets/{dock,mod,tray}.rs`
- `crates/app/src/dock/context_menu.rs`
- `crates/app/src/icon_resolution.rs`
- `crates/app/src/tray_menu/{mod,view}.rs`
- `crates/services/src/tray/{menu,types}.rs`

`Cargo.lock` был загрязнён до текущего review: ни один `Cargo.toml` в T263 не
менялся, а lock переключает источники `wgpu`/`xim`/`font-kit` и добавляет
`dirs 5`. Эти чужие изменения нельзя включать в T263 commit и нельзя
самовольно откатывать.

В worktree также есть чужие untracked-файлы (`.workbuddy`, design HTML,
другие task docs и `skills/software-development`). Они не изменялись и не
должны попадать в T263 commit.

---

## Приёмка архитектора (2026-08-13): НЕ ПРИНЯТО — скоуп не выполнен

Причина отказа — не кадры, а код: **пункт 0 (submenu widest-reserve) в
дереве отсутствует**. `MENU_WIDTH` фиксирован (`tray_menu/mod.rs:49` = 300,
`dock/context_menu.rs:33` = 230), решение архитектора из тикета не
применено. Поэтому живой заход сейчас бессмыслен: кадры submenu заведомо
покажут клиппинг, который мы и так предсказали статически.

Всё остальное статически сходится, проверено мной в дереве:
`last_nodes: Option<Vec<MenuNode>>` (`view.rs:78`) — гонка пустого
Steam-попапа закрыта; `PopupDone` обработан (`mod.rs:254`); `icon-name` и
`shortcut` запрашиваются сервисом (`services/tray/menu.rs:40,185`); мапинг
палитры в `theme_config.rs::sync_gpui_component_theme` с тестами
dark/light. Тесты прогнаны мной: services `tray::menu` 10/10,
`tray_menu::` 27/27, `dock::context_menu::` 5/5 — цифры отчёта
подтверждены или превышены.

Отдельно, потому что это редкость: отчёт честный. Исполнитель сам написал,
что кадр от ydotool «не является доказательством popup и в приёмку не
включается», и что подменять наблюдение описанием кода было бы ровно той
ошибкой, из-за которой появился тикет. Так и надо.

Порядок закрытия: пункт 0 в поле → возврат → статика → ОДИН живой заход
(submenu, палитра, anchor, две tray-иконки, dock + кадр бара над светлым
окном для хвоста T267) → коммит T263 (+ T264 в `done/`) → коммит T265-0.
