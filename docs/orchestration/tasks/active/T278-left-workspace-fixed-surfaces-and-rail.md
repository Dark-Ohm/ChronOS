# T278 — левая workspace: fixed surfaces, rail и геометрия

**Статус:** READY.
**Приоритет:** P1.
**Роль:** GPUI / layer-shell lifecycle.
**Зависимость:** T276 — канонический reference implementation.
**Следующий тикет:** T279; параллельно не выполнять.

## Канон

Выполнить только Tasks 1–2 из
`docs/superpowers/plans/2026-08-13-left-ai-workspace-slice-a.md`.
Approved design:
`docs/superpowers/specs/2026-08-13-left-ai-workspace-redesign-design.md`.

При расхождении памяти агента с файлами побеждают plan и текущее дерево.
Код T276 в `crates/app/src/side_panel_right/` читать как эталон; приватные
типы справа не импортировать, а зеркально портировать в left.

## Цель

Заменить один динамически ресайзящийся left layer-shell window на одну
логическую панель из двух постоянно живущих поверхностей:

- rail: 40 px, `TOP | LEFT`, владелец exclusive zone;
- content: прозрачный fixed canvas 920 px с left margin 40 px и
  `exclusive_zone: -1`.

В visual rail-only обе поверхности остаются открыты: `panel_width = 40`,
видимый content и input region равны нулю. `Super+A` открывает обе поверхности
в этом состоянии; повторный bind закрывает обе.

## Обязательные контракты

1. Hard drag clamp `40..=960`; 360 — только soft floor при открытии.
2. LEFT geometry:
   - `visible_w = clamp(panel_width - 40, 0, 920)`;
   - input region начинается с `x = 0`;
   - во время drag `interactive_w = max(visible_w, 4)`;
   - handle `x = clamp(visible_w - 4, 0, 916)`;
   - delta `start_width + (current_x - start_x)`.
3. Отдельная runtime-only память Chat 560, Plan 480, Context Files 560.
   Fixed widths: Project 440, Sessions 400, Tools 440, Skills 440,
   Archive 440.
4. Открытие content → rail. Content failure — ранний выход. После успешного
   content используется точный T276-контракт:

```rust
pub(crate) fn two_surface_open_outcome(rail_opened: bool) -> TwoSurfaceOpen;
fn rail_window_options(display_id: Option<DisplayId>, cx: &App) -> WindowOptions;
fn content_window_options(display_id: Option<DisplayId>, cx: &App) -> WindowOptions;
```

5. Rail failure откатывает content. Частичного состояния быть не может.
6. `SidePanelLeftState_` — единственный lifecycle/UI SoT. Rail не владеет
   копией active tab, widths, dock или session.
7. Fixed canvas меняет только видимый slice и `set_input_region`; surface
   bounds не меняются.
8. Тонкий rail separator и внешний separator content рисуются только по
   видимым кромкам. Resize hitbox прозрачный и лежит поверх правой кромки
   content.
9. Hover strip остаётся выключенной. Не вызывать
   `hover_strip::init_hover_strip`.
10. `Source/gpui/` не менять. `window.resize()` запрещён во всём
    `side_panel_left`.

Чтобы промежуточный коммит оставался рабочим, существующий
`Entity<SidePanelLeft>` временно становится product-only child внутри
`WorkspaceView`; он теряет window/lifecycle/width/dock ownership. Второго
runtime window path не оставлять.

## TDD и проверки

Сначала написать тесты из Tasks 1–2 плана: полный tab policy, LEFT formulas,
handle-at-zero, input region, две ветки `two_surface_open_outcome`, ранний
content failure, window-option contracts и rollback.

```bash
cargo test -p chronos side_panel_left::tabs --lib
cargo test -p chronos side_panel_left::state --lib
cargo test -p chronos side_panel_left --lib --bins
cargo check -p chronos --lib
rg -n 'window\.resize\(' crates/app/src/side_panel_left
```

Ожидается: тесты/check зелёные; `rg` не находит совпадений. Live UX в T278 не
заявлять доказанным — это gate T281.

## Запрещено

- динамически ресайзить Wayland surface;
- закрывать content surface при collapse до rail-only;
- копировать right-aligned input-region offset;
- создавать второй global state;
- включать hover strip;
- чинить fork или чужие dirty-файлы;
- начинать T279 до принятия отчёта T278 Архитектором.

## Отчёт

Создать inbox-файл
`docs/orchestration/tasks/report/T278-left-workspace-fixed-surfaces-and-rail-report.md`.

Указать изменённые символы, тесты с exit code, ownership двух surfaces,
rollback, доказательство отсутствия `window.resize()`, что не проверялось
живьём, и hash implementation commit. В `report-log/` не переносить.

