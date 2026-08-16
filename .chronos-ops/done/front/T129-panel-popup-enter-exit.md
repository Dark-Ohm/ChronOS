# T129 — Panel / popup enter–exit motion (visual depth wave 2/4)

**Статус: OPEN.**  
**Очередь:** после T128 (elevation) — **панели и темы рабочие**, exclusive
OK, theme wire closed 2026-07-26. Это **motion only**, не новые фичи.  
**Не:** toast motion (T130), 3D (T131–T132), theme tokens, exclusive_zone,
resize logic, TBD polish (art well / rail icons…).

| | |
|---|---|
| **Skills** | `vendored-gpui-animation`, `easing-and-springs`, `chronos-gpui-popup`, `gpui-layer-shell`, `chronos-shell` |
| **Эталон** | `volume_popup/view.rs` — `SpringBack` + `transition_when`; `side_panel_right/view.rs` — `revealed` + delayed flip |
| **Отчёт** | `docs/orchestration/tasks/report/T129-panel-popup-enter-exit-report.md` |
| **Коммит** | `ui : panel/popup enter-exit scale+opacity (T129)` (или split L/R + popups) |

## Цель

Единый язык **enter/exit** для поверхностей шелла:

- **scale + opacity** (не только fade)
- `gpui_animation`: `.with_transition(id)` + `.transition_when(open, dur, easing, |s| …)`
- Easing: **EaseOutBack-обёртка** как volume (`SpringBack(1.4…1.8)`), не Linear
- Длительность: **220–280 ms** enter (как volume devices 260); exit можно короче **160–200 ms** если API позволяет, иначе тот же dur
- Boot: `gpui_animation::init(window, cx)` уже в `bar/mod.rs` (idempotent) — **не дублировать** без нужды; layer-shell окна панелей анимируются, пока init сессии прошёл (bar up). Если transition dead — проверь, что bar уже открыт / init вызван

Unit green ≠ done. **Live grim** open/close left + right + ≥2 popups.

## Сейчас → цель

| Поверхность | Сейчас | Цель |
|---|---|---|
| Right panel | `revealed` + Linear **opacity only** 180ms | scale(~0.96→1) + opacity, SpringBack, ~240ms |
| Left panel | **нет** reveal transition | тот же паттерн, что right после upgrade |
| volume / system / updates / history | partial (volume: in-card sections) | **root card** enter при open окна (revealed flip после open) |
| launcher | ? | should, если cheap; иначе skip + note |
| toast | — | **T130, не трогать** |

## Паттерн (канон)

```rust
// View state
revealed: bool,  // false at new(); true after short spawn

// new():
cx.spawn(async move |this, cx| {
    cx.background_executor().timer(Duration::from_millis(16)).await;
    let _ = this.update(cx, |this, cx| {
        this.revealed = true;
        cx.notify();
    });
}).detach();

// render — INNER wrapper (not the sole on_hover root if hover is used):
div()
    .id("…-motion")
    .with_transition("…-enter")
    .opacity(if revealed { 1.0 } else { 0.0 })
    // if scale API exists on Style — use it; else opacity+translate if scale unsupported
    .transition_when(revealed, Duration::from_millis(240), SpringBack(1.5), |s| {
        s.opacity(1.0) /* + scale 1.0 if available */
    })
```

**Кровные правила:**

1. **Один `on_hover` на элемент** (fork debug_assert). Motion — на **inner** child, hover debounce — на outer (как right panel сейчас).
2. **Не** `transition_on_hover` для open/close окна.
3. **Не** трогать exclusive_zone / dock / width / IPC.
4. Close: окно уходит через `remove_window` — full exit animation **может** не успеть (Wayland). Acceptable: **enter polish** first; exit best-effort if cheap (delay close by ~dur only if already a pattern in tree — **не** изобретать ghost-window). Если exit delay = ghost risk → document skip exit, ship enter only.
5. `SpringBack` — copy from `volume_popup` (local `Transition` impl), **не** new crate.
6. Scale: check fork Style API (`scale` / transform). Если scale **нет** на `Div` style — opacity + slight `mt`/`ml` slide (2–4px) toward edge (left panel from left, right from right). Report which.

