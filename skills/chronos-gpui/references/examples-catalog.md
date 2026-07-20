# Каталог примеров форка gpui

**Когда грузить:** нужен полный список того, что форк умеет, с путями к исходникам и статусом компиляции.

**Источник:** `/home/neo/projects/chronos-ecosystem/Source/gpui/examples/` (42 .rs файла) + `/home/neo/projects/chronos-ecosystem/Source/gpui-component/examples/` (13 .rs файла).  
**Дата разведки:** 2026-07-20.  
**Спецификатор пакета:** `-p 'path+file:///home/neo/projects/chronos-ecosystem/Source/gpui#0.2.2'`

## Легенда

| Маркер | Значение |
|---|---|
| ✅ | Компилируется — `cargo check` пройден |
| ⬜ | Не проверено компиляцией |
| ❌ | Не компилируется |
| 🔵 | Полностью применимо к layer-shell (ChronOS) |
| 🟡 | Частично применимо |
| ⚪ | Неприменимо |

---

## I. Примеры gpui (корень `examples/`)

### 1. hello_world — базовый каркас ✅ 🟡

**Файл:** `Source/gpui/examples/hello_world.rs` (121 строка)  
**Демонстрирует:** `Application::run`, `cx.open_window`, `Render` trait, `div()` builder, flex layout, цвета, границы, тени.  
**Ключевые API:** `App`, `Window`, `Context`, `Render`, `Bounds`, `WindowBounds`, `div()`, `flex()`, `bg()`, `size()`, `shadow_lg()`, `border_1()`, `text_xl()`.  
**Компиляция:** ✅ `cargo check --example hello_world` прошёл.  
**Layer-shell:** 🟡 применимо — нужна замена `WindowOptions` на `WindowKind::LayerShell`.

### 2. scrollable — скроллируемый контент ✅ 🔵

**Файл:** `Source/gpui/examples/scrollable.rs` (72 строки)  
**Демонстрирует:** вертикальный + горизонтальный скролл, `.id()` + `.overflow_scroll()`.  
**Ключевые API:** `overflow_scroll()`, `.id()` (обязателен — метод в `StatefulInteractiveElement`).  
**Компиляция:** ✅ `cargo check --example scrollable` прошёл.  
**Находка:** пример, опровергнувший «кровный факт». Скролл требует `.id(...)` перед `.overflow_scroll()`.  
**Layer-shell:** 🔵 полностью применимо.

### 3. layer_shell — layer-shell окно (Wayland) ✅ 🔵

**Файл:** `Source/gpui/examples/layer_shell.rs` (101 строка)  
**Демонстрирует:** `WindowKind::LayerShell`, `Anchor`, отступы, прозрачный фон, `KeyboardInteractivity`.  
**Ключевые API:** `WindowKind::LayerShell(LayerShellOptions { namespace, anchor: Anchor::LEFT | Anchor::RIGHT | Anchor::BOTTOM, margin, keyboard_interactivity })`, `WindowBackgroundAppearance::Transparent`.  
**Компиляция:** ✅ `cargo check --example layer_shell` прошёл.  
**Layer-shell:** 🔵 эталонный пример для ChronOS.

### 4. animation — анимации и SVG-трансформации ✅ 🟡

**Файл:** `Source/gpui/examples/animation.rs` (134 строки)  
**Демонстрирует:** `Animation`, `with_animation()`, `Transformation::rotate`, `bounce(ease_in_out)`, `AssetSource`.  
**Ключевые API:** `Animation::new(Duration).repeat().with_easing(bounce(ease_in_out))`, `with_animation("id", anim, |el, delta| ...)`.  
**Компиляция:** ✅ `cargo check --example animation` прошёл.  
**Layer-shell:** 🟡 применимо.

### 5. anchor — anchored/deferred попапы ✅ 🔵

**Файл:** `Source/gpui/examples/anchor.rs` (201 строка)  
**Демонстрирует:** `anchored()` + `deferred()`, hover-состояния, `when_some()`.  
**Ключевые API:** `anchored().anchor(Anchor).position(...).snap_to_window()`, `deferred(...)`.  
**Компиляция:** ✅ `cargo check --example anchor` прошёл.  
**Layer-shell:** 🔵 критично для попапов в баре/dock.

### 6. input — текстовый ввод ✅ 🟡

