# hermes-report-22 — SIDEBAR V2 (мокап → rsx)

**Исход:** DELIVERED (не коммит — по брифу).  
**INTENT:** code was v1 body (300px, 14 bars, blue-cyan, no header/permission/disks/footer chrome); task expects pixel-close mockup layout on `gpui-rsx` where 1:1; mockup `design/System Sidebar.dc.html` is authority for hex/sizes; height under bar unchanged.  
**AUTH:** user said «го /fable …» — execute №22 in-session (fable-loop). No git commit (brief: не коммить).

## Done criteria (observed)

| Check | Result |
|---|---|
| `cargo test -p chronos --bin chronos side_panel` | **12 passed** (ring 24, arm/confirm, mpris title, peek pin) |
| `cargo build --release -p chronos` | **Finished release** (warnings only, pre-existing outside zone) |
| `todo!` / `let _ =` in zone | none (hygiene) |
| Release smoke `CHRONOS_SMOKE_SIDE_PANEL=1` | log `side_panel_right: opened (pinned)` |
| grim DP-1 right strip | `orchestration/reports/smoke-22/panel-dp1-v2.png` (+ earlier v1) |

## Touch list (every file)

### `crates/app/src/side_panel_right/`

| File | Change |
|---|---|
| `mod.rs` | `PANEL_WIDTH` **300 → 352**; mods `header`/`permission`/`disks`; comment height = display−BAR only |
| `view.rs` | 4-region layout: header → permission → **scroll middle** (`id`+`overflow_y_scroll`+`ScrollHandle`) → footer; mockup hex shell `#181825` / border `#313244`; meters 24 + mockup palette; net labels ↓/↑; **no** second `on_hover` on children; `transition_when` on inner body kept |
| `spectrum_row.rs` | `HISTORY_LEN` **14 → 24**; heights CPU38/RAM34/GPU26/net26; colors `#89dceb`/`#89b4fa`/`#f9e2af`/`#6c7086`; label-above layout; always paint 24 columns (pad zeros) |
| `mpris_card.rs` | Media card: 16:9 art ~198h, static progress 38%, transport tray; live title/transport/mute; SVG icons mute/prev/play/next |
| `power_row.rs` | Footer: live clock (RU months) + live net rates; 4-col power tiles **icon+label**; Power red `#f38ba8`; arm/confirm 3s + `cx.listener` + timeout `match`/`warn!` preserved; Switch disabled |
| `header.rs` | **NEW** — rsx header «kitty» + close (`icons/x.svg`) → `close_this` |
| `permission.rs` | **NEW** — rsx static Claude Code Allow/Deny (no wiring) |
| `disks.rs` | **NEW** — static Disk + USB (usage bars green `#a6e3a1`); USB actions RU stubs; **Battery removed** |
| `hover_strip.rs` | **not modified** (geometry already bottom-aligned baseline) |

### Outside pure zone (icons — required by brief)

| File | Change |
|---|---|
| `crates/app/src/assets.rs` | register new icons |
| `crates/app/assets/icons/{x,skip-back,skip-forward,power,sign-out,users,arrows-clockwise}.svg` | **NEW** Phosphor-style from mockup paths |

`Cargo.toml` **not** touched.

## Live vs static (as shipped)

| Section | Status |
|---|---|
| CPU/RAM/GPU meters | **LIVE** `system_resources` |
| Net ↓/↑ | **LIVE** `net_stats` (render-time sample, history time-gated) |
| Footer clock | **LIVE** `chrono::Local` each paint |
| Footer net summary | **LIVE** rates (not mockup's `5%↓ 410 ▲ 562` shape) |
| MPRIS title + transport + mute | **LIVE** |
| Media art / progress / `-14:22` | **STATIC** |
| Header title | **STATIC** «kitty» |
| Permission card | **STATIC** |
| Disks | **STATIC** |
| Power actions | **LIVE** arm/confirm → `AppState::power` |

## Вердикт rsx (флагман-тест)

| Surface | Path |
|---|---|
| Header | **`rsx!` 1:1-ish** (flex, pad, colors, hover, onClick) |
| Permission | **`rsx!` 1:1-ish** (flat bg `#1e1e30` — mockup gradient not expressed; Allow/Deny outline) |
| Disks wrapper | **`rsx!`** outer; card body **div** (progress fill `relative()`) |
| View shell / scroll / meters / media / power | **div builder** — dynamic bars, `cx.listener`, `ScrollHandle`, `transition_when` cleaner as builder |

**Итог rsx:** годен для статического chrome; для spectrum (N dynamic heights) + power listeners + animation shell — div остаётся pragmatic. Это данные вердикта, не провал compile.

## Посекционный grim vs мокап

Скрин: `orchestration/reports/smoke-22/panel-dp1-v2.png` (DP-1, 380×1440 crop).

| Region | Match |
|---|---|
| Width 352 under bar | **yes** (crop shows panel clear of bar chrome) |
| Header kitty + close | **yes** (SVG X) |
| Permission Allow/Deny | **yes** (accent outline Allow) |
| Media 16:9 + progress + tray | **yes** (icons wired) |
| Meters 24 + yellow GPU | **yes** |
| Net grey bars | **yes** |
| Disk + USB (no Battery) | **yes** |
| Footer clock + power grid + red Power | **yes** (icon+label) |
| Scroll middle | **wired**; on 1440 content fits without overflow — not exercised by overflow smoke |

## Caveats / not claimed

- Gradient on permission card not replicated (GPUI no CSS linear-gradient in rsx path used).
- Power icons approximate mockup (users / sign-out / arrows-clockwise / power) — not pixel-identical path to every mockup glyph (Switch mock used a speaker-ish path).
- USB button labels shortened (`размонт.` / `извлечь`) so they fit 352 grid.
- Hover-peek round-trip / arm-confirm / play-mute **live clicks** not automated (ydotool dual-head known broken) — for user on master binary.
- Scroll overflow not proven (content short enough).
- Task 12 bar trigger still open.
- **No commit** per brief.

## Adversarial pass

First attacker pass **REFUTED** (icons missing, rsx minority, no report). After fix pass:
- icons + assets registered
- always-24 bars
- report written
- re-release + re-grim

Residual: not literal 100% every mockup SVG path; rsx not majority of LOC — honest above.

## Commands

```text
cargo test -p chronos --bin chronos side_panel   # 12 passed
cargo build --release -p chronos                 # Finished
pkill -x chronos; CHRONOS_SMOKE_SIDE_PANEL=1 ./target/release/chronos
grim -g "2180,0 380x1440" panel-dp1-v2.png
```
