# T259 — desktop-terminal: Edit Mode (drag/resize/add/remove)

**Статус:** DONE (код закоммичен; живой смоук заблокирован системным
драйвер-мисматчем NVIDIA, см. «Верификация» — повтор после ребута)
**Дата:** 2026-08-05
**Коммит:** `9d7b060` — `ui : desktop-terminal ghost-window fix (T257
review) + edit-mode drag/resize/add/remove (T259, code lands here — report
pending)` (включает T257-review фикс `close_one`, см. ниже)
**Зависимость:** T257 (реестр `open_one`/`close_one` + `config`) — код
слит в том же дереве, тикет закрыт отдельно.

## Что сделано

### 1. Drag (переместить) — `view.rs`

- В edit mode (`crate::edit_mode::is_active(cx)`, тот же тумблер
  Super+Shift+E) вся верхняя полоса виджета становится drag-хэндлом:
  `.cursor_move()` + `on_drag(WidgetDrag, …)` + `on_mouse_up` +
  `on_mouse_up_out`.
- **Drag-preview:** использован нативный механизм форка — ghost-view
  (`active_drag.view`, рендерится фреймворком за курсором, grab-anchored).
  `on_drag`-конструктор возвращает `TerminalDragGhost` — полупрозрачная
  рамка того же размера, что и виджет, с акцентным цветом. **Выбор
  задокументирован (тикет §1):** отдельное overlay-окно невозможно в
  этом форке — нет runtime API на repositioning layer-shell surface
  (проверено: в `Source/gpui` нет `move_to`/`set_position`/`set_margin`),
  поэтому ghost рендерится внутри текущего окна и клиппится его
  границами — это честный визуал, который форк физически может дать.
  На mouse-up окно «телепортируется» на новую позицию (согласовано в
  брейншторме).
- **Mouse-up:** `finalize_drag(ev.position, window, cx)` —
  `cursor_global = window.bounds().origin + position` (окно за время
  драга не двигается, дельта от стартового грипа точная),
  `new_anchor = start_anchor + delta` (clamp ≥ 0). `config::save()` с
  обновлённым spec → `move_window()` (закрыть окно → `open_one` с новым
  spec). PTY не трогается: реестр ключует по `spec.id`, тот же id → та
  же живая сессия.
- Sub-threshold релиз (клик без движения, <2px) — no-op, без лишнего
  close+reopen.

### 2. Resize — `view.rs`

- Хэндл в правом нижнем углу (16×16, `.cursor_nwse_resize()`), edit mode
  only. Паттерн тот же, что у панелей: `on_mouse_down` (фиксирует
  стартовые размер+курсор) → `on_drag(WidgetResize, …)` →
  `on_drag_move` → `on_mouse_up`/`on_mouse_up_out`.
- **Живой resize:** в отличие от драга, форк даёт `window.resize()` на
  размер — поэтому resize идёт live во время драга (anchor-модель:
  `start_size + delta`, никогда не от текущего кадра — урок T216 про
  async Wayland-ack). На mouse-up — только персист новых width/height в
  конфиг, без close/reopen.
- `MIN_WIDTH=320` / `MIN_HEIGHT=192` — floor, чтобы виджет нельзя было
  ужать до нуля; помещается ≥40 колонок / ≥10 строк сетки 8×16 (тест
  это пиннит). Максимума нет (растягивать можно свободно).

### 3. Удаление (крестик) — `view.rs` + `mod.rs`

- Крестик в правом краю верхней полосы, edit mode only, hover →
  красный фон. `on_click` → новый `close_one_in_window(id, window, cx)`:
  1) `registry.kill(id)` — **PTY реально убивается** (не просто окно);
  2) spec убирается из `desktop_terminal.toml` (`config::save` без id);
  3) окно закрывается. НЕ путать с T256 — фейковый крестик в `header.rs`
  правой панели это отдельная находка, не связана.

### 4. Добавление — `mod.rs` + `system.rs`

- Кнопка «＋ Add terminal» в System-табе правой панели, видна только в
  edit mode (тот же принцип, что reorder-кнопки bar-виджетов). Стиль —
  паттерн T231: `elevated_card` + `section_header` из `tab/ui.rs`, новый
  визуальный язык не изобретался.
- Клик → `desktop_terminal::add_widget(cx)`: новый spec (`make_spec`,
  свежий uuid, дефолт 600×400), позиция — `next_anchor(specs)`:
  последний spec + (40, 40) по диагонали (пусто → (48, 80)), чтобы
  серия кликов не стакала окна друг на друга. `config::save()` +
  `open_one()`.

