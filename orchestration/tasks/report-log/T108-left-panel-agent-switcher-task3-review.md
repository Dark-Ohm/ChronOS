# T108 Task3 — Architect review (acceptance)

**Дата:** 2026-07-24  
**Вердикт:** **ACCEPTED** (code + suite). Живой клик по пункту меню после
ребилда в этой сессии **не** прогонялся — running `chronos` был pre-task3.

## Сверка отчёта с деревом

| Утверждение отчёта | Факт |
|---|---|
| Absolute `size_full` оверлеи ломали клик | **Да.** В HEAD до фикса: `dropdown.map` добавлял `.absolute().size_full().on_click` поверх пунктов — дифф `panel.rs` подтверждает удаление. |
| `on_click` на самих пунктах | **Да.** `panel.rs` ~109–111: `cx.listener` → `switch_agent`. |
| Dropdown строится до chat/composer (E0502) | **Да.** Порядок: resize handlers → sidebar → dropdown → chat → composer. |
| `cargo test -p chronos --lib` 26 pass | **Да.** Повторено: 26/26. Это **не** тесты switcher — suite правой панели + `state`; имена в отчёте не фабрикованы, но scope suite не T108-специфичен. |
| `side_panel_left` unit tests | 2 реальных теста через bin: `state_starts_as_peek`, `state_default_width` — зелёные. `cargo test -p chronos --lib side_panel_left` → 0 (фильтр lib не видит bin-модуль). |
| Jank #7 / ghost-trail #8 out of scope | **Согласовано** — не часть task3. |

## Scope T108 в целом (после task3)

**Принято (core switcher):**

1. Registry `known_agents()` — честно только Hermes (ACP stdio verified path).
2. Multi-instance `clients: HashMap` + lazy spawn в `switch_agent`.
3. UI dropdown в хедере (agent-cluster toggle + list + checkmark).
4. Sessions cleared on agent switch (session bound to agent).
5. Item #6 real modes/models from ACP — already accepted (task2).
6. Item #9 resize-at-min-width — closed live (`fbcadd6`).
7. Task3 clickability fix — this review; commit in tree.

**Не закрыто / долг (не блокируют приёмку core T108):**

- #7 dropdown jank (~20fps) — profiling, не гадание.
- #8 / #8-bis ghost-trail на ресайзе — форк (`PlatformWindow::resize` buffer lag); отдельная gpui-задача.
- Живой round-trip: models/modes list **после** реального prompt в composer — PENDING (ydotool layer-shell unreliable).
- Multi-agent live: второй ACP backend в реестре нет — переключение «на другого» живьём нечем проверить, пока кто-то не добавит verified backend.

## Код

- Task3 fix: `crates/app/src/side_panel_left/panel.rs` (закоммичен при приёмке).
- Бриф → `done/T108-left-panel-agent-switcher.md`.
- Отчёт task3 → `report-log/` рядом с task1/task2.
