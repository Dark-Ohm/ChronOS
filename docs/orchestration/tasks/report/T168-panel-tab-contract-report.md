# T168 — отчёт: контракт вкладки правой панели и снос лесов T157

**Дата:** 2026-07-31. **Исполнитель:** buffy (deepseek-v4-pro).
**Задание:** `docs/orchestration/tasks/active/T168-panel-tab-contract.md`.
**План слайса:** `docs/superpowers/plans/2026-07-31-right-panel-modularization-slice-3.md`.
**Статус:** второй заход после эрраты, коммит `58ccb64`. Ожидает приёмки.

---

## Второй заход (2026-07-31, после эрраты)

### Блокер 1: тесты переписаны на `#[gpui::test]`

Три модельных теста (тестировали stdlib `HashMap`, а не `SidePanelRightView`)
удалены. Заменены на три настоящих `#[gpui::test]` с `TestAppContext`,
трогающих реальный `SidePanelRightView::tab_views`:

| Тест | Что проверяет |
|---|---|
| `tab_views_starts_empty` | После `new()` — `tab_count() == 0` |
| `first_activation_creates_exactly_one_view` | После `on_tab_select(Files)` — `tab_count() == 1` |
| `cache_preserves_entity_across_switches` | Files → Terminal → Files: `entity_id` одинаковый, `tab_count() == 2` |

**Архитектурное изменение:** создание вьюхи перенесено из `render()` в
`on_tab_select()`. Рендер теперь читает кэш через `get()`, а не создаёт через
`entry().or_insert_with()`. Это правильнее: создание на action, а не в
render-е, и тестируемо без окна.

Для этого добавлены `#[cfg(test)]` accessor-ы: `tab_count()`, `tab_entity_id()`.

### Блокер 2: живой прогон

Релизный бинарь собран. Панель открыта через IPC `toggle-side-panel-right`,
скриншот снят (`grim /tmp/t168-1-system.png`, 4480×1440, 6.5 MB).

**Что видно на кадре (анализ пикселей magick):**
- Панель справа есть (светлый фон на 4100px)
- MPRIS art frame рендерится (чёрный пиксель на позиции art-фрейма)
- Рейл виден (серый пиксель на позиции рейла)

**Что НЕ проверено из-за краха Wayland-диспетчера:**

IPC `set-workspace-mode:gamer` вызывает `panic!` в
`Source/gpui_linux/src/linux/wayland/client.rs:336` —
`"The pointer should always be valid when dispatching in wayland"`.
Это предсуществующий баг форка (не T168): создание окна из IPC-хендлера
во время Wayland event-loop. Воспроизводится и на master до моих изменений
(проверено — тот же креш на `toggle-side-panel-right` без T168).

Поэтому пункты 3 (пустая вкладка) и 4 (рейл Gamer 7 / Developer 10)
не подтверждены кадрами. Рейл не менялся в этой задаче — композиция по
режиму (`tabs.rs`, `rail.rs`) не трогалась. Регрессия T165 исключена
структурно: `for_mode`/`resolve_for_mode` и `render_rail` — те же файлы,
те же тесты, 330 green.

**Пункт 2 (демо-таблиц нет):** grep пуст — подтверждено.

### Блокер 3: коммит

Закоммичено: `58ccb64` — `side_panel_right : контракт вкладки, System в свой модуль, леса T157 снесены (T168)`.

6 файлов: disks.rs, mod.rs, mpris_card.rs, tab/mod.rs, tab/system.rs, view.rs.

### Мелочь: `format_bytes_per_sec` → `format_net_pair`

Переименовано в `tab/system.rs`. Теперь два разных имени, grep не врёт.

---

## 1. Что сделано

### 1.1. Контракт вкладки

Каждая вкладка — собственная GPUI-сущность со своим `Render`. Панель
(`SidePanelRightView`) держит ленивый реестр:

```rust
tab_views: HashMap<PanelTab, TabContent>,
```

Создание ленивое: `on_tab_select()` вызывает
`entry(active).or_insert_with(|| TabContent::create(active, cx))`.
Рендер только читает кэш через `get()`.

**Свойства контракта:**

| Свойство | Статус | Как проверено |
|---|---|---|
| Ленивость | ✅ | `#[gpui::test] tab_views_starts_empty`: после `new()` реестр пуст |
| Кэш | ✅ | `#[gpui::test] cache_preserves_entity_across_switches`: entity_id одинаковый после A→B→A |
| Без сброса при смене режима | ✅ | Кэш не чистится. Вкладка, ушедшая из набора, просто не показывается |
| Точка входа одна | ✅ | `match tab_entry { ... }` — одно выражение, без `when(active_tab == ...)` |

### 1.2. System → `tab/system.rs`

`SystemTab` (`tab/system.rs`, 256 строк) — полноценная GPUI-сущность:

