# gpui fork API surface — VERIFIED facts (this repo's `../Source/gpui`)

Authoritative because the donor `reference/gpui-shell-main` tracks upstream zed
`main` (unpinned) and DRIFTS from our fork. Never assume an upstream gpui API
exists here — verify against `Source/gpui/src/*` first. Every fact below was
checked against the actual fork source on 2026-07-16 (and extended 2026-07-17).

## Colors (`Source/gpui/src/color.rs`)
- `pub fn rgb(hex: u32) -> Rgba` (line 14): `hex.to_be_bytes()`, alpha forced to
  `1.0`. So `rgb(0x1e1e2e)` = opaque `#1e1e2e`.
- `pub fn rgba(hex: u32) -> Rgba` (line 20): BE bytes, i.e. `0xRRGGBBAA`.
  - `rgba(0xffffffff)` = white, opaque. `rgba(0x00000000)` = fully transparent.
- `pub struct Hsla { h, s, l, a: f32 }` (line 334), all components 0..1.
  - derives `Default, Copy, Clone, Debug` (line 332) AND `impl PartialEq for
    Hsla` (line 374) — so `Hsla` values are directly `==`-comparable in tests.
  - `impl From<Rgba> for Hsla` (line 677) and `impl From<Hsla> for Rgba`
    (line 194) both exist.
- **`Hsla::parse_hex` DOES NOT EXIST in this fork** (upstream zed has it; this
  fork cut it). If you need to parse `"#1e1e2e"`/`"1e1e2e"`/`"1e1e2eff"`,
  write your own: strip `#`, `u32::from_str_radix(raw, 16)`, pack 6-digit to
  `0xRRGGBBff`, then `Hsla::from(rgba(packed))`. See `crates/ui/src/theme/mod.rs::parse_hex`.

### Pitfall — no implicit Rgba→Hsla coercion
`rgba(...)` returns `Rgba`. Assigning it to an `Hsla` field does NOT compile:
```rust
error[E0308]: expected `Hsla`, found `Rgba`
```
You must `.into()` it: `primary: rgba(0xffffffff).into()`. `From<Rgba>` is a
trait conversion, not an implicit one.

## Geometry / `px` (`Source/gpui/src/geometry.rs`)
- `pub struct Pixels(pub(crate) f32)` (line 2677): derives `Clone, Copy,
  Default, Add, AddAssign, Sub, ... PartialEq`, `Serialize, Deserialize,
  JsonSchema` — **NOT `Debug`**.
- `impl From<f32> for Pixels` (line 2903) exists.
- `pub const fn px(pixels: f32) -> Pixels` (line 3736) — **`px` is a crate-root
  function, not a method/auto-import.** You MUST `use gpui::px;` (or
  `gpui::px`) or it won't resolve (`error[E0425]: cannot find function px`).

### Derive-limit rule (important for theme/color structs)
- A struct of ONLY `Hsla` fields CAN derive `Debug` (Hsla has it) — e.g.
  `BgColors`, `TextColors`, `StatusColors` in `crates/ui`.
- A struct containing `Pixels` fields CANNOT derive `Debug` (Pixels lacks it)
  — e.g. `Theme`, `FontSizes`. Derive `Clone, Copy, PartialEq` only, or omit
  derives that need `Debug`.

## Globals (`Source/gpui/src/global.rs`, `app.rs`)
- `pub trait Global: 'static` (global.rs:22) — a marker trait. Implement it on
  a `'static` type to store it in gpui's global state.
- On `App` (app.rs): `global<G: Global>(&self) -> &G` (1868),
  `global_mut<G: Global>(&mut self) -> &mut G` (1884),
  `set_global<G: Global>(&mut self, global: G)` (1906).
- Pattern for a theme-as-global: `impl Global for Theme {}`, then
  `Theme::init(cx) = cx.set_global(Theme::default())`,
  `Theme::global(cx) = cx.global::<Theme>()`,
  `Theme::global_mut(cx) = cx.global_mut::<Theme>()`,
  `Theme::set(t, cx) = *cx.global_mut::<Theme>() = t`. Add an
  `ActiveTheme` trait (`fn theme(&self) -> &Theme`) impl'd for `App` so any
  `&App` can call `.theme()`.

