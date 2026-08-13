# T263 — контекст-меню (tray + dock): позиционирование у курсора/виджета
# + пересверка визуала с эталоном живьём

**Приоритет:** P1 — T260/T260-wave2 приняты по коду/тестам, но живая
проверка (2026-08-12, архитектор) показала расхождение с каноном по ВСЕМ
трём осям: позиционирование, визуал, контент-модель (иконки/шорткаты).
**Роль:** FRONTEND + SERVICES (см. Часть 3 — контент-модель, зона
`crates/services/src/tray/`).
**Эталон:** `docs/design/Chronos-Context-Menu.dc (1).html` (CANON) —
единственный источник истины, сверяться с ним пиксель-в-пиксель, не по
памяти/скиллам (они и так признаны устаревшими, см. T260-wave2 отчёт).

**Живые прогоны разблокированы (2026-08-13).** Причина смерти ввода найдена, и она внешняя: незавершённая Wayland drag-сессия при drag-out из Chronos-FM (`T270`, фикс принят статически в `Source`). Гипотеза popup-grab опровергнута, T264 закрыт. Единственное ограничение до живой проверки T270 — не перетаскивать файлы из Chronos-FM во время сессии.

**Дефект, пойманный живьём 2026-08-12 (кадр архитектора):** меню трея рисуется
СТОКОВОЙ палитрой gpui-component — плоский почти-чёрный прямоугольник рядом с
попапом обновлений, который использует токены ChronOS. Причина не в разметке
меню: с переходом на `gpui-component::PopupMenu` цвета берутся из глобальной
темы gpui-component, а мы синхронизируем её только по режиму, шрифтам и
подсветке активной строки — см. `crates/app/src/theme_config.rs:113-140`, там
прямым текстом «We don't map tokens 1:1 (that's a separate concern)». Чинить
там же: замапить `popover`, `popover_foreground`, `accent`, `border`,
`muted_foreground`, selection-токены из `chronos_ui::Theme` в
`gpui_component::theme::Theme::global_mut(cx)` внутри
`sync_gpui_component_theme`, чтобы правка досталась разом всем
component-виджетам, а не одному меню. Проверять кадром рядом с попапом
обновлений — они должны читаться как одна система.

**Решение архитектора по submenu-блокеру (2026-08-12):** требования «использовать
`gpui-component::PopupMenu`» и «canonical side-by-side submenu» совместимы.
Основной путь — считать размер popup-surface на открытии по всему уже
полученному дереву меню (layout фетчится целиком до показа) с запасом под
самое широкое submenu; surface прозрачный вне карточки, клик по пустой зоне
закрывает меню своим обработчиком. Если замер окажется нереальным — запасной
путь: отдельная surface на submenu, форк это тянет, popup парентится к
`parent.xdg_surface()` (`Source/gpui_linux/src/linux/wayland/window.rs:215-224`).

## Почему этот тикет существует — живая улика

Архитектор поднял живой шелл (release-бинарь, собран 2026-08-12 00:10:43,
**после** коммита `e793964` T260-wave2 — свежая сборка, не устаревшая) и
снял 4 живых кадра правым кликом по разным трей-иконкам (easyeffects,
vivaldi, steam — `grim`, не выдумка, PID/mtime бинаря сверены).

**Все четыре кадра идентичны по проблеме:**

1. **Позиционирование:** меню каждый раз открывается **в одном и том же
   месте** (верхний правый угол экрана, чуть ниже бара) — независимо от
   того, какая трей-иконка была кликнута и где она физически находится в
   баре. Это буквально совпадает с кодом: `tray_menu/mod.rs::window_options`
   ставит layer-shell `Anchor::TOP | Anchor::RIGHT` с фиксированным
   `margin` — иконка вообще не участвует в расчёте позиции.
