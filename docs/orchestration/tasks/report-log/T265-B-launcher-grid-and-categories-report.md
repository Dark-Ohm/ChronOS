# T265-B — Launcher app grid and category bar — Report

**Date:** 2026-08-16
**Role:** FRONTEND. Zone: `crates/app/src/launcher/**`.
**Commit:** `bcd08cc` (3 files: `grid.rs` new, `view.rs` rewrite, `mod.rs` +1 line).

## Status

**Done (code + unit tests green).** Live release verification deferred — see
"Что НЕ сделано".

## Что сделано

### 1. `crates/app/src/launcher/grid.rs` (NEW — pure helpers, no GPUI)

- `GRID_COLUMNS = 7`, `CELL_WIDTH/HEIGHT = 88.`, `GRID_GAP = 8.`, `PAGE_ROWS = 4`
  (7×88 + 6×8 = 664px ≤ 704px content width of the 720px card).
- `Move2D` enum + `move_2d(selected, columns, len, mv, page_rows)` — flat
  row-major index clamping for Up/Down/Left/Right/Home/End/PageUp/PageDown.
  Left/Right do NOT wrap across rows; top/bottom/left/right edges clamp.
- `build_categories(&[AppEntry]) -> Vec<(String, usize)>` — distinct XDG
  categories with counts, sorted count desc then name asc; empty (`""`)
  categories dropped, so a category with no entries never reaches the bar.
- `filter_by_category(entries, Option<&str>)` — `None` = "All".
- 6 unit tests (move_2d edges, home/end/page, empty grid, category
  counts/sort/drop-empty, filter All vs specific).

### 2. `crates/app/src/launcher/view.rs` (rewritten)

- **Grid replaces the T265-0 list** (`render_results` → `render_grid` +
  `render_cell`): icon 36px + label, cell 88×88, flex rows of
  `GRID_COLUMNS` cells inside `overflow_y_scroll` + the existing
  `ScrollHandle`. Selection highlight + hover stay on the cell; right-click a
  cell opens the existing `pin_menu::open` (T275 signature with
  `catcher_anchor_for`); click/Enter record frecency then `launch`.
- **Category bar** (`render_category_bar`) between search and grid:
  "All" always first, then `build_categories` chips with per-category counts.
  **Hover-open** (`on_hover` sets `hover_category`, leave clears it) + **click-lock**
  (`on_click` sets `selected_category`); effective category = hover over
  selected. Horizontally scrollable chip strip + a compact chevron.
- **Compact mode**: chevron `▾/▸` toggles `self.compact`; when compact the grid
  is not rendered (search + category bar stay). Default = expanded.
- **2D keyboard + Tab sections**: `FocusSection { Search, Categories, Grid }`,
  cycled by Tab. Real focus routing via `sync_focus`: Search → `InputState`
  focus; Categories/Grid → the view's own `FocusHandle` (`cx.focus_handle()` +
  `.track_focus`), so the single-line `Input` no longer eats arrows/Home/End
  when the keyboard is on the grid. Arrows do 2D `move_2d`; Home/End/PgUp/PgDn
  jump; Enter launches (Categories: Enter locks the chip + focuses Grid); Esc
  closes. Category bar keyboard cursor = `category_index` (left/right navigate,
  Enter locks). Collapsed grid is skipped in the Tab cycle.
- Ghost completion (T265-A) now hints `visible[0]` (first item of the active
  category), not `results[0]`.
- `resolve_app_icon` parametrized by size (18 for list, 36 for grid cells).

### 3. `crates/app/src/launcher/mod.rs`

- `pub mod grid;` added. Window untouched (`WindowKind::Normal`, 720×560 — the
  grid lives inside the mockup card, no layer-shell switch).

## Расхождения со спекой

1. **Kit `Button`/`Select` не использованы.** Чипы категорий и шеврон — сырые
   `div` (`on_click`/`on_hover`), как существующие ряды списка и APPS-чип в
   шапке. Причина: hover-open-чип — кастомный аффорданс (AppGrid-поведение),
   kit-кнопка не мапится чисто; `Select` в этой волне вообще не нужен. Это
   решение, не забывчивость.
2. **VirtualList кита недоступен.** `virtual_list` в
   `../Source/gpui-component/crates/ui/src/lib.rs` объявлен `mod virtual_list;`
   (приватно, не `pub mod`). Поэтому сетка — обычные `div` + `ScrollHandle`,
   как список T265-0. **Не виртуализована** (самописный virtualizer не писал —
   его просто нет). При N≤200 ячеек это дешёво, как и текущий список.
