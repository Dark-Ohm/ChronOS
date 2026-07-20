# State, async, executors — App / Entity / Task / observers

**When to load:** Before touching globals (`Theme`, registries), spawning
tasks (`cx.spawn` / `background_spawn` / `.detach()`), writing
`observe`/`subscribe`/`observe_global`, or claiming "the fork can't do
timers/animations/hot-reload of theme". Every claim below is anchored to
`../Source/` (our gpui-ce chronos edition), not upstream Zed.

**Ground-truth rule (from SKILL.md):** a "the fork cannot X" claim needs a
`file:line` from `Source/` or a runnable example.

**How this doc was produced (batches):**
1. App/globals/`update_window_id` + Entity/Context/AsyncApp
2. executor / `Task` / `gpui_scheduler` / `gpui_tokio` / timers
3. subscribe/observe family + animation/easing + zone examples + vendored collections

---

## 1. Globals — `set_global` / `global` / `global_mut` / `has_global` / `try_global`

Marker trait: `Global` (`Source/gpui/src/global.rs:22`) — empty, `'static` only.
Helpers: `ReadGlobal` / `UpdateGlobal` (`global.rs:30-75`) call through to `App`.

| Method | File:line | Panics? | Notes |
|---|---|---|---|
| `has_global::<G>()` | `app.rs:1862` | no | `globals_by_type.contains_key(TypeId)` |
| `try_global::<G>()` | `app.rs:1876` | no | `Option<&G>` |
| `global::<G>()` | `app.rs:1868` | **yes** | `"no state of type {} exists"` |
| `global_mut::<G>()` | `app.rs:1884` | **yes** | same message; **also pushes** `Effect::NotifyGlobalObservers` |
| `default_global::<G>()` | `app.rs:1895` | no (inserts `Default`) | notifies observers |
| `set_global::<G>(g)` | `app.rs:1906` | no | insert + notify |
| `remove_global::<G>()` | `app.rs:1919` | **yes** if missing | notifies observers |
| `clear_globals()` | `app.rs:1914` | — | test-only; does **not** notify |

### Cold-start trap (ChronOS Theme)

`global_mut` / `global` **require the type to already be in the map**.
Calling `Theme::set` as `*cx.global_mut::<Theme>() = …` **before**
`cx.set_global(Theme::…)` panics with `no state of type … exists`
(`app.rs:1890`). Correct order:

```rust
// first assignment
cx.set_global(Theme::default());
// later mutations
*cx.global_mut::<Theme>() = new_theme;
// or read-safe
if cx.has_global::<Theme>() { let t = cx.global::<Theme>(); … }
```

`set_global` / `global_mut` / `default_global` / `remove_global` all push
`Effect::NotifyGlobalObservers` (`app.rs:1886, 1897, 1908, 1921`), applied in
`apply_notify_global_observers_effect` (`app.rs:1669`). That is what makes
**hot-reload of theme** possible via `observe_global` (see §5).

---

## 2. Reentrancy of `update_window_id` — root of ghost windows

```
// Source/gpui/src/app.rs:1728-1781
pub(crate) fn update_window_id<T, F>(&mut self, id: WindowId, update: F) -> Result<T> {
    self.update(|cx| {
        let mut window = cx.windows.get_mut(id)?.take()?;   // ← slot emptied
        // … run callback with &mut Window …
        trail(id, window, cx)?;                            // ← put back or remove
        Some(result)
    })
    .context("window not found")
}
```

While the callback runs, `cx.windows[id]` is **`None`** (taken out of the slot).
A nested `handle.update(cx, …)` / `AnyWindowHandle::update` on the **same** id
hits `.take()?` again → `None` → outer `Result` becomes
`Err(… "window not found")` (`app.rs:1781`).

If that `Err` is swallowed with `let _ =`, `remove_window()` never runs →
**ghost window**. This is the ChronOS blood fact from MEMORY/HANDOFF.

**Rule:** if the callback already holds `&mut Window` for this surface, call
`window.remove_window()` **directly**. Never re-enter via a second
`handle.update` on the same id.

---

## 3. Entity / Context / AsyncApp

