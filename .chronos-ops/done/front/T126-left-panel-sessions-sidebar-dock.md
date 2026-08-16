# T126 — Left panel: sessions sidebar as bar + chat dock exclusive

**Статус: OPEN, не назначен.**  
**Канон:** `docs/DECISIONS.log` → `## 2026-07-25 — Left panel: sessions sidebar is the bar; chat overlay + dock switch`  
**HANDOFF:** шапка 2026-07-25 (вечер+) left-panel модель.  
**Код:** `crates/app/src/side_panel_left/`  
**Форк:** `../Source/gpui` — **только читать** API `Window::set_exclusive_zone` / `set_exclusive_edge`. **Не** менять Source.  
**Skills:** `gpui-layer-shell` (exclusive_edge blood), `zed-ai-for-chronos` (layout map), `chronos-shell` gotcha exclusive_zone.

## Цель (продукт)

1. **Sessions sidebar = бар** левой панели. Свернутый — **чуть уже** (~36px), **не** status-dot rail.
2. Super+A (`toggle-side-panel-left` IPC) открывает **sidebar**; exclusive zone = ширина sidebar.
3. **ACP-чат выезжает** (resize / open thread): окно шире, exclusive **не** растёт — чат overlay, окна не тайлятся под чат.
4. **Свитч Dock** в UI: exclusive = full width → подтайливает сразу. Off → снова sidebar-only exclusive.
5. Выпилить `is_rail` / `PANEL_RAIL_*` / `rail_view` — это не продукт.

Unit green ≠ done. Live Wayland + `hyprctl monitors` `reserved` + grim.

## Текущее (факт в дереве)

| | Сейчас | Цель |
|---|---|---|
| Min width | `PANEL_RAIL_TOTAL_WIDTH` (~36 = 26+10 handle) | `sessions_sidebar_w + HANDLE` (36 или 200 + 10) |
| Collapse-to-min UI | `is_rail` → зелёная точка, **нет** sessions | Sessions sidebar остаётся |
| `sessions_collapsed` | 48 / 200 | **~36** / ~200 |
| Super+A open | `open_pinned` width **352**, full chat chrome | Sidebar-only width (collapsed ~36+handle или last-used sidebar mode) |
| `exclusive_zone` | `None` всегда | Sidebar default; full width if Dock on |
| `exclusive_edge` | нет | **`LEFT`** всегда когда zone > 0 (якорь `LEFT\|TOP` — без edge зона мёртвая) |
| Dock switch | нет | Toggle в header / sidebar chrome |

Кровь (уже в DECISIONS 2026-07-23): без `set_exclusive_edge(LEFT)` Hyprland **молча** игнорирует exclusive на угловом якоре. Live `set_exclusive_zone` работает mid-session (`gpui/src/window.rs`).

## Модель exclusive

```text
fn exclusive_px(state) -> f32 {
  if !dock_chat { sessions_sidebar_width(state) }  // 36 collapsed / 200 expanded
  else          { state.width }                     // full panel incl. chat
}
// window.set_exclusive_edge(Anchor::LEFT);
// window.set_exclusive_zone(px(exclusive_px));
```

Обновлять **только** когда значение сменилось (как `last_resized_width` → `last_exclusive_zone`), не каждый paint впустую.

При **close**: перед `remove_window` по возможности `set_exclusive_zone(px(0.))` (если compositor не чистит сам — проверить live).

## Задачи

### Task 1 — Убить rail, min = sessions sidebar

Файлы: `mod.rs`, `state.rs`, `panel.rs`, `sessions_list.rs`.

- Удалить / перестать использовать: `PANEL_RAIL_WIDTH`, `PANEL_RAIL_TOTAL_WIDTH`, `is_rail`, `rail_view` и ветку `.when(is_rail, …)`.
- `SIDEBAR_COLLAPSED_WIDTH` / `SIDEBAR_ICON_WIDTH`: **~36** (было 48). Иконки/кнопки подогнать (например hit ~28 + pad), **тот же chrome** (expand `>`, `+`, session dots) — не новый виджет.
- `SIDEBAR_EXPANDED_WIDTH` оставить ~200, если mockup не требует иначе.
- `SidePanelLeftState::min_width` = `sidebar_w(collapsed) + HANDLE_WIDTH` (HANDLE сейчас 10 в `panel.rs`). При toggle collapse — **пересчитать min_width** (и если `width < new_min`, clamp up; если chat hidden и width > sidebar-only, см. Task 2).
- При `width` на «sidebar only» всегда рендерить **sessions sidebar** + handle (+ optional thin chrome), **никогда** status-dot strip.

### Task 2 — Super+A = sidebar; chat = выдвижная колонка

Состояния окна (логически):

| Mode | width | content |
|---|---|---|
| `SidebarOnly` | sidebar + handle | sessions sidebar only (collapsed or expanded list) |
| `ChatOpen` | sidebar + thread (≥ default chat budget) | sidebar + thread header/chat/composer |