3. **Сетка — flex-ряды, не CSS `.grid()`.** Форк умеет `.grid()/.grid_cols()`,
   но grid+scroll (overflow в taffy) не проверен живьём; flex-ряды —
   предсказуемы, а `ScrollHandle::scroll_to_item` скроллит к **строке**
   (`selected / GRID_COLUMNS`), не к ячейке. Итог тот же визуально.
4. **`launcher.toml` ключи columns/rows НЕ заведены.** Спека разрешает
   «константы с дефолтом под текущее окно» и «ключи можно завести»; взял
   константы (UI крутилок — T265-G). Опциональный пункт, не сделано сознательно.

## Проверено фактом, не на словах

| Команда | Результат |
|---|---|
| `cargo test -p chronos --lib launcher` | **24 passed; 0 failed** (6 новых в `grid::tests`, остальные T275/T265-A регрессия) |
| `cargo build --release -p chronos` | clean (только pre-existing warnings в чужих файлах; в `launcher/{grid,view}.rs` warnings нет) |

- Регрессия T265-A не тронута: `search.rs` без изменений, tier-ранжирование и
  frecency-тесты зелёные (`exact_name_beats_fuzzy`,
  `frecency_does_not_override_exact_name` и т.д.).
- Pin (T275) сохранён: `pin_menu::open` вызывается ровно как в списке, с
  window-local `anchor_rect` + `event_position`.
- `Source/gpui/` и `Cargo.lock` не тронуты.

## Что НЕ сделано (за Архитектором)

1. **Live + release грим (T265-B §Верификация).** Юниты доказывают навигацию/
   категории/фильтр на уровне чистых функций, но для окон/UX «компилируется и
   тесты зелёные» ничего не значит. Не гонял (нужен живой шелл + `pkill -x
   chronos` + grim). Требуемое живое:
   - сетка рендерится (кадр рядом с `Chronos-OSD-Launcher.dc.html`);
   - клик/ховер категории режет выдачу; пустая категория отсутствует в баре;
   - стрелки ходят по клеткам и доводят скролл (в т.ч. Home/End/PgUp/PgDn);
   - Tab гоняет поиск→категории→сетку и фокус реально уходит с `Input`;
   - компакт ↔ полный шевроном;
   - Enter и клик запускают; pin с клетки жив.
2. **Самоприём / перенос в `done/` — НЕ делать.** Принимает Архитектор.

## Статус docs/ARCHITECTURE.md / docs/DECISIONS.log

Не обновлял: это код-волна внутри `launcher/**`, нового архитектурного решения
не добавляет (сетка и категории — целевой функционал из T265-родителя). Если
нужно зафиксировать «сетка = не-виртуализованные div, VirtualList кита
приватный» — правка за Архитектором.

## Коммит

```
feat(launcher): app grid and category bar (T265-B)
```

(3 files: +604 / −189; `Cargo.lock` не тронут, `Source/gpui/` нетронут.)

## Приёмка архитектора (2026-08-16)

Отчёт сверен с деревом построчно, не принят на слово. Подтверждаю каждый
пункт: `git show bcd08ccf --stat` — ровно 3 файла заявленной зоны;
`VirtualList` действительно `mod virtual_list;` (приватный) в
`../Source/gpui-component/crates/ui/src/lib.rs`; pin с клетки зовёт
`pin_menu::open` с той же сигнатурой T275, окно/размер не тронуты.

Мой прогон (независимо от чужих цифр в отчёте):

```
cargo test -p chronos --lib launcher   → 24/24
cargo test -p chronos --bins           → 692/692 (было 686 до этого коммита)
cargo build --release -p chronos       → чисто, 3m32s
```

**Код принят.** Единственный открытый пункт — тот же, что честно
указан выше: живой прогон против мокапа (сетка/категории/клавиатура/
компакт/pin) за владельцем, не блокер для кода.

Процессная заметка: коммит `bcd08ccf` прилетел в master до формальной
приёмки T265-A (владелец подтвердил — "по глупости уже пустил"), но A
к тому моменту уже была проверена мной и принята чисто, так что риск
не реализовался. В этот раз, в отличие от T265-A, отчёт лёг в `report/`
как положено — дисциплина параллельной сессии выросла.