2. **Визуал:** ни на одном кадре нет ни намёка на эталон — плоский
   прямоугольник с острыми/едва скруглёнными углами, обычный sans-serif
   (не `font_family(theme.font_mono)` для шорткатов, как в эталоне
   `.ci-short`), никакого различимого 2px акцент-бара слева от строк
   (даже с поправкой на то, что в rest-состоянии он `opacity 0` — в
   эталоне есть `sticky`-заголовок с `border-bottom`, скруглённая
   `min-width:230px` карточка с тенью — этого тоже не видно), никакой
   заметной подложки/блюра, различимого от плоского фона бара. T260-wave2
   отчёт заявляет: акцент-бар, `transition_when_else`, кастомный
   overlay-скроллбар, sticky-заголовок — на живых кадрах этого нет.

**Это прямое противоречие приёмке T260-wave2.** Та приёмка была основана
на `cargo check`/`cargo test`/статичном ревью диффа — отчёт сам честно
писал «живые кадры меню НЕ сняты» (см.
`docs/orchestration/tasks/report-log/T260-wave2-context-menu-enter-accent-report.md`),
и приёмка это приняла как открытый долг, а не как незакрытый риск. Теперь
живые кадры есть, и они долг не подтверждают, а опровергают.

**Первое действие по этому тикету — не позиционирование, а диагностика:**
объяснить, почему код `tray_menu/view.rs` (акцент-бар/wash/скроллбар из
T260-wave2) не проявляется в реально запущенном бинаре, собранном ПОСЛЕ
этого коммита. Гипотезы для проверки (не готовые ответы):
- Рендерится другая ветка/другой `Render` impl (не тот, что читал
  архитектор при code review T260-wave2)?
- Тема/`cx.theme()` в момент рендера меню возвращает не тот палет
  (заглушка/дефолт), из-за чего цвета сливаются с фоном?
- Строки в этих трёх примерах (easyeffects/vivaldi/steam) — все
  `enabled` без `nav`/`hover`, поэтому акцент-бар реально невидим
  (opacity 0) — это ожидаемо и НЕ баг; но тогда нужен кадр с `hover`
  (по позиции курсора после фикса анкора) чтобы подтвердить хотя бы это.
- Карточка (радиус/тень/`sticky`-заголовок) — это не per-row стиль, а
  стиль контейнера всего меню. Проверить, применяется ли он вообще к
  layer-shell окну меню (`WindowBackgroundAppearance::Transparent` +
  скругление у корневого `div()` — смотреть `tray_menu/view.rs` render
  корневого контейнера, не только строк).

## Часть 1 — позиционирование у курсора/виджета (факт из канона)

Канон (`docs/design/Chronos-Context-Menu.dc (1).html`) сам задаёт
позиционирование через JS, это не домысел архитектора:

```js
function positionRoot(el, x, y){          // строка 525
  var w = el.offsetWidth, h = el.offsetHeight;
  var left = x, top = y;                   // курсор клика — стартовая точка
  if(left + w > window.innerWidth - 8) left = window.innerWidth - w - 8;
  if(top + h > window.innerHeight - 8) top = window.innerHeight - h - 8;
  if(left < 8) left = 8;
  if(top < 8) top = 8;
  el.style.left = left + 'px'; el.style.top = top + 'px';
}
function positionSub(el, parentRect){      // строка 536 — субменю у родительской строки
  var left = parentRect.right + 4;
  if(left + w > window.innerWidth - 8) left = parentRect.left - w - 4;
  ...
}
```

Открытие: `openRoot(key, x, y)` — вызывается с координатами клика (строка
547), `openSub(rowEl, ...)` — с `rowEl.getBoundingClientRect()` (строка
566). **Ни разу фиксированный угол экрана.**

### Как это сделать в форке — готовый прецедент в дереве

`AnchoredPopup` (`WindowKind::AnchoredPopup`, скилл `anchored-popups`) уже
используется тремя попапами в этом дереве:
`crates/app/src/volume_popup/mod.rs`, `updates_popup/mod.rs`,
`history_popup/mod.rs` — паттерн:

1. Виджет-триггер захватывает свой `Bounds<Pixels>` через
   `canvas(...)` + `Rc<Cell<Bounds<Pixels>>>` (образец —
   `crates/app/src/bar/widgets/volume.rs:108-149`).
2. По `on_mouse_down` (не `on_click` — форк требует grab на mouse-down,
   см. скилл `anchored-popups` «The Grab Rules») читает `bounds_cell.get()`
   и зовёт `open(cx, anchor_rect, window.window_handle())`.
