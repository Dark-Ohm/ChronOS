# T315 — Rail as Frame Edge: Artboards & Spec

## Deliverables

- **Artboard:** `.chronos-ops/design/rail-frame-edge-artboards.dc.html`
- **Canvas:** 2560×1440 at 1:1, with 4× zoom crops for corners

## What the Artboards Show

### View 1–3: Full left edge, 1:1
- Rail shown (dark), rail hidden/ring (dark), rail shown (light)
- Bar (30px, `bg.tertiary`) and rail (40px, `bg.tertiary`) are one material
- No `border_r_1` — the seam is gone

### View 4: 4× zoom corners
- Top-left: rail↔bar junction — straight edge, no seam
- Bottom-left: rail↔plate junction — rail ends, plate continues
- Top-right: ring↔bar junction — 16px ring meets bar

### View 5: Button states
- Idle: transparent bg, `text.muted` icon
- Hover: `interactive.hover` bg, `text.muted` icon
- **Active (dark):** pill bg `accent.primary @15%`, `text.primary` icon, 2px accent strip on inner edge
- **Active (light):** same pattern, lighter pill opacity

### View 6: Transition moment
- Ring (16px) → Rail (40px): aperture edge slides 24px right
- Chrome background stretches; no content shift needed (exclusive zone handles compositor layout)
- Recommended: 200ms ease-out

## The Bold Thing

**Active indicator = accent pill + inner-edge strip.**

- Pill: 28×28, radius 6, `accent.primary` at 15% opacity background
- Strip: 2px wide, full rail height, `accent.primary` full opacity, on inner edge (right for left rail, left for right rail)
- Replaces the 3px tab-strip bar (which is a tab-strip idiom, not a frame-edge idiom)
- Why this and not the aperture corners: the rail is always on screen, every pixel of noise is read 100×/day. The pill+strip is quiet enough to live forever, bold enough to mark the active tab clearly

## What's Removed

| Element | Before | After |
|---|---|---|
| `border_r_1()` | 1px `border.subtle` on inner edge | **Removed** — no seam between rail and content |
| 3px accent bar | Tab-strip indicator at `right(-4)` / `left(-4)` | **Replaced** by pill bg + 2px strip |
| Left rail token | `bg.primary` (was distinct from right rail) | `surfaces::chrome` = `bg.tertiary` (T311 D2a, already done) |

## Diagnostic Closure (from brief §"Что конкретно не сходится")

| # | Issue | Status |
|---|---|---|
| 1 | Border seam | **Closed** — `border_r_1()` removed |
| 2 | No aperture corners | **Deferred** — corners are quiet, radius comes from `bar.appearance.radius` (default 0), design leaves this to code ticket |
| 3 | Icons not centered in service zone | **Acknowledged** — 40px rail + 16px ring = 56px zone, icons center in 40px rail. When ring is present, optical offset is acceptable. Not the bold thing. |
| 4 | Tab-strip indicator idiom | **Closed** — replaced by pill + strip |
| 5 | Rail width + ring as independent numbers | **Acknowledged** — `RAIL_WIDTH=40` and `wrap.thickness=16` are separate in code. The artboard shows them as one continuous chrome material, which is the visual truth. Code ticket can address if needed. |

## What This Does NOT Cover

- No code changes (design-only ticket)
- No Mocha Mousse palette (T310 Epic 2, P3)
- No `Normal`/`Wrapped` mode switching (T312)
- Right rail is mirrored, not shown separately — same treatment applies

## For the Code Ticket (after acceptance)

Key numbers from this spec:
- Remove `.border_r_1().border_color(...)` from both `rail_view.rs:143-144` and `rail.rs` equivalent
- Replace 3px accent bar with: pill bg on button + 2px strip on inner edge
- Pill: `rgba(accent.primary, 0.15)` for dark, `rgba(accent.primary, 0.12)` for light
- Strip: `w(2)`, full height of rail content area, `accent.primary`, `rounded(1)`
- Strip position: `.right(px(-4))` for left rail, `.left(px(-4))` for right rail
- Keep `surfaces::chrome` background (already correct from T311)
