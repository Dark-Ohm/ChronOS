# T206 — Right panel: resize works + no gray lip + stick to cursor

**Статус:** active. **Роль:** FRONTEND. **Приоритет: UX residual (user pain).**
**Модель: Sonnet 5.**
**Предшественник:** T204 `96c40d4` + errata `1d9b71b`; further uncommitted
experiments broke resize entirely (overlay + `right−pointer` snap).
**Правила:** `docs/orchestration/agents/RULES.md`.

**Параллельно:** T205 editor — **не** трогай `tab/preview*`.  
T202 — **не** трогай `bar_settings*`.

**Зона:**
- `crates/app/src/side_panel_right/view.rs` (handle + start/update_resize + layout)
- `crates/app/src/side_panel_right/mod.rs` (RAIL_ONLY_WIDTH, tests)
- `tab/terminal.rs` only if HANDLE_WIDTH in avail_w must match layout

**НЕ:** left panel rewrite; bar appearance; Editor chrome (T205).

**Отчёт:** `docs/orchestration/tasks/report/T206-right-panel-resize-stick-report.md`.

---

## Symptoms (user / architect, 2026-08-02)

1. **Gray lip** past white hairline — panel «не въехала»; handle column
   painted chrome on Transparent layer-shell next to rail.
2. **Resize drift** then **resize dead** after overlay + abs math experiments.
3. Left panel OK (LEFT anchor + local delta).

## Diagnosis (locked)

| bug | cause |
|---|---|
| Gray lip | flex handle with solid chrome fill **or** empty body + border beside rail |
| Snap / dead resize | `width = display_right − pointer_abs` after rail→expand leaves cursor near **right** edge → width snaps ~36 |
| Overlay unhittable | absolute transparent handle without reliable hit path |

## Required behavior

1. **Handle:** 4px flex hit strip (must receive `on_drag`). Paint
   **transparent** (no gray lip). Border hairline only when content open
   (`content_open`), not in rail-only.
2. **RAIL_ONLY_WIDTH** = `RAIL_WIDTH + HANDLE_WIDTH` (40) while handle is flex
   column — window must include hit strip. Visual rail stays 36.
3. **Resize math (right-anchored):**
   - `new = start_w - (current_x - start_x)` local delta (same as pre-experiment).
   - On rail-only mousedown: expand to tab preferred width; set
     `resize_start_width = target`; **do not** recompute width from abs pointer
     on the same click.
   - Window resize: may stay coalesced in `render` (left panel pattern) **or**
     immediate if proven; no per-move rebaseline of start_x that fights origin.
4. **Live verify:** drag expand + shrink follows mouse; rail-only no gray column
   past rail; left panel unchanged.

## Working tree note

Uncommitted edits may already exist in `view.rs`/`mod.rs` from architect
session — **read them**, finish/clean to this brief, don't re-invent thrice.

## Verification

```
cargo test -p chronos side_panel_right::
cargo build --release -p chronos
```

Live: rail-only → drag left → panel expands; drag right → shrinks; no snap-to-36;
no gray lip. grim optional.

Коммит: `panels : right resize stick + transparent handle (T206)`.