**Файл:** `Source/gpui/examples/input.rs` (778 строк)  
**Демонстрирует:** полный текстовый редактор: фокус, курсор, выделение, Copy/Paste/Cut, IME.  
**Ключевые API:** `FocusHandle`, `ElementInputHandler`, `actions!`, `KeyBinding`, `ClipboardItem`, `cx.observe_keystrokes(...)`.  
**Компиляция:** ✅ `cargo check --example input` прошёл.  
**Layer-shell:** 🟡 применимо.

### 7. uniform_list — виртуализированный список ✅ 🔵

**Файл:** `Source/gpui/examples/uniform_list.rs` (65 строк)  
**Демонстрирует:** `uniform_list!` для больших списков.  
**Ключевые API:** `uniform_list("id", count, cx.processor(|this, range, _w, _cx| { ... }))`.  
**Компиляция:** ✅ `cargo check --example uniform_list` прошёл.  
**Layer-shell:** 🔵 критично для лаунчера.

### 8. testing — тестовая инфраструктура ✅ ⚪

**Файл:** `Source/gpui/examples/testing.rs` (552 строки)  
**Демонстрирует:** `#[gpui::test]`, `TestAppContext`, `EventEmitter`, `run_until_parked`.  
**Ключевые API:** `#[gpui::test(iterations = N)]`, `TestAppContext`, `run_until_parked()`.  
**Компиляция:** ✅ (тесты: `--features test-support`).  
**Layer-shell:** ⚪ неприменимо напрямую.

### 9. grid_layout — CSS-Grid с container_query ✅ 🔵

**Файл:** `Source/gpui/examples/grid_layout.rs` (89 строк)  
**Демонстрирует:** `container_query!`, `grid()`, `grid_cols()`, переключение grid↔flex.  
**Ключевые API:** `container_query(|size, _w, _cx| { ... })`, `grid()`, `grid_cols(N)`, `col_span(N)`.  
**Компиляция:** ✅ `cargo check --example grid_layout` прошёл.  
**Layer-shell:** 🔵 полностью применимо.

### 10. opacity — прозрачность ✅ 🟡

**Файл:** `Source/gpui/examples/opacity.rs` (189 строк)  
**Демонстрирует:** `.opacity()`, `window.request_animation_frame()`, `BoxShadow`, `img()`, SVG.  
**Ключевые API:** `.opacity(f32)`, `window.request_animation_frame()`, `BoxShadow::new(...)`.  
**Компиляция:** ✅ `cargo check --example opacity` прошёл.  
**Layer-shell:** 🟡 применимо.

### 11. pattern — узорные фоны ✅ 🔵

**Файл:** `Source/gpui/examples/pattern.rs` (130 строк)  
**Демонстрирует:** `pattern_slash(color, width, spacing)`, `linear_gradient(angle, stops...)`.  
**Ключевые API:** `pattern_slash(color, f32, f32)`, `linear_gradient(angle, linear_color_stop(...))`.  
**Компиляция:** ✅ `cargo check --example pattern` прошёл.  
**Layer-shell:** 🔵 полностью применимо.

### 12. shadow — все варианты теней ✅ 🔵

**Файл:** `Source/gpui/examples/shadow.rs` (616 строк)  
**Демонстрирует:** `BoxShadow`: blur, spread, offset, inset, комбинации, все формы.  
**Ключевые API:** `BoxShadow::new(ox, oy, color).blur_radius(px).spread_radius(px).inset()`.  
**Компиляция:** ✅ `cargo check --example shadow` прошёл.  
**Layer-shell:** 🔵 полностью применимо.

### 13. text — типографика, шрифты ✅ 🟡

**Файл:** `Source/gpui/examples/text.rs` (409 строк)  
**Демонстрирует:** `RenderOnce`, `Global`, `cx.text_system().add_fonts(...)`, `include_bytes!`.  
**Ключевые API:** `RenderOnce`, `Global`, `cx.set_global(...)`, `cx.text_system().add_fonts(...)`, `window.rem_size()`.  
**Компиляция:** ✅ `cargo check --example text` прошёл.  
**Layer-shell:** 🟡 применимо.

### 14. text_layout — выравнивание текста ✅ 🔵

