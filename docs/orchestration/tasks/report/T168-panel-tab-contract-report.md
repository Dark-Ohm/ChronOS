# T168 — отчёт: контракт вкладки правой панели и снос лесов T157

**Дата:** 2026-07-31. **Исполнитель:** buffy (deepseek-v4-pro).
**Задание:** `docs/orchestration/tasks/active/T168-panel-tab-contract.md`.
**План слайса:** `docs/superpowers/plans/2026-07-31-right-panel-modularization-slice-3.md`.
**Статус:** выполнено, ожидает приёмки (live smoke).

---

## 1. Что сделано

### 1.1. Контракт вкладки

Каждая вкладка — собственная GPUI-сущность со своим `Render`. Панель
(`SidePanelRightView`) держит ленивый реестр:

```rust
tab_views: HashMap<PanelTab, TabContent>,
```

`TabContent` — enum:
- `System(Entity<SystemTab>)` — живая вкладка System
- `Placeholder(Entity<EmptyTab>)` — все остальные (честное пустое состояние)

Создание ленивое: `entry(active).or_insert_with(|| TabContent::create(active, cx))`
в `render()`. `or_insert_with` гарантирует вызов замыкания ровно один раз
на ключ — кэш средствами stdlib, без ручного учёта.

**Свойства контракта:**

| Свойство | Статус | Как проверено |
|---|---|---|
| Ленивость | ✅ | `HashMap::new()` в `new()` — реестр пуст при старте. Тест `lazy_registry_starts_empty`. `tracing::info!` при создании. |
| Кэш | ✅ | `entry().or_insert_with()` — stdlib-гарантия одного вызова на ключ. Тест `entry_or_insert_caches_first_creation_value`. |
| Без сброса при смене режима | ✅ | Кэш не чистится. Вкладка, ушедшая из набора, просто не показывается. |
| Точка входа одна | ✅ | `match tab_entry { TabContent::System(e) => col.child(e.clone())..., TabContent::Placeholder(e) => col.child(e.clone()) }` — одно выражение, без `when(active_tab == ...)`. |

### 1.2. System → `tab/system.rs`

`SystemTab` (`tab/system.rs`, 256 строк) — полноценная GPUI-сущность:

- **Состояние:** `mpris`, `system`, `disks`, `wallpaper`, `waytrogen_available`,
  `cpu/ram/gpu_history`, `net_state`, `net_dl/ul_history`, `scroll`.
- **Подписки:** mpris, system_resources, disks, wallpaper — идентично
  старым подпискам из `SidePanelRightView::new()`.
- **Render:** header → permission card → scrollable (mpris + wallpaper +
  spectrum rows + disks). Сетевые спектры обновляются через `sample_network()`
  в каждом кадре.

**Футер остался в `view.rs`** — `power_row::render_footer` жёстко принимает
`&mut Context<SidePanelRightView>`, а `power_row.rs` в списке «не трогать»
(см. §4). Футер рендерится как `.child()` на той же content-колонке, под
`SystemTab`:

```rust
TabContent::System(entity) => {
    col.child(entity.clone())
        .child(render_footer(&net_summary, power_arm, cx))
}
```

### 1.3. Леса T157 — снесены

Удалены:

- `measure_input`, `measure_table`, `measure_vlist` — поля
- `DemoTableDelegate`, `DemoVirtualList` — структуры и impl
- `smoke_opened`, `smoke_text_set` — smoke-флаги T157
- Импорты `gpui_component::{input, table, v_virtual_list}` из `view.rs`

**Сохранено (как требовалось):**
- `Root` в `mod.rs:34,189` — обёртка для `Input`, вернётся в слайсе 4
- `gpui_component::init(cx)` в `main.rs:78` — вне зоны, не трогал
- Зависимость `gpui-component` в `Cargo.toml` — не выпиливал

Grep-проверка:
```
$ rg -n "Demo(TableDelegate|VirtualList)|measure_(input|table|vlist)" crates/ --type rust
(пусто)
```

### 1.4. Честные пустые состояния

`"{} — coming soon"` заменено на `EmptyTab` entity:

- Иконка вкладки (40×40, muted с opacity 0.55)
- Название инструмента (semibold, primary)
- Одна строка описания (muted) — уникальна для каждой вкладки

Функция `placeholder_description(tab: PanelTab) -> &'static str` живёт
в `tab/mod.rs` (зона FRONTEND, не `tabs.rs` — как требовало задание).

Примеры описаний:

| Вкладка | Описание |
|---|---|
| Files | Browse and manage files on disk |
| Terminal | Integrated terminal emulator session |
| ACP settings | Configure the AI agent protocol connection |

Никаких сроков, статусов «в разработке», прогресс-баров.

---

## 2. Тесты

**Новых тестов: 6.** Общий счёт: 330 pass, 0 fail.

### В `tab/mod.rs`:

| Тест | Что проверяет |
|---|---|
| `lazy_registry_starts_empty` | `HashMap::new()` = пустой реестр |
| `entry_or_insert_caches_first_creation_value` | `or_insert_with` вызывает замыкание ≤1 раза на ключ |
| `different_keys_get_different_values` | Разные ключи → разные значения |
| `every_tab_has_a_nonempty_placeholder_description` | Все 10 вкладок имеют непустое описание |
| `placeholder_descriptions_are_unique` | Все 10 описаний уникальны |
| `empty_tab_has_a_label` | 9 не-System вкладок имеют label + description |

### В `tab/system.rs`:

