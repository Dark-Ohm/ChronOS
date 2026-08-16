# T200 — Apply bar appearance live (hot-reload, no kill)

**Статус:** active. **T199 ACCEPTED** `31ec352` — go. **Роль:** FRONTEND.
**Модель:** Sonnet 5 / GLM 5.2.
**План:** `docs/superpowers/plans/2026-08-02-live-customization.md` §5–6.
**RECON:** `report-log/T198-…` (**ACCEPTED WITH NOTE**).
**Schema:** `crates/app/src/bar/appearance.rs` + `cached_appearance()`.
**Правила:** `docs/orchestration/agents/RULES.md`.

**Параллельно запрещено:** ломать T194* (`tab/preview*`, `tab/terminal*`).

**Зона:**
- `crates/app/src/bar/mod.rs` — window_options, render chrome (radius/clip/border edge), **store `WindowHandle`**, apply path
- `crates/app/src/bar/layout_config.rs` and/or `appearance.rs` — hook from `apply()` only (call into bar apply; no schema redesign)
- consumers of `BAR_HEIGHT` that must track live height:
  - `side_panel_right/mod.rs` (`PANEL_EDGE_GAP`)
  - `side_panel_right/hover_strip.rs`
  - `side_panel_left/mod.rs`
  - `side_panel_left/hover_strip.rs`
- **optional fork** only if implementing live re-anchor: `../Source/gpui` +
  `gpui_linux` `set_anchor` / `set_margin` / `set_layer` — **separate
  commit**, document in report; prefer v1 without fork if edge stays top

**НЕ:**
- agent tools (T201), presets UI (T202), vertical bar, multi-bar
- hug width feedback loop (schema accepts hug; apply treats as **full** + warn once, or no-op)
- theme.toml token overrides
- dock as separate window (dock is bar widget)

**Отчёт:** `docs/orchestration/tasks/report/T200-bar-appearance-apply-report.md`.

---

## Цель

Смена `~/.config/chronos/bar.toml` `[appearance]` (или defaults) → **без
pkill/re-login** бар и зависимые панели подстраиваются.

Минимальный **must ship** (v1 apply):

| field | apply |
|---|---|
| `height` | `Window::resize` width×height; `set_exclusive_zone` if exclusive |
| `exclusive` / `floating` | floating ⇒ exclusive zone `None`/0; else `Some(height)` |
| `radius` | root div `.rounded(px(r))` + `.overflow_hidden()` when r>0 |
| `elevation` | map soft/strong → existing `ElevationTokens` helpers (`elevation_blur_layer` / glow) **if cheap**; `none` = current flat bar |
| widgets | already applied — don't regress |

**Should if cheap (same PR):**

| field | apply |
|---|---|
| `width` full | current |
| `width` fraction:f | `resize(display_w * f, height)`; align via **margin math** if live `set_margin` exists — else only resize + left-anchored, document |
| `edge` top↔bottom | **only if** fork `set_anchor` shipped **or** you implement carefully; **default: cold-path** — apply edge at `window_options()` open time; mid-session edge change → log warn «restart shell to flip edge» OR implement fork patch (preferred long-term, optional this task) |

**Explicit defer (report as NOT done):**
- hug measure loop
- live edge flip without fork
- full floating pill + perfect input region if geometry incomplete

### T198 NOTE (обязательно учесть)

`Window::set_input_region` **уже есть** (`window.rs` ~2029, Wayland impl).
Не пиши «нужен fork для input_region». Для floating/fraction: после resize
выставь input region на видимую «пилюлю» если width < full; иначе full.

Live **нет:** `set_anchor` / `set_margin` / `set_layer` on PlatformWindow —
T198. Create-time only in wayland window ctor.

---

## Architecture requirements

1. **`open_on_display` currently discards `WindowHandle`** (`bar/mod.rs`
   `Ok(_) => true`). **Store** handle globally or on a small `BarSurface`
   global so `apply_appearance` can `resize` / exclusive / input_region.
2. **`layout_config::apply`** after widget rebuild must call
   `bar::apply_appearance(cx)` (name free) reading `cached_appearance()`.
3. **Do not** `remove_window` + reopen for height/radius/exclusive — skill
   `wayland-window-lifecycle` (ghost windows).
4. **Panels:** `PANEL_EDGE_GAP = BAR_HEIGHT` const breaks when height
   changes. Introduce **live** gap source (e.g. `cached_appearance().height`
   or `BarGeometry` global updated on apply). Resize/reposition panels if
   they already have handles; if only open-time geometry — at least use
   live height on next open + document residual.
5. **OSD** is bottom-anchored (`osd/mod.rs`). If you implement bottom bar
   edge, note collision; for top-only v1 no change.
6. **Border:** top bar uses `border_b_1()`; if edge bottom ever applies,
   use top border — only if edge apply lands.

---

## Implementation order (recommended)

1. WindowHandle store + `apply_appearance` for height + exclusive.
2. Wire into `layout_config::apply` / watcher (already 300ms debounce).
3. radius/clip in `Bar::render`.
4. fraction width resize + optional input_region.
5. Consumers: replace `const PANEL_EDGE_GAP = BAR_HEIGHT` with live height.
6. (Optional) fork set_anchor + bottom edge live.

---

## Верификация

```
cargo test -p chronos bar::
cargo test -p chronos
cargo build --release -p chronos
```

**Live (обязательно если доступен шелл; иначе NOT VERIFIED честно):**

1. Edit `~/.config/chronos/bar.toml` — add `version = 2` +
   `[appearance] height = 40` → bar thicker without restart.
2. `exclusive = false` or `floating = true` → windows can go under bar area
   (hypr reserved zone change).
3. `radius = 12` → visible clip (grim).
4. Invalid height 999 → clamped, no crash; last-good if parse fail.
5. Widgets still hot-reload.
6. `RUST_LOG=info`, 0 panic.

Коммит(ы):
- `bar : apply appearance hot-reload (T200)` 
- if fork: `gpui : live set_anchor/set_margin for layer-shell` **отдельный**

Поимённый git add. Не тащи docs dirt.

---

## Отчёт — формат

```markdown
# T200 report
## What applies live (table field → API)
## WindowHandle store
## Panel gap consumers
## Fork changes (or explicitly none)
## Verification (tests + live / NOT VERIFIED)
## Что НЕ сделано (hug, edge flip, …)
```

## Acceptance criteria

- [ ] height change via bar.toml without process restart
- [ ] exclusive/floating coherent
- [ ] radius visible when >0
- [ ] no ghost bar windows
- [ ] widgets path unbroken
- [ ] honest live section
