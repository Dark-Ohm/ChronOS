---
name: tokio-coop-budget-on-main-thread
description: Use when the GPUI main thread pins at 100% CPU and the UI freezes after a fixed number of async events (~125-128), when a foreground task reschedules itself in a storm, or before putting any tokio primitive (mpsc recv, time::timeout, Mutex::lock) on the main thread / inside cx.spawn.
---

# tokio coop budget on the GPUI main thread

## The symptom

The UI freezes and the main thread sits at ~100% CPU. The tell that separates
this from every other hang:

- **The freeze happens after a constant number of events** — measured in ChronOS
  T143: always exactly **125** streaming chunks, regardless of answer length
  (3837 / 5167 / 10374 chars) or duration (13 s / 18 s / 72 s).
- Background work keeps running fine. Only the main thread is wedged.
- `gdb` shows no single stuck function. Samples land in `epoll_wait`,
  `Ping::ping`, `dispatch_idles` — a **wake storm**, not a deadlock.

The decisive stack shape:

```
Runnable::run                       <- a foreground task is being polled
  RawTask::run       raw.rs:660
    RawTask::schedule raw.rs:438    <- ...and reschedules itself DURING its own poll
      foreground_executor::{closure}  platform_scheduler.rs:47
        Ping::ping
```

A task waking itself inside its own `poll` = infinite reschedule at full speed.

## The cause

`tokio::task::coop` limits how many tokio operations one task poll may perform:

```rust
// tokio/src/task/coop/mod.rs:115
const fn initial() -> Budget { Budget(Some(128)) }
```

When the budget runs out, tokio resources do this:

```rust
// tokio/src/task/coop/mod.rs:372-407
if has_budget_remaining() { Poll::Ready(()) }
else { register_waker(cx); Poll::Pending }   // register_waker -> cx.waker().wake_by_ref()
```

Pending **plus a self-wake**. That is fine inside a real tokio task: the budget
is replenished on the next poll, so it just yields.

The trap is `block_on`:

```rust
// tokio/src/runtime/park.rs:284
loop {
    if let Ready(v) = crate::task::coop::budget(|| f.as_mut().poll(&mut cx)) { return Ok(v); }
    self.park();
}
```

The budget is granted **per poll of the outer future**. If that future never
returns from its *first* poll — which is exactly what `app.run()` does, because
the entire Wayland event loop lives inside it — then the main thread receives
**one budget of 128 operations for the whole process lifetime**, and it is never
replenished.

Every `rx.recv()`, every `time::timeout` poll, every `Mutex::lock().await` on
the main thread spends one. After ~128 the thread is permanently over budget and
every subsequent poll self-wakes forever.

The magic constant `125` is `128` minus the handful of ops spent during startup.
It never depended on the UI at all.

**Amplifier (this fork):** `Source/gpui_linux/src/linux/dispatcher.rs:276-281` —
on any non-empty dispatch pass the receiver re-pings itself and does not clear
readiness. Any task feedback loop therefore becomes a hard 100% spin instead of
degrading gracefully.

## The rule

**Never run the GPUI event loop inside `block_on`.** Enter the runtime instead:

```rust
let rt = tokio::runtime::Builder::new_multi_thread().enable_all().build()?;
let _rt_guard = rt.enter();   // Handle::current() + tokio::spawn still work
app.run(move |cx| { /* ... */ });
```

`enter()` gives runtime context without imposing a coop budget on the thread.
The multi-thread runtime is driven by its own workers; `block_on` was never
needed to "keep the reactor alive". See `crates/app/src/main.rs` — the comment
there is load-bearing, do not "simplify" it away.

Corollary: prefer keeping tokio primitives off the main thread entirely
(DECISIONS.log "Runtime split": tokio for IPC/D-Bus, GPUI executor for UI). The
budget fix removes the cliff, it does not make main-thread tokio a good idea.

## How to diagnose this in one shot

1. `top -b -n2 -d1 -H -p <pid>` — confirm it is the **main** thread, not a worker.
2. Count the events before the freeze across runs of different sizes. **A
   constant** count is the signature; it rules out content, layout, and scroll.
3. One line, and it is decisive:

```rust
tracing::warn!("BUDGET-PROBE #{n} remaining={}", tokio::task::coop::has_budget_remaining());
```

   (`tokio::task::coop` is public with the `rt` feature.) If it flips to `false`
   exactly at the freezing event, the diagnosis is proven, not argued.

4. If you need to know *which* task storms, add a counter in the fork's
   main-thread dispatch that prints `runnable.metadata().location` every N
   runnables. `Source/gpui_linux/src/linux/wayland/client.rs`, in the closure
   that calls `Runnable::run`. **Temporary — `Source/` is shared by all sibling
   projects; revert with `git -C Source checkout <path>` before committing.**

## What this is NOT

Ruled out live in T143 — do not re-check these when the symptom reappears:

- Event/channel wiring (events were delivered fine, all 125 of them).
- Per-event timer spawn (`cx.background_executor().timer()` inside the loop) —
  a real defect, fixed, and the freeze survived it unchanged.
- Scroll (`set_offset(f32::MAX)` vs `ScrollHandle::scroll_to_bottom()`) — worth
  fixing on its own merits, irrelevant here.

Three plausible UI hypotheses, three misses. The symptom looked like a rendering
bug and lived entirely in the runtime.

## Verified

**Measured against:** tokio 1.52.3, calloop 0.14.4, rustc 1.97.1, gpui fork
`Source@3e4715f`, ChronOS `44ba823`. The line numbers below are from those
versions — if `Budget::initial()` or `poll_proceed` moved, re-locate them
before quoting this skill at anyone. The mechanism outlives the line numbers;
the line numbers do not outlive a minor bump.

ChronOS T143, 2026-07-27, live release build, one log file across the change:

| | before | after |
|---|---|---|
| events per turn | froze at 125 | 975, `turn END (reason=ok)` |
| coop budget | `remaining=false` at #125 | `remaining=true` at all 975 |
| runnables dispatched | 417,935,000 | 15,000 (none from the panel) |
| main thread CPU | 99.6% | 10% |