## `on_click` requires `.id(...)` — it lives in `StatefulInteractiveElement`

`Div` implements `InteractiveElement` (base), NOT `StatefulInteractiveElement`.
The `on_click` method (and `role`, `aria_label`, hover/edit-state helpers)
is defined on `StatefulInteractiveElement` (`Source/gpui/src/elements/div.rs`:
`pub trait StatefulInteractiveElement: InteractiveElement`, `on_click` at ~1475).
So calling `.on_click(...)` directly on a `Div` fails:
```
error[E0599]: no method named `on_click` found for struct `gpui::Div` in the current scope
(and `InteractiveElement` is reported unused even though it's imported)
```
**Fix:** every clickable element must first go through `.id(...)`, which
returns `Stateful<Self>` (implements `StatefulInteractiveElement`):
```rust
use gpui::{InteractiveElement, ...};
div()
    .id(format!("notif-action-{id}-{key}"))   // -> Stateful<Div>, now has on_click
    .on_click(move |_event, _window, cx| { ... })
```
`id` takes `impl Into<ElementId>` (a `format!`/`String` works). `Stateful<Div>`
still implements `Styled`/`ParentElement`, so all the `.bg()/.child()/.rounded()`
chaining you already wrote stays valid after `.id()`. Only the clickable leaf
needs it — the surrounding `div()` wrappers don't.

## Border helpers use PREFIX `border_l`, not `border_left`

`Source/gpui/src/elements/div.rs` generates border methods from prefix ×
suffix macros (`border_style_methods!`). The left-side prefix is `border_l`
(NOT `border_left`), and suffixes are numeric width tokens `0..=8`
(mapped to `px(0.)`..`px(8.)`). So:
- `border_left_3()` → **WRONG** (`border_left` prefix doesn't exist in this fork).
- `border_l_3()` → **CORRECT** (prefix `border_l` + suffix `3` = 3px left border).
Same for every side: `border_t_3`, `border_r_2`, `border_b_1`, `border_x_2`,
`border_y_4`, or just `border_3` for all sides. Other confirmed-present
border macros: `border_color(...)`, `border_dashed()`. As always in this fork,
verify a border helper against `Source/gpui/src/**` before assuming upstream
names like `border_left_3` exist — they usually don't here.

## `Fn` click closures can't move a captured `String` into `async move`

`Div::on_click`'s listener is `Fn`, i.e. it may be CALLED MULTIPLE TIMES
and is therefore not allowed to move a captured variable out. If your closure
captures a `String` (e.g. an action key) and you move it straight into the
inner `async move { ... }` block, you get:
```
error[E0507]: cannot move out of `key`, a captured variable in an `Fn` closure
```
**Fix:** clone the value *inside* the (outer) `Fn` closure, then move the
clone into the inner `async move`:
```rust
.on_click(move |_event, _window, cx| {
    let svc = AppState::notification(cx).clone();
    let action_key = key.clone();              // clone here, while still borrowable
    cx.background_spawn(async move {
        let _ = svc.dispatch(NotificationCommand::InvokeAction(id, action_key)).await;
    });
})
```
(This is the pattern the notifications popup uses — see
`references/notifications-module-patterns.md` §2.)

## Workspace membership gotcha

A new crate under `crates/` is INVISIBLE to cargo until it's listed in the
root `Cargo.toml` `members` array. `cargo build -p <name>` fails with:
```rust
error: package ID specification `<name>` did not match any packages
```
Adding `"crates/<name>"` to `members` is REQUIRED and is NOT the same as
editing `[workspace.dependencies]` — the latter is off-limits, the former is
mandatory to make the package addressable. (See `crates/ui` added 2026-07-16.)

## Edition

Workspace is `edition = "2024"` (root `Cargo.toml`, `crates/*/Cargo.toml`).
Trust `cargo build`/`cargo test` over inline linters that may mis-parse 2024
syntax as 2015.
