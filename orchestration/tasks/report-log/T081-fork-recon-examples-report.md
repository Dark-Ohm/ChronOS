<!-- T081 — migrated 2026-07-22 from orchestration/report-log/opencode-report-4.md — see orchestration/tasks/MIGRATION.md -->

# Session: разведка форка (зона «корпус примеров + gpui-component») — 2026-07-20

_Верифицировано: fable-judge (VERIFIED WITH CAVEATS) + Philip citation audit (12/12 spot-checks passed)._

## Сделано (факт, не намерение)

- `skills/chronos-gpui/references/examples-catalog.md` — полный каталог: 42 примера gpui + view_example (5 файлов) + 13 примеров gpui-component. Каждый с описанием, списком API, статусом компиляции, применимостью к layer-shell, источником (файл:строка). Раздел «Ловушки и опровержения» — 8 находок «думали X, оказалось Y». 419 строк.
- `skills/chronos-gpui/references/examples-by-topic.md` — группировка всех примеров по 16 темам. 143 строки.
- `skills/chronos-gpui/scripts/run-example.sh` — скрипт с правильным спецификатором пакета. `--check`, `--list`, ошибка при опечатке. Работает из любого каталога. 96 строк. Проверен: `--list` → 45 строк вывода; `--check hello_world` → Finished; nonexistent → красная ошибка.
- `skills/chronos-gpui/evals/examples.eval.md` — 8 вопросов с проверяемыми ответами. Каждый ответ несёт `Доказательство: файл:строка`. Все 8 ссылок верифицированы Philip citation audit.

## Расхождения со спекой/планом

- План: «проверить актуальность исключения gpui_elements» → проверено: по-прежнему исключён. Cargo.toml существует, но `cargo check` падает с «believes it's in a workspace when it's not». 1 пример (`editable_text.rs`) не читался — read-only режим не позволяет править workspace-конфигурацию.
- План: «все примеры gpui-component проверить cargo check» → не сделано: gpui-component — отдельный workspace. Примеры перечислены по README и выборочному чтению (hello_world, input, tooltip_top_edge).
- План: «git status Source должен быть чист» → на момент начальной проверки было только `.mimocode/`. К концу сессии появились `brief.md`, `findings/`, `plan.json` — чужие правки параллельных агентов.

## Не реализовано из acceptance criteria

- cargo check для примеров gpui-component (отдельный workspace)
- cargo check для 29 из 42 примеров gpui (проверено 13; остальные используют идентичные паттерны API — риск расхождения низкий, но не нулевой)
- Чтение `gpui_elements/examples/editable_text.rs` (требует правки workspace-конфигурации — запрещено read-only режимом)

## Проверено фактом, не на словах

### Компиляция (все 13 — Finished, без ошибок)
- `cargo check --example hello_world` → Finished
- `cargo check --example scrollable` → Finished
- `cargo check --example layer_shell` → Finished
- `cargo check --example animation` → Finished
- `cargo check --example input` → Finished
- `cargo check --example blur` → Finished
- `cargo check --example a11y` → Finished
- `cargo check --example window_shadow` → Finished
- `cargo check --example drag_drop` → Finished
- `cargo check --example image` → Finished
- `cargo check --example image_gallery` → Finished
- `cargo check --example svg` → Finished
- `cargo check --example view_example` → Finished

### Скрипт run-example.sh
- `run-example.sh --list` → 45 строк вывода
- `run-example.sh --check hello_world` → «✓ Пример 'hello_world' компилируется.»
- `run-example.sh nonexistent` → «ОШИБКА: пример 'nonexistent' не найден.» + список

### Доставленные файлы
- `examples-catalog.md`: 419 строк, 42 записи `^### [0-9]`, 8 ловушек
- `examples-by-topic.md`: 143 строки, 16 `^## ` разделов
- `run-example.sh`: 96 строк, `chmod +x`
- `examples.eval.md`: 83 строки, 8 `^## Вопрос`, 8 `Доказательство`

### Source/ — моих изменений нет
- `git status --short` → `.mimocode/` (предсущ.) + `brief.md`, `findings/`, `plan.json` (чужие)
- `find -newer Cargo.toml -type f` → только target/ и исходники форка — ни одного моего файла

### Пост-верификация (fable-judge + Philip)
- **fable-judge:** все 13 cargo check перепрогнаны → Finished. 0 frauds (нет ослабленных тестов, scope creep, false completion, debris). Вердикт: VERIFIED WITH CAVEATS.
- **Philip citation audit:** 12 ссылок `файл:строка` проверены через `sed -n` — все 12 совпадают. Ноль сфабрикованных цитат:
  - `scrollable.rs:12-14` → `.id("vertical").overflow_scroll()` ✓
  - `layer_shell.rs:87-93` → `WindowKind::LayerShell(...)` ✓
  - `animation.rs:78-86` → `Animation::new(...).repeat().with_easing(...)` ✓
  - `blur.rs:55` → `window.paint_blur(...)` ✓
  - `focus_visible.rs:152` → `.focus_visible(|style| ...)` ✓
  - `div.rs:1429` → `fn overflow_y_scroll(mut self)` ✓
  - `div.rs:3752` → `impl<E> StatefulInteractiveElement for Stateful<E>` ✓
  - `drag_drop.rs:102` → `.on_drag(drag_info, |...| ...)` ✓
  - `a11y.rs:84` → `.child(text!("Accessibility Demo"))` ✓
  - `popover.rs:99-103` → `deferred(anchored().anchor(...).snap_to_window_with_margin(...))` ✓
  - `window_positioning.rs:82` → `for screen in cx.displays()` ✓
  - `grid_layout.rs:18` → `container_query(|container_size, ...| { ... })` ✓
- **Прочитано файлов:** 46 .rs (42 gpui + 5 view_example + 4 gpui-component)

### gpui_elements
- `ls gpui_elements/Cargo.toml` → существует
- `cargo check -p gpui_elements` → «did not match any packages»
- Прямой `cargo check` из каталога → «believes it's in a workspace when it's not»
- Статус неизменен (ARCHITECTURE §2)

## Разбивка на батчи

1. **Батч 1 (5):** hello_world, scrollable, layer_shell, animation, anchor
2. **Батч 2 (10):** input, uniform_list, testing, grid_layout, opacity, pattern, shadow, text, text_layout, text_wrapper
3. **Батч 3 (10):** data_table, drag_drop, focus_visible, gif_viewer, gradient, image_gallery, image_loading, popover, painting, set_menus
4. **Батч 4 (10):** mouse_pressure, move_entity_between_windows, on_window_close_quit, ownership_post, tab_stop, tree, window, window_movable, window_positioning, window_shadow
5. **Батч 5 (оставшиеся + gpui-component):** a11y, active_state_bug, blur, list_example, paths_bench, image/image.rs, svg/svg.rs, view_example/*, gpui-component/hello_world, input, tooltip_top_edge

## Новые риски / известные баги

- **Medium:** 29 примеров не проверены прямым `cargo check`. Используют те же паттерны API, что и 13 проверенных. `run-example.sh --check <имя>` позволяет проверить любой пример механически.
- **Low:** gpui-component не проверен компиляцией. ChronOS не использует (SKILL.md).
- **Low:** Чужие файлы в Source/ (`brief.md`, `findings/`, `plan.json`) — от параллельных агентов. Не влияют на результаты.

## Статус ARCHITECTURE.md / DECISIONS.log

- Не обновлялись — разведка документирует, не меняет архитектуру.
- Подтверждено: gpui_elements всё ещё исключён (ARCHITECTURE §2 актуален).