3. `window_options` строит `WindowKind::AnchoredPopup(PopupOptions {
   parent, anchor_rect, anchor: PopupAnchor::BottomLeft/BottomRight,
   gravity: ..., constraint_adjustment: SLIDE_X|FLIP_X, offset, grab:
   true })` — образец целиком в `volume_popup/mod.rs:103-130`.
4. `cx.open_window` — если `PopupNotSupportedError`, фолбэк на текущий
   fixed-corner `WindowKind::LayerShell` (тот же код, что уже есть в
   `tray_menu`/`dock/context_menu` сегодня — не выкидывать, оставить как
   fallback-ветку, образец `volume_popup/mod.rs:145-155`).

### Зона правок — позиционирование

- `crates/app/src/tray_menu/mod.rs` — `window_options` получает
  `anchor_rect`+`parent` вместо только `display_id`+`height`; добавить
  `fallback_window_options` (переименовать текущую `window_options` в
  неё) по образцу `volume_popup`.
- `crates/app/src/bar/widgets/tray.rs` — захватить bounds иконки
  (`canvas`+`Cell`, см. `line 88` текущий `on_mouse_down(MouseButton::Right, ...)`
  — уже mouse-down, менять только на передачу `anchor_rect`+`parent`).
- `crates/app/src/dock/context_menu.rs` — то же самое, `anchor:
  PopupAnchor::TopCenter`/`BottomCenter` в зависимости от того, где
  физически дозиционируется dock-иконка на экране (dock — обычно
  внизу/сбоку, сверить с текущей `Anchor::TOP` — вероятно нужен
  `PopupAnchor::Top` + `gravity: Bottom`, но это решает исполнитель по
  факту геометрии дока, не архитектор).
- `crates/app/src/bar/widgets/dock.rs` — захватить bounds dock-иконки
  (строка 130, сейчас `on_mouse_down(MouseButton::Right, move |_event,
  _window, cx: &mut App| { ... })` — не хватает `window`/`bounds`, нужен
  тот же `canvas`+`Cell` паттерн что и в `tray.rs`/`volume.rs`).
- Субменю (если/когда появятся у dock/tray) — `positionSub`-эквивалент,
  анкор от `Bounds` родительской строки, не от корня меню. Если субменю
  нет в текущей реализации — не изобретать, зафиксировать как
  «субменю вне зоны T263» в отчёте.

## Часть 1.5 — не писать рендер руками, `gpui-component` уже несёт это

**Проверено в дереве:** `gpui-component` (`../Source/gpui-component`, наш
форк, `crates/app/Cargo.toml:28` — уже реальная зависимость `chronos`,
используется в `side_panel_right`) содержит готовый
`crates/ui/src/menu/popup_menu.rs` (`PopupMenu`/`PopupMenuItem`, 1387
строк) — иконка (`.icon()`), чекбокс/радио (`.checked()` +
`check_side()`), сепаратор (`PopupMenuItem::separator()`), **submenu со
своим анкором** (`PopupMenuItem::submenu(label, Entity<PopupMenu>)`,
`submenu_anchor: (Anchor, Pixels)`), disabled (`.disabled()`) — то есть
ровно модель эталона (`MENUS.*.items[].{icon,shortcut,sub,check,disabled}`)
почти один в один. Модуль `menu` в `crates/ui/Cargo.toml:17-28` **не
гейтится фичами** (`pub mod menu;` безусловно, `lib.rs:53`) — уже
скомпилирован в бинарь `chronos`, предельные затраты на переиспользование
малы (в отличие от `Input`, который платит +1.74 MiB — см. скилл
`gpui-component-in-chronos`, там же матрица приёмки фич).

**Ловушка Linux/Wayland (прочитано в `native_menu/fallback.rs:1-4`,
буквально в докстроке файла):** штатный путь `PopupMenu` рисуется через
`anchored()`+`deferred()` **внутри того же окна** — на Linux (`fallback.rs`,
в отличие от `macos.rs`/`windows.rs` с реальным native popup) это
**обрезается границами хост-окна**. Наш бар — 32px layer-shell полоска;
хостить `PopupMenu` прямо в окне бара means clip на 32px, бесполезно.

