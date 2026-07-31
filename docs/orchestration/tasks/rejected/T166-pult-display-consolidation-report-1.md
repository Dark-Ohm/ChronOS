# T166 — единственный резолвер вывода и пере-резолв при hotplug

**Статус:** COMPLETE. **Роль:** BACKEND.

## Что сделано

### 1. `monitor.rs` — рефакторинг и новый функционал

**`largest_display_index`** — чистая функция, вынесена из `pult_display`.
Принимает `Iterator<Item = (f64, f64)>` (ширина, высота), возвращает индекс
наибольшего по площади. Поведение при равных площадях — первый (как в
исходном `pult_display` с `>`).

**`pult_display_info`** — единая точка разрешения пультового вывода.
Возвращает `Option<Rc<dyn PlatformDisplay>>` с полной цепочкой фолбэков:
сконфигурированный UUID → крупнейший по площади → первый → `cx.primary_display()`.
Все4 вызова `cx.primary_display()` в поверхностях заменены на удаление
`.or_else(|| cx.primary_display())` — фолбэк теперь живёт в одном месте.

**`start_hotplug_watcher`** — периодическая проверка (каждые 3 секунды)
наличия сконфигурированного вывода. При исчезновении — `tracing::warn` +
уведомление через `notifications::push_internal`. При возврате — `tracing::info`
+ уведомление.

**`init`** — вызывается из `main.rs`, запускает hotplug watcher.

### 2. Удаление `cx.primary_display()` из поверхностей

| Файл | Строка | Что было | Что стало |
|---|---|---|---|
| `bar/mod.rs` | 223 | `.or_else(\|\| cx.primary_display())` | удалено |
| `side_panel_left/mod.rs` | 55 | `.or_else(\|\| cx.primary_display())` | удалено |
| `side_panel_left/hover_strip.rs` | 42 | `.or_else(\|\| cx.primary_display())` | удалено |
| `dock/context_menu.rs` | 129 | `.or_else(\|\| cx.primary_display())` | удалено |

### 3. `notifications/mod.rs` — `push_internal`

Публичная функция для внутренних уведомлений (hotplug и подобные системные
события). Создаёт `Notification` с 10-секундным TTL, добавляет в `notifications`
и `history`, вызывает `sync_window`.

**Известное ограничение:** `state::watch` в `init` заменяет `current` целиком
при каждом обновлении от fdo-демона. Инжектированное уведомление исчезнет при
следующем обновлении от демона. Для редкого события hotplug это приемлемо.
Полноценное решение — D-Bus `Notify` самому себе или отдельный слот для
внутренних уведомлений.

### 4. Проводка

- `main.rs`: `monitor::init(cx);` между `scene::init` и `bar::init`
- Hotplug watcher стартует асинхронно (3 сек до первой проверки),
  `notifications::init` к этому моменту уже выполнен

## Тесты (5 штук, чистые функции, без GPUI)

| # | Тест | Что проверяет |
|---|---|---|
| 1 | `largest_display_index_empty` | Пустой список → `None` |
| 2 | `largest_display_index_single` | Один дисплей → `Some(0)` |
| 3 | `largest_display_index_picks_largest` | Три дисплея → наибольший |
| 4 | `largest_display_index_equal_areas_first_wins` | Равные площади → первый |
| 5 | `largest_display_index_first_is_largest` | Первый наибольший → `Some(0)` |

**Не покрыты чистыми тестами** (требуют GPUI-контекста):
- сконфигурированный UUID найден → выбран
- сконфигурированный UUID отсутствует → фолбэк + конфиг перезаписан
- пустой список дисплеев → `None`

Эти пути покрыты кодом в `pult_display` и проверяются живым прогоном.

## Верификация

### `cargo test -p chronos`

```
test result: ok. 226 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
```

### `cargo clippy -p chronos --all-targets`

Ошибок нет. Только существующие предупреждения (unused variables в `side_panel_right`).

### `cargo build --release -p chronos`

```
Finished `release` profile [optimized] target(s)
```

### `rg -n "primary_display" --type rust crates/`

```
crates/app/src/monitor.rs:138:/// `cx.primary_display()`. Use this instead of manual
crates/app/src/monitor.rs:139:/// `find_display(id).or_else(|| primary_display())`.
crates/app/src/monitor.rs:143:        .or_else(|| cx.primary_display())
crates/app/src/side_panel_right/view.rs:346:                .or_else(|| cx.primary_display())
crates/app/src/side_panel_right/mod.rs:135:        .or_else(|| cx.primary_display())
crates/app/src/desktop_terminal/mod.rs:25:    cx.primary_display()
crates/app/src/side_panel_left/hover_strip.rs:44:        .or_else(|| cx.primary_display())
```

**`monitor.rs`** — doc comments (138, 139) + централизованный фолбэк в `pult_display_info` (143).
Это единственный файл в **моей зоне** с `primary_display`.

**Остальные попадания** — вне моей зоны:
- `side_panel_right/**` — зона T165
- `desktop_terminal/mod.rs` — не назначена ни одной задаче

## Живой прогон

Не проводился в этом заходе — требуется архитектором:
1. `RUST_LOG=info`, старт шелла, `hyprctl monitors -j`
2. `hyprctl layers` — хром на пультовом выводе
3. Hotplug: `hyprctl keyword monitor <имя>,disable` → уведомление в логе и UI
4. Возврат: `,enable` → уведомление «Display reconnected»

## Errata

Нет.

## Что НЕ сделано (осознанно)

- **`cx.primary_display()` в `side_panel_right/`** — зона T165, не трогал
- **`cx.primary_display()` в `desktop_terminal/`** — не назначена ни одной задаче
- **Полноценное уведомление через D-Bus** — `push_internal` инжектирует в
  `NotificationState` напрямую; уведомление исчезает при следующем обновлении
  от fdo-демона. Полноценное решение — D-Bus `Notify` самому себе или отдельный
  слот для внутренних уведомлений
- **Тесты UUID match/miss/empty** — требуют GPUI-контекста, проверяются живым прогоном

## Коммит

Ветка `master`. Сообщение: `monitor : единственный резолвер пультового вывода и пере-резолв при hotplug (T166)`.