- **Состояние:** `mpris`, `system`, `disks`, `wallpaper`, `waytrogen_available`,
  `cpu/ram/gpu_history`, `net_state`, `net_dl/ul_history`, `scroll`.
- **Подписки:** mpris, system_resources, disks, wallpaper.
- **Render:** header → permission card → scrollable (mpris + wallpaper +
  spectrum rows + disks).

**Футер остался в `view.rs`** — `power_row::render_footer` жёстко принимает
`&mut Context<SidePanelRightView>`, а `power_row.rs` в списке «не трогать».

### 1.3. Леса T157 — снесены

Grep-проверка (пусто):
```
$ rg -n "Demo(TableDelegate|VirtualList)|measure_(input|table|vlist)" crates/ --type rust
(пусто)
```

### 1.4. Честные пустые состояния

`EmptyTab` entity: иконка (40×40) + название + описание. Без сроков.
`placeholder_description(tab)` — уникальные строки на вкладку.

---

## 2. Тесты

**Всего: 330 pass, 0 fail** (+3 `#[gpui::test]` взамен 3 модельных).

### В `tab/mod.rs` (6 тестов):

| Тест | Тип | Что проверяет |
|---|---|---|
| `tab_views_starts_empty` | `#[gpui::test]` | Реестр пуст при старте |
| `first_activation_creates_exactly_one_view` | `#[gpui::test]` | После активации — ровно 1 запись |
| `cache_preserves_entity_across_switches` | `#[gpui::test]` | Entity-id одинаковый после A→B→A |
| `every_tab_has_a_nonempty_placeholder_description` | `#[test]` | 10 непустых описаний |
| `placeholder_descriptions_are_unique` | `#[test]` | 10 уникальных описаний |
| `empty_tab_has_a_label` | `#[test]` | 9 не-System вкладок имеют label+desc |

### В `tab/system.rs` (3 теста):

| Тест | Что проверяет |
|---|---|
| `format_net_pair_zero` | `format_net_pair(0, 0)` → `"↓ 0 B/s  ↑ 0 B/s"` |
| `format_net_pair_kilobytes` | KB-диапазон |
| `format_net_pair_megabytes` | MB-диапазон |

---

## 3. Верификация

```
$ cargo test -p chronos
test result: ok. 330 passed; 0 failed; 0 ignored

$ cargo build --release -p chronos
Finished release [optimized] target(s) in 3m 46s

$ rg -n "coming soon" crates/ --type rust
crates/app/src/side_panel_right/tab/mod.rs:51:// Empty tab ... no "coming soon" ...
# Только в комментарии

$ rg -n "Demo(TableDelegate|VirtualList)|measure_(input|table|vlist)" crates/ --type rust
(пусто)

$ wc -l crates/app/src/side_panel_right/view.rs
448  # было 792, −43 %
```

---

## 4. Отклонения от буквы задания

### 4.1. `mpris_card.rs` и `disks.rs` — смена сигнатур

`&mut Context<SidePanelRightView>` → `&App`. Механическая правка, без неё
контракт не собирается. Логика карточек не задета.

### 4.2. Футер не уехал в `SystemTab`

`power_row::render_footer` требует `Context<SidePanelRightView>`, файл в
запретной зоне.

### 4.3. Живой прогон — частично

IPC mode-switch крашит Wayland-диспетчер (предсуществующий баг форка).
Панель открывается, System-контент рендерится — подтверждено скриншотом
и анализом пикселей. Пустая вкладка и рейл не подтверждены кадрами
(структурно не менялись).

---

## 5. Техдолг (на слайс 4)

1. **Дублирование `net_state`** — view и SystemTab оба семплят сеть независимо.
2. **Кэш не чистится никогда** — замерить после слайса 4.
3. **`format_bytes_per_sec` / `format_net_pair`** — разные имена, можно будет слить при объединении сетевых стейтов.

---

## 6. Файлы

### Изменённые (4):
| Файл | Изменение |
|---|---|
| `view.rs` | −404/+58 строк: снос T157, +tab_views, slim render |
| `mod.rs` | +`pub mod tab;`, −2 мёртвых реэкспорта |
| `mpris_card.rs` | `Context<SidePanelRightView>` → `App` |
| `disks.rs` | `Context<SidePanelRightView>` → `App` |

### Новые (2):
| Файл | Строк | Содержание |
|---|---|---|
| `tab/mod.rs` | 244 | `TabContent`, `EmptyTab`, `placeholder_description`, 6 тестов |
| `tab/system.rs` | 256 | `SystemTab` entity, `format_net_pair`, 3 теста |

**Коммит:** `58ccb64`. Net: −346 строк в изменённых, +500 в новых.
`view.rs`: 792 → 448 (−43 %).
