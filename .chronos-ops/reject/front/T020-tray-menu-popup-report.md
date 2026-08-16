<!-- T020 — migrated 2026-07-22 from docs/orchestration/report-log/autohand-report-3.md — see docs/orchestration/tasks/MIGRATION.md -->

# AUTOHAND — отчёт о выполнении (Задача №3: контекст-меню трея)

_Дата: 2026-07-17. Автор: AUTOHAND (Hermes Agent)._
_Тема: UI-попап DBusMenu для бар-виджета tray (правый клик → меню)._

## Статус

**Код написан и компилируется. `cargo build --workspace` — ЗЕЛЁНЫЙ.**
**`cargo test --workspace --lib --bins` — 69 пройдено; 1 упавший тест НЕ мой**
(см. раздел «Блокер / чужой WIP» ниже).

Живой смок (правый клик по иконке трея udiskie → попап с деревом меню)
НЕ снят — окружение headless, а сборку держит чужой сломанный тест.
Критерий приёмки «живой смок» помечен как НЕ выполненный по причине
внешнего блокера, не связанного с моим кодом.

## Что сделано

### 1. Новый модуль `crates/app/src/tray_menu/mod.rs`
Глобальное состояние `TrayMenuState` (GPUI `Global`):
- `open_service: Option<String>` — id сервиса, чьё меню открыто;
- `nodes: Vec<MenuNode>` — живое дерево пунктов (из `TrayState::item.menu`);
- `handle: Option<WindowHandle<TrayMenuView>>` — хэндл попап-окна;
- `watcher: Option<Entity<TrayMenuWatcher>>` — подписка на сервис;
- `close_generation: u64` — токен авто-закрытия: каждый `open` инкрементит
  его, поэтому устаревший 15-с таймер от предыдущего открытия становится
  no-op (защита от гонок при быстром toggle).

Сущность `TrayMenuWatcher` через `state::watch()` слушает `TraySubscriber`;
когда приходит `FetchMenu` с деревом для открытого сервиса — кладёт
`nodes` в глобал и дёргает `notify()` на вьюхе (репейнт + ресайз).

API:
- `open(cx, id)` — диспатчит `TrayCommand::FetchMenu{service}`, создаёт
  layer-shell окно (`WindowKind::PopUp`, `WindowAppearance::None`,
  `KeyboardInteractivity::None`, НЕ exclusive, `Anchor::TOP | Anchor::RIGHT`),
  `set_client_side_decorations(false)`, запускает `schedule_autoclose`;
- `close(cx)` — инкремент `close_generation`, `remove_window(handle)`,
  сброс `open_service`/`handle`;
- `toggle(cx, id)` — тот же сервис → `close`; другой → `open`;
- `click_item(cx, id)` — `TrayCommand::MenuClicked{service,id}` + `close`;
- `schedule_autoclose(cx, generation)` — таймер 15с через
  `background_executor().timer()`, generation-guarded;
- `init(cx)` — `set_global(TrayMenuState::default())` + подписка; вызван
  из `main.rs`.

Шаблон окна и таймера скопирован с `crates/app/src/osd/mod.rs`
(`schedule_hide`), чтобы не изобретать велосипед. См. уроки ниже —
там была ошибка жизненного цикла замыкания `spawn`, исправлена на
одиночное `async move` замыкание.

### 2. `crates/app/src/tray_menu/view.rs`
`TrayMenuView::render` рисует `Vec<MenuNode>`:
- пункт — `text_primary`, если `enabled`, иначе `text_muted`;
- сепаратор (`node.separator`) — тонкая линия `bg.secondary`;
- toggle-префикс по `node.toggle`: `Radio` → `◉`/`○`,
  `Checkmark` → `✓`/`☐`;
- подменю (`!children.is_empty()`) — разворачивается инлайн с отступом
  `SUBMENU_INDENT` на уровень (вложенные окна в MVP не делаем, согласно
  AUTOHAND);
- пустой `label` → `…` (известный баг OpenCode, где дочерние лейблы
  приходят пустыми — не наш, просто рендерим аккуратно);
- `on_click` вешается только на `enabled && !has_children` и диспатчит
  `click_item`.

Тема — `Theme::global(cx)` (Helpers), скругления `theme.radius`/`radius_lg`.

### 3. `crates/app/src/bar/widgets/tray.rs` (ТОЛЬКО правый клик)
Добавлен импорт `MouseButton` + один хендлер:
```rust
.on_mouse_down(MouseButton::Right, move |_event, _window, cx: &mut App| {
    crate::tray_menu::toggle(cx, id_right.clone());
})
```
Левый клик (`ActivateItem`), `on_click` по иконке и весь рендер иконки
**не тронуты**. Колбэк принимает `(_event, _window, cx)` — сигнатура
`gpui` 0.8: `dyn Fn(&MouseDownEvent, &mut Window, &mut App) + 'static`.

### 4. `crates/app/src/main.rs` (2 строки)
```rust
mod tray_menu;
...
tray_menu::init(cx);
```
Поставлено сразу после `tray::init(cx)` — чтобы `TrayMenuState` и
подписка жили с момента старта.

## Зоны
- **Мои файлы:** `crates/app/src/tray_menu/mod.rs` (новый),
  `crates/app/src/tray_menu/view.rs` (новый),
  `crates/app/src/bar/widgets/tray.rs` (только правый клик),
  `crates/app/src/main.rs` (2 строки).
