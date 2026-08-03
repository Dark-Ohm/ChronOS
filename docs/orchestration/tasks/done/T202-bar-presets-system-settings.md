# T202 — Bar presets + System settings «Bar» page

**Статус:** active **T200 ACCEPTED `dc811e9` — go** (live apply must work).
**Роль:** FRONTEND. **Модель: Sonnet 5 / GLM**.
**План:** live-customization §5 T202 + §6.4 (presets + 6–8 controls, not 200 fields).
**Канон:** PRODUCT live customization. **Правила:** `RULES.md`.

**Зависимости:** T199 schema + T200 apply. T201 nice-to-have (same schema keys
in UI labels) but **not** required to start UI if schema types exist.

**Параллельно:** не bar apply redesign; не T204 rails; не T194c.

**Зона:**
- `crates/app/src/side_panel_right/tab/` — extend **EditorSettings**
  («System settings» label) **or** new slim `bar_settings.rs` mounted from
  that tab / System — pick one place, don't scatter
- `crates/app/src/bar/` — `presets.rs`: named presets → `BarAppearance` +
  optional widget layout snapshots; load/save optional
  `~/.config/chronos/bar-presets.toml` **or** builtin-only v1
- wire controls → same save+apply path as T200 (no duplicate apply logic)

**НЕ:** full control center; token color pickers; vertical bar; agent tools
(T201); Follow (T203).

**Отчёт:** `docs/orchestration/tasks/report/T202-bar-presets-system-settings-report.md`.

---

## Цель

Человек без агента и без raw TOML:

1. Выбирает **пресет** → бар меняется live.
2. Крутит **6–8 контролов** → пишет appearance (и при необходимости widgets)
   в `bar.toml` → apply.

### Builtin presets (v1, code defaults — files optional)

| id | intent |
|---|---|
| `top-full` | edge top, full width, height 30, radius 0, exclusive on (today) |
| `bottom-full` | edge bottom, full, same (needs T200 edge support; if edge mid-session deferred, preset still writes config + cold-apply/warn) |
| `bottom-pill` | bottom, fraction ~0.7, center, radius 12, floating true, elevation soft |
| `minimal` | height 26, no elevation, widgets unchanged or thinner set |
| `gaming-quiet` | hide cava/mpris if present; rest unchanged |

Applying a preset: merge into config, save, `apply`. **Undo:** optional
«Revert last» using `.bak` if T200/T201 created one; else second preset click
back to `top-full`.

### Controls (max ~8)

Bind to appearance fields only (widgets stay edit-mode / agent):

1. Edge: top | bottom (segmented)
2. Height: slider 20–48
3. Width: full | 70% | 50% (maps fraction)
4. Floating: toggle (forces exclusive off via sanitize)
5. Radius: slider 0–16
6. Elevation: none | soft | strong
7. Exclusive: toggle (disabled/hidden when floating)
8. **Open bar.toml** → set `PreviewTarget` to config path (T194 Editor)

No color wheel. Labels = **schema key names** in muted subtitle
(`appearance.height`) so agent/UI/docs share vocabulary (PRODUCT).

### UX

- Section title **Bar** inside System settings (or dedicated subview).
- Preset chips/row on top; controls below.
- Empty/error: §13, no panic if apply fails — show last warning string.

### Tests

- preset `top-full` → appearance equals defaults
- control patch height → sanitized config field
- widget lists not wiped by appearance-only control (unless preset defines widgets)

```
cargo test -p chronos bar::
cargo test -p chronos side_panel_right::
cargo build --release -p chronos
```

Live: open System settings → preset bottom-pill → see bar; slider height.
NOT VERIFIED ok if honest.

Коммит: `ui : bar presets + system settings page (T202)`.

---

## Отчёт

```markdown
# T202 report
## Where UI lives (tab path)
## Presets table
## Controls → schema keys
## Apply path reuse
## Tests + live
## Что НЕ сделано
```


---

## Field note (2026-08-02 architect — resume)

**Partial work already in tree:**
- `crates/app/src/bar_settings.rs` — presets + patch TOML (lib) — ~519 lines
- `crates/app/src/side_panel_right/tab/bar_settings.rs` — UI (in flux; had
  compile errors E0277/E0308 on listeners vs App/Context)
- Full broken WIP copy: `docs/orchestration/tasks/notes/T202-bar_settings_tab-wip.rs`
  (from `/tmp/bar_settings_tab_wip2.rs`) — **restore from here**, don't rewrite
  from zero
- Wired: `PanelTab::EditorSettings` → `TabContent::BarSettings`, `lib.rs` mod

**Known blockers:**
- `cx.listener` in `Render` → `&mut App` not `Context<Self>` — use
  `entity.update` pattern (see comment in wip file)
- `theme.danger` doesn't exist — use `status.error`
- Drag slider: match gpui-component / existing panel drag signature
  (`&mut Window, &mut Context<T>`)

**T200 apply path:** write bar.toml → inotify → `apply_appearance` — do not
duplicate apply logic.

**Do not** leave the tree non-building; if UI incomplete, ship compile-clean
stub + keep full wip in notes/ until green.