- `open_pinned` / `toggle` open path: стартовать в **`SidebarOnly`** (не 352 full chat).
- Выдвинуть чат: drag handle вправо **или** клик по session / «open thread» (если есть; иначе достаточно resize past threshold).
- Порог: `width > sidebar_w + handle + epsilon` → show chat column; иначе hide chat column (sidebar остаётся).
- Default chat width when expanding first time: ~352 total **or** `sidebar + ~300` — зафиксируй константу в коде/отчёте.
- Collapse chat: drag to sidebar-only min **or** явная кнопка (nice-to-have). Не уничтожать sessions.

`PanelState` / flags: завести явное `chat_open: bool` **или** выводить из width — что проще и тестируемо; в отчёте одно предложение «как решили».

### Task 3 — Dock switch + live exclusive

- Поле `dock_chat: bool` (default **false**) на view/state.
- UI toggle: в thread header **или** sessions header (видимый в SidebarOnly). Label/icon на твой вкус, но **очевидный** (pin/dock/tile). Active state читаем.
- Persist: **не обязателен** в v1 (session-only ok). Если сделаешь `~/.config/chronos/` — бонус, не блокер.
- На open / resize / collapse toggle / dock toggle / sidebar collapse:

  ```rust
  window.set_exclusive_edge(Anchor::LEFT); // cfg wayland ok as in fork
  window.set_exclusive_zone(px(zone));
  ```

- Dock ON + resize → zone tracks `state.width`.
- Dock OFF → zone = sidebar width only, even if chat wide.

### Task 4 — Unit + live smoke

**Unit (минимум):**
- `min_width` / collapsed 36 / expanded 200 constants.
- `exclusive_px(dock, width, sidebar_w)` pure fn tests (table).
- Нет `is_rail` / `PANEL_RAIL` в tree (grep clean).

**Live (обязательно):**

```bash
chronos-rebuild && chronos-stop && chronos-start
# Super+A (or IPC toggle-side-panel-left)
# 1) opens slim sessions strip; hyprctl monitors → reserved left ≈ 36+handle
# 2) expand chat → windows do NOT reflow under chat (reserved stays ~sidebar)
# 3) Dock ON → reserved ≈ full width; tiled clients reflow
# 4) Dock OFF → reserved back to sidebar
# 5) collapse sessions 36↔200 → reserved follows
# 6) drag to min → still sessions UI, NOT green status-dot rail
# 7) close panel → reserved left back (bar-only ~30 top etc.)
# grim of sidebar-only + chat overlay + docked
```

Отчёт:  
`docs/orchestration/tasks/report/T126-left-panel-sessions-sidebar-dock-report.md`  
— constants, exclusive helper, rsx/div if any, `hyprctl monitors` before/after numbers, grim paths.

## Зона файлов

**Писать:**
- `crates/app/src/side_panel_left/mod.rs`
- `crates/app/src/side_panel_left/state.rs`
- `crates/app/src/side_panel_left/panel.rs`
- `crates/app/src/side_panel_left/sessions_list.rs` (widths only, if needed)
- optional: `composer.rs` / `chat_view.rs` только если hide chat column requires gates
- tests in `mod.rs` / `state.rs`

**Не трогать:**
- `../Source/**` (форк)
- `side_panel_right/**`
- bar / notifications / volume / system popups
- hermes_acp service protocol (ACP connect path leave alone unless compile forces)
- hover_strip re-enable (остаётся off)

**Читать как образец:**
- `bar/mod.rs` exclusive_zone on bar
- DECISIONS 2026-07-23 exclusive_edge blood
- `Window::set_exclusive_zone` / `set_exclusive_edge` in Source (read-only)

## Что НЕ делать

- Не оставлять `is_rail` «на всякий»  
- Не exclusive = full width by default  
- Не exclusive без `exclusive_edge: LEFT`  
- Не два layer-shell окна (rail + chat)  
- Не `let _ = fallible` без лога  
- Не `pkill -f chronos`  
- Не `git add -A` / AI trailers  
- Не фабриковать hyprctl/grim  

## Accept / Reject

**Accept:**
- Super+A → sessions sidebar (collapsed ~36), no status-dot rail  
- Chat expand = overlay (reserved stays sidebar)  
- Dock ON = reserved full width; OFF = sidebar; live `hyprctl` numbers in report  
- `is_rail` / `PANEL_RAIL_*` gone  
- release build + live grim  

**Reject:**
- Rail-dot still at min width  
- Chat always tiles  
- exclusive without edge (reserved stuck 0)  
- unit-only claim  
- fork patches without need  

## Out of scope

- Session list real ACP multi-session wiring (stubs ok if already stubs)  
- Persist dock preference (optional)  
- Hover-peek re-enable  
- Right IDE panel  
- Animation of dock reflow  

## Commit style

`side_panel_left : sessions sidebar bar + dock exclusive (T126)`  
named `git add` only own files; `git diff --staged` before commit.

## Report path

`docs/orchestration/tasks/report/T126-left-panel-sessions-sidebar-dock-report.md`
