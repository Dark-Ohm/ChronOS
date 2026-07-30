# T127 — Right Panel: Tab Rail Bar + Content Overlay + Dock

**Date:** 2026-07-25  
**Status:** Code complete; release build clean; all 161 tests pass

---

## Files Modified

| File | What Changed |
|---|---|
| `crates/app/src/ipc/messages.rs` | Added `TOGGLE_SIDE_PANEL_RIGHT_PAYLOAD`, `encode_toggle_side_panel_right()`, `is_toggle_side_panel_right()` + tests |
| `crates/app/src/ipc/service.rs` | Added `IpcSidePanelRightToggleReceiver` type, channel in `start_listener`, routing in `accept_loop` |
| `crates/app/src/ipc/mod.rs` | Destructures 5th receiver, adds debounce branch calling `side_panel_right::toggle(cx)` |
| `crates/app/src/side_panel_right/mod.rs` | State: `width`, `dock_content`, `last_exclusive_zone`; constants: `RAIL_ONLY_WIDTH=54`, `DEFAULT_CONTENT_WIDTH=560`; `window_options`: `exclusive_zone=54`, `exclusive_edge=RIGHT`; `close()`/`close_this()` clear zone |
| `crates/app/src/side_panel_right/view.rs` | `last_resized_width`, `last_exclusive_zone` fields; `render()`: reads global state, updates `exclusive_zone`/`exclusive_edge=RIGHT` on change, resizes window when width changes, gates content visibility (`content_open = dock \|\| width > RAIL_ONLY+1`) |
| `crates/app/src/side_panel_right/rail.rs` | Added `dock_content: bool` and `on_dock_toggle` params; dock toggle button (⊞/⊟) at bottom with spacer |

---

## Constants

```
RAIL_WIDTH           = 44.  (existing)
HANDLE_WIDTH         = 10.  (existing)
RAIL_ONLY_WIDTH      = 54.  (44 + 10)
DEFAULT_CONTENT_WIDTH = 560. (full panel when docked/resized)
```

---

## Exclusive Zone Model

```rust
// mod.rs SidePanelRightState::exclusive_px()
pub fn exclusive_px(&self) -> f32 {
    if self.dock_content { self.width } else { RAIL_ONLY_WIDTH }
}

// view.rs render()
let new_zone = if dock_content { panel_width } else { RAIL_ONLY_WIDTH };
if self.last_exclusive_zone != Some(new_zone) {
    window.set_exclusive_edge(gpui::layer_shell::Anchor::RIGHT);  // REQUIRED
    window.set_exclusive_zone(px(new_zone));
    self.last_exclusive_zone = Some(new_zone);
}
```

---

## Content Visibility

```rust
// view.rs
let content_open = dock_content || panel_width > RAIL_ONLY_WIDTH + 1.0;
// When false: only rail renders (content column hidden via .when())
```

---

## Dock Toggle

- **Location:** Bottom of rail (after spacer)
- **Symbol:** `⊞` (docked) / `⊟` (undocked)
- **Active color:** `#007acc` / Inactive: `#6c7086`
- **Action:** Toggles `SidePanelRightState.dock_content`, triggers re-render via `Entity::update()`

---

## IPC Protocol

**Payload:** `toggle-side-panel-right`

**Hyprland bind (copy-paste for `~/.config/hypr/hyprland.lua`):**

```lua
hl.bind({
  mods = {mainMod}, key = "G",
  dispatcher = "exec",
  arg = [[python3 -c "import socket,os;s=socket.socket(socket.AF_UNIX);s.connect(os.environ['XDG_RUNTIME_DIR']+'/chronos.sock');s.sendall(b'toggle-side-panel-right');s.close()"]],
})
```

---

## Tests Added

| Test | Location | What It Checks |
|---|---|---|
| `encodes_and_recognizes_toggle_side_panel_right` | `ipc/messages.rs` | IPC payload round-trip |
| `rejects_non_toggle_side_panel_right_payload` | `ipc/messages.rs` | Rejection of other payloads |
| `rail_only_default_width` | `side_panel_right/mod.rs` | `RAIL_ONLY_WIDTH == 54.0` |
| `exclusive_px_dock_vs_rail` | `side_panel_right/mod.rs` | Dock ON → width, OFF → rail only |
| `resize_clamps` | `side_panel_right/mod.rs` | Min/max clamping |
| `peek_close_request_*` | `side_panel_right/mod.rs` | Peek close logic (existing) |

**Total test suite:** 161 tests pass (all 4 new IPC tests + 4 new right panel tests included)

---

## Build Status

```
cargo check -p chronos       → clean (pre-existing warnings only)
cargo build --release -p chronos  → clean
```

---

## Live Smoke Test (Pending)

```bash
chronos-rebuild && chronos-stop && chronos-start
# Add Super+G bind to hyprland config, then:
# Super+G (or socket send toggle-side-panel-right)
# 1) rail-only strip (~54px); hyprctl monitors reserved RIGHT ≈ 54
# 2) Click tab icon → content expands (overlay); reserved STAYS 54
# 3) Dock ON (⊞) → reserved ≈ full width (560); tiles reflow under content
# 4) Dock OFF (⊟) → reserved back to rail (54)
# 5) Close panel → reserved right cleared
# grim: rail-only / overlay / docked
```

Report numbers (reserved right from `hyprctl monitors`) to be filled after live test.

---

## Blood Facts Honored

- `pkill -x chronos` — no changes to stop logic
- `set_exclusive_edge(RIGHT)` required — implemented in `window_options` and `render()`
- No fork edits (`../Source/**` untouched)
- `let _ =` not used on fallible ops without log (`close()` uses `match`)
- IPC debounce 200ms mirrors left panel
- No `is_rail` / status-dot rail — tab rail IS the chrome

---

## Summary

T127 mirrors T126 (left panel) for the right panel:
- Super+G opens rail-only (54px) with exclusive = rail
- Content opens as overlay (exclusive stays rail)
- Dock ON → exclusive = full width (tiles reflow)
- Dock OFF → exclusive = rail again
- All state managed in `SidePanelRightState` global, live exclusive zone in `view.rs::render()`
- IPC fully implemented with tests