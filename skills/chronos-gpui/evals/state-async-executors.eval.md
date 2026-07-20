# state-async-executors — eval (verifiable)

Each question has a concrete answer and the `file:line` or example that proves it.
Use these to confirm the `state-async-executors.md` reference actually teaches the fork,
not a retelling. Answers were checked against `../Source/` at the commit this skill was
written against (2026-07-20).

---

## Q1 — Does `cx.global_mut::<T>()` create `T` if it was never set?

**A:** No. `App::global_mut` looks up `globals_by_type` and panics with
`no state of type {} exists` if the entry is missing
(`Source/gpui/src/app.rs:1884-1890`). First assignment must be
`cx.set_global(value)` (`app.rs:1906`) or `default_global` (`app.rs:1895`).

**Proven by:** app.rs:1884-1909. ChronOS cold-start Theme bug was this call order.

---

## Q2 — Why does a nested `handle.update(cx, …)` on the same window return `Err("window not found")`?

**A:** `App::update_window_id` (`Source/gpui/src/app.rs:1728`) does
`cx.windows.get_mut(id)?.take()?` (`:1733`), leaving the slot empty for the
duration of the callback. A nested update on the same id hits `.take()` on
`None` → `None` propagates → `.context("window not found")` at `:1781`.

**Proven by:** app.rs:1728-1781. This is the ghost-window root when
`remove_window` is attempted via a second `handle.update` from inside an
activation/click callback that already holds `&mut Window`.

---

## Q3 — What happens if you drop a `Task` without calling `.detach()`?

**A:** The task is cancelled immediately. `Task` is `#[must_use]`
(`Source/gpui_scheduler/src/executor.rs:288`); the doc comment at
`:287-288` states drop cancels and `detach` lets it run without returning a
value. `detach` is at `:327-333`.

**Proven by:** gpui_scheduler/src/executor.rs:282-333; platform test name
`spawn_dedicated_dropping_task_cancels_future` in
`Source/gpui/src/platform_scheduler.rs:269`.

---

## Q4 — How does `App::spawn` differ from `background_spawn`?

**A:**
- `App::spawn` (`Source/gpui/src/app.rs:1810-1823`) runs on the **foreground**
  executor; closure is `AsyncFnOnce(&mut AsyncApp) -> R` (can touch UI state
  via AsyncApp).
- `AppContext::background_spawn` (`app.rs:2660-2665`) /
  `BackgroundExecutor::spawn` (`executor.rs:89-94`) runs on a **background**
  pool; future must be `Send`, output `Send + 'static`; no AsyncApp handle
  unless you captured one yourself.

**Proven by:** app.rs:1810-1823, app.rs:2660-2665, executor.rs:89-94.

---

## Q5 — Is there an API to react when a global (e.g. Theme) changes?

**A:** Yes. `App::observe_global::<G>` (`Source/gpui/src/app.rs:1931-1944`),
`Context::observe_global` (`app/context.rs:176-190`), and
`Window::observe_global` (`window.rs:5277`). Mutations go through
`Effect::NotifyGlobalObservers` pushed by `set_global` / `global_mut` /
`default_global` / `remove_global` (e.g. app.rs:1908) and applied in
`apply_notify_global_observers_effect` (app.rs:1669).

**Proven by:** app.rs:1931, context.rs:176, app.rs:1669 + 1906-1909.

---

## Q6 — Does the fork ship easing curves, or must ChronOS port Kael from scratch?

**A:** The fork already has both:
1. Element free helpers in `elements/animation.rs` (`linear`, `ease_in_out`,
   `bounce`, `pulsating_between`, … — module at `:236+`).
2. ChronOS-edition `EasingCurve` enum in `Source/gpui/src/easing.rs:14-71`,
   header credits Kael (`easing.rs:1-2`).

**Proven by:** easing.rs:1-71; example `animation.rs` uses
`bounce(ease_in_out)` with `with_animation`.

---

## Q7 — How do you get a one-shot delay on the GPUI executor (not tokio)?

**A:** `cx.background_executor().timer(Duration)` returns `Task<()>`
(`Source/gpui/src/executor.rs:162-167`). Zero duration short-circuits to
`Task::ready(())`. There is **no** public `interval` API — repeat by
looping `timer().await` (see `examples/move_entity_between_windows.rs:35-50`).

**Proven by:** executor.rs:162-167; move_entity_between_windows.rs:35-50;
`rg interval` on executor/scheduler public API shows only one-shot timer.

---

## Q8 — What does `gpui_tokio::Tokio::spawn` do on GPUI Task drop?

**A:** It aborts the Tokio task. Implementation (`Source/gpui_tokio/src/gpui_tokio.rs:61-71`):
`handle.spawn(f)` + `abort_handle`; a `defer(|| abort_handle.abort())` is
dropped when the outer GPUI `background_spawn` future completes or is
cancelled — so drop of the returned `Task` cancels both layers.

**Proven by:** gpui_tokio.rs:55-72. Also: `init` must have been called first
or `read_global::<GlobalTokio>` panics (global missing).

---

## Q9 — `on_click`'s callback only gets `&mut App`. How do you mutate the view's own state from a click, without a `Global`?

**A:** `Context::listener` (`Source/gpui/src/app/context.rs:252-260`). It
takes `impl Fn(&mut T, &E, &mut Window, &mut Context<T>)` and returns
`impl Fn(&E, &mut Window, &mut App)` — precisely `on_click`'s listener
type (`elements/div.rs:1475`; alias `ClickListener`, `div.rs:1584`).
Internally it downgrades the entity and runs
`view.update(cx, |view, cx| f(view, e, window, cx)).ok()`.

`Context::processor` (`:264-272`) is the same adapter for callbacks that
must return a value.

**Proven by:** context.rs:252-272; div.rs:1475 + 1584. Live in ChronOS at
`crates/app/src/volume_popup/view.rs:199` (mutates `this.expanded` from
`on_click`); in the fork at `examples/opacity.rs:92`
(`on_click(cx.listener(Self::start_animation))` — a bare method reference).

**Anti-answer to reject:** "use a `Global` + a manual repaint helper."
That is a workaround for a non-problem, and it moves per-view UI state
into process-wide storage — wrong the moment the view has two instances
(multi-monitor). A grep for `on_click(move |` cannot match
`on_click(cx.listener(..))`, which is how this wrong conclusion gets
reached; grep both.

---

## Q10 — Can you call `.on_hover(..)` twice on one element to layer two behaviors?

**A:** No. `Interactivity::on_hover`
(`Source/gpui/src/elements/div.rs:618-626`) opens with
`debug_assert!(self.hover_listener.is_none(), "calling on_hover more than
once on the same element is not supported")` — the second call panics in
debug builds and silently overwrites the field's semantics otherwise.
There is exactly one `hover_listener` slot. Compose the two behaviors into
a single closure, or attach the second to a wrapping element.

Signature is `impl Fn(&bool, &mut Window, &mut App)` — the `&bool` is
"is now hovered", so enter and leave arrive through the same callback.

**Proven by:** div.rs:618-626 (imperative) and div.rs:1524-1530 (fluent
wrapper delegating to it).