| Type | Where | Role |
|---|---|---|
| `Entity<T>` | `app/entity_map.rs:414` | Strong typed handle; `read`/`update`/`downgrade` (`:464-481`) |
| `WeakEntity<T>` | `entity_map.rs:740` | Non-retaining; upgrade may fail after release |
| `Context<'a, T>` | `app/context.rs:20` | `&mut App` + weak self; `Deref`→`App` (`:25-37`) |
| `AsyncApp` | `app/async_context.rs:22` | Cloneable, `'static`, held across `await`; weak `AppCell` |
| `AsyncWindowContext` | same module | Window-scoped async handle (from `Window::spawn`) |

`Context::entity()` upgrades the weak handle and panics if the entity is dead
(`context.rs:50-54`) — safe only while GPUI still owns the entity (inside
update callbacks).

`Context::spawn` (`context.rs:237-245`) gives the closure
`(WeakEntity<T>, &mut AsyncApp)` on the **foreground** executor.

### `Context::listener` — the bridge from element callbacks to view state

**Every element event callback takes `&mut App`, never `&mut Context<T>`.**
`on_click`'s listener is `Fn(&ClickEvent, &mut Window, &mut App)`
(`elements/div.rs:1475`, alias `ClickListener` at `:1584`); `on_hover` is
`Fn(&bool, &mut Window, &mut App)` (`div.rs:618` imperative, `:1524`
fluent). Same shape across the family.

This does **not** mean view state is unreachable from a click.
`Context::listener` (`context.rs:252-260`) is the adapter built for
exactly this:

```rust
pub fn listener<E: ?Sized>(
    &self,
    f: impl Fn(&mut T, &E, &mut Window, &mut Context<T>) + 'static,
) -> impl Fn(&E, &mut Window, &mut App) + 'static
```

It downgrades `self.entity()` to a weak handle and calls
`view.update(cx, |view, cx| f(view, e, window, cx)).ok()` internally — so
you write a closure over `&mut T` + `&mut Context<T>` and hand the result
straight to `on_click`. `Context::processor` (`:264-272`) is the same
trick for callbacks that must **return** a value.

Note the `.ok()` at `:258`: if the entity is already released the listener
is a **silent no-op**. That is intentional (a click on a dying view should
do nothing), but it means a listener that never fires is a symptom of a
dead entity, not necessarily of bad wiring.

**Live proof, in ChronOS itself:** `crates/app/src/volume_popup/view.rs:199`
mutates view-local state (`this.expanded`) from an `on_click` this way;
`launcher/view.rs:160` and `desktop_terminal/view.rs:343` do it for
`on_key_down`. In the fork, 15 examples use it —
`Source/gpui/examples/opacity.rs:92` (`on_click(cx.listener(Self::start_animation))`)
is the shortest read, and shows a **method reference** works directly, no
closure needed, when the signature matches.

**The grep trap that hides this** (cost a real planning error 2026-07-20):
searching `on_click(move |` finds only raw-closure call sites and
*structurally cannot* match `on_click(cx.listener(..))`. Conclude from
that grep that "clicks only get `&mut App`, so view state needs a
`Global`" and you will invent a `Global` + manual-repaint workaround for
a problem the framework already solved — and put per-view UI state into
process-wide storage, which breaks the moment a second instance of that
view exists (multi-monitor). **Grep `cx.listener` too, always.**

`Context::notify` (`context.rs:230`) → `App::notify(entity_id)` → observers fire.

`Context::emit` (`context.rs:765-779`) pushes `Effect::Emit` for
`EventEmitter<Evt>` subscribers.

**Proof example:** `examples/ownership_post.rs` — `cx.new`, `cx.subscribe`,
`cx.emit(Change)`, `cx.notify`, `entity.update` / `read`. Compiles
(`cargo check --example ownership_post -p 'path+file://…/Source/gpui#0.2.2'` ✓).

---

## 4. Tasks and cancellation

### `Task<T>` — blood fact, confirmed

