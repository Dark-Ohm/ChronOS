# T204 report

**Зона:** `crates/app/src/side_panel_left/{mod,panel,sessions_list,state}.rs`,
`crates/app/src/side_panel_right/{mod,rail,view}.rs`. `bar/`, `tab/preview*`,
`tab/files*`, `preview_target` — не тронуты (проверено `git diff --name-only`).
Коммит `96c40d4` — только эти 7 файлов, без чужих незастейдженных изменений.

## Constants table (before → after)

| | before | after |
|---|---|---|
| Right `RAIL_WIDTH` (`mod.rs`) | 44 | **36** |
| Left `SIDEBAR_COLLAPSED_WIDTH` | 36 | **36** (не менялась) |
| Both `HANDLE_WIDTH` / `SIDEBAR_HANDLE_WIDTH` | 10 | **4** |
| Right `RAIL_ONLY_WIDTH` | 54 | **40** (`36+4`) |
| Left `SIDEBAR_MIN_WIDTH` (`36+handle`) | 46 | **40** (`36+4`) |
| Right `BUTTON_SIZE` | 36 | **28** |
| Icon svg size | 20 | **18** |
| Active indicator `left` | −8 | **−4** (flush к краю 36-рейла) |

## Sync strategy (shared const / dual + test)

Выбрал **duo + тест**, а не общий модуль. Правая `RAIL_WIDTH`/`HANDLE_WIDTH` —
источник правды; `rail.rs` пере-экспортирует `pub(crate) use super::RAIL_WIDTH`
(одна правка в `mod.rs` автоматически тянет рейл — дрейф правого крыла
невозможен). Левая пара (`SIDEBAR_COLLAPSED_WIDTH`/`SIDEBAR_HANDLE_WIDTH`) —
отдельные литералы **36./4.**, но живёт кросс-крыло-тест
`rails_and_handles_match_right_panel` в `side_panel_left/mod.rs::tests`:

```rust
assert_eq!(crate::side_panel_right::RAIL_WIDTH, sessions_list::SIDEBAR_COLLAPSED_WIDTH);
assert_eq!(crate::side_panel_right::HANDLE_WIDTH, sessions_list::SIDEBAR_HANDLE_WIDTH);
```

Тест помещён именно в левый `mod.rs`, потому что это единственное место дерева,
где видны оба крыла: `side_panel_left` в bin-tree, `side_panel_right` в lib-tree
(подтверждено на сборке — резолвятся оба). Если кто-то поменяет одну пару —
тест упадёт. Общий модуль `panel_chrome.rs` не заводил — задание само
предупреждает «don't over-engineer»; два литерала + тест-эквивалентность
покрывают ровно тот риск, что был (дрейф вперёд).

## Handle paint (L+R)

Оба handle теперь **4px ghost**:

- **Право (`view.rs`)**: `.bg(gpui::transparent_black())` вместо
  `bg(surfaces::chrome)` — убран solid-филл, убран `border_r_1`/`border_color`,
  убрана постоянная 1px центр-полоса (`theme.text.disabled`). Хейрлайн на краю
  контента даёт `side-panel-body`'s собственный `border_l_1` (`theme.border.default`).
- **Лево (`panel.rs`)**: `.bg(gpui::transparent_black())` вместо
  `bg(theme.bg.tertiary)` + `flex/items_center/justify_center`, убрана
  центр-полоса. Хейрлайн `border_l_1` + `theme.border.subtle` на внутреннем крае.

Drag-семантика не тронута нигде: `cursor_col_resize()`, `on_mouse_down`,
`on_drag`, `on_drag_move` идентичны до/после в обоих файлах (дифф подтверждает).

Layout order сохранён: право `[handle | body]` + rail последним, лево
`[sidebar | handle]` — не переворачивал.

## Rail button/icon sizes

- `rail.rs`: `BUTTON_SIZE` 36→**28**, svg 20→**18** (комментарий: «keep
  readable» — 18 в 28 читается, 16 было бы впритык к иконкам-символам слева).
