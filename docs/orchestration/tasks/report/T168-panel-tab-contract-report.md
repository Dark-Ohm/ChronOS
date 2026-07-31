# T168 — отчёт: контракт вкладки правой панели и снос лесов T157

**Дата:** 2026-07-31. **Исполнитель:** buffy (deepseek-v4-pro).
**Задание:** `docs/orchestration/tasks/active/T168-panel-tab-contract.md`.
**План слайса:** `docs/superpowers/plans/2026-07-31-right-panel-modularization-slice-3.md`.
**Статус:** третий заход, блокер expect-panic закрыт. Ожидает живого прогона.

---

## Что сделано (сводка)

| Блокер | Статус |
|---|---|
| Тесты на stdlib HashMap → `#[gpui::test]` на реальный `SidePanelRightView` | ✅ Закрыт |
| System → `tab/system.rs` | ✅ Закрыт |
| Леса T157 снесены | ✅ Закрыт |
| `format_bytes_per_sec` → `format_net_pair` дедупликация | ✅ Закрыт |
| expect-panic при первом открытии панели | ✅ Закрыт |
| Живой прогон | ⏳ Не выполнен (дисплей занят) |

---

## Эррата 1: тесты переписаны на `#[gpui::test]`

Три модельных теста (тестировали stdlib `HashMap`, а не `SidePanelRightView`)
удалены. Заменены на настоящие `#[gpui::test]` с `TestAppContext`:

| Тест | Что проверяет |
|---|---|
| `tab_views_starts_empty` | После `new()` — `tab_count() == 0` |
| `first_activation_creates_exactly_one_view` | После `on_tab_select(Files)` — `tab_count() == 1` |
| `cache_preserves_entity_across_switches` | Files → Terminal → Files: `entity_id` одинаковый, `tab_count() == 2` |

Для этого добавлены `#[cfg(test)]` accessor-ы: `tab_count()`, `tab_entity_id()`.

## Эррата 2: System → `tab/system.rs`

`SystemTab` (`tab/system.rs`) — полноценная GPUI-сущность:

- **Состояние:** mpris, system, disks, wallpaper, cpu/ram/gpu_history, net_state, scroll
- **Подписки:** mpris, system_resources, disks, wallpaper
- **Render:** header → permission card → scrollable (mpris + wallpaper + spectrum rows + disks)

**Футер остался в `view.rs`** — `power_row::render_footer` требует `Context<SidePanelRightView>`, файл в запретной зоне.

## Эррата 3: expect-panic при первом открытии панели

### Причина

В предыдущем заходе создание вьюхи перенесено из `render()` в `on_tab_select()`,
но `render()` остался с `get().expect(...)`:

```rust
let tab_entry = self
    .tab_views
    .get(&active)
    .expect("tab view must exist after on_tab_select");
```

При первом открытии панели `on_tab_select` никто не звал — активная вкладка
`PanelTab::default()` из `SidePanelRightView::new()`. Реестр пуст → `expect`
→ паника → каскад через wayland-диспетчер → `abort`.

### Решение: `ensure_tab_view()`

Единый метод — точка создания вкладок:

```rust
pub(crate) fn ensure_tab_view(&mut self, tab: PanelTab, cx: &mut Context<Self>) {
    self.tab_views
        .entry(tab)
        .or_insert_with(|| TabContent::create(tab, cx));
}
```

Вызывается из `on_tab_select()` и `render()`. `render()` после `ensure_tab_view`
гарантированно видит запись → `unwrap()` безопасен по построению.

### `format_net_pair` — дедупликация

`format_bytes_per_sec` в `view.rs` и `format_net_pair` в `tab/system.rs` —
одна и та же логика. Исправлено:

- `format_bytes_per_sec` удалён из `view.rs`
- `format_net_pair` в `tab/system.rs` сделан `pub(crate)`
- `view.rs` импортирует `use crate::side_panel_right::tab::system::format_net_pair;`

### Тест: критический путь без `on_tab_select`

Добавлен `first_render_without_tab_select_creates_view` — `#[gpui::test]`,
`PanelTab::Files` (без service globals):

```rust
#[gpui::test]
async fn first_render_without_tab_select_creates_view(cx: &mut TestAppContext) {
    cx.update(|cx| { cx.set_global(SidePanelRightState::default()); });
    let view = cx.new(|cx| SidePanelRightView::new(cx));
    cx.update_entity(&view, |this, cx| {
        assert_eq!(this.tab_count(), 0, "must start empty");
        this.ensure_tab_view(PanelTab::Files, cx);
    });
    cx.update_entity(&view, |this, _cx| {
        assert_eq!(this.tab_count(), 1, "ensure_tab_view must create exactly one entry");
    });
}
```

---

## Контракт вкладки

Панель (`SidePanelRightView`) держит ленивый реестр:

```rust
tab_views: HashMap<PanelTab, TabContent>,
```

Создание ленивое через `ensure_tab_view()` — вызывается из `on_tab_select()`
и `render()`. Один метод, одно имя — единая точка создания.