**Решение — не переизобретать, а совместить с уже принятым паттерном
T263 Часть 1:** окно под меню мы и так открываем отдельным
`AnchoredPopup`/`LayerShell`, размером под контент (как `volume_popup`
уже делает). Внутри ЭТОГО окна вместо кастомных `div()`-строк
(`tray_menu/view.rs`, `dock/context_menu.rs`) — хостить
`gpui_component::Root::new(popup_menu_entity, window, cx)` +
`PopupMenu::build(window, cx, |menu, window, cx| { ... })`, собранный
рекурсивным обходом `MenuNode` (`icon_name`/`shortcut` из Части 3,
`toggle` → `.checked()`, `children` → `.submenu()`).

Два условия из скилла `gpui-component-in-chronos` (уже проверены на
`Input`, обязаны так же сработать здесь):
1. Окно ОБЯЗАНО быть `WindowHandle<Root>` — без обёртки в
   `gpui_component::Root` паника на `window.root()`.
2. Если нужна клавиатурная навигация по пунктам (стрелки/Enter — сверить,
   даёт ли это эталон; в `.dc (1).html` есть `navIdx`/`paintNav()` —
   значит да) — `KeyboardInteractivity::OnDemand`, не `None` (текущий
   `tray_menu/mod.rs` использует `None` осознанно, с комментарием «no
   Escape handling — rare popup, mouse only»; это решение придётся
   пересмотреть, если клавиатурная навигация из канона обязательна).

Живой образец полного цикла (сборка меню из данных + submenu-анкор) —
`native_menu/fallback.rs::build_popup` (рекурсивная сборка) и
`crates/story/src/stories/menu_story.rs` (готовый рабочий `PopupMenu` в
дереве, включая `ContextMenuExt`/`DropdownMenu`). Смотреть их, не
изобретать API по памяти.

**Открытый вопрос — решает исполнитель по факту, не архитектор:** тема.
`sync_gpui_component_theme` (`crates/app/src/theme_config.rs`) уже
синхронизирует палитру ChronOS → `gpui_component::Theme` для
`side_panel_right`; проверить, что `PopupMenu` реально забирает эту же
тему (а не дефолтный скин компонента) — если нет, визуал снова разойдётся
с каноном, и это не обнаружится, пока не сверено живым кадром.

## Часть 2 — визуальная пересверка (после диагностики из вступления)

- Открыть оба меню живьём **после фикса позиционирования** (иначе кадр
  снова уедет в угол и сравнение с каноном будет некорректным).
- `grim -g` по геометрии из `hyprctl layers` — минимум 3 состояния на
  каждое меню: rest, hover одной строки, disabled-строка (если есть в
  тестовом наборе — easyeffects подходит, там явно disabled-пункты
  вида `...`).
- Сверить бок-о-бок с `docs/design/Chronos-Context-Menu.dc (1).html`,
  открытым в браузере рядом (не по памяти): радиус карточки, тень,
  ширина `230-300px` из эталона (`min-width:230px;max-width:300px` —
  сверить с текущим `MENU_WIDTH`), sticky-заголовок с `border-bottom`,
  hairline-разделители, `ci-short` моно-шрифт для шорткатов, `ci-arrow`
  для пунктов с субменю.
- Если что-то из T260-wave2 (акцент-бар/wash/скроллбар) на живых кадрах
  всё ещё не видно после диагностики — не молчать, зафиксировать как
  отдельную находку в отчёте, не пытаться чинить вслепую без понимания
  причины.

## Часть 3 — контент-модель: иконки/шорткаты не долетают с шины (bug, не UI)

Эталон кодирует на каждый пункт меню (`var MENUS`, строка 324):
`icon` (напр. `{label:'Cut', icon:'cut', shortcut:'⌃X'}`), `shortcut`
(глиф вида `⌃X`/`↵`/`F2`), заголовок меню с иконкой приложения
(`head:'Music Player', headIcon:'music'`), чекбокс/радио
(`check:true, checked:…, radio:true` — трей: `Set Status` → 4 радио-пункта
Online/Idle/DND/Invisible), подменю (`sub:[...]`, до 2 уровней вложенности
в примерах).

