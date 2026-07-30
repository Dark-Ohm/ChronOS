# T127 — Right panel: tab rail bar + content overlay + dock (mirror left)

**Статус: OPEN, не назначен.**  
**Канон:** `DECISIONS.log` → `## 2026-07-25 — Right panel: tab rail bar + content overlay + dock`  
**Зеркало продукта:** T126 left (`side_panel_left` post-errata `f89e27d`) —  
sessions sidebar ↔ **tab rail**, chat ↔ **content**, Super+A ↔ **Super+G**.  
**Код:** `crates/app/src/side_panel_right/` + IPC  
**Форк:** read-only.  
**Skills:** `gpui-layer-shell`, T126 code as pattern.

## Цель (продукт)

> Сайдбар со вкладками = правый **бар** (exclusive).  
> Контент панели = **без** exclusive по умолчанию (overlay).  
> Свитч Dock = exclusive на всю ширину.  
> Бинд **Super+G** (как Super+A слева).

| | Rail-only | Content open (overlay) | Dock ON |
|---|---|---|---|
| Width | ~rail+handle (≈54) | rail + content (default ~560) | same |
| Exclusive | **rail width** | **rail width** | **full width** |
| Edge | RIGHT | RIGHT | RIGHT |
| Tiles under content? | n/a | **no** | **yes** |

## Текущее

| | Сейчас | Цель |
|---|---|---|
| Open | full 560, content+rail | Super+G → **rail-only** first |
| exclusive | `None` | rail default; full if dock |
| exclusive_edge | нет | **`RIGHT`** |
| Resize | нет | handle на **левом** (inner) краю |
| Dock UI | нет | toggle (⊞/⊟ как left) |
| IPC Super+G | **нет** (только left) | `toggle-side-panel-right` + hypr bind note |
| Hover strip | on | leave on or off — **не** ломай; keybind is primary |

`RAIL_WIDTH = 44` in `rail.rs`. Content is left of rail in flex
(`view.rs`: content column then rail flush right).

## Exclusive model

```text
fn exclusive_px(dock, width, rail_w) -> f32 {
  if dock { width } else { rail_w }  // rail only, not full content
}
// window.set_exclusive_edge(Anchor::RIGHT);
// window.set_exclusive_zone(px(exclusive_px(...)));
```

Update only when zone changes (`last_exclusive_zone`).  
Close / close_this: zone → 0 before remove.

## Задачи

### Task 1 — IPC Super+G + toggle(cx)

Mirror left IPC exactly:

1. `ipc/messages.rs`:  
   `TOGGLE_SIDE_PANEL_RIGHT_PAYLOAD = "toggle-side-panel-right"`  
   + `encode_*` / `is_*` + unit tests.
2. `ipc/service.rs` + `ipc/mod.rs`: channel + debounce +  
   `side_panel_right::toggle(cx)` (App-only, no Window — like left).
3. `side_panel_right::toggle(cx: &mut App)` if not already App-only  
   (today `toggle(_window, cx)` — add or change to `toggle(cx)` for IPC;
   keep bar-widget overload if any).
4. **Документируй** строку для `~/.config/hypr/hyprland.lua` (не обязан
   править чужой home, но дай copy-paste в отчёте):

```lua
hl.bind({
  mods = {mainMod}, key = "G",
  dispatcher = "exec",
  arg = [[python3 -c "import socket,os;s=socket.socket(socket.AF_UNIX);s.connect(os.environ['XDG_RUNTIME_DIR']+'/chronos.sock');s.sendall(b'toggle-side-panel-right');s.close()"]],
})
```

(подгони под стиль существующих `toggle-side-panel-left` / launcher binds)

### Task 2 — Rail-only open + content expand

- State: `width`, `dock_content: bool` (default false),  
  `last_resized_width`, `last_exclusive_zone`.
- Constants e.g.:

```text
RAIL_WIDTH      = 44.   // existing
HANDLE_WIDTH    = 10.
RAIL_ONLY_WIDTH = RAIL_WIDTH + HANDLE_WIDTH  // min / Super+G open
DEFAULT_CONTENT_WIDTH = 560.
MIN_CONTENT_TOTAL = RAIL_ONLY + ~280  // when content forced open
MAX_WIDTH = 960.
```

- Super+G / `open_pinned`: start **`width = RAIL_ONLY_WIDTH`**, content
  column **hidden** (only rail + handle visible). Exclusive = rail.
- Expand content: drag handle **left** past threshold **or** click a tab
  icon (selecting a tab opens content if rail-only).  
  `content_open = dock_content || width > RAIL_ONLY + epsilon`.
- Collapse content: drag to rail-only (content hides, rail stays).
- **Do not** invent a status-dot rail — tab rail is already the chrome.

Layout when rail-only:

```text
[ handle ][ rail icons ]
```

When content open:

```text
[ handle ][ content tabs body ][ rail ]
```

(Order may match current `content then rail` — keep rail flush **right**
edge of screen; handle on the **left** of the whole window.)

### Task 3 — Dock switch + exclusive live

- Toggle in rail (bottom) and/or content header — visible in rail-only.
- Dock ON: `ensure_content_width()` if still rail-only; exclusive = width.
- Dock OFF: exclusive = rail width; content may stay open as overlay.
- `render`: `set_exclusive_edge(RIGHT)` + `set_exclusive_zone(px(zone))`
  when zone changes; `window.resize` coalesce like left.

Drag formula (right-anchored):  
`new_width = start_width - (current_x - start_x)` — verify live; flip if wrong.  
Own marker type `RightPanelResize` (never share left's `LeftPanelResize`).

### Task 4 — Tests + live smoke

**Unit:** rail-only default width, exclusive_px(dock/rail/content),  
IPC encode/recognize right payload, clamp.

**Live (обязательно):**

```bash
chronos-rebuild && chronos-stop && chronos-start
# Super+G (after bind) or socket send toggle-side-panel-right
# 1) rail-only strip; hyprctl monitors reserved RIGHT ≈ rail (~44–54)
# 2) open content (tab click / drag) → reserved STAYS rail; content overlays
# 3) Dock ON → reserved ≈ full width; tiles reflow under content
# 4) Dock OFF → reserved rail again
# 5) drag resize; close → reserved right cleared
# grim rail-only / overlay / docked
```

Отчёт:  
`docs/orchestration/tasks/report/T127-right-panel-exclusive-resize-report.md`

## Зона файлов

**Писать:**
- `crates/app/src/side_panel_right/{mod,view,rail}.rs` (+ tiny state if needed)
- `crates/app/src/ipc/{messages,service,mod}.rs` — right toggle only

**Не трогать:**
- `side_panel_left/**` (T126)
- `../Source/**`
- tab body widgets except parent flex for hide/show content column
- bar exclusive

**Читать:** T126 left `exclusive_px`, `ensure_chat_width`, dock toggle,
`chat_open = dock || past_sidebar` (errata — do **not** invert).

## Что НЕ делать

- Не «exclusive always = full width» (отклонённый черновик T127)  
- Не exclusive без `exclusive_edge: RIGHT`  
- Не `DragMoveEvent` shared with left  
- Не забывать IPC + copy-paste Super+G bind  
- Не `pkill -f` / AI trailers / fake hyprctl  

## Accept / Reject

**Accept:** Super+G rail-only + exclusive rail; content overlay keeps rail
zone; Dock full zone; resize works; close clears; hyprctl numbers + grim.

**Reject:** open always 560 exclusive full; no rail-only mode; no IPC;
edge missing; unit-only.

## Out of scope

- Persist width/dock  
- New tabs content  
- Kill hover-strip (optional leave as secondary open path — if hover opens,
  same width/exclusive rules)

## Commit style

`side_panel_right : rail exclusive + content dock overlay (T127)`  
`ipc : toggle-side-panel-right for Super+G (T127)`

## Report path

`docs/orchestration/tasks/report/T127-right-panel-exclusive-resize-report.md`