```
// Source/gpui_scheduler/src/executor.rs:282-333
/// If you drop a task it will be cancelled immediately. Calling [`Task::detach`]
/// allows the task to continue running, but with no way to return a value.
#[must_use]
pub struct Task<T>(TaskState<T>);

pub fn detach(self) { /* task.detach() for Spawned */ }
```

- **`#[must_use]`** at `executor.rs:288` (and `FallibleTask` at `:378`).
- **Drop = cancel** — documented on the struct; `detach()` opts out.
- `TaskExt::detach_and_log_err` (`gpui/src/executor.rs:31-58`) — for
  `Task<Result<T,E>>`: re-spawns on foreground with `log_tracked_err`, then
  `.detach()`. Needs `&App`.

Platform test explicitly names cancel-on-drop:
`platform_scheduler.rs` test `spawn_dedicated_dropping_task_cancels_future`
(`:269` in that file).

### `cx.spawn` vs `cx.background_spawn` / `background_executor`

| API | Executor | Closure shape | `Send`? | File:line |
|---|---|---|---|---|
| `App::spawn` / `Context::spawn` | **foreground** (main) | `AsyncFnOnce(&mut AsyncApp) -> R` | R: `'static` only | `app.rs:1810`, `context.rs:237` |
| `AppContext::background_spawn` | **background** pool | `Future + Send` | R: `Send + 'static` | `app.rs:2660` |
| `background_executor().spawn` | same | same | same | `executor.rs:89` |
| `Window::spawn` | foreground via `App::spawn` | `AsyncFnOnce(&mut AsyncWindowContext)` | — | `window.rs:2252` |
| `Context::spawn_in` | foreground + window | window-bound async | — | `context.rs:676` |

Foreground path panics if `self.quitting` (`app.rs:1816-1817`,
`foreground_executor` at `:1801-1803`).

**Bare `cx.background_spawn(fut)` without `.detach()` / holding the `Task`**
cancels on drop of the temporary — ChronOS blood fact (fixed e.g. battery
`8766c31`).

### `gpui_tokio` — Tokio bridge

`Source/gpui_tokio/src/gpui_tokio.rs`:

- `init(cx)` builds a multi-thread runtime (**2 worker threads**), stores
  `GlobalTokio` via `set_global` (`:12-25`).
- `init_from_handle` if you already own a `Handle` (`:28-33`).
- `Tokio::spawn` / `spawn_result` (`:55-95`): `handle.spawn(f)` + GPUI
  `background_spawn` that awaits the `JoinHandle`; **abort on GPUI Task drop**
  via `defer(abort_handle.abort)`.
- `Tokio::handle(cx)` (`:98`) clones the `Handle`.

**No `spawn_blocking` wrapper in this crate.** Calling bare
`tokio::spawn_blocking` without a entered runtime / without
`Tokio::handle(cx).spawn_blocking(…)` has nowhere to run — the classic hang
(or panic "no reactor") ChronOS hit historically. Confirm path: use
`gpui_tokio::init` first, then either `Tokio::spawn` or
`Tokio::handle(cx).spawn_blocking(...)`.

---

## 5. Timers and frame callbacks

| Mechanism | API | Notes |
|---|---|---|
| One-shot timer | `background_executor().timer(Duration)` | `executor.rs:162-167`; zero duration → `Task::ready(())` |
| Foreground loop | `cx.spawn` + `timer().await` in a loop | ChronOS bar tick pattern |
| Next frame | `Window::request_animation_frame` | `window.rs:2229-2231` → `on_next_frame` + `notify` current view |
| Decorative animation | `AnimationExt::with_animation` | auto-respects `App::reduce_motion` (doc at `animation.rs:52-56`, `window.rs:2224-2228`) |

**There is no public `interval()` API** in `BackgroundExecutor` /
`gpui_scheduler` — only one-shot `timer`. Repeating work = loop + await timer
(see `move_entity_between_windows.rs:35-50`).

`request_animation_frame` is the `rAF` analogue; for spinners/pulses prefer
`with_animation` so reduce-motion is honored.

---

## 6. Subscriptions and observers — full fork surface

### App-level (`App`)