## Задачи

### Task 1 — Shared motion helper (optional but preferred)

`crates/ui` или `crates/app/src/motion.rs` (app-local OK):

- `ENTER_MS`, `SpringBack` re-export or thin wrap
- `fn enter_style(open: bool) -> …` only if it reduces copy-paste; **no** over-engineered MotionSystem

Unit: SpringBack(t=0)=0, t=1=1 (smoke).

### Task 2 — Right panel upgrade

`side_panel_right/view.rs`:

- Replace Linear opacity-only with SpringBack + scale/slide
- Keep `REVEAL_MS` / 16ms spawn pattern
- Rail-only: motion on body still OK (current target)
- Do **not** animate exclusive zone changes

### Task 3 — Left panel parity

`side_panel_left` panel root / main-content when window opens:

- Add `revealed` + same enter language as right
- Sessions rail may stay un-animated or share parent — prefer animating **chat column + sidebar chrome together** as one shell, not two staggered fights

### Task 4 — Popups root enter (should)

For each: on first paint after open, `revealed: false → true`:

- `volume_popup/view.rs` (root card, not re-break device list transition)
- `system_popup/view.rs`
- `updates_popup/view.rs`
- `notifications/history_popup/view.rs`

Skip if window lifecycle makes it ugly; note in report. **Do not** change anchor/height.

### Task 5 — Live smoke + report

```bash
chronos-rebuild && chronos-stop && chronos-start
# Super+A left open/close ×2 — enter visible
# Super+G right open/close ×2 — enter visible (not Linear snap)
# volume + system (or updates) — card eases in
# grim optional: open mid-animation hard; prefer RUST_LOG + user eyeball
# exclusive dock toggle still works (regression)
```

Report checklist:

- [ ] scale available? yes/no + API used  
- [ ] exit delayed? yes/no + ghost risk  
- [ ] files touched  
- [ ] exclusive/dock regression smoke  
- [ ] commit hash  

## Зона файлов

**Писать:**

- `crates/app/src/side_panel_right/view.rs`
- `crates/app/src/side_panel_left/panel.rs` (+ `mod.rs` only if `revealed` in view new)
- popup views listed Task 4
- optional `crates/app/src/motion.rs` or `crates/ui/src/…`

**Не трогать:**

- `exclusive_zone` / `dock_content` / resize handlers  
- `theme` / `surfaces.rs` / elevation (T128)  
- toast (`T130`)  
- services, IPC payloads  
- Source/ fork (except read-only API check for scale)  
- `reference/`

**Читать:**

- `volume_popup/view.rs` (`SpringBack`, `transition_when`)  
- `side_panel_right/view.rs` (revealed + on_hover split)  
- skill `chronos-gpui-popup` §animation boot  
- skill `easing-and-springs`

## Accept criteria

1. Left + right: visible enter (not hard cut) on pin-open.  
2. Right: no longer Linear-only opacity.  
3. ≥2 popups: root enter eases.  
4. Dock/exclusive toggle still works (user confirmed baseline).  
5. No new ghost windows; no double `on_hover`.  
6. Release build + lib tests for touched modules green.  
7. Report in `docs/orchestration/tasks/report/`.

## Reject

- Redesign layout / theme / new features  
- Toast animation  
- `sleep` in UI thread / busy-loop animation  
- Exit delay that leaves zombie layer-shell clients  
- Fabricated “looks smooth” without describing what changed  

## Commit style

```
ui : panel enter scale+opacity (T129)
side_panel_left : enter motion parity (T129)
volume_popup : root enter transition (T129)
```

No AI trailers.