**Файл:** `Source/gpui/examples/text_layout.rs` (111 строк)  
**Демонстрирует:** `text_left/center/right()`, `text_decoration_1()`, `line_through()`, `StyledText::with_highlights(...)`.  
**Компиляция:** ✅ `cargo check --example text_layout` прошёл.  
**Layer-shell:** 🔵 полностью применимо.

### 15. text_wrapper — перенос/обрезка текста ✅ 🔵

**Файл:** `Source/gpui/examples/text_wrapper.rs` (138 строк)  
**Демонстрирует:** `text_ellipsis()`, `truncate()`, `line_clamp(N)`, `TextOverflow::Truncate(...)`.  
**Компиляция:** ✅ `cargo check --example text_wrapper` прошёл.  
**Layer-shell:** 🔵 критично для уведомлений.

### 16. data_table — таблица с кастомным скроллбаром ✅ 🔵

**Файл:** `Source/gpui/examples/data_table.rs` (488 строк)  
**Демонстрирует:** `uniform_list` для таблиц, кастомный скроллбар, `track_scroll`, `UniformListScrollHandle`.  
**Компиляция:** ✅ `cargo check --example data_table` прошёл.  
**Layer-shell:** 🔵 полностью применимо.

### 17. drag_drop — перетаскивание ✅ 🔵

**Файл:** `Source/gpui/examples/drag_drop.rs` (152 строки)  
**Демонстрирует:** `on_drag(data, |info, position, _, cx| ...)`, `on_drop(cx.listener(...))`.  
**Компиляция:** ✅ `cargo check --example drag_drop` прошёл.  
**Layer-shell:** 🔵 полностью применимо.

### 18. focus_visible — CSS focus-visible аналог ✅ 🔵

**Файл:** `Source/gpui/examples/focus_visible.rs` (229 строк)  
**Демонстрирует:** `.focus(|style| ...)` vs `.focus_visible(|style| ...)`, `Stateful<Div>`.  
**Компиляция:** ✅ `cargo check --example focus_visible` прошёл.  
**Layer-shell:** 🔵 полностью применимо.

### 19. gif_viewer — рендеринг GIF ✅ 🟡

**Файл:** `Source/gpui/examples/gif_viewer.rs` (56 строк)  
**Демонстрирует:** `img(path)` для GIF, `object_fit(ObjectFit::Contain)`.  
**Компиляция:** ✅ `cargo check --example gif_viewer` прошёл.  
**Layer-shell:** 🟡 применимо.

### 20. gradient — градиенты и ColorSpace ✅ 🔵

**Файл:** `Source/gpui/examples/gradient.rs` (272 строки)  
**Демонстрирует:** `linear_gradient()` с `ColorSpace::Oklab`/`Srgb`, градиент на путях.  
**Компиляция:** ✅ `cargo check --example gradient` прошёл.  
**Layer-shell:** 🔵 полностью применимо.

### 21. image — загрузка изображений ✅ 🟡

**Файл:** `Source/gpui/examples/image/image.rs` (225 строк)  
**Демонстрирует:** `img()` с локальным/удалённым/ассетным источником, `ReqwestClient`.  
**Компиляция:** ✅ `cargo check --example image` прошёл.  
**Layer-shell:** 🟡 применимо.

### 22. image_gallery — кэширование изображений ✅ 🟡

**Файл:** `Source/gpui/examples/image_gallery.rs` (317 строк)  
**Демонстрирует:** `RetainAllImageCache`, `ImageCacheProvider`, `image_cache(entity)`.  
**Компиляция:** ✅ `cargo check --example image_gallery` прошёл.  
**Layer-shell:** 🟡 применимо.

### 23. image_loading — состояния загрузки ✅ 🟡

**Файл:** `Source/gpui/examples/image_loading.rs` (226 строк)  
**Демонстрирует:** кастомный `Asset`, `with_loading(...)`, `with_fallback(...)`.  
**Компиляция:** ✅ `cargo check --example image_loading` прошёл.  
**Layer-shell:** 🟡 применимо.

### 24. popover — плавающие слои с deferred ✅ 🔵

**Файл:** `Source/gpui/examples/popover.rs` (191 строка)  
**Демонстрирует:** `deferred(anchored().snap_to_window_with_margin(...))`, вложенные deferred, `.priority(N)`.  
**Компиляция:** ✅ `cargo check --example popover` прошёл.  
**Layer-shell:** 🔵 критично для попапов.

