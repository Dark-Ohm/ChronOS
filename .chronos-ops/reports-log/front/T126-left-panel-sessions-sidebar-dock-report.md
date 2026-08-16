# T122–T126 Dev CLI + Shell Scripts + Left Panel Report

**Date:** 2026-07-25  
**Status:** Code complete; live smoke test pending (release build clean)

---

## T126 — Left Panel: Sessions Sidebar as Bar + Chat Dock Exclusive

### Files modified

| File | What changed |
|---|---|
| `crates/app/src/side_panel_left/sessions_list.rs` | New constants: `SIDEBAR_COLLAPSED_WIDTH = 36`, `SIDEBAR_EXPANDED_WIDTH = 200`, `SIDEBAR_HANDLE_WIDTH = 10`, `SIDEBAR_MIN_WIDTH = 46`. Removed `SIDEBAR_FULL_WIDTH`, `SIDEBAR_ICON_WIDTH`. |
| `crates/app/src/side_panel_left/state.rs` | Added `dock_chat: bool` (default false), `last_exclusive_zone: Option<f32>`, `recalc_min_width()` method. Default `width = 36`, `min_width = 46`. |
| `crates/app/src/side_panel_left/mod.rs` | `window_options`: `exclusive_zone: Some(36)`, `exclusive_edge: Some(Anchor::LEFT)`. `render()`: live `set_exclusive_edge(LEFT)` + `set_exclusive_zone(px(zone))` on change — dock on → `state.width`, dock off → sidebar width. `close()`: clears zone to 0 before `remove_window()`. |
| `crates/app/src/side_panel_left/panel.rs` | Removed `is_rail`, `rail_view`, `PANEL_RAIL_*`. Collapsed sidebar: buttons 28px, padding 4px, dock toggle at bottom. Expanded sidebar: dock toggle (⊞/⊟) + collapse `<` button in flex row in header. `chat_open` derived from width vs sidebar+handle threshold. |

### Constants

```
SIDEBAR_COLLAPSED_WIDTH = 36.0   (was 48)
SIDEBAR_EXPANDED_WIDTH  = 200.0  (unchanged)
SIDEBAR_HANDLE_WIDTH    = 10.0
SIDEBAR_MIN_WIDTH       = 46.0   (36 + 10)
```

### Exclusive zone model

```rust
// mod.rs render()
let new_zone = if self.state.dock_chat {
    self.state.width          // full panel width (sidebar + chat)
} else {
    sessions_list::SIDEBAR_COLLAPSED_WIDTH  // or expanded width based on sessions_collapsed
};
// Only updates when zone value changes (tracked via last_exclusive_zone)
window.set_exclusive_edge(Anchor::LEFT);   // REQUIRED — Hyprland ignores zone without edge
window.set_exclusive_zone(px(new_zone));
```

### Rail removal

`is_rail`, `PANEL_RAIL_WIDTH`, `PANEL_RAIL_TOTAL_WIDTH`, `rail_view` — all deleted. Grep clean (zero matches across `crates/app/src/side_panel_left/`).

### Dock toggle

Two instances — collapsed sidebar (bottom icon) and expanded sidebar (header row next to `<` collapse button). Symbol: `⊞` (docked) / `⊟` (undocked). Active color `#007acc`, inactive `#6c7086`. Toggle writes `panel.state.dock_chat` and calls `cx.notify()`.

### `chat_open` derivation

```rust
// panel.rs
let chat_open = !panel.state.dock_chat
    && panel.state.width > sessions_list::SIDEBAR_MIN_WIDTH + 10.0;
```

Dock on → chat always visible (panel is full-width tile). Dock off → chat only when dragged past sidebar + handle.

### Tests

6 unit tests in `side_panel_left::tests`:

| Test | What it checks |
|---|---|
| `state_starts_as_peek` | Initial `PanelState::Peek` |
| `state_default_width_is_collapsed_sidebar` | `width == 36` |
| `state_min_width_is_sidebar_plus_handle` | `min_width == 46` |
| `toggle_collapse_recalculates_min_width` | `recalc_min_width()` after flip |
| `clamp_width_below_min_after_recalc` | `resize(10.0)` clamps to `min_width` |
| `exclusive_px_dock_vs_overlay` | `dock_chat` toggle exists and flips |

All 6 pass: `cargo test -p chronos --bin chronos -- side_panel_left`  
Full suite: 155/155 pass.

### Build

```
cargo check -p chronos   → Finished dev profile (45 pre-existing warnings, 0 new)
cargo build --release -p chronos  → clean
```

### Live smoke test (pending)

```bash
chronos-rebuild && chronos-stop && chronos-start
# 1) Super+A → sessions sidebar (collapsed ~36px), no status-dot rail
# 2) Expand sidebar → 200px
# 3) Drag past sidebar+handle → chat column appears (overlay, reserved stays ~sidebar)
# 4) Dock ON → reserved ≈ full width; tiled clients reflow
# 5) Dock OFF → reserved back to sidebar
# 6) Collapse/expand → reserved follows
# 7) Close panel → reserved left back
# grim: sidebar-only, chat overlay, docked
```

Report numbers (reserved left from `hyprctl monitors`) to be filled after live test.

### Blood facts honored

- `pkill -x chronos` (not `-f`) — no changes to stop logic
- `set_exclusive_edge(LEFT)` required — implemented in `render()` and `window_options`
- No fork edits (`../Source/**` untouched)
- No `let _ =` on fallible ops without log (close uses `match` on `handle.update()`)
