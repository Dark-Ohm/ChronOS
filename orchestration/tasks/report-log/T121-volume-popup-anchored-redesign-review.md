# T121 review — ACCEPTED WITH CAVEATS (2026-07-25)

**Report:** `report/T121-volume-popup-anchored-redesign-report.md`  
**Verdict:** **ACCEPTED WITH CAVEATS**

## Evidence (Architect)

| Claim | Result |
|---|---|
| Bell/volume anchor: canvas + `.relative()` + `mouse_down` | ✅ `bar/widgets/volume.rs` |
| Scroll ±5% on bar preserved | ✅ `on_scroll_wheel` still there |
| AnchoredPopup BottomRight/BottomLeft + fallback | ✅ `volume_popup/mod.rs` |
| Width 360 | ✅ `POPUP_WIDTH` |
| Header «Sound» via `rsx!` | ✅ |
| Sections Volume / Microphone, mono %/Muted | ✅ |
| Footer Mute output / Mute mic | ✅ |
| Slider click + drag → Set*Volume, unmute-on-change | ✅ `VolumeSliderDrag` + `set_volume_unmute_if_needed` |
| Device menu + SetDefault* | ✅ |
| Mic SVG assets + assets.rs | ✅ committed |
| Zone: only volume_popup / volume widget / assets | ✅ (working tree had **unrelated rustfmt noise** — **not** included in commit) |
| `cargo test -p chronos volume` 12 passed | ✅ re-run accept session |
| `cargo build --release -p chronos` | ✅ Finished release |
| Live grim / wpctl | ❌ **honestly PENDING** in report |

## Commit hygiene

Report: «коммиты не сделаны». Architect squashed-scope commit on accept:

```
54a54c0 volume_popup : anchored Sound redesign (T121)
```

Only the six T121 paths. **Do not** land the parallel dirty tree (battery/tray/updates/… formatting-only noise) as T121.

## Caveats (non-blocking for code accept; block full UX close)

1. **Live smoke not done** — Task 5 open for Architect/user on Hyprland:
   open near icon, drag sink → `wpctl get-volume @DEFAULT_SINK@`, footer mutes, mic, device pick, grim.
2. **Mute icon nested under title-row expand `on_click`** — mockup used `stopPropagation`; no stop here. Live may expand device menu when clicking mute icon. Fix if confirmed.
3. **`frac_from_window_x` assumes popup-local x + full-width track** — OK if events are window-local; verify under AnchoredPopup in live (if coordinates are screen-space, slider will jump wrong).
4. **rsx only on header** — report documents this; acceptable per skill (div for live).
5. **Slider UI cap 100%** (`clamp` on frac); bar still shows boost up to 150% — as planned.
6. **Track bg** = `text_muted` helper, not literal `#313244` — visual debt.

## Accept checklist

- [x] Anchored + fallback present  
- [x] Sound / 360 / dual sections / footer  
- [x] Sliders not only ±5%  
- [x] Unit green + release green  
- [ ] Live evidence (deferred — caveat)  

---

## Follow-up accept (2026-07-25, later same day)

**Report v2:** `report-log/T121-volume-popup-followup-blur-anim-report.md`  
**Verdict:** **ACCEPTED WITH CAVEATS** (scope expansion on top of `54a54c0`)

### Added scope verified

| Claim | Result |
|---|---|
| `window.paint_blur` frosted layer + `bg.alpha(0.82)` | ✅ `view.rs` |
| `with_transition` / `transition_on_hover` / `transition_when` + SpringBack | ✅ already in tree; boot required |
| `gpui_animation::init` from `Bar::render` | ✅ `bar/mod.rs` |
| Public `gpui_animation::init` (Delta 4) | ✅ Source `lib.rs` + PATCHES |
| `[patch]` all 16 crates → `../Source/*` | ✅ ChronOS `Cargo.toml` |
| `BASE_HEIGHT` 240→290 footer clip | ✅ `mod.rs` |
| Skill `chronos-gpui-popup` + symlink | ✅ Source + ChronOS link |
| `cargo test -p chronos volume` 12/12 | ✅ |
| Live grim / acrylic / spring | ❌ still PENDING (user eyes) |

### Commits (follow-up)

```
ChronOS  1ad55c2  volume_popup : blur, anim boot, fork path-patch, BASE_HEIGHT (T121+)
Source   2ad11e2  gpui-animation : public init (Delta 4) + chronos-gpui-popup skill
```

Earlier base still: `54a54c0` (+ T122 volume row_id in `23c5cda`).

### Caveats

1. Live UX (blur, springs, anchor) not architect-grimmed — shell restarted post-commit (`chronos-start`).
2. Path-patch of entire fork is a **dev machine** contract: clone without sibling `../Source` will fail until patch removed or Source present.
3. Report named agent “Hermes” — ok in report body; briefs stay T-ID only.
4. Animation without `init` was dead before Delta 4 — good catch; other popups still lack blur/anim (out of scope).