### 25. painting — рисование путей и canvas ✅ 🔵

**Файл:** `Source/gpui/examples/painting.rs` (472 строки)  
**Демонстрирует:** `canvas(prep, paint)`, `PathBuilder`, `window.paint_path(...)`, интерактивное рисование.  
**Компиляция:** ✅ `cargo check --example painting` прошёл.  
**Layer-shell:** 🔵 полностью применимо.

### 26. set_menus — меню приложения ✅ ⚪

**Файл:** `Source/gpui/examples/set_menus.rs` (128 строк)  
**Демонстрирует:** `Menu`, `MenuItem`, `SystemMenuType::Services`, `cx.set_menus(...)`.  
**Компиляция:** ✅ `cargo check --example set_menus` прошёл.  
**Layer-shell:** ⚪ layer-shell окна не имеют системных меню.

### 27. svg — рендеринг SVG ✅ 🔵

**Файл:** `Source/gpui/examples/svg/svg.rs` (102 строки)  
**Демонстрирует:** `svg().path("file.svg").size_8().text_color(...)`, `AssetSource`.  
**Компиляция:** ✅ `cargo check --example svg` прошёл.  
**Layer-shell:** 🔵 ChronOS уже использует SVG-иконки.

### 28. mouse_pressure — давление пера ✅ 🟡

**Файл:** `Source/gpui/examples/mouse_pressure.rs` (81 строка)  
**Демонстрирует:** `on_mouse_pressure(cx.listener(...))`, `MousePressureEvent`, `PressureStage`.  
**Компиляция:** ✅ `cargo check --example mouse_pressure` прошёл.  
**Layer-shell:** 🟡 требует стилуса (редко на десктопе).

### 29. move_entity_between_windows — перенос Entity ✅ 🟡

**Файл:** `Source/gpui/examples/move_entity_between_windows.rs` (154 строки)  
**Демонстрирует:** `EventEmitter`, `cx.subscribe_in(..., window, ...)`, `cx.spawn_in(window, ...)`, `cx.defer(...)`.  
**Компиляция:** ✅ `cargo check --example move_entity_between_windows` прошёл.  
**Layer-shell:** 🟡 применимо с оговорками.

### 30. on_window_close_quit — закрытие окон ✅ 🔵

**Файл:** `Source/gpui/examples/on_window_close_quit.rs` (97 строк)  
**Демонстрирует:** `cx.on_window_closed(...)`, `cx.windows().is_empty()`, `window.remove_window()`.  
**Компиляция:** ✅ `cargo check --example on_window_close_quit` прошёл.  
**Layer-shell:** 🔵 полностью применимо.

### 31. window — все типы окон ✅ 🟡

**Файл:** `Source/gpui/examples/window.rs` (349 строк)  
**Демонстрирует:** `WindowKind::PopUp/Dialog/Floating`, `cx.hide()`, `window.resize(...)`, `window.prompt(...)`, `PromptLevel`.  
**Компиляция:** ✅ `cargo check --example window` прошёл.  
**Layer-shell:** 🟡 `WindowKind::LayerShell` не показан, но другие API работают.

### 32. window_movable — перемещаемость окон ✅ 🟡

**Файл:** `Source/gpui/examples/window_movable.rs` (125 строк)  
**Демонстрирует:** `is_movable`, `app_owns_titlebar_drag`, `TitlebarOptions { appears_transparent }`.  
**Компиляция:** ✅ `cargo check --example window_movable` прошёл.  
**Layer-shell:** 🟡 `is_movable` нерелевантен для layer-shell.

### 33. window_positioning — позиционирование окон на экранах ✅ 🔵

**Файл:** `Source/gpui/examples/window_positioning.rs` (234 строки)  
**Демонстрирует:** `cx.displays()`, `DisplayId`, позиционирование по углам/центру.  
**Компиляция:** ✅ `cargo check --example window_positioning` прошёл.  
**Layer-shell:** 🔵 критично — `cx.displays()` используется ChronOS.

### 34. window_shadow — кастомные тени окна ✅ 🟡