Сверка с бэкендом — **иконки и шорткаты физически не запрашиваются с
D-Bus**, это не баг рендера:

```rust
// crates/services/src/tray/menu.rs:35-36 — GetLayout property filter
let names: Vec<&str> = vec![
    "label", "enabled", "visible", "type",
    "toggle-type", "toggle-state", "children-display",
];
```

Протокол `com.canonical.dbusmenu` (`GetLayout`) поддерживает `icon-name`
(и `icon-data` для inline-пикселей) и `shortcut` как стандартные
свойства — здесь они не в списке, соответственно `MenuNode`
(`crates/services/src/tray/types.rs:135-148`) их не хранит вообще (только
`id/label/enabled/visible/separator/toggle/children`). `toggle`
(checkbox/radio) и `children` (подменю) — уже есть, это единственное из
модели эталона, что потенциально уже долетает и может просто не
рендериться (проверить отдельно от иконок/шорткатов).

Заголовок меню (`head`+`headIcon` — имя+иконка приложения-владельца трея)
в модели вообще не предусмотрен ни на уровне `MenuNode`, ни, вероятно, на
уровне `TrayItem` для меню-контекста — сверить, есть ли уже где-то
рядом (`TrayItem.icon_name`/`label` в `tray/mod.rs:233-252` относятся к
самой иконке в баре, не к заголовку popup-меню) и переиспользовать, а не
дублировать поле.

### Зона правок — контент-модель

- `crates/services/src/tray/menu.rs` — добавить `"icon-name", "shortcut"`
  в `names` (property filter `GetLayout`); распарсить в `MenuNode`.
- `crates/services/src/tray/types.rs` — `MenuNode` получает
  `icon_name: Option<String>`, `shortcut: Option<String>` (сырой
  DBusMenu-формат вида `[["Control","X"]]` — уточнить точный формат по
  спеке `com.canonical.dbusmenu`, сконвертировать в отображаемый глиф
  типа `⌃X` при рендере, не хранить уже сконвертированным в сервисе).
- `crates/app/src/tray_menu/view.rs` — рендер иконки слева от `label`
  (если `icon_name` есть; freedesktop icon-theme lookup — проверить, есть
  ли уже готовый резолвер иконок в дереве, не писать заново, до dock/
  launcher наверняка есть похожая логика для app-иконок), шортката
  моно-шрифтом справа (`ci-short` эталона), радио-точка вместо чекбокса
  когда `toggle.0 == Radio`.
- Заголовок меню (`head`/`headIcon`) — по факту наличия данных на уровне
  `TrayItem`; если для DBusMenu-трея данных о заголовке нет вообще
  (протокол их не даёт отдельно от самого меню) — зафиксировать как
  ограничение протокола в отчёте, не выдумывать источник данных.

## Не трогать

`crates/app/src/volume_popup/**`, `updates_popup/**`, `history_popup/**`
(уже приняты, рабочий прецедент — только читать, не менять),
`crates/app/src/launcher/**`, `crates/app/src/osd/**` (T262 — отдельная
задача, не пересекаться).

## Коммит

`tray_menu/dock : anchor-меню у курсора/виджета (T263) + визуал по канону`.
Без AI-трейлеров — архитектор проверяет `git show --format=full`, не
тело сообщения (прецедент T262 — уже ловили `Co-Authored-By` в git-
метаданных при чистом теле сообщения).

## Отчёт

`docs/orchestration/tasks/report/T263-context-menus-anchor-and-visual-fidelity-report.md`.
Обязательно: (1) объяснение диагностики из вступления — почему
T260-wave2-стиль не проявлялся живьём; (2) таблица «эталон vs живой код»
по образцу первого захода T262 (тот отчёт задал хороший формат); (3)
живые кадры ДО и ПОСЛЕ фикса позиционирования, минимум 2 разные
трей-иконки в разных x-позициях бара — доказать, что меню теперь
реально следует за иконкой, а не просто переехало в другой
фиксированный угол.
