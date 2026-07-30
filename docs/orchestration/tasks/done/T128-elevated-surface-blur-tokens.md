# T128 — Elevated surface + blur tokens (visual depth wave 1/4)

**Статус: OPEN, не назначен.**  
**Канон:** `DECISIONS.log` / `MEMORY.md` / `HANDOFF.md` — visual depth 1–3  
  (glass/glow → panel motion → real 3D spike). **This task = step 1 only.**  
**Skills:** `backdrop-blur`, `chronos-gpui-popup`, `chronos-shell` theming.  
**Не:** enter/exit animation (T129), toast motion (T130), 3D mesh (T131–132).

## Цель

Один **продуктовый язык глубины** для карточек шелла:

1. **Theme tokens / helpers** — elevated card shadows + blur params, dark/light.  
2. **Apply** на anchored popups + panel chrome (не каждый div в дереве).  
3. Убрать copy-paste `BoxShadow::new(px(0.), px(6.), …)` / ad-hoc `paint_blur`.

Unit green ≠ done. **Live grim** dark (+ light if switchable).

## Текущее

| | Сейчас | Цель |
|---|---|---|
| Shadows | hard-coded в `volume_popup` / `system_popup` / `updates_popup` | `theme.elevated_shadows()` или `SurfaceElevation` |
| Blur | `paint_blur(..., px(18.), …, alpha 0.06)` в volume/system | `theme.blur_popup()` radius + tint |
| `bg.elevated` | цвет есть | остаётся fill; shadows — отдельно |
| Light C | watermark/glow per-popup | tokens + optional glow ring helper |
| Panels L/R | flat `#1e1e2e` / theme.bg | elevated chrome на root card (rail can stay flat) |

**Эталон визуала:** volume/system post-T121/T125 (Light C + blur) — **не** изобретать новую эстетику.

## API shape (предложение; зафиксируй в коде)

В `crates/ui` (рядом с `Theme` / `on_fill`):

```rust
// names flexible — document in report
pub struct ElevationTokens {
    pub shadows: Vec<BoxShadow>,      // or small fixed array
    pub blur_radius: Pixels,
    pub blur_tint: Hsla,              // paint_blur tint
    pub blur_saturation: f32,         // if API needs it
    pub radius: Pixels,               // often theme.radius_lg
}

impl Theme {
    pub fn elevation_popup(&self) -> ElevationTokens { … }
    // optional: elevation_panel() if panels differ
}
```

Или free functions:

```rust
chronos_ui::elevation::popup_card(theme) -> (bg, radius, shadows, BlurSpec)
```

**Constraint:** helpers must work without pulling `Window` — shadows/style only.  
`paint_blur` stays in view paint (`canvas` closure) but **reads radii/tint from tokens**.

## Задачи

### Task 1 — Tokens in `crates/ui`

- Add elevation/blur tokens for **dark + light** schemes (`schemes.rs` / `mod.rs`).  
- Defaults match current volume popup recipe (measure from code, not invent):  
  - drop shadow ~ y=6, soft indigo  
  - accent ring ~ 1px / glow `#007acc` low alpha  
  - blur ~18px, light tint alpha ~0.06  
- Unit tests: tokens present both schemes; light ≠ dark where intentional.  
- `[lints] workspace = true` already on ui — no new unwrap.

### Task 2 — Apply popups (must)

Migrate to tokens (style + blur params):

- `volume_popup/view.rs`  
- `system_popup/view.rs`  
- `updates_popup/view.rs`  
- `notifications/history_popup/view.rs`  
- ephemeral toast card chrome if shares same card (`notifications/view.rs`) — if cheap

**Do not** redesign layout/widths. Shadow/blur only.

### Task 3 — Apply panel chrome (should)

- Left: main content / chat column root when `chat_open` (not rail-only strip).  
- Right: content column when open (not icon rail alone).  
- Optional: launcher card if one root surface.

Rail/sidebar icon strips may stay flat (like macOS sidebars).

### Task 4 — Live smoke

```bash
chronos-rebuild && chronos-stop && chronos-start
# open volume, system, updates, history — same depth language
# open left chat + right content — elevated content, not muddy
# toggle light theme if available — still readable
# grim: volume dark, system dark, one panel, light if possible
```

Отчёт:  
`docs/orchestration/tasks/report/T128-elevated-surface-blur-tokens-report.md`  
— API surface, file list, before/after note, grim paths.

## Зона файлов

**Писать:**
- `crates/ui/src/theme/**` (+ maybe `crates/ui/src/elevation.rs`)
- popup views listed above  
- optional panel roots in `side_panel_{left,right}`

**Не трогать:**
- services, IPC, exclusive_zone logic  
- `gpui_animation` enter/exit (T129)  
- Source/ fork shaders (T131)  
- tray_menu full redesign (later polish wave)

**Читать:**
- volume_popup blur+shadow block (canonical recipe)  
- skill `backdrop-blur`  
- `Theme::on_fill` pattern for scheme-safe accents

## Что НЕ делать

- Не gpui-d3rs / 3D mesh  
- Не animate exclusive zone  
- Не `let _ = fallible`  
- Не менять POPUP_WIDTH / BASE_HEIGHT «заодно»  
- Не фабриковать grim  

## Accept / Reject

**Accept:** tokens in ui; ≥3 popups use them; panels content elevated; dark grim; release build; no visual regression on mockup fidelity.

**Reject:** only docs; shadows still copy-pasted hex in 5 files; blur params still magic numbers only in volume; unit-only.

## Out of scope (queued)

| T | What |
|---|---|
| **T129** | Panel/popup enter-exit (scale+opacity springs); `gpui_animation::init` discipline |
| **T130** | Toast enter/exit motion |
| **T131** | Fork spike: 3D scene primitive + example |
| **T132** | Wire one 3D demo surface in shell |

## Commit style

`ui : elevation + blur tokens (T128)`  
`popups : use elevation tokens (T128)`  
named `git add` only.

## Report path

`docs/orchestration/tasks/report/T128-elevated-surface-blur-tokens-report.md`