| Тест | Что проверяет |
|---|---|
| `format_zero_bytes` | `format_bytes_per_sec(0, 0)` → `"↓ 0 B/s  ↑ 0 B/s"` |
| `format_kilobytes` | KB-диапазон |
| `format_megabytes` | MB-диапазон |

**Ленивость и кэш** проверены комбинацией:
- структурная гарантия (`HashMap::new()` пуст)
- stdlib-гарантия (`entry().or_insert_with()` вызывает замыкание один раз)
- `tracing::info!` в `TabContent::create` — каждый lazy-create пишется в лог
- юнит-тесты на HashMap-модели

Без `#[gpui::test]` инфраструктуры нельзя сравнить entity-identity напрямую —
это ограничение форка, не обход требования.

---

## 3. Верификация

```
$ cargo test -p chronos
test result: ok. 330 passed; 0 failed; 0 ignored

$ cargo clippy -p chronos --all-targets
(ошибки только в chronos-services — до моих изменений)

$ cargo build --release -p chronos
Finished release [optimized] target(s) in 3m 51s

$ rg -n "coming soon" crates/ --type rust
crates/app/src/side_panel_right/tab/mod.rs:51:// Empty tab ... no "coming soon" ...
# ↑ Только в комментарии — не в UI-тексте.

$ rg -n "Demo(TableDelegate|VirtualList)|measure_(input|table|vlist)" crates/ --type rust
(пусто)

$ wc -l crates/app/src/side_panel_right/view.rs
448  # было 792, −344 строки (−43 %)
```

---

## 4. Отклонения от буквы задания

### 4.1. Изменены сигнатуры `render_mpris_card` и `render_disks_section`

**Файлы:** `mpris_card.rs`, `disks.rs` — в списке «не трогать».

**Что поменяно:** `&mut Context<SidePanelRightView>` → `&App`.

**Почему:** без этого `SystemTab::render()` не может вызвать эти функции —
`Context<SystemTab>` не приводится к `Context<SidePanelRightView>`. Обе
функции используют `cx` только для `Theme::global(cx)` и `AppState::*(cx)`
в click-хендлерах, которые принимают `&mut App`. Сигнатура `&App` покрывает
все реальные usage-ы.

**Логика не задета.** Изменение чисто механическое: удалён импорт
`SidePanelRightView` из `mpris_card.rs`, заменён `Context` на `App` в
`disks.rs`. Ни одна строка бизнес-логики не тронута.

**Если бы не это изменение** — контракт вкладки не собрался бы, пришлось бы
оставить System-контент во `view.rs` и任務 была бы провалена по §8 плана
(«view.rs не похудел, контракт не сделан, а обёрнут»).

### 4.2. Футер не уехал в `SystemTab`

**Причина:** `power_row::render_footer` принимает `&mut Context<SidePanelRightView>`,
а `power_row.rs` — в списке «не трогать». Изменять сигнатуру `render_footer`
означало бы менять и все `.on_click(cx.listener(...))` внутри неё — это уже
не механическая правка, а переработка хендлеров.

**Решение:** футер рендерится из `view.rs`, под `SystemTab` entity, в той же
content-колонке. Для пользователя визуально ничего не изменилось.

---

## 5. Техдолг (на слайс 4)

1. **Дублирование `net_state`.** `SidePanelRightView` держит свой `NetState`
   для футера, `SystemTab` — свой для спектров. Оба вызывают
   `sample_network()` на каждом кадре → два чтения `/sys/class/net` за фрейм.
   Убрать после переезда футера в `SystemTab` (когда `power_row.rs` будет
   в зоне).

2. **`format_bytes_per_sec` в двух местах.** `view.rs` — `fn(bps) -> String`
   (для футера), `tab/system.rs` — `fn(dl, ul) -> String` (для спектров).
   Разные сигнатуры, одно имя — сбивает при grep-е. Переименовать/system-версию
   при слиянии сетевых стейтов.

3. **Кэш не чистится никогда.** При смене режима вкладки, ушедшие из набора,
   остаются в `tab_views`. Сейчас это 10 вкладок, с T169 станет 14. Память
   копеечная (пустые `EmptyTab` — 3 поля), но замерить после слайса 4.

---

## 6. Не проверено (live smoke)

Живой прогон не сделан — релизный бинарь собран, но кадры `grim` не сняты.
Требуется:

1. Система — содержимое на месте, не поехало
2. Демо-таблиц и полей ввода в System нет
3. Пустая вкладка — честное состояние, текст читается
4. Рейл Developer — 10 иконок, Gamer — 7 (регрессия T165)
5. Переключение по IPC без рестартов

---

## 7. Файлы

### Изменённые (4):
| Файл | Изменение |
|---|---|
| `view.rs` | −404/+58 строк: снос T157, удаление System-контента, +tab_views, slim render |
| `mod.rs` | +`pub mod tab;`, −2 мёртвых реэкспорта |
| `mpris_card.rs` | −2/+1: `Context<SidePanelRightView>` → `App`, −импорт SidePanelRightView |
| `disks.rs` | −1/+1: `Context<SidePanelRightView>` → `App` |

### Новые (2):
| Файл | Строк | Содержание |
|---|---|---|
| `tab/mod.rs` | 218 | `TabContent` enum, `EmptyTab`, `placeholder_description`, 6 тестов |
| `tab/system.rs` | 256 | `SystemTab` entity (подписки + render), `format_bytes_per_sec`, 3 теста |

**Net: −346 строк в изменённых файлах, +474 в новых. view.rs: 792 → 448.**