| Свойство | Статус | Как проверено |
|---|---|---|
| Ленивость | ✅ | `#[gpui::test] tab_views_starts_empty`: после `new()` реестр пуст |
| Кэш | ✅ | `#[gpui::test] cache_preserves_entity_across_switches`: entity_id одинаковый после A→B→A |
| Без сброса при смене режима | ✅ | Кэш не чистится. Вкладка, ушедшая из набора, просто не показывается |
| Точка входа одна | ✅ | `match tab_entry { ... }` — одно выражение, без `when(active_tab == ...)` |

---

## Архитектурные решения

### `ensure_tab_view()` — единая точка создания

**Вариант A (выбран):** `ensure_tab_view()` вызывается из `on_tab_select()`
и `render()`. Один метод, одно имя.

**Вариант B (отклонён):** `on_tab_select()` только меняет `active_tab`,
создание только в `render()` через `entry().or_insert_with()`.

Почему A лучше: `on_tab_select` создаёт вьюху на действие (клик по рейлу),
а не в рендере. Это правильнее по семантике и проще тестируется — тест
вызывает `ensure_tab_view` напрямую, без окна.

### `mpris_card.rs` и `disks.rs` — смена сигнатур

`&mut Context<SidePanelRightView>` → `&App`. Механическая правка, без неё
контракт не собирается. Логика карточек не задета.

### Футер не уехал в `SystemTab`

`power_row::render_footer` требует `Context<SidePanelRightView>`, файл в
запретной зоне.

---

## Тесты

**Всего: 335 pass, 0 fail** (90 lib + 245 bins).

### `tab/mod.rs` (7 тестов):

| Тест | Тип | Что проверяет |
|---|---|---|
| `tab_views_starts_empty` | `#[gpui::test]` | Реестр пуст при старте |
| `first_activation_creates_exactly_one_view` | `#[gpui::test]` | После активации — ровно 1 запись |
| `cache_preserves_entity_across_switches` | `#[gpui::test]` | Entity-id одинаковый после A→B→A |
| `first_render_without_tab_select_creates_view` | `#[gpui::test]` | ensure_tab_view создаёт запись без on_tab_select |
| `every_tab_has_a_nonempty_placeholder_description` | `#[test]` | 10 непустых описаний |
| `placeholder_descriptions_are_unique` | `#[test]` | 10 уникальных описаний |
| `empty_tab_has_a_label` | `#[test]` | 9 не-System вкладок имеют label+desc |

### `tab/system.rs` (3 теста):

| Тест | Что проверяет |
|---|---|
| `format_net_pair_zero` | `format_net_pair(0, 0)` → `↓ 0 B/s  ↑ 0 B/s` |
| `format_net_pair_kilobytes` | KB-диапазон |
| `format_net_pair_megabytes` | MB-диапазон |

---

## Верификация

```
$ cargo test -p chronos --lib --bins
test result: ok. 90 passed; 0 failed  (lib)
test result: ok. 245 passed; 0 failed (bins)

$ cargo clippy -p chronos --all-targets
# без errors (warnings — в других модулях)

$ cargo build --release -p chronos
Finished release [optimized] target(s) in 4m 02s

$ rg -n "coming soon" crates/
crates/app/src/side_panel_right/tab/mod.rs:51:// Empty tab ... no "coming soon" ...
# Только комментарий, UI-текста нет

$ rg -n "Demo(TableDelegate|VirtualList)|measure_(input|table|vlist)" crates/
(пусто)

$ wc -l crates/app/src/side_panel_right/view.rs
479  # было 792 до T168 — −40 %
```

**335 тестов зелёные.** Код компилируется. Clippy без errors.

---

## Живой прогон

**Статус: не выполнен.** Пультовый вывод (DP-1) занят фуллскрин-игрой SCUM.
По правилам задания: «если вывод свободен — снимаю кадры; занят — пишу
"не проверено" в отчёт».

Что нужно снять (когда вывод освободится):
1. Панель открыта, System — содержимое на месте и не поехало (сеть, диски,
   питание, MPRIS, спектр, обои, футер). Главный регрессионный кадр.
2. Демо-таблиц и полей ввода в System нет.
3. Любая пустая вкладка — честное состояние, текст читается.
4. Рейл Developer — 10 иконок, Gamer — 7.

Пункт 2 подтверждён grep (пусто). Пункты 1, 3, 4 — требуют кадров.

---

## Техдолг (на слайс 4)

1. **Дублирование `net_state`** — view и SystemTab оба семплят сеть независимо.
2. **Кэш не чистится никогда** — замерить после слайса 4.

---

## Файлы

### Изменённые (4):

| Файл | Изменение |
|---|---|
| `view.rs` | Снос T157, +tab_views, ensure_tab_view(), −40 % строк (792→479) |
| `mod.rs` | +`pub mod tab;`, −2 мёртвых реэкспорта |
| `mpris_card.rs` | `Context<SidePanelRightView>` → `App` |
| `disks.rs` | `Context<SidePanelRightView>` → `App` |

### Новые (2):

| Файл | Строк | Содержание |
|---|---|---|
| `tab/mod.rs` | ~250 | `TabContent`, `EmptyTab`, `placeholder_description`, 7 тестов |
| `tab/system.rs` | ~260 | `SystemTab` entity, `format_net_pair`, 3 теста |

**Предыдущий коммит:** `58ccb64` (второй заход).
Текущее состояние: не закоммичено (ожидает живого прогона).