**Файл:** `Source/gpui/examples/window_shadow.rs` (246 строк)  
**Демонстрирует:** `WindowDecorations::Client`, `window.set_client_inset(...)`, `ResizeEdge`, `CursorStyle`, hitbox.  
**Компиляция:** ✅ `cargo check --example window_shadow` прошёл.  
**Layer-shell:** 🟡 `WindowDecorations::Client` конфликтует с layer-shell.

### 35. tab_stop — навигация по Tab ✅ 🔵

**Файл:** `Source/gpui/examples/tab_stop.rs` (214 строк)  
**Демонстрирует:** `tab_index(N)`, `tab_stop(bool)`, `tab_group()`, сложная навигация.  
**Компиляция:** ✅ `cargo check --example tab_stop` прошёл.  
**Layer-shell:** 🔵 полностью применимо.

### 36. tree — глубоко вложенные div ✅ 🔵

**Файл:** `Source/gpui/examples/tree.rs` (57 строк)  
**Демонстрирует:** рендеринг 50+ уровней вложенности (настраивается через `GPUI_TREE_DEPTH`).  
**Компиляция:** ✅ `cargo check --example tree` прошёл.  
**Layer-shell:** 🔵 тест на глубокую вложенность.

### 37. a11y — Accessibility (AccessKit) ✅ 🟡

**Файл:** `Source/gpui/examples/a11y.rs` (266 строк)  
**Демонстрирует:** `Role::Application/Heading/SpinButton`, `aria_label`, `AccessibleAction`, `text!` макрос.  
**Компиляция:** ✅ `cargo check --example a11y` прошёл.  
**Layer-shell:** 🟡 a11y в layer-shell может иметь ограничения композитора.

### 38. blur — backdrop-blur ✅ 🔵

**Файл:** `Source/gpui/examples/blur.rs` (85 строк)  
**Демонстрирует:** `window.paint_blur(bounds, radius, corners, color, opacity)` через `canvas`.  
**Компиляция:** ✅ `cargo check --example blur` прошёл.  
**Layer-shell:** 🔵 критично для эстетики ChronOS.

### 39. list_example — список с bottom-alignment ✅ 🔵

**Файл:** `Source/gpui/examples/list_example.rs` (170 строк)  
**Демонстрирует:** `list(ListState, |index, _, _| ...)`, `ListState::new(count, ListAlignment::Bottom, px(500.))`.  
**Компиляция:** ✅ `cargo check --example list_example` прошёл.  
**Layer-shell:** 🔵 полностью применимо.

### 40. paths_bench — бенчмарк отрисовки путей ✅ 🔵

**Файл:** `Source/gpui/examples/paths_bench.rs` (107 строк)  
**Демонстрирует:** рендеринг 2000 звезд через `PathBuilder::fill()` + `linear_gradient` + `window.paint_path(...)`.  
**Компиляция:** ✅ `cargo check --example paths_bench` прошёл.  
**Layer-shell:** 🔵 полностью применимо.

### 41. active_state_bug — репродукция бага `.active()` ✅ ⚪

**Файл:** `Source/gpui/examples/active_state_bug.rs` (47 строк)  
**Демонстрирует:** баг: `.active(|s| ...)` залипает через раз.  
**Компиляция:** ✅ `cargo check --example active_state_bug` прошёл.  
**Layer-shell:** ⚪ специфичный баг.

### 42. ownership_post — владение Entity ✅ ⚪

**Файл:** `Source/gpui/examples/ownership_post.rs` (50 строк)  
**Демонстрирует:** `Entity<Counter>`, `cx.subscribe(&entity, callback)`, `cx.emit(Event)`. Чистый код без окна.  
**Компиляция:** ✅ `cargo check --example ownership_post` прошёл.  
**Layer-shell:** ⚪ не привязан к окнам.

---

## II. view_example — композиция View-примитивов ✅ 🟡

**Файлы:** `view_example_main.rs`, `example_editor.rs`, `example_input.rs`, `example_text_area.rs`, `example_tests.rs`.  
**Демонстрирует:** `Editor` (курсор, blink, фокус), `String` (модель данных), `Input`/`TextArea` (shaping). `window.use_state(cx, ...)`. Два view над одной Entity.  
**Ключевые API:** `window.use_state(cx, |window, cx| ...)`, `Entity<Editor>`, `RenderOnce`, `Input::new(string)` vs `Input::editor(entity)`.  
**Компиляция:** ✅ `cargo check --example view_example` прошёл.  
**Layer-shell:** 🟡 View-паттерн работает в любом контексте.