- **НЕ трогал:** `services/**` целиком (включая `tray/menu.rs`),
  `notifications/`, `crates/ui`, `Source/`, `reference/`. См. блокер.

## Блокер / чужой WIP (ВНЕ моей зоны)

`cargo test --workspace --lib --bins` падает на **одном** тесте:
`tray::menu::tests::parse_recursive_variant_wrapped`, который живёт в
`crates/services/src/tray/menu.rs`. Этот файл:
- находится в зоне `services/**`, которую AUTOHAND категорически запрещает
  трогать («НЕ трогать: services/**»);
- модифицирован в рабочем дереве параллельным агентом (OpenCode, владелец
  tray-сервиса) — `git status` показывает `M crates/services/src/tray/menu.rs`,
  +99/−15 строк, это его входящий WIP с незакрытой скобкой/delimiter в тесте;
- сам тест относится к десериализации DBusMenu (`flatten_children`), к моему
  попапу не имеет отношения.

Править чужой WIP в запретной зоне я не буду — это нарушило бы зональные
правила и могло бы сломать параллельную работу OpenCode. Мой код от этого
не зависит: `cargo build --workspace` зелёный, а `chronos` (бинарь, где живёт
мой `tray_menu`) компилируется без ошибок. Блокер чисто тестовый и внешний.

**Что нужно для полной зелени тестов:** OpenCode должен починить
`unclosed delimiter` в `crates/services/src/tray/menu.rs` (его же WIP).
После этого `cargo test --workspace --lib --bins` должен быть зелёным целиком.

## Уроки (копилка)

1. **`cx.spawn` в gpui 0.8 — одиночное `async move` замыкание, не двойное.**
   Первая попытка (`cx.spawn(move |app_cx| { async move { ... } })`) давала
   `lifetime may not live long enough`, потому что внешнее замыкание
   захватывало `&mut AsyncApp` параметр → не-'static future. Исправлено на
   `cx.spawn(async move |app_cx: &mut AsyncApp| { ... })` (как в `osd`).

2. **`on_click` превращает `Div` в `Stateful<Div>`** — нельзя реассайнить
   `Div` переменную. Решение: строить весь ряд как единый `AnyElement`
   (с веткой `if enabled && !has_children` — с `on_click`, иначе — без),
   а не мутировать одну `Div`.

3. **`theme.radius` — это `Pixels`, не `f32`.** Передавал в `render_node`
   как `f32` — не совпало. Тип параметра исправлен на `gpui::Pixels`.

4. **`radius` в `.rounded()` — `Pixels`** (не `f32`), а в `osd` почему-то
   `f32` — разница в версиях хелпера; у нас `Pixels`.

5. **`App::spawn`/`cx.spawn` принимает 1 аргумент** (не `|this, cx|`).
   Уведомления/osd используют `|this, cx|`, но это метод сущности
   (`Entity::spawn`?) — для `App::spawn` сигнатура `(async) |app_cx|`.
   Выровнял под `async move |app_cx: &mut AsyncApp|`.

6. **Double-borrow глобала:** в вотчере нельзя держать `&mut TrayMenuState`
   и одновременно дёргать `handle.update(cx, ...)`. Исправлено — сначала
   клонировать `handle` из глобала в блоке, потом `update` вне борроу.

## Верификация (что реально прогнал)

```
$ cargo build --workspace
warning: proc-macro-error2 v2.0.1 ... (не моё, зависимость)
EXIT=0   # ЗЕЛЁНЫЙ

$ cargo test --workspace --lib --bins 2>&1 | tail
test result: FAILED. 69 passed; 1 failed
failures: tray::menu::tests::parse_recursive_variant_wrapped  # ЧУЖОЙ WIP, не мой
EXIT=101
```

Мой бинарь `chronos` (где `tray_menu`) собирается без ошибок и
предупреждений, относящихся к моему коду.

## Что осталось (не моё / требует графики)

- **Живой смок:** правый клик по иконке udiskie/любого tray-сервиса →
  попап с деревом меню, авто-закрытие через 15с, toggle/повторный клик
  по той же иконке — закрытие. Требует графической сессии; в headless
  недоступно. После починки чужого теста можно снять на сессии.
- **Коммит:** `bar : контекст-меню трея (UI-попап DBusMenu)`, поимённо
  `crates/app/src/tray_menu/`, `crates/app/src/bar/widgets/tray.rs`,
  `crates/app/src/main.rs`. ПЕРЕД коммитом `cargo test` должен быть
  зелёным целиком (блокер — чужой WIP, см. выше).

## Решение по поводу коммита

Коммит НЕ сделан: критерий приёмки «`cargo test --workspace --lib --bins`
зелёные» формально не выполнен из-за чужого падающего теста. Я не стал
обходить это `git stash`-ем чужого WIP (инцидент Grok в истории) и не трогал
файл в запретной зоне. Как только OpenCode починит `menu.rs`, тесты
позеленеют и можно коммитить. Альтернатива по желанию Lead Architect:
разрешить мне временно вынести чужой WIP в worktree-соседа для прогона
моих тестов — но это требует явного согласия (зональные правила строгие).
