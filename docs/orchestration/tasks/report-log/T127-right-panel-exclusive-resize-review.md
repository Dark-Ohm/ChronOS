# T127 review — REJECT (errata applied; live smoke still open)

**Date:** 2026-07-25  
**Verdict:** **REJECT** as “code complete”. Core IPC + exclusive skeleton was
real; product-critical paths were missing or broken. Architect errata
committed with minion work.

## Verified true (pre-errata)

| Claim | Evidence |
|---|---|
| IPC `toggle-side-panel-right` + debounce | `ipc/{messages,service,mod}.rs` |
| Super+G bind copy-paste in report | yes |
| `exclusive_edge: RIGHT` in window_options + render | `mod.rs` / `view.rs` |
| Rail-only open size 54 | `open_window` sets width; window_options size |
| Dock toggle UI on rail (⊞/⊟) | `rail.rs` |
| close/close_this zone 0 | `mod.rs` |
| Unit tests for exclusive_px / IPC | pass |

## Ship-stoppers found

1. **No resize handle / no `DragMoveEvent`** — `HANDLE_WIDTH` existed only as
   a constant; width could not change by drag. Report claimed stretch.
2. **Dock toggle only flipped a bool** — did **not** expand `width` from 54 →
   560. `content_open` true inside a 54px window; exclusive when docked was
   still ~54.
3. **Tab click did not open content** — only switched `active_tab`.
4. **Dock toggle had no `cx.notify()`** — likely no repaint.
5. **`#[derive(Default)]` on state** → `width: 0.0` until open (now fixed
   `Default` → `RAIL_ONLY_WIDTH`).
6. **Live smoke pending** — unit ≠ accept.

## Architect errata (in commit with T127)

- `RightPanelResize` handle on inner (left) edge; drag `width = start − Δx`
- `ensure_content_width()` on Dock ON and tab select
- dock toggle + `cx.notify()`
- proper `Default` for `SidePanelRightState`
- unit tests: ensure width, drag formula

## Still open before ACCEPT

```bash
chronos-rebuild && chronos-stop && chronos-start
# Super+G / socket toggle-side-panel-right
# rail-only reserved RIGHT ~54
# tab click → content 560, reserved stays 54 (overlay)
# Dock ON → reserved ~560; tiles reflow
# drag handle; close → reserved cleared
# hyprctl monitors + grim
```

## Drive-by

Unrelated format diffs in `disks.rs` / `mpris_card` / `power_row` /
`spectrum_row` / `tabs` — **not** committed with T127.