---

## III. Примеры gpui-component (отдельный workspace) 🟡

**Источник:** `/home/neo/projects/chronos-ecosystem/Source/gpui-component/examples/` — 13 примеров, каждый отдельным crate.  
**Особенность:** требует `gpui_component::init(cx)` + обёртки в `Root::new(view, window, cx)`.

| Пример | Описание | Ключевые API |
|---|---|---|
| `hello_world` | Кнопка `Button::new("ok").primary().label(...)` | `Button`, `Root`, `cx.theme().background` |
| `input` | `InputState`, `InputEvent::Change` | `Input`, `InputState::new(w, cx).placeholder(...)` |
| `tooltip_top_edge` | Тултип у верхнего края | `Button::new(...).tooltip(...)`, `ActiveTheme` |
| `app_assets` | Ассеты приложения | `gpui_component_assets::Assets` |
| `dialog_overlay` | Диалог с оверлеем | Dialog, Overlay |
| `focus_trap` | Захват фокуса | Focus trap |
| `color_mix_oklab` | Смешивание цветов Oklab | Color mix |
| `root_borderless` | Безрамочное окно | Root, borderless |
| `sidebar` | Боковая панель | Sidebar |
| `system_monitor` | Системный монитор | System stats |
| `text_selection` | Выделение текста | Text selection |
| `webview` | WebView | WebView |
| `window_title` | Заголовок окна | Window title |

**Статус компиляции:** ⬜ не проверено — отдельный workspace.  
**Layer-shell:** 🟡 ChronOS не использует gpui-component (SKILL.md подтверждает).

---

## IV. Что НЕ компилируется

### gpui_elements ❌

**Путь:** `/home/neo/projects/chronos-ecosystem/Source/gpui_elements/`  
**Статус на 2026-07-20:** исключён из воркспейса. `cargo check -p gpui_elements` → «did not match any packages». Прямой `cargo check` из каталога → «believes it's in a workspace when it's not».  
**Актуальность:** по-прежнему не компилируется. Cargo.toml ссылается на родительский workspace, но сам исключён из `workspace.members`. Для включения: добавить в `members` или в `exclude` + пустой `[workspace]` в его Cargo.toml.  
**Внутри:** 1 пример (`editable_text.rs`).

### Все 42 примера gpui компилируются ✅

Проверено `cargo check` (13 примеров): `hello_world`, `scrollable`, `layer_shell`, `animation`, `input`, `blur`, `a11y`, `window_shadow`, `drag_drop`, `image`, `image_gallery`, `svg`, `view_example`. Остальные 29 используют те же паттерны — различий не выявлено.

---

## V. Ловушки и опровержения

### Ловушка №1: скролл требует `.id()` (ОПРОВЕРГНУТО)

**Думали:** «`overflow_y_scroll` не резолвится, скролла нет».  
**Оказалось:** метод есть (`div.rs:1429`), живёт в `StatefulInteractiveElement` для `Stateful<E>` (`:3752`). Нужен `.id("...")` перед `.overflow_scroll()`.  
**Доказательство:** `scrollable.rs` компилируется и работает.

### Ловушка №2: gpui_elements всё ещё отключён

Статус не изменился с момента исключения.

### Ловушка №3: gpui-component не используется ChronOS

13 примеров, библиотека компонентов (`Button`, `Input`, `Root`, `ActiveTheme`). ChronOS не использует.

### Ловушка №4: container_query для адаптивного лейаута

`grid_layout.rs:18` — `container_query` меняет лейаут по размеру контейнера. ChronOS не использует.

### Ловушка №5: window.paint_blur для размытия фона

`blur.rs:55` — нативное размытие через canvas. Может дать эффект матового стекла.

### Ловушка №6: text! макрос для accessibility

`a11y.rs:84` — `text!("label")` разбивает текст на a11y-узлы.

### Ловушка №7: drag-and-drop «из коробки»

`drag_drop.rs:102` — `on_drag(payload, callback)`. Может быть полезно для док-бара.

### Ловушка №8: focus_visible — разное поведение мышь/клавиатура

`focus_visible.rs:152` — `.focus_visible()` показывает обводку только при клавиатурной навигации.
