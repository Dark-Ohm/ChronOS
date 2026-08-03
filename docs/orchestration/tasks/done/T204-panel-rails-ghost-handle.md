# T204 — Ghost resize handles + unified thinner rails (L+R)

**Статус:** active. **Роль:** FRONTEND. **Модель: Sonnet 5 / GLM 5.2**.
**Канон:** `docs/PRODUCT.md`. **Правила:** `docs/orchestration/agents/RULES.md`.

**Зачем:** пользователь — полоска-распределитель 10px с заливкой chrome
бесит («для слепых, которые не видят рейл»). Плюс рейлы разной ширины
(left **36**, right **44**) и оба чуть жирнее, чем нужно.

**Параллельно:** T194c (preview/edit) — **не** трогай `tab/preview*`,
`tab/files*`, `preview_target`. T199/T200 — **не** трогай `bar/`.

**Зона:**
- `crates/app/src/side_panel_right/mod.rs` — `RAIL_WIDTH`, `HANDLE_WIDTH`,
  `RAIL_ONLY_WIDTH`, tests (`assert_eq!(RAIL_ONLY_WIDTH, …)`)
- `crates/app/src/side_panel_right/rail.rs` — **дубль** `RAIL_WIDTH = 44`,
  `BUTTON_SIZE = 36` (сейчас кнопка 36 в рейле 44 — при 36px rail кнопки
  должны сесть, как слева ~28)
- `crates/app/src/side_panel_right/view.rs` — render resize handle
- `crates/app/src/side_panel_left/sessions_list.rs` —
  `SIDEBAR_COLLAPSED_WIDTH`, `SIDEBAR_HANDLE_WIDTH`, `SIDEBAR_MIN_WIDTH`
- `crates/app/src/side_panel_left/panel.rs` — collapsed rail buttons +
  resize handle paint
- consumers that **import** the consts (geometry math only — keep using
  consts, update hard-coded `54.0` / magic numbers if any):
  - `side_panel_right/tab/terminal.rs` (uses `RAIL_WIDTH`+`HANDLE_WIDTH`)
  - left `state.rs` / tests that assert absolute px
  - `hover_strip` if width tied to rail-only

**НЕ:** bar appearance; tab content logic; change resize **behavior**
(drag still works); remove resize entirely.

**Отчёт:** `docs/orchestration/tasks/report/T204-panel-rails-ghost-handle-report.md`.

---

## 1. Целевые константы (жёстко)

| | сейчас | target |
|---|---|---|
| Right `RAIL_WIDTH` | 44 | **36** |
| Left `SIDEBAR_COLLAPSED_WIDTH` (rail-only strip) | 36 | **36** (same as right; already) |
| Both handle hit width | 10 | **4** |
| Right `RAIL_ONLY_WIDTH` | 54 | **40** (`36+4`) |
| Left min rail+handle | 46 | **40** (`36+4`) |
| Right rail `BUTTON_SIZE` | 36 | **28** (match left collapsed buttons in `panel.rs` ~28) |
| Icon svg size | 20 | **16–18** if 28 btn needs it — keep readable |

**Одинаковая ширина рейлов:** collapsed left icon strip **==** right
`RAIL_WIDTH` **== 36**.

Optional cleanup (preferred): one shared constant module or re-export so
they cannot drift — e.g. right `RAIL_WIDTH` is source, left
`SIDEBAR_COLLAPSED_WIDTH = crate::side_panel_right::RAIL_WIDTH` **or**
tiny `crates/app/src/panel_chrome.rs` with `pub const RAIL_WIDTH` /
`HANDLE_WIDTH`. Don't over-engineer: two equal literals **36.** / **4.**
with a comment `// keep in sync with …` is OK if tests assert equality:

```rust
assert_eq!(RAIL_WIDTH, SIDEBAR_COLLAPSED_WIDTH);
assert_eq!(HANDLE_WIDTH, SIDEBAR_HANDLE_WIDTH);
```

(Put that test in either panel or a small chrome test.)

---

## 2. Ghost handle (оба панели)

**Сейчас (оба):** `w=10`, solid `bg(chrome|tertiary)`, border, **+ 1px
center line** `theme.text.disabled` — выглядит как третья колонка.

**Нужно:**
- `w = HANDLE_WIDTH` (**4**)
- **no solid panel fill** — `bg(transparent)` / no bg
- **no** permanent fat center stripe
- optional: `border` none; on hover only — 1px hairline
  (`theme.border.subtle` or `text.disabled` at low opacity) **or** always
  a **1px** line at the **content edge** (not a 4px painted column)
- keep `cursor_col_resize()`, `on_mouse_down`, `on_drag` / `on_drag_move`
  unchanged in semantics
- hit target 4px is tight but OK on desktop; if drag feels bad, **max 6**
  — still ghost, not 10 chrome

Layout order stays:
- Right: `[handle | content? | rail]`
- Left: existing `[sidebar | handle | thread?]` (don't reverse)

Exclusive zone / `window.resize` / `RAIL_ONLY_WIDTH` must use new sum so
rail-only window is **40** not 54.

---

## 3. Rail density

Right `rail.rs`:
- `RAIL_WIDTH` must match mod.rs (today **duplicated** `44.` in two files —
  fix to **one** definition + use, or both 36 with sync test).
- `BUTTON_SIZE` 36 → **28**; icon ~16–18; active indicator bar position
  (`left(px(-8.))` etc.) re-check so it doesn't clip outside 36 rail.
- `py`/`gap` can tighten slightly if cramped.

Left collapsed rail (`panel.rs` ~428+): already ~36 / buttons 28 — only
handle paint change + ensure width const stays 36. If any padding makes
visual wider than right, match.

---

## 4. Tests / hard-coded numbers

Update:
- `side_panel_right/mod.rs` `assert_eq!(RAIL_ONLY_WIDTH, 54.0)` → **40.0**
- any test assuming width 54 / handle 10 / rail 44
- terminal grid still uses consts — should follow automatically

```
cargo test -p chronos side_panel_right::
cargo test -p chronos side_panel_left::
cargo test -p chronos
cargo build --release -p chronos
```

Live: rail-only left+right same visual width; no beige strip; drag still
resizes; grim optional. NOT VERIFIED ok if honest.

Коммит: `panels : ghost resize handle + unified 36px rails (T204)`.

---

## Отчёт

```markdown
# T204 report
## Constants table (before → after)
## Handle paint (L+R)
## Rail button/icon sizes
## Sync strategy (shared const / dual + test)
## Tests + verification
## Что НЕ сделано
```

## Acceptance

- [ ] Left collapsed rail width == right rail width == 36
- [ ] Handle hit ≤ 6px, no solid chrome column on either side
- [ ] Resize still works both sides
- [ ] `RAIL_ONLY_WIDTH` / left min updated; no 54 magic left
- [ ] T194c / bar zones untouched