## Ключевые решения

- **HANDOFF «СИСТЕМНЫЙ БАГ» правило:** все close-пути из window-колбэков
  (`✕`, drag-телепорт) идут через **живую** `window: &mut Window`
  ссылку (`window.remove_window()` напрямую), НЕ через реентерабельный
  `handle.update`. Новые `close_one_in_window`/`move_window` в `mod.rs`
  следуют этому правилу; старый `close_one` (внешний путь) оставлен как
  есть — в 9d7b060 он дополнительно получил T257-review фикс: логирует
  реальный исход `handle.update` вместо молчаливого `let _ =` (тот самый
  антипаттерн, что рождал ghost-окна в launcher/tray_menu).
- **Двойной финализ исключён:** `on_mouse_up` (hovered) и
  `on_mouse_up_out` (не-hovered, срабатывает и при релизе ВНЕ окна —
  Wayland implicit grab доставляет mouse-up поверхности, начавшей
  драг) взаимоисключаемы по hover-состоянию, плюс `drag_state.take()` /
  `resize_state.take()` делают финализ идемпотентным.
- **lib.rs:** добавлен `pub(crate) mod desktop_terminal;` — без этого
  `system.rs` (компилируется и в lib-крэйт для тестов) не видел
  `crate::desktop_terminal::add_widget`.

## Верификация

### Юнит/сборка — зелёные

- `cargo build --release -p chronos` — чисто (только pre-existing
  warnings).
- `cargo test --release -p chronos --lib -- desktop_terminal` — **12
  passed** (5 новых: `next_anchor_*` ×3, `resize_floors_at_minimum`,
  `min_constants_leave_room_for_a_few_terminal_rows`).
- `cargo test --release -p chronos --lib -- side_panel_right` — **171
  passed, 0 failed** (существующие не сломаны).

### Живой прогон — ЗАБЛОКИРОВАН (системный драйвер, не код)

При попытке `chronos-start` (штатный stop/start) новый процесс падает на
GPU-инициализации ДО любого кода T259:

```
ERROR wgpu_hal::vulkan::instance: enumerate_adapters: Initialization of an object has failed
… fallback на llvmpipe (GL, CPU) …
thread 'main' panicked … wgpu error: Validation Error
  In Surface::configure — Requested usage TextureUses(COPY_SRC | COLOR_TARGET)
  is not in the list of supported usages: TextureUses(COLOR_TARGET)
```

Диагноз (не связан с T259):
- `vulkaninfo` — `Failed to detect any valid GPUs`; `nvidia-smi` —
  **`Failed to initialize NVML: Driver/library version mismatch`**
  (NVML 610.57) — разнобой ядра NVIDIA и userspace после обновления
  пакетов без ребута. Vulkan не видит GPU, wgpu падает на софтварном
  фоллбэке.
- Ранее запущенный (до ребута) chronos работал — он инициализировал GPU
  до поломки драйвера.

**План повторной приёмки после ребута** (пункты из тикета):
1. Чистый конфиг (0 виджетов) → Edit Mode → «+ Add terminal» → виджет
   появился, реальный prompt печатается.
2. `echo $$` в виджете — записать PID.
3. Drag → отпустить → `hyprctl layers` координаты изменились → `echo $$`
   тот же PID (PTY пережила close+reopen).
4. Resize → новые width/height видны живьём.
5. Крестик → окно закрылось, `ps -p <PID>` не находит процесс.
6. Рестарт `chronos` (штатный stop/start) → виджет восстановился из
   `desktop_terminal.toml`.

## Зона файлов

- `crates/app/src/desktop_terminal/mod.rs` — `close_one_in_window`,
  `move_window`, `add_widget`, `next_anchor`, ре-экспорт config API.
- `crates/app/src/desktop_terminal/view.rs` — edit-mode chrome:
  drag-хэндл + ghost, resize-хэндл, крестик, MIN-константы, state-машина
  драгов.
- `crates/app/src/side_panel_right/tab/system.rs` — «+ Add terminal»
  карточка (edit mode only).
- `crates/app/src/lib.rs` — `pub(crate) mod desktop_terminal;` (lib-крэйт).

**Не трогал:** `crates/services/src/terminal/*` (T257/T258 зона),
kitty-тему (T258), рендер-константы.
