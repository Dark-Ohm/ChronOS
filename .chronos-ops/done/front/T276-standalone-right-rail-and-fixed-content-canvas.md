# T276 — правая панель: standalone rail и фиксированный content-canvas

**Статус:** DONE — 2026-08-13, live UX принят владельцем (`+`).

**Приоритет:** P1 — заменяет неустойчивый resize одной right-anchored
layer-shell поверхности.
**Исполнитель:** Sonnet 5.
**Роль:** FRONTEND + GPUI/Wayland lifecycle.
**Зависимость:** архитектурно заменяет T273; его экспериментальные правки
не являются основой решения.

## Цель

Разделить правую панель на две независимо живущие layer-shell поверхности:

1. `rail` — постоянная поверхность шириной `RAIL_ONLY_WIDTH` (40 px),
   `TOP | RIGHT`, namespace `side_panel_right_rail`;
2. `content` — поверхность справа от rail, с фиксированным canvas
   `MAX_WIDTH - RAIL_ONLY_WIDTH`, namespace `side_panel_right_content`.

Rail переключает вкладки и управляет content через общий GPUI state/entity.
Во время drag нельзя менять Wayland/WGPU drawable size ни одной поверхности.
Меняется только видимая часть content внутри фиксированного canvas.

## Почему принято это решение

T273 доказал асимметрию right anchor: при изменении ширины compositor меняет
origin поверхности, пока buffer может относиться к предыдущей геометрии.
Левая панель визуально иммунна, потому что её origin при resize не движется.
Попытки синхронизировать `set_size`, configure, Scene и buffer во внутреннем
форке не дали владельцу положительный результат.

Проверенная инфраструктура для фиксированного canvas уже есть:

- `Window::set_input_region` — `Source/gpui/src/window.rs`;
- Wayland-реализация через `wl_surface.set_input_region` —
  `Source/gpui_linux/src/linux/wayland/window.rs`;
- живой пример составного input region —
  `crates/app/src/popup_click_catcher.rs`;
- rail actions уже передаются callback-ами в
  `crates/app/src/side_panel_right/rail.rs::render_rail`.

## Контракт геометрии

- Rail всегда занимает последние 40 px монитора и не ресайзится.
- Content canvas имеет постоянную максимальную ширину 920 px
  (`MAX_WIDTH - RAIL_ONLY_WIDTH`) и стоит непосредственно слева от rail.
- Текущая пользовательская ширина панели сохраняет прежнюю семантику
  `RAIL_ONLY_WIDTH..=MAX_WIDTH`. Видимая ширина content равна
  `max(0, state.width - RAIL_ONLY_WIDTH)`.
- Контент прижимается к правой стороне своего canvas, рядом с rail. При
  сужении освобождается левая часть canvas; rail и правая кромка content не
  двигаются относительно экрана.
- Прозрачная неиспользуемая часть canvas не принимает ввод. На каждом
  изменении видимой ширины content устанавливает input region только на
  видимый прямоугольник. При закрытом content region пустой.
- Exclusive zone принадлежит rail surface. В overlay-режиме это 40 px; в
  dock-режиме — текущая полная ширина панели. Content surface не резервирует
  пространство самостоятельно.
- Высота и верхний gap сохраняют действующий контракт
  `panel_edge_gap()`/bar height.

## Контракт состояния и вкладок

Расширить `SidePanelRightState`, не плодить два несвязанных state:

- отдельные handles для rail и content;
- weak entity content-view для `select_tab`/`preview_target`;
- единые `pinned`, `peek_generation`, `resizing`, `width`, `dock_content`;
- открытие rail создаёт обе поверхности как одну логическую панель;
- закрытие удаляет обе поверхности и обнуляет handles до `remove_window()`;
- частичный open запрещён: если вторая поверхность не создалась, первая
  откатывается;
- повторный open/toggle идемпотентен;
- rail-кнопка вызывает существующую семантику `on_tab_select`: выбранная
  вкладка открывает content, повторный выбор закрывает content, память ширины
  вкладок сохраняется;
- IPC `select-tab:*` и `preview_target` открывают логическую панель и доходят
  до content-view;
- pin/peek/hover-leave/resize-grab работают на границе обеих поверхностей как
  единый объект; переход курсора между content и rail не закрывает панель.

## Зона кода

Основная:

- `crates/app/src/side_panel_right/mod.rs`;
- `crates/app/src/side_panel_right/view.rs`;
- `crates/app/src/side_panel_right/rail.rs`;
- новый небольшой view-модуль rail допустим и предпочтителен, если отделяет
  lifecycle от content;
- тесты рядом с модулем.

`Source/` в рамках T276 не менять. Незавершённые T273-правки в
`Source/gpui_linux/src/linux/wayland/window.rs`, `Source/gpui/src/scene.rs` и
`Source/gpui/src/window.rs` нужно удалить, сохранив чужие изменения.

## Запрещённые короткие пути

- Не ресайзить content window во время drag.
- Не прятать дрожание анимацией, clip-маской или задержкой rail.
- Не оставлять один старый combined window параллельно двум новым.
- Не связывать поверхности через IPC; внутри процесса используется общий
  state/entity.
- Не делать весь прозрачный canvas кликабельным.
- Не менять левую панель «для симметрии».
- Не коммитить `Cargo.lock`, T266 и прочой чужой dirty worktree.

## TDD и проверки

До реализации добавить тесты на чистые решения/геометрию:

- вычисление видимой ширины content и её clamp;
- bounds input region: правая часть canvas, корректная ширина и пустой region
  при закрытии;
- exclusive zone: 40 px overlay, полная ширина dock;
- lifecycle state: оба handles очищаются как единое целое;
- выбор вкладки из rail открывает content и повторный выбор закрывает его;
- resize меняет state/input region, но не window bounds canvas.

Обязательная автоматическая проверка:

```bash
cargo test -p chronos side_panel_right --lib --bins
cargo build --release
```

Живая проверка владельцем, без `wf-recorder`:

1. Rail виден на правой кромке и запускает все вкладки.
2. Плавное сужение: rail неподвижен, обои и пустые кадры внутри content не
   мелькают.
3. Быстрые рывки в обе стороны.
4. Длинный drag до обоих clamp.
5. Overlay не блокирует клики в прозрачной части canvas.
6. Dock резервирует ровно выбранную ширину.
7. Toggle, peek, pin, переход курсора rail ↔ content и закрытие не оставляют
   ghost/orphan surfaces (`hyprctl layers`).
8. `select-tab:preview` открывает content и фокус/клавиатурный ввод работают.

Финальный критерий UX принадлежит владельцу: `+` означает принято, `-` —
задача не выполнена. Повторный видеоаудит без запроса владельца не нужен.

## Отчёт исполнителя

Создать
`docs/orchestration/tasks/report-log/T276-standalone-right-rail-and-fixed-content-canvas-report.md`:

- список изменённых файлов и символов;
- схема ownership двух handles и закрытия;
- доказательство постоянных window bounds при drag;
- доказательство input region;
- точные команды и результаты тестов;
- что не проверено живьём;
- commit hash только после проверки и разрешения Архитектора.

## Коммит

`side_panel_right: split rail from fixed content canvas (T276)`