| Method | File:line | Fires when |
|---|---|---|
| `observe(entity, …)` | `app.rs:1050` | entity `notify` |
| `subscribe(entity, …)` | `app.rs:1139` | entity `emit` of `Event` |
| `observe_global::<G>(…)` | `app.rs:1931` | global set/mut/remove (via effect) |
| `observe_new::<T>(…)` | `app.rs:1977` | new entity of type T created |
| `observe_release(entity, …)` | `app.rs:1998` | entity released |
| `observe_release_in` | `app.rs:2019` | release with window context |
| `observe_keystrokes` | `app.rs:2037` | keystroke after other handlers |

### Context-level (`Context<T>`) — preferred inside views

| Method | File:line | Notes |
|---|---|---|
| `observe` / `observe_self` | `context.rs:63` / `:84` | other / self notify |
| `subscribe` / `subscribe_self` | `:98` / `:120` | typed events |
| `on_release` | `:135` | self release |
| `observe_release` | `:151` | other entity release |
| `observe_global::<G>` | `:176` | **theme hot-reload hook** |
| `observe_global_in` | `:705` | global + window |
| `observe_in` / `subscribe_in` | `:321` / `:355` | with `&mut Window` |
| `observe_release_in` | `:404` | |
| `observe_window_bounds` | `:426` | |
| `observe_window_activation` | `:444` | needs `&mut Window` — register in open-window closure |
| `observe_window_appearance` | `:462` | |
| `observe_button_layout_changed` | `:480` | |
| `observe_keystrokes` | `:500` | |
| `observe_pending_input` | `:528` | |
| `emit` | `:765` | push event |
| `notify` | via `:230` | |

### Window-level

| Method | File:line |
|---|---|
| `Window::observe` | `window.rs:2120` |
| `Window::subscribe` | `:2149` |
| `Window::observe_release` | `:2184` |
| `Window::observe_global` | `:5277` |
| `Window::observe_window_appearance` | `:1829` |
| `Window::observe_button_layout_changed` | `:1845` |

### Subscription handle

`Subscription` (`subscription.rs:147-168`): **`#[must_use]`**, drop =
unsubscribe. `.detach()` keeps the callback alive until emitters die
(same pattern as `Task::detach`).

**Theme hot-reload:** yes — `observe_global::<Theme>` on `App` or `Context`
fires when `set_global` / `global_mut` / `remove_global` run (effect path
`app.rs:1669` + registration `app.rs:1931` / `context.rs:176`).

---

## 7. Animations and easing

### Element animation API (`elements/animation.rs`)

- `Animation { duration, oneshot, easing }` — `animation.rs:14-47`
- `Animation::new(d).repeat().with_easing(f)`
- `element.with_animation(id, anim, |el, delta| …)` — `AnimationExt` `:57-74`
- `with_animations` for a chain — `:77-92`
- Free easing helpers in the same module's `easing` submodule (`:236+`):
  `linear`, `quadratic`, `ease_in_out`, `ease_out_quint`, `bounce`,
  `pulsating_between`

**Proof:** `examples/animation.rs` — rotating SVG via
`with_animation(…, Animation::new(2s).repeat().with_easing(bounce(ease_in_out)), …)`.
`cargo check --example animation …` ✓.

### ChronOS fork easing port from Kael (`easing.rs`)

Header: `// Portions derived from Kael…` (`easing.rs:1-2`).

`EasingCurve` enum (`easing.rs:14-71`) — large set: Linear, EaseIn/Out/InOut
(quad/cubic/quart/quint/expo/circ), Back, Elastic, Steps, CubicBezier, Custom.
`EasingCurve::sample(delta)` clamps to `0..=1`.

Also free functions / `SpringPreset` in the same file. **We already have a
full easing port** — no need to re-port Kael curves from scratch for basic UI
motion. Element animations still take `Fn(f32)->f32` (Animation.easing), so
wire `EasingCurve::sample` or the free helpers as needed.

`image_loading.rs` uses `pulsating_between` + `background_executor().timer` for
async asset load — bridge between animation and tasks.

---

## 8. Zone examples (compile status)

Package selector (required — name `gpui` is ambiguous):

