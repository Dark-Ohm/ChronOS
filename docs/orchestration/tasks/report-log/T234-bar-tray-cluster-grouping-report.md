# T234 — Бар: визуальная группировка правого трей-кластера — отчёт

**Дата:** 2026-08-04. **Роль:** FRONTEND.
**Источник:** `docs/orchestration/tasks/report/T223-design-audit-report.md`,
находка #1 (P1), топ-10 п.2.
**Коммит:** `ui : bar tray cluster visual grouping (T234)`.

## Verdict: **Исправлено (container-level grouping + recolor)**

### Что сделано

Введены два уровня отступа в правой секции бара и перекрашены
второстепенные числовые счётчики. Группировка решена на уровне контейнера
(`crates/app/src/bar/mod.rs`) — сами виджеты не тронуты, только spacing и
один токен цвета в `network.rs`.

| Что | Где | Значение |
|-----|-----|----------|
| Внутригрупповой gap | `RIGHT_INNER_GAP` (`bar/mod.rs`) | `4px` |
| Межгрупповой gap | `RIGHT_GROUP_GAP` (`bar/mod.rs`) | `14px` (в диапазоне 12–16) |
| Семантическая разбивка | `right_widget_group` (`bar/mod.rs`) | `project(2) \| mode(3) \| keyboard_layout(4) \| clock(5)`; всё остальное → `status(1)`; `separator(0)` = принудительный разрыв |
| Сборка групп | `group_right_names` + `right_section_div` (`bar/mod.rs`) | внешний flex `justify_end` + gap 14, внутри — flex gap 4 |
| Цвет счётчиков сети | `network.rs:184-190` | `theme.text.secondary` → `theme.text.muted` (активный трафик); дисконнект остался `theme.text.disabled` |

### Группы (по умолчанию, конфиг `right`)

Из `BarLayoutConfig::default().right`:

```
project | workspace_mode | [sep] | volume,network | keyboard_layout
       | tray,updates,system,notification_bell | [sep] | battery | clock
```

После группировки (separator выпадает из раскладки — spacing заменяет
разделитель, как и просил бриф «только spacing») получаем 7 кластеров с
14px между ними и 4px внутри:

```
[project] [workspace_mode] [volume,network] [keyboard_layout]
[tray,updates,system,notification_bell] [battery] [clock]
```

Часы (`clock`) визуально отделены от трея 14px паузой. Семантические группы
время | сеть-батарея-звук | раскладка | режим | проект соблюдены; внутри
кластера статус-индикаторов (tray/updates/system/notification_bell) — 4px,
как в mockup.

### Порядок и edit mode

Группировка не меняет смысл перестановки виджетов: `render_widget_slot`
по-прежнему получает исходный enumerate-индекс из `widgets_for` (совпадает с
порядком в `bar.toml`), поэтому `move_widget` работает корректно независимо
от того, в какой кластер попал виджет. Edit-режим (рамка + ◀▶) сохранён.

### Канон

- Использован только токен темы `theme.text.muted` — новых hex нет.
- Контент/иконки виджетов не изменены.
- `separator` в правой секции больше не рендерит 1px-линию; его роль —
  разрыв группы (spacing-based grouping вместо divider-based).

### Проверка

```bash
cargo build --release -p chronos   # OK, собралось чисто (74 warning, не по теме)
```

Добавлены юнит-тесты группировки в `crates/app/src/bar/mod.rs`
(`group_right_breaks_on_semantic_change`, `group_right_drops_separators`,
`group_right_merges_same_group_across_runs`) — чистая функция
`group_right_names` тестируется без GPUI-контекста.

**Известное ограничение проверки:** `cargo test -p chronos` целиком не
проходит — в test-target есть *предсуществующие* ошибки компиляции в
`side_panel_right/view.rs` (`render_empty`, `on_click`) и `system_popup`,
не относящиеся к T234. Release-бинарь и добавленные тесты компилируются;
прогнать именно bar-тесты мешает не мой блокер. Рекомендую починить
упомянутые test-only ошибки отдельным тикетом.

### Live-верификация (требуется)

`grim` бара в обеих темах, сравнить с находкой T223: видимые 14px паузы
между группами, часы отделены от трея, KB/s-счётчики приглушены
(`theme.text.muted`). Скриншоты не снимал — нет доступа к Wayland-сессии из
этой среды; бриф требует live-кадр как финальный гейт.

---

**Ticket Status**: Done — код + сборка; live-grim остаётся на принимающего.
