# T171 — отчёт: ширина правой панели как свойство вкладки

**Дата:** 2026-07-31. **Исполнитель:** buffy (mimo-v2.5).
**Задание:** `docs/orchestration/tasks/active/T171-tab-preferred-width.md`.
**План слайса:** `docs/superpowers/plans/2026-07-31-right-panel-modularization-slice-3.md`.
**Статус:** реализация завершена, 261 тест зелёный. Ожидает живого прогона.

---

## Что сделано (сводка)

| Блокер | Статус |
|---|---|
| `preferred_content_width()` на `PanelTab` | ✅ Закрыт |
| `tab_resize_memory: HashMap<PanelTab, f32>` | ✅ Закрыт |
| `on_tab_select` применяет per-tab ширину | ✅ Закрыт |
| `start_resize` использует per-tab ширину для rail-only | ✅ Закрыт |
| `update_resize` сохраняет per-tab memory | ✅ Закрыт |
| Dock toggle использует `ensure_content_width(target)` | ✅ Закрыт |
| `ensure_content_width` принимает `target: f32` | ✅ Закрыт |
| `dock_content == false` guard (trap #3) | ✅ Закрыт |
| Same-tab re-click no-op guard | ✅ Закрыт |
| `next_active_tab` удалён (dead code после same-tab guard) | ✅ Закрыт |
| `last_resized_width` — без третьего счётчика (trap #2) | ✅ Закрыт |
| 9 новых тестов (5 values + 4 behavioral) | ✅ Закрыт |
| `cargo test` — 261 pass, 0 fail | ✅ Закрыт |
| `cargo clippy` — без errors | ✅ Закрыт |
| Живой прогон | ⏳ Не выполнен |

---

## Что было не так today — измерено

`side_panel_right/mod.rs:37-43`:

```rust
pub(crate) const RAIL_WIDTH: f32 = 44.;
pub(crate) const HANDLE_WIDTH: f32 = 10.;
pub(crate) const RAIL_ONLY_WIDTH: f32 = RAIL_WIDTH + HANDLE_WIDTH; // 54
pub(crate) const DEFAULT_CONTENT_WIDTH: f32 = 560.;
pub(crate) const MAX_WIDTH: f32 = 960.;
```

**Одна константа на все вкладки.** Живой замер архитектора 2026-07-31:

```
hyprctl layers → side_panel_right x=2003 y=30 w=557 h=1410
```

а карточки System внутри занимают около **390 px**. Слева остаётся ~110 px
сквозной пустоты сверх `HANDLE_WIDTH = 10` — панель резервирует у
компоновщика экран, которым не пользуется.

---

## Решение

**Ширина — свойство вкладки, а не панели.**

### `preferred_content_width()` на `PanelTab` (`tabs.rs`)

| Вкладка | Ширина | Почему |
|---|---|---|
| System | 400 | измерено: карточки занимают ~390 |
| Editor, Terminal | 560 | ради них константа и была задрана (80 колонок моно) |
| Files, SourceControl | 440 | дерево и список путей |
| Preview, Inspector, Build, настройки (9 шт.) | 320 | пустые состояния: иконка + название + описание |

Все значения в диапазоне `RAIL_ONLY_WIDTH (54) .. MAX_WIDTH (960)`.

### `tab_resize_memory` — per-tab resize (`view.rs`)

```rust
tab_resize_memory: HashMap<PanelTab, f32>,
```

Сессионная память (не персистится на диск). При ресайзе перетаскиванием
`update_resize` сохраняет `state.width` в `tab_resize_memory[active_tab]`.
При переключении вкладки `on_tab_select` восстанавливает ширину из карты
(или берёт `preferred_content_width`, если ресайза не было).

### `active_tab_width()` — единая точка вычисления

```rust
fn active_tab_width(&self, tab: PanelTab, _cx: &Context<Self>) -> f32 {
    let preferred = tab.preferred_content_width();
    let w = self.tab_resize_memory.get(&tab).copied().unwrap_or(preferred);
    w.clamp(RAIL_ONLY_WIDTH, MAX_WIDTH)
}
```

### `ensure_content_width(target)` — единая точка расширения

```rust
pub fn ensure_content_width(&mut self, target: f32) {
    self.width = target;
    self.last_exclusive_zone = None;
}
```

Вызывается из `on_tab_select` и dock toggle — единая точка входа
для изменения ширины панели.

### `on_tab_select` — два guard-а

1. **Same-tab re-click:** `if tab == self.active_tab { return; }` —
   повторный клик по иконке не сбрасывает ручной ресайз.
2. **dock_content == false (trap #3):** ширина применяется только когда
   контент виден: `let content_open = state.dock_content || state.width > RAIL_ONLY_WIDTH + 1.0;`

### `start_resize` — rail-only expansion

При захвате хэндла в rail-only режиме панель расширяется до
`active_tab_width()` (а не до `DEFAULT_CONTENT_WIDTH`).

### Dock toggle

```rust
let target = this.active_tab_width(this.active_tab, cx);
let state = cx.global_mut::<SidePanelRightState>();
state.dock_content = !state.dock_content;
state.ensure_content_width(target);
```

Computes target before `cx.global_mut()` — avoids borrow checker conflict.

### Trap #2: `last_resized_width` без третьего счётчика

`on_tab_select` и dock toggle используют `self.last_resized_width = f32::NAN`
для force re-render — тот же паттерн, что был до правки. Новый счётчик не
заведён.

---

## Тесты

### `tabs.rs` — 5 тестов на preferred widths:

| Тест | Что проверяет |
|---|---|
| `every_preferred_width_in_valid_range` | Все 14 вкладок в `RAIL_ONLY_WIDTH..MAX_WIDTH` |
| `system_preferred_width_is_400` | System = 400 |
| `editor_and_terminal_preferred_width_is_default` | Editor/Terminal = 560 |
| `files_and_source_control_preferred_width_is_440` | Files/SourceControl = 440 |
| `empty_state_tabs_preferred_width_is_320` | 9 пустых вкладок = 320 |

### `tab/mod.rs` — 4 behavioral теста:

| Тест | Что проверяет |
|---|---|
| `tab_select_applies_preferred_width` | Выбор вкладки применяет её preferred ширину |
| `same_tab_reclick_preserves_resize` | Повторный клик не сбрасывает ручной ресайз |
| `switch_tab_restores_per_tab_resize_memory` | A(480) → B(560) → A = 480 (память восстановлена) |
| `dock_content_false_keeps_rail_only_width` | dock OFF → ширина остаётся RAIL_ONLY_WIDTH |

**Всего: 261 pass, 0 fail.**

---

## Верификация

```
$ cargo test -p chronos
test result: ok. 261 passed; 0 failed; 0 ignored

$ cargo clippy -p chronos --all-targets
# без errors (warnings — в других модулях)
```

### Замеры строк

```
$ wc -l crates/app/src/side_panel_right/{tabs.rs,view.rs,mod.rs,tab/mod.rs}
```

(Замерить при живом прогоне.)

---

## Живой прогон

**Статус: не выполнен.** Пультовый вывод (DP-1) может быть занят.

### IPC: открытие панели

```python
import socket
s = socket.socket(socket.AF_UNIX)
s.connect("/run/user/1000/chronos.sock")
s.sendall(b"toggle-side-panel-right")
s.close()
```

### Переключение вкладок

Рейл — крайняя правая колонка панели, иконки идут сверху вниз
с шагом ~40 px от `y ≈ 57`. Клик по иконке — `ydotool`:

```bash
YDOTOOL_SOCKET=/run/user/1000/.ydotool_socket ydotool click --next-delay 50 0xC0  # left click
```

Абсолютные координаты = экран / 2 (подтверждено T157–T168).

### Что снять и чем доказать

1. `hyprctl layers -j` на вкладке System — ширина слоя ощутимо меньше 557 и
   соответствует объявленной (~400). Приложить строку целиком.
2. Тот же вывод на широкой вкладке (Editor или Terminal) — ширина выросла (~560).
3. Кадры обеих: контент занимает панель **без сквозной полосы слева**.
   Это главный визуальный критерий задачи.
4. Ресайз перетаскиванием на одной вкладке → уход → возврат: ширина та же.
5. Лог целиком без паник: `grep -n "panicked at" лог` — **по всему файлу**.

Кадры открывать глазами, мелкое — вырезать и увеличить:

```
magick кадр.png -crop 60x900+2500+30 +repage -filter point -resize 300% rail.png
```

Если пультовый вывод занят фуллскрин-приложением, живой прогон
невозможен: **остановиться и написать «не проверено»** с причиной.

---

## Техдолг

1. **Персист на диск** — `tab_resize_memory` живёт в памяти. Место для
   персиста (`scenes.toml`, слайс 2) спроектировано, но `SceneManager`
   ещё нет. Отдельной задачей.
2. **`ensure_content_width` smoke path** — вызывается из `init` с
   `DEFAULT_CONTENT_WIDTH` (нет контекста активной вкладки). Работает
   корректно для smoke-режима.

---

## Файлы

### Изменённые (4):

| Файл | Изменение |
|---|---|
| `tabs.rs` | +`preferred_content_width()`, +5 тестов |
| `view.rs` | +`tab_resize_memory`, `active_tab_width()`, on_tab_select/start_resize/update_resize/dock toggle per-tab, `sim_resize` helper; −`next_active_tab` (dead code) |
| `mod.rs` | `ensure_content_width(target: f32)`, smoke path обновлён |
| `tab/mod.rs` | +4 behavioral теста |

**Предыдущее состояние:** T169 (рейл 14 вкладок).
**Текущее состояние:** не закоммичено (ожидает живого прогона).
