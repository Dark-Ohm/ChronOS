# Task 1: Layer-Shell Window + Peek/Pin — Report

## Status: DONE

## Files created/modified

| File | Action |
|---|---|
| `crates/app/src/side_panel_left/state.rs` | Created — `PanelState` enum + `SidePanelLeftState` |
| `crates/app/src/side_panel_left/mod.rs` | Created — `SidePanelLeft` entity, window creation, open/close/toggle, init |
| `crates/app/src/side_panel_left/panel.rs` | Created — stub `render_panel()` returns `div().w_full().h_full()` |
| `crates/app/src/side_panel_left/sessions_list.rs` | Created — stub (later task) |
| `crates/app/src/side_panel_left/chat_view.rs` | Created — stub (later task) |
| `crates/app/src/side_panel_left/composer.rs` | Created — stub (later task) |
| `crates/app/src/side_panel_left/tool_card.rs` | Created — stub (later task) |
| `crates/app/src/main.rs` | Modified — added `mod side_panel_left` + `side_panel_left::init(cx)` |

## Deviations from task brief

1. **Import paths:** Task brief used `gpui_platform::LayerShellOptions` / `gpui_platform::Layer::Overlay` etc. The actual codebase imports these from `gpui::layer_shell::*` (the right panel does this). Adapted to match existing patterns.

2. **LayerShellOptions anchor:** Task brief specified `Anchor::LEFT | Anchor::TOP | Anchor::BOTTOM` (TOP|BOTTOM stretch). Per `gpui-layer-shell` skill, TOP|BOTTOM stretch on Hyprland Overlay causes gap skew. Used `Anchor::LEFT | Anchor::TOP` with fixed height instead — matching the right panel's proven pattern (mirrored to LEFT side).

3. **exclusive_zone:** Task brief specified `Some(px(0.0))`. Used `None` to match the right panel (functionally equivalent — both mean "no exclusive zone").

4. **Global state struct:** Named `SidePanelLeftState_` (trailing underscore) to avoid collision with the `state.rs` `SidePanelLeftState`. The right panel avoids this because its module-level state type is also called `SidePanelRightState` but it's in a different scope. In my case, both the global and the per-view state had the same name.

5. **init() pattern:** Adapted from right panel (spawn with 50ms delay + optional smoke env var).

## Test results

```
cargo build --release -p chronos
```
Build succeeded. 30 warnings total (most pre-existing). New warnings from this module are all expected dead-code warnings for the skeleton (unused functions, fields, variants — will be used in later tasks).

## Commit

```
51ba643 feat(side_panel_left): layer-shell window with peek/pin state
```

## Concerns

1. **TOP|BOTTOM stretch:** The task brief code had `Anchor::LEFT | Anchor::TOP | Anchor::BOTTOM` with `exclusive_zone: Some(px(0.))`. This conflicts with the `gpui-layer-shell` skill which documents that TOP|BOTTOM stretch + exclusive zone skews gaps on Hyprland. I used `Anchor::LEFT | Anchor::TOP` with fixed height instead, matching the right panel pattern. If the product spec intentionally wants full-height stretch, we need to validate on live Hyprland with `hyprctl layers` before committing to that geometry.

2. **No hover_strip yet:** The right panel has a hover_strip for peek mode. Left panel's peek mode won't function until that's implemented (a later task).