- Индикатор: `left(-8)`→`left(-4)`. При 28-кнопке в 36-рейле отступ 4px с
  каждой стороны; `left(-4)` от кнопки = x=0 рейла, индикатор (3×20) упирается
  в край — не клипается за рейл (при −8 был бы в хроме вне 36px).
- Слева collapsed rail (`panel.rs`) кнопки и так были 28 — только handle-paint
  менялся + константа ширины осталась 36. Визуально лево/право теперь одной
  толщины.

## Tests + verification

```
$ cargo test -p chronos side_panel_left::
test result: ok. 7 passed  (левый bin-tree; включает rails_and_handles_match_right_panel)

$ cargo test -p chronos side_panel_right::
test result: ok. 130 passed

$ cargo test -p chronos
test result: ok. 208 (lib) + 389 (bin) = 597 passed; 0 failed

$ cargo build --release -p chronos
Finished `release` profile [optimized] target(s) (exit 0)
```

Новых тестов — один: `rails_and_handles_match_right_panel` (синк-тест двух пар
констант). `rail_only_default_width` обновлён 54.0→**40.0**.

**Важно для следующего прогона:** тестовый бинарь левого крыла сначала был
**stale** — `cargo test rails_and_handles` давал 0, при том что файл содержал
тест, а mtime бинаря был новее файла. Понадобился `touch` исходника +
`cargo test --no-run` (реальная пересборка 22s), после чего тест нашёлся.
Симптом: свежие правки в `side_panel_left/` не попадают в тест-бинарь без
принудительной пересборки. Зафиксировано, не лечил — выходит за зону.

## Что НЕ сделано

- **Живой смок** — LIVE NOT VERIFIED (нет интерактивной сессии в этой работе).
  Статика зелёная: константы, тесты, release-сборка. Визуал (оба rail-only
  одной ширины, нет бежевой колонки, drag ресайзит с обеих сторон) требует
  живого прогона release-бинаря.
- **Ghost-handle ширина 6px** — оставил 4, как основной target в задании
  («max 6 if drag feels bad»). Если на живом смоке драг 4px окажется неудобным —
  отдельная эррата, не превентивно.
- **Общий модуль констант** — сознательно не заводил (см. sync strategy).
- **Левый `state.rs` комментарий** — обновил «window 46»→«window 40 = 36 rail
  + 4 handle» (был единственный оставшийся магический номер в комментариях
  зоны; грепом проверил, что других 54/44/10-handle не осталось).

## Acceptance

- [x] Left collapsed rail width == right rail width == 36 (тест-эквивалентность)
- [x] Handle hit ≤ 6px (4px), no solid chrome column on either side
- [x] Resize still works both sides (diff: семантика не тронута) — **живым не подтверждено**
- [x] `RAIL_ONLY_WIDTH` 40 / left min 40; no 54 magic left (греп + тесты)
- [x] T194c / bar zones untouched (`git diff --name-only`)

---

## Приёмка (Lead Architect / Grok, 2026-08-02)

**Вердикт: ACCEPTED WITH RESIDUAL (+ errata commit)**

Коммит T204: `96c40d4`.  
**Errata (архитектор, live):** `1d9b71b` — white strip + right resize drift.

| claim | check |
|---|---|
| rails 36/36, handles 4/4, RAIL_ONLY 40 | ✅ |
| BUTTON 28, icon 18 | ✅ |
| sync test left↔right | ✅ in left mod |
| zone 7 files, no bar/ | ✅ |
| drag semantics «unchanged» | ⚠️ right drag **was broken** in abs coords after thin handle |
| LIVE | user reported: **white strip + resize drift on right**; left OK |

**Root cause (live):**
1. `transparent_black` handle on Transparent layer-shell → hole at desktop edge.
2. Right panel TOP\|RIGHT: window-local mouse X drifts as origin moves left.

**Errata landed:** chrome fill on right handle; `width = right_abs − pointer_abs`.

**Residual:** re-verify live after errata (grim optional); left still transparent
handle (inner edge — lower risk).

