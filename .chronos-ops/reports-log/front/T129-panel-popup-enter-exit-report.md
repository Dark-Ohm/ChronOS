# T129 report — panel/popup enter–exit

**Status:** implemented (awaiting live eyeball by Architect)  
**Date:** 2026-07-26

## What

Shared enter language: **opacity + translate** (no scale API on `div` in this fork — `gpui-animation` documents translate via relative+inset). Easing: `SpringBack` = EaseOutBack(1.5). Duration 240ms. Reveal flip after 16ms spawn.

## Scale?

**No.** Fork has no composited scale on Div. Used `State::translate` + base `.left`/`.top` closed pose.

## Exit delay?

**No.** Avoid ghost layer-shell risk. Close still instant `remove_window`.

## Files

| Path | Change |
|------|--------|
| `crates/app/src/motion.rs` | NEW — SpringBack, ENTER_MS, slide helpers, tests |
| `main.rs` / `lib.rs` | `mod motion` |
| `side_panel_right/view.rs` | Linear opacity → SpringBack + slide from right |
| `side_panel_left/mod.rs` + `panel.rs` | `revealed` + motion wrapper (outer hover, inner transition) |
| `volume_popup/view.rs` | root enter; local SpringBack removed → `crate::motion` |
| `system_popup/view.rs` | root enter |
| `updates_popup/view.rs` | root enter |
| `history_popup/view.rs` | root enter |

## Not touched

exclusive_zone, dock, theme, toast (T130), launcher.

## Verify

- `cargo check -p chronos` green  
- `cargo test -p chronos --lib motion` 2/2  
- `cargo test -p chronos --lib side_panel` 29/29  

**Live (Architect):** Super+A / Super+G open enter; volume/system/updates/history card rise; exclusive toggle still OK.

## Commit

(pending this report)
