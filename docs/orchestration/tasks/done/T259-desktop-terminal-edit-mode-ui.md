# T259 — desktop-terminal: Edit Mode (drag/resize/add/remove)

**Роль:** FRONTEND (Rust, GPUI).
**Приоритет:** P1, **зависит от T257** (реестр окон `open_one`/`close_one`
+ config `TerminalWidgetSpec`/`load`/`save` должны быть слиты первыми —
не начинай, пока T257 не в `done/`). Независим от T258 (тема
kitty.conf не нужна для этой работы).
**Источник:** `docs/superpowers/specs/2026-08-05-desktop-terminal-widget-design.md`
§«Архитектура» п.5. Читать целиком перед началом.

## Контекст

После T257 у нас есть `desktop_terminal::open_one(spec, cx)` /
`close_one(id, cx)`, `config::{load,save}`, `TerminalWidgetSpec { id,
anchor_x, anchor_y, width, height }`. Виджеты сидят на `Layer::Background`
без какого-либо UI для управления — нельзя подвинуть/зарезайзить/удалить/
добавить без ручной правки TOML-файла.

Фича: пока активен `crate::edit_mode::is_active(cx)` (глобальный тумблер,
Super+Shift+E, тот же что красит рамки у bar-виджетов — смотри
`crates/app/src/bar/widgets/tray.rs:79` для образца паттерна проверки), у
каждого открытого desktop-terminal виджета — рамка + drag-хэндл +
resize-хэндл + крестик. Плюс кнопка "+ Add terminal" в System-табе правой
панели.

**Важное ограничение форка (проверено в брейншторме, НЕ пытайся найти
"более правильный" способ):** live-repositioning layer-shell окна во
время самого drag НЕВОЗМОЖНО — форк не даёт runtime API менять
margin/позицию открытого layer-shell surface, только `window.resize()` на
размер. Значит drag — это НЕ "окно едет за курсором", это "тащишь,
визуально двигается drag-preview overlay (обычный div, не окно), на
mouse-up — закрыть текущее layer-shell окно, посчитать новый
anchor_x/anchor_y, открыть новое окно на новой позиции". Согласовано с
пользователем явно.

## Что сделать

### 1. Drag (переместить)

- В edit mode верхняя полоса виджета (или вся рамка — на твой вкус,
  сверься с тем, как выглядит на живом кадре, чтобы не мешать кликам по
  терминалу) — `cursor_grab`, `on_drag_move` (паттерн —
  `side_panel_right/view.rs:670`/`panel.rs:485`, но конечное действие
  другое, см. ниже).
- Пока драг идёт — двигай **preview-элемент** (полупрозрачная рамка того
  же размера, обычный div внутри текущего layer-shell окна ИЛИ отдельное
  временное overlay-окно, если drag должен визуально выходить за
  границы текущего окна виджета — реши по месту, что технически проще в
  этом форке, задокументируй выбор в отчёте).
- Mouse-up: посчитать новый `anchor_x`/`anchor_y` из финальной позиции
  курсора (абсолютные экранные координаты минус позиция монитора, сверь
  с тем, как `hover_strip`/`side_panel` уже переводят координаты — не
  изобретай заново). `config::save()` с обновлённым spec.
  `desktop_terminal::close_one(id, cx)` → `open_one(updated_spec, cx)`.
  PTY НЕ трогается (реестр из T257 хранит по id, тот же id → та же
  сессия).

### 2. Resize

- Хэндл в правом нижнем углу виджета, тот же `on_drag_move` принцип,
  считает новую `width`/`height` вместо позиции. Тот же
  close+reopen-с-новым-spec цикл на mouse-up (можно объединить с drag в
  один обработчик, если резайз и муваться геометрически похожи — на твоё
  усмотрение, лишь бы оба сценария живьём работали раздельно и вместе).
- Минимальные размеры — не давай ужать виджет до нуля/отрицательных
  (`MIN_WIDTH`/`MIN_HEIGHT` константы, разумные — терминал должен
  вмещать хотя бы несколько строк/колонок, ориентируйся на текущий
  `COLS`/`ROWS` минимум из `view.rs`).

