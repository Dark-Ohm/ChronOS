# T123 — ПРИНЯТ WITH CAVEATS (2026-07-25)

**Статус: ACCEPTED WITH CAVEATS.** Volume drag coalesce + light re-read.
Commit: `5cad0bb`. Errata: coalesce must wait on watch `changed()`, not spin.
Review: `report-log/T123-audio-volume-drag-coalesce-review.md`.

---

<!-- T123 — Volume slider lag: drag storms wpctl + full read_state(pw-dump).
     Агент не в имени брифа. -->

# T123 — Audio volume drag: coalesce + light re-read (slider lag)

**Статус: DONE**  
**Симптом:** ползунки Volume/Mic в `volume_popup` лагают / дёргаются при drag.  
**Диагноз (Архитектор, 2026-07-25, сверка с кодом):** фронт только
рисует `AudioState`; на **каждый** `MouseDown`/`DragMove` зовётся
`set_volume_unmute_if_needed` → `AudioSubscriber::dispatch(Set*Volume)`.

## Корневая причина (доказано кодом)

### UI — `crates/app/src/volume_popup/view.rs`

```text
on_mouse_down / on_drag_move
  → frac_from_window_x
  → set_volume_unmute_if_needed(kind, frac, cx)
  → AppState::audio(cx).dispatch(SetSinkVolume | SetSourceVolume)
```

**Нет** троттлинга, **нет** optimistic local paint (thumb = `ep.volume` из
глобального стейта после round-trip).

### Service — `crates/services/src/audio/mod.rs`

`dispatch` (sync entry, async body):

1. `apply_command` → `spawn_blocking` → **`wpctl set-volume …`**
2. **сразу** `read_state()` → `spawn_blocking` который делает:
   - `wpctl get-volume` + `inspect` **sink**
   - `wpctl get-volume` + `inspect` **source**
   - **`pw-dump` + parse device lists** (тяжёлый JSON)
3. `data.set(full AudioState)`

При drag 60–120 событий/с: N параллельных `wpctl` + N×`pw-dump`, гонки
`data.set`, UI перерисовывается от «старых» re-read’ов → лаг/дрожь.

Poll loop уже 250 ms — **не** источник лага драга; лагает **командный** path.

## Цель

Плавный drag: thumb следует курсору, sink/source volume в PipeWire
догоняет без шторма subprocess’ов.

## Что сделать (порядок)

### Task 1 — Service: cheap path for volume set (обязательно)

В `AudioSubscriber::dispatch` (или helper’е):

Для **`SetSinkVolume` / `SetSourceVolume` only**:

1. **Optimistic (sync, before spawn):**  
   `data.lock` / update clone: set `sink.volume` или `source.volume` to
   clamped value; if unmuting was requested by UI separately — keep current
   mute protocol (UI already may `Toggle*Mute` on muted drag — see
   `set_volume_unmute_if_needed`).  
   Prefer **one place** for unmute-on-volume-change: either UI (today) or
   service — don't double-toggle.

2. **Spawn:** only `wpctl set-volume` (`apply_command`).  
   **Do NOT** call full `read_state()` (no `pw-dump`, no inspect both ends)
   after every volume set.

3. **Optional light confirm:** after set, single `wpctl get-volume` for that
   endpoint only; patch volume+muted fields; **preserve** existing
   `available` device lists from current state (do not clear them).

4. **Coalesce (strongly preferred):** latest-wins for volume commands:
   - shared `AtomicU64` / watch channel / `Mutex<Option<f64>>` pending
   - in-flight flag: while one `set-volume` runs, newer values overwrite
     pending; when free, apply only the **latest**
   - prevents N concurrent wpctl for same sink

Mute / SetDefault / ToggleStream: keep current path (or light re-read
without full dump if easy). Full `read_state`+pw-dump stays on **poll** only.

Tests (unit where pure):

- `command_to_wpctl_args` unchanged for volume.
- If you extract `merge_volume_into_state(state, sink|source, v)` — unit test.
- Document: no fabricated integration that shells wpctl in CI unless already
  pattern exists (`audio-dispatch-smoke` optional live).

### Task 2 — UI: throttle + optional local drag paint

`volume_popup/view.rs` (and bar scroll if same storm — bar is ±5% steps, low risk):

1. **Throttle dispatch** during drag: min interval **16–33 ms** (one per
   frame-ish) OR only dispatch when `|new - last_sent| >= 0.01` (1%).  
   Mouse **up** / drag end: always flush final value (need
   `on_drag` end or mouse up — if fork has no drag-end, flush last pending
   on next non-move or keep service coalesce as safety net).

2. **Optimistic paint (recommended):** while dragging, view field
   `drag_preview: Option<(EndpointKind, f64)>` drives fill/thumb; clear on
   end when service state catches up (or always prefer preview while
   `Some`). Avoid waiting for Mutable round-trip for the thumb.

3. Keep unmute-on-change semantics from mockup.

### Task 3 — Verify

```bash
cargo test -p chronos-services --lib audio -- --nocapture
cargo test -p chronos volume -- --nocapture
cargo build --release -p chronos
chronos-rebuild && chronos-stop && chronos-start
# live: open Sound popup, drag sink hard — thumb tracks finger, no multi-second lag
# optional: RUST_LOG=info, no flood of "command failed"; wpctl not pegging CPU
```

Report:  
`orchestration/tasks/report/T123-audio-volume-drag-coalesce-report.md`

## Зона файлов

**Писать:**
- `crates/services/src/audio/mod.rs` (+ small helper/types if needed)
- `crates/app/src/volume_popup/view.rs` (throttle / preview)

**Не трогать:**
- pipewire native rewrite (DECISIONS long-term — out of scope)
- volume popup visual redesign
- MPRIS / stream mute path unless needed to share helper

## Что НЕ делать

- Не «чинить» лагом `sleep` в UI-потоке.
- Не `read_state()` + pw-dump после каждого SetVolume.
- Не убирать poll 250 ms (нужен для внешних изменений).
- Не `let _ =` fallible without log.

## Accept

- Drag sink: thumb visually tracks without multi-event wpctl storm
  (architect: hard drag + optional `pidstat`/`strace -e execve` sanity).
- Volume still applies to PipeWire (hear / `wpctl get-volume`).
- Device list still populated on poll / open (not wiped by volume sets).
- Unit tests green.

**Reject:** only UI sleep without service change; still full `read_state`
after every set; dual unmute bugs.

## Связь

- T121 volume popup UX (accepted).  
- Backend still MVP `wpctl` (DECISIONS 2026-07-17) — this task makes MVP
  usable under drag; native PipeWire later.