```
cargo check --example <name> -p 'path+file:///home/neo/projects/chronos-ecosystem/Source/gpui#0.2.2'
```

| Example | Demonstrates | `cargo check` | ChronOS transfer |
|---|---|---|---|
| `ownership_post` | Entity, subscribe, emit, notify | ✓ | Core state pattern for any view |
| `animation` | `with_animation`, bounce/ease_in_out, SVG | ✓ | Decorative motion; works on layer-shell elements the same way |
| `image_loading` | `Asset` trait, timer delay, loading anim | ✓ | Async assets; usable if we expose AssetSource (bar icons already) |
| `gif_viewer` | `img(path)` animated GIF via `.id` | ✓ | Simple; not ChronOS-critical |
| `move_entity_between_windows` | `spawn_in` + timer loop, `subscribe_in`, re-host entity across windows | ✓ | Entity lifetime across surfaces; less relevant for single bar window |

**No `async_*` examples exist** in `Source/gpui/examples/` (listing verified
2026-07-20). Brief named them generically — treat `image_loading` +
`move_entity_between_windows` as the async corpus.

---

## 9. Vendored internals (what they are, why ChronOS cares)

| Crate | Path | Purpose |
|---|---|---|
| `gpui_scheduler` | `Source/gpui_scheduler/` | `Task`, `BackgroundExecutor`, `LocalExecutor`, `Priority`, clocks — **drop=cancel lives here** |
| `gpui_tokio` | `Source/gpui_tokio/` | Optional Tokio runtime as GPUI global; bridge only |
| `gpui_collections` | `Source/gpui_collections/` | FxHashMap/Set, IndexMap/Set, TypeIdHashMap/Set, vecmap — hash collections used throughout gpui |
| `gpui_sum_tree` | `Source/gpui_sum_tree/` | Copy-on-write B+ tree with monoidal summaries (editor/list internals) |
| `gpui_refineable` + `gpui_derive_refineable` | `Source/gpui_refineable/` | `Refineable` trait + cascade for style/config hierarchies (`Style` refinements) |

ChronOS does not depend on these crates directly in app code; they come in
via `gpui`. Knowing them matters when reading fork panics or style cascade
behavior, not for day-to-day bar widgets.

---

## 10. Ловушки и опровержения

| Thought (wrong) | Reality | Evidence |
|---|---|---|
| `global_mut` creates the global | No — panics if missing; use `set_global` first | `app.rs:1884-1890` |
| Nested `handle.update` on same window is fine | Slot is `.take()`'d — nested update → `window not found` | `app.rs:1733` |
| `let _ = cx.background_spawn(…)` is fire-and-forget | Drop cancels; need `.detach()` | `gpui_scheduler/.../executor.rs:287-288` |
| Fork has no easing / must port Kael | `EasingCurve` already ported from Kael | `easing.rs:1-71` |
| Fork has no `requestAnimationFrame` | `Window::request_animation_frame` | `window.rs:2229` |
| `observe_global` doesn't exist / theme hot-reload impossible | Exists on App, Context, Window | `app.rs:1931`, `context.rs:176` |
| `async_*` examples ship in the fork | **They do not** — only image_loading / move_entity / … | `ls examples/` |
| `tokio::spawn_blocking` just works next to GPUI | Needs Tokio runtime; use `gpui_tokio::init` + handle | `gpui_tokio.rs:12-25, 98-99` |
| There is a built-in interval timer | Only one-shot `timer`; loop yourself | `executor.rs:162` |
| `on_click` gives `&mut App`, so view state needs a `Global` | `cx.listener` adapts `Fn(&mut T, &E, &mut Window, &mut Context<T>)` → the `&mut App` shape | `context.rs:252`, live at `volume_popup/view.rs:199` |
| `on_hover` can be attached twice to refine behavior | `debug_assert!` — second call panics in debug builds | `div.rs:622-625` |

---

## Note for ChronOS (one line)

Bar tickers and service watches should keep using `cx.spawn` +
`background_executor().timer` (or service channels) — never raw
`tokio::spawn` driving UI; and always `.detach()` or store the `Task`.
