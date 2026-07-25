# Slow service dispatch (audio / brightness) — 2026-07-25

Patterns proven during T123 (volume drag lag) and T125 brightness "jumps
for minutes". Canonical code: `crates/services/src/audio/mod.rs`,
`crates/services/src/brightness/mod.rs`.

## When this applies

Any command path that shells out or blocks >~50ms **and** can be fired
from drag / rapid clicks:

| Service | Backend | Drag risk |
|---|---|---|
| Audio volume | `wpctl set-volume` | medium (full `read_state`+`pw-dump` after each set was deadly) |
| Brightness | `ddcutil setvcp` × N displays | **high** (~0.5–1.5s each) |

## Required shape

1. **Optimistic** `Mutable` update **synchronously** in `dispatch` so UI
   re-renders before DDC/wpctl returns.
2. **Single writer task** fed by `tokio::sync::watch` (or equivalent
   latest-wins channel) — never `runtime.spawn` a new write per sample.
3. **Debounce** for very slow backends (brightness: ~150ms quiet after last
   Set, then one write of latest). Audio can apply more eagerly but still
   latest-wins, no full device dump on command path.
4. **No expensive re-read after every set** (`pw-dump`, dual `getvcp`).
   Confirm lightly or trust the set; reserve full re-read for poll /
   `Refresh` / open.
5. **Generation-gate Refresh** against concurrent Set so open-time re-read
   cannot clobber an in-flight user write.
6. **Never flip `available: false`** on a failed mid-session write/read if
   the service was available — that "breaks" the widget (n/a, disabled ±).

## Anti-patterns (lived)

```text
// BAD: every DragMove
cx.runtime.spawn(async {
    wpctl/ddc set;
    full_read_including_pw_dump_or_getvcp(); // races
    data.set(stale_or_failed);
});
```

```text
// GOOD sketch
dispatch(Set(v)) {
    data.set(optimistic v);
    set_tx.send(Some(v)); // latest-wins
}
// background: debounce → write_all(latest) → optional light confirm
// Refresh: if set_epoch changed during work → discard
```

## UI coupling

- Thumb/label: `view.dispatched.unwrap_or(service.value)`.
- Clear `dispatched` only when `service.value == dispatched`.
- ± buttons: absolute `Set(next)` from `dispatched.unwrap_or(value)`, not
  `Step` after an optimistic write (double-step).
- Distinct drag marker types per slider — see `chronos-gpui-popup`.

## Dev loop

```bash
chronos-rebuild && chronos-stop && chronos-start
# docs/dev-cli.md
```