### 3. Удаление (крестик)

- Видим только в edit mode, на рамке виджета (не путать с T256 —
  отдельная находка про фейковый крестик в `header.rs` правой панели,
  никак не связана с этим тикетом).
- Клик → `registry.kill(id)` (сервисный слой, T257) + убрать spec из
  `desktop_terminal.toml` (`config::save` без этого id) +
  `desktop_terminal::close_one(id, cx)`.

### 4. Добавление

- Кнопка "+ Add terminal" в `crates/app/src/side_panel_right/tab/
  system.rs` — видна только когда `edit_mode::is_active(cx)` (тот же
  принцип, что reorder-кнопки у bar-виджетов). Стиль — сверься с
  соседними карточками System-таба (`elevated_card`/`section_header`
  паттерн из T231, `tab/ui.rs`), не изобретай новый визуальный язык.
- Клик → новый `TerminalWidgetSpec` со случайным id, дефолтные
  `width`/`height` (например текущие `TERM_WIDTH=600`/`TERM_HEIGHT=400`
  как fallback-константы, см. T257), позиция — смещена от последнего
  добавленного виджета (например `+40,+40` по диагонали от последнего
  spec в списке), чтобы новые окна не спавнились друг на друге при
  многократном клике. `config::save()` + `open_one()`.

## Зона файлов

- `crates/app/src/desktop_terminal/{mod.rs,view.rs}` — рамка/drag/resize/
  крестик в edit mode (используй `open_one`/`close_one`/`config` из
  T257 — не переизобретай).
- `crates/app/src/side_panel_right/tab/system.rs` — кнопка "+ Add
  terminal".

**НЕ трогать:** `crates/services/src/terminal/*` (T257/T258 зона), тему
kitty.conf (T258 — если ещё не слит к твоему старту, просто не трогай
рендер-константы вообще).

## Верификация

- `cargo build --release -p chronos` — чисто.
- `cargo test --release -p chronos --lib -- desktop_terminal side_panel_right` —
  зелёные (существующие 167 в side_panel_right не сломаны).
- Живой прогон, обе темы:
  1. Чистый конфиг (0 виджетов) → Edit Mode → "+ Add terminal" → виджет
     появился, реальный prompt печатается.
  2. Напечатать команду `echo $$` в виджете, записать PID.
  3. Drag виджет в другую позицию, отпустить → окно на новой позиции
     (`hyprctl layers` координаты изменились) → `echo $$` **тот же PID**
     (доказательство, что PTY пережила close+reopen).
  4. Resize виджета → новые width/height видны живьём.
  5. Крестик → окно закрылось, `ps -p <PID>` → процесс не найден (PTY
     реально убита, не просто окно скрыто).
  6. Рестарт `chronos` (не `pkill -9`, штатный stop/start) → виджет(ы) в
     последних сохранённых позициях/размерах восстановились из
     `desktop_terminal.toml` (новая PTY-сессия — предыдущая не
     переживает полный рестарт приложения, это ожидаемо, только сама
     drag-операция должна её сохранять).

## Коммит

`ui : desktop-terminal edit-mode drag/resize/add/remove (T259)`.

## Отчёт

`docs/orchestration/tasks/report/T259-desktop-terminal-edit-mode-ui-report.md`.

## Статус (2026-08-05) — DONE

Код закоммичен (`9d7b060`, вместе с T257-review фиксом `close_one`):
drag/resize/add/remove, отчёт в `report-log/T259-…-report.md`. Юнит-тесты
и сборка зелёные (12 desktop_terminal + 171 side_panel_right). Живой смоук
**заблокирован** системным NVML driver/library mismatch (NVIDIA драйвер
сломан до ребута — Vulkan не видит GPU, wgpu падает на llvmpipe) — это
окружение, не код. Повторная приёмка по пунктам «Верификация» — после
ребута.
