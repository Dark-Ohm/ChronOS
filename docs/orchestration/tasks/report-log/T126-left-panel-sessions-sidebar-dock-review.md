# T126 review — REJECT (errata applied; live smoke still open)

**Date:** 2026-07-25  
**Verdict:** **REJECT** as “code complete / accept”. Scaffold is real; product logic
had a ship-stopper; architect errata applied in tree (uncommitted until
smoke). Live `hyprctl`/grim still **PENDING** per minion report.

## Verified true

| Claim | Evidence |
|---|---|
| `is_rail` / `PANEL_RAIL_*` / `rail_view` gone | `rg` clean under `side_panel_left/` |
| Collapsed 36 / expanded 200 / handle 10 | `sessions_list.rs` |
| `dock_chat` + exclusive_edge LEFT | `mod.rs` `render` + `window_options` |
| close() clears zone before remove | `mod.rs` `close` |
| No Source fork edits | no path under `../Source` in diff |
| Unit tests pass (after errata) | 9 side_panel_left-related tests green |

## Ship-stoppers found in minion code

1. **`chat_open = !dock_chat && width > …`** — inverted vs product and vs the
   report itself (“Dock on → chat always visible”). Dock **hid** the chat.
2. **Default `width = 36` with `min_width = 46`** — open narrower than min;
   handle + sidebar cannot both fit.
3. **`recalc_min_width` ignored expanded 200** — min stayed 46 while sidebar
   column demanded 200.
4. **Dock toggle did not expand width** from sidebar-only → exclusive “full”
   was still ~36–46px.
5. **`close_this` did not clear exclusive zone** (only `close` did).
6. **`exclusive_px` test was a bool flip**, not a zone table.
7. **Live smoke not run** — report admits pending; unit green ≠ accept.

## Architect errata (applied in working tree)

- `chat_open = dock_chat \|\| past_sidebar` (sidebar width-aware threshold)
- default width = `SIDEBAR_MIN_WIDTH` (46)
- open window size = sidebar-only (not 352 flash)
- `recalc_min_width` = sidebar_w + handle
- `exclusive_px()` / `ensure_chat_width()` on dock-on
- `close_this` → `set_exclusive_zone(0)`
- real unit tests for exclusive / ensure width

## Still open before ACCEPT

```bash
chronos-rebuild && chronos-stop && chronos-start
# Super+A → ~46 reserved left (36 sidebar + handle), sessions UI not status-dot
# drag chat → reserved stays ~36/200; windows not under chat
# Dock ON → reserved ≈ width; clients reflow; chat VISIBLE
# Dock OFF → reserved sidebar; chat can stay open as overlay
# hyprctl monitors reserved; grim; close → reserved drops
```

## Other notes

- Report title “T122–T126” is wrong scope (T126 only).
- `tool_card.rs` drive-by format churn — not T126, harmless.
- Sidebar-only still paints agent **header** above sessions; product said
  “sessions sidebar is the bar” — acceptable v1 caveat if live looks OK,
  else follow-up to hide outer header when `!chat_open`.
