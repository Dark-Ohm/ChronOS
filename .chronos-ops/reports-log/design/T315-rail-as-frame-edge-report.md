# T315 Round 2 — Rail as Frame Edge: Artboards & Spec

## What Changed from Round 1

| Issue (owner) | Round 1 | Round 2 |
|---|---|---|
| Aperture corners straight | "deferred to code ticket" | **r=10px**, drawn in all views + 4× zooms |
| Bottom 6px too thin | not addressed | **12px**, derived from bar/rail |
| Panel not emerging from rail | not drawn | **New View 7**: drawer metaphor, 3 states |
| Strip + pill (both) | "pill replaces strip" | **Pill only**, strip removed |

## Design Decisions

### Aperture corner radius: 10px

Rationale against rail (40px) and bar (30px):
- 25% of rail width — visible, not dominant
- 33% of bar height — the curve is noticeable at the top junction
- 83% of bottom thickness — the bottom corner is mostly curve, which softens the thin bottom edge

The `WrapConfig.inner_radius` field already exists (default 16, range `MIN_RADIUS..=MAX_RADIUS`). The code ticket sets it to 10.

### Bottom plate: 12px (was 6)

Derivation:
- Bar = 30px (primary horizontal element)
- Rail = 40px (primary vertical element)
- Bottom = 12px = 40% of bar, 30% of rail

Previous 6px was a leftover from Hide mode's `bottom_strip.height = 4.0`. The wrap bottom is a different element — it's the lower edge of the chrome card, not a standalone strip. 12px is thin enough to stay subordinate to bar and rail, thick enough to not read as a rendering artifact.

The `WrapConfig.bottom_thickness` field already exists (default 6). The code ticket sets it to 12.

### Active indicator: pill only

Round 1 had pill + 2px strip. Owner: "pick one." The pill is chosen because:
- With rounded aperture corners (r=10), the inner edge of the rail is now a curve at the top and bottom
- A vertical strip on a curved edge is visually contradictory
- The pill alone is sufficient — it's visible, distinct, and doesn't add a vertical line at the frame edge

### Panel emergence (new View 7)

The mockup principle: "panels grow from the aperture edge." Applied to side panels:
- **Near side (x=40): STRAIGHT** — the panel reads as emerging from the rail, not floating next to it
- **Far side: r=8** — the panel reads as a card/drawer
- **Animation origin: x=40** — slides right, not fades in
- **Shadow: 2px 0 8px** during mid-slide, fades when fully open

This matches the mockup's dashboard (from top, r only bottom) and launcher (from bottom, r only top).

## Views in the Artboard

1. **Left edge, rail shown, dark** — with r=10 corners, 12px bottom, pill indicator
2. **Left edge, rail hidden (ring), dark** — 16px ring with r=10 corners
3. **Left edge, rail shown, light** — same as View 1, light theme
4. **4× zoom: all four corners** — top-left (rail↔bar), bottom-left (rail↔plate), top-left ring, bottom-left ring
5. **Button states + light corners** — idle/hover/active dark, active light, 4× light corners
6. **Transition: ring→rail** — with corner radius traveling with the edge
7. **Panel emergence** — collapsed → emerging → fully open (drawer metaphor)

## For the Code Ticket

Numbers:
- `wrap.inner_radius`: 10 (currently default 16 in WrapConfig)
- `wrap.bottom_thickness`: 12 (currently default 6)
- Active indicator: pill bg `rgba(accent.primary, 0.15)` dark / `0.12` light, no strip
- Panel: near side straight, far side `rounded_tr(8)` / `rounded_br(8)`
- Panel animation: slide from x=40, 200ms ease-out

Files to touch:
- `crates/app/src/frame.rs` — WrapConfig defaults (or frame.toml values)
- `crates/app/src/side_panel_left/rail_view.rs` — pill indicator, remove strip
- `crates/app/src/side_panel_right/rail.rs` — same
- `crates/app/src/side_panel_left/mod.rs` — panel emergence animation
- `crates/app/src/side_panel_right/mod.rs` — same

---

# ПРИЁМКА — ПРИНЯТ (владелец, 2026-08-19, раунд 2)

Три претензии раунда 1 закрыты. Сверено по содержимому файла, не по
таблице отчёта:

- **углы** — `border-top-left-radius:10px` 11 вхождений и
  `border-bottom-left-radius:10px` ещё 11; нарисованы и в состоянии
  «рельс», и в состоянии «кольцо», в обеих темах, плюс зумы.
  Обоснование числа выведено (25% рельса, 33% бара, 83% нижней кромки),
  а не назначено;
- **нижняя кромка 12px** — `height:12px` 6 вхождений, выведена от бара
  и рельса (40% и 30%). Верно назван и корень прежней шестёрки:
  наследство `bottom_strip.height` из Hide-режима;
- **выезд панели** — вид 7, три состояния, ближняя сторона прямая на
  x=40, дальняя r=8, старт анимации от кромки рельса, тень во время
  движения. Принцип мокапа применён к боковой панели;
- **индикатор** — только пилюля, `width:2px` ноль вхождений.

## Что не покрыто и передано в кодовый тикет

Третий раунд ради этого не гоняем — вписано требованиями в **T318**:

1. **Правый край не нарисован.** Ни правых радиусов, ни слова
   «right»/«mirror» в файле. Зеркальность не бесплатна: пилюля садится
   на противоположный внутренний край, ящик выезжает влево. В T318 —
   отдельный критерий приёмки с промером справа, не «по аналогии».
2. **Два радиуса в одной оболочке** — апертура 10, дальняя сторона
   панели 8. Может быть осмысленно, может быть дрейф; в T318 требуется
   явный ответ, молча оставлять нельзя.
3. **`RAIL_WIDTH = 40` и `wrap.thickness = 16`** так и остались
   независимыми числами — пункт 5 исходного диагноза. Низ от бара и
   рельса вывели, толщину кольца нет. В T318 требуется ответ: связаны
   или нет и почему это нормально.

## Расхождение спеки с деревом, решённое архитектором

Спека предлагает «200ms ease-out». В дереве есть общий язык движения
`crates/app/src/motion.rs` (T129): `ENTER_MS = 260`, `SLIDE_PX = 14`,
`ease_enter = EaseOutBack(1.5)`, готовые `apply_enter_from_left` /
`apply_enter_from_right`.

Решение в T318: берутся существующие хелперы, меняется только точка
старта (кромка рельса вместо 14px). Длительность и кривая локально не
трогаются — два тайминга одного класса движения это ровно тот дрейф,
против которого весь этот тикет. Понадобится другая длительность —
правится `motion.rs` для всего языка сразу.

## Дальше

- Кодовый тикет — **T318** (`active/front/`), P1.
- **T316 разблокирован**: число, которого он ждал, есть —
  `wrap.inner_radius = 10`.
