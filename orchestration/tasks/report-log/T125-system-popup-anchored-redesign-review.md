# T125 review — ACCEPTED WITH CAVEATS (2026-07-25)

**Verdict:** ACCEPTED WITH CAVEATS  
**Commits:** `fc71215` (+ slider errata in same commit)

## Evidence
- AnchoredPopup + fallback, width 360, bar canvas + mouse_down: yes
- Mockup blocks brightness/power/gaming: yes
- Icons brightness/minus/plus in assets: yes
- release build green after errata

## Slider bug (user: same as volume dual-knob class)

Not shared DragMove markers (only one brightness track). Real bugs:
1. **Optimistic cleared on every drag** (`dispatched_brightness = None`) → thumb/DDC fight
2. **Frac used full popup width** while track sits between −/+ → wrong mapping / jumpy fill

Fix: `track_bounds` canvas + `brightness_frac_from_bounds`; keep `Some(value)` optimistic; throttle Set 50ms.

## Caveat — "both monitors"
MVP `write_all` still sets **all** DDC displays from one slider (by design, brightness service). Independent per-monitor later.

## Live
User verifies drag feels 1:1 on the track; dual-display policy is intentional.
