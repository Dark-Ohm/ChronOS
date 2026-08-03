# T218 — Отчёт: фиксированные ширины вкладок, ресайз только Editor+Settings

**Дата:** 2026-08-03
**Статус:** Принят (код в дереве, приёмочные тесты зелёные)

## Что принималось

Код уже лежал в дереве (архитектор, некоммичен). Правило из задачи:

| вкладка | `PanelTab` | натуральная | ресайз |
|---|---|---|---|
| Editor (блокнот) | `Preview` | 560 | да |
| System settings | `EditorSettings` | 320 | да |
| System | `System` | 400 | нет |
| Files | `Files` | 440 | нет |
| Hyprland binds | `HyprlandBinds` | 320 | нет |
| ACP agents | `AcpSettings` | 320 | нет |

## Проверка (приёмка из задачи)

```bash
cargo test -p chronos --lib side_panel_right
```

Результат: **141 passed; 0 failed** (включает и T222, выполненный рядом).
Три специфичных T218-теста, зовущих настоящий код, прошли:

- `side_panel_right::tab::tests::fixed_width_tab_keeps_its_natural_width` — drag не
  двигает Files, уход/возврат сажают ровно на `preferred_content_width`.
- `side_panel_right::tab::tests::switch_tab_restores_per_tab_resize_memory` —
  память ширины работает там, где ресайз разрешён (Preview).
- `side_panel_right::view::tests::mode_fallback_applies_fixed_system_width` —
  System фиксирована и игнорирует записанную ширину.

Контракт из задачи подтверждён кодом:

- `tabs.rs::PanelTab::resizable()` = `matches!(self, Preview | EditorSettings)`;
- `view.rs::active_tab_width` — фиксированная вкладка игнорирует `tab_resize_memory`
  целиком;
- `view.rs::start_resize` — раскрытие rail→content работает для всех, якоря drag'
  взводятся только для ресайзных; фиксированные пишут `tab is fixed width, drag ignored`;
- ручка на фиксированной вкладке без `cursor_col_resize`/`on_drag`; колонка 4px
  остаётся в раскладке (rail-only = rail + handle, mouse-down открывает контент);
- `view.rs::sim_resize` — тестовый хелпер отказывает, как UI.

## Замечание по окружению

На момент взятия задачи проект не собирался из-за независимого билд-блокера в
`gpui-component` (`layout_match_range` отсутствовал, есть `layout_match_ranges`) —
это была задача **#217**. К моменту приёмки #217 закрыта (local `gpui-component`
уже вызывает `layout_match_ranges`), тесты и релизная сборка проходят.

## Live

По задаче live-прогон сделан 2026-08-03 (`/tmp/t218-run.sh`): ACP agents (фикс.) не
сдвинулась с 40→320, 0 drag-событий; Editor (ресайз) 560→782 монотонно.
Остаток: прогон синтетическим `ydotool` (телепорт курсора, мусорные события) —
модель их не накапливает. Подтверждение живой рукой за пользователем желательно,
но кодом контракт закрыт.

## Вывод

T218 готов к коммиту. Сообщение: `panels : fixed per-tab widths, resize only editor+settings (T218)`.
