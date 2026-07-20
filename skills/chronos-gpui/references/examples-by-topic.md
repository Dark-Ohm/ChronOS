# Примеры форка gpui — группировка по темам

**Когда грузить:** нужно найти пример под конкретную задачу («мне нужен пример про X»).

**Источник:** каталог `examples-catalog.md` — там полные детали по каждому примеру.

---

## Базовый каркас приложения

| Пример | Что показывает |
|---|---|
| `hello_world` | Минимальное GPUI-приложение: `App`, `Window`, `Render`, `div()` |
| `ownership_post` | Модель владения: `Entity`, `cx.subscribe`, `cx.emit` (без окна) |

## Layer-shell (ChronOS)

| Пример | Что показывает |
|---|---|
| `layer_shell` | **Эталонный пример:** `WindowKind::LayerShell`, `Anchor`, `margin`, `keyboard_interactivity` |

## Лейаут и стили

| Пример | Что показывает |
|---|---|
| `grid_layout` | CSS-Grid: `grid()`, `grid_cols()`, `container_query` для адаптивного лейаута |
| `opacity` | `.opacity()`, `request_animation_frame()`, `BoxShadow` |
| `pattern` | `pattern_slash()`, `linear_gradient()` |
| `shadow` | **Исчерпывающий каталог:** все варианты `BoxShadow` (drop, inset, комбо) |
| `blur` | `window.paint_blur()` — backdrop-blur через canvas |

## Текст и типографика

| Пример | Что показывает |
|---|---|
| `text` | `RenderOnce`, `Global` шрифты, `TextStyle`, `include_bytes!` |
| `text_layout` | Выравнивание: `text_left/center/right`, `text_decoration_1`, `line_through`, `StyledText` |
| `text_wrapper` | Обрезка: `text_ellipsis()`, `truncate()`, `line_clamp(N)`, `TextOverflow` |

## Скролл и списки

| Пример | Что показывает |
|---|---|
| `scrollable` | **Важнейший:** скролл требует `.id()` перед `.overflow_scroll()` |
| `uniform_list` | Виртуализированный список (лаунчер, уведомления) |
| `data_table` | Таблица на `uniform_list` с кастомным скроллбаром |
| `list_example` | `list()` с `ListState`, bottom-alignment, диагностика бага скроллбара |

## Попапы и плавающие элементы

| Пример | Что показывает |
|---|---|
| `anchor` | `anchored()` + `deferred()` — позиционирование попапов |
| `popover` | Вложенные `deferred`, `.priority(N)`, `on_mouse_down_out` |

## Анимации

| Пример | Что показывает |
|---|---|
| `animation` | `Animation`, `with_animation()`, `Transformation::rotate`, `bounce(ease_in_out)` |

## Изображения и SVG

| Пример | Что показывает |
|---|---|
| `image` | `img()`: локальные, удалённые, ассеты. `ReqwestClient` |
| `image_gallery` | `RetainAllImageCache`, `ImageCacheProvider`, HTTP-загрузка |
| `image_loading` | `with_loading()`, `with_fallback()`, кастомный `Asset` |
| `gif_viewer` | GIF через `img()`, `object_fit` |
| `svg` | `svg().path(...)`, тонировка `text_color` |

## Ввод и фокус

| Пример | Что показывает |
|---|---|
| `input` | Полный текстовый редактор: фокус, выделение, Copy/Paste, IME, `KeyBinding` |
| `view_example` | View-композиция: `Editor`, `Input`, `TextArea`, `window.use_state()` |
| `focus_visible` | `.focus()` vs `.focus_visible()` — разное поведение мышь/клавиатура |
| `tab_stop` | `tab_index()`, `tab_stop()`, `tab_group()`, навигация |

## Окна и приложение

| Пример | Что показывает |
|---|---|
| `window` | Все типы окон: `PopUp`, `Dialog`, `Floating`, `cx.hide()`, `window.prompt()` |
| `window_movable` | `is_movable`, `app_owns_titlebar_drag`, кастомный titlebar |
| `window_positioning` | `cx.displays()`, позиционирование по экранам |
| `window_shadow` | `WindowDecorations::Client`, `ResizeEdge`, `set_client_inset()`, hitbox |
| `on_window_close_quit` | `cx.on_window_closed()`, закрытие всех окон → выход |
| `move_entity_between_windows` | `cx.subscribe_in(window, ...)`, `cx.spawn_in(window, ...)`, `cx.defer()` |

## Интерактивность

| Пример | Что показывает |
|---|---|
| `drag_drop` | `on_drag(payload, callback)`, `on_drop(cx.listener(...))` |
| `mouse_pressure` | `on_mouse_pressure()`, `MousePressureEvent` |

## Рисование (canvas)

| Пример | Что показывает |
|---|---|
| `painting` | `canvas()`, `PathBuilder`, `window.paint_path()`, интерактивное рисование |
| `paths_bench` | Бенчмарк: 2000 путей через `paint_path()` |
| `gradient` | Градиенты на canvas-путях, `ColorSpace::Oklab` |

## Системное

| Пример | Что показывает |
|---|---|
| `set_menus` | `Menu`, `MenuItem`, `SystemMenuType::Services` |
| `a11y` | Accessibility: `Role`, `aria_label`, `AccessibleAction`, `text!` макрос |

## Тестирование

| Пример | Что показывает |
|---|---|
| `testing` | `#[gpui::test]`, `TestAppContext`, `run_until_parked`, `MockNetwork` |

## Специфичные / отладочные

| Пример | Что показывает |
|---|---|
| `active_state_bug` | Репродукция: `.active()` залипает через раз |
| `tree` | Стресс-тест: 50+ уровней вложенности div |

## gpui-component (отдельный workspace)

| Пример | Что показывает |
|---|---|
| `hello_world` | `Button::new("ok").primary().label(...)`, `Root`, `cx.theme()` |
| `input` | `InputState`, `InputEvent::Change`, подписки |
| `tooltip_top_edge` | `Button::new(...).tooltip(...)`, `ActiveTheme` |
| `app_assets` | `gpui_component_assets::Assets` |
| `dialog_overlay` | Dialog + Overlay |
| `focus_trap` | Захват фокуса |
| `color_mix_oklab` | Смешивание цветов Oklab |
| `root_borderless` | Безрамочное Root-окно |
| `sidebar` | Боковая панель |
| `system_monitor` | Системный монитор |
| `text_selection` | Выделение текста |
| `webview` | WebView-компонент |
| `window_title` | Заголовок окна |
