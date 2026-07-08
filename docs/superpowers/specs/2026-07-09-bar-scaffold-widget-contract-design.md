# Chronos Bar — Scaffold & Widget Contract (Design Spec)

> Status: approved design (brainstorming complete)
> Date: 2026-07-09
> Scope: bar module scaffold + `BarWidget` runtime-contract. No concrete widgets, no LuaU runtime, no config file.
> Branch: `feat/bar-scaffold-widget-contract` (worktree: `.worktrees/feat-bar-scaffold-widget-contract`)
> Relation: implements `ARCHITECTURE.md` §6 (runtime module registry) for the bar.

## 1. Goal

Turn the current bar — a single `div` with a background color (`crates/app/src/bar.rs`, 81 lines, no content) — into a **scaffold ready to accept widgets without recompiling the core**:

- A stable, object-safe `BarWidget` contract.
- A runtime registry (not `enum Widget` + `match` like the `gpui-shell` reference) so widgets plug in dynamically.
- A 3-section layout (left / center / right) rendered as a horizontal strip.
- A registration API that both future Rust widgets and the future LuaU adapter (`crates/luau`) call.

The bar renders as a **plain strip** today (registry empty). This session designs the *structure*, not the content.

## 2. Current state (verified in source)

- `crates/app/src/bar.rs`: `struct Bar; impl Render` returns `div().size_full().bg(rgb(BAR_COLOR))`. Constants `BAR_HEIGHT = 32.0`, `BAR_COLOR = 0x1e1e2e`.
- `crates/app/src/main.rs`: `bar::init(cx)` opens one layer-shell window per display (`cx.displays()`), top-anchored, `Layer::Top`, `exclusive_zone = BAR_HEIGHT`. Logic unchanged by this design.
- Reference `reference/gpui-shell/crates/app/src/bar/modules/` uses `trait BarWidget: Sized` (chrome only) + `enum Widget` (dispatch). **We do NOT copy the enum** — we make the trait itself dyn-safe and the dispatch mechanism.

## 3. File structure

Refactor `crates/app/src/bar.rs` into a module `crates/app/src/bar/`:

```
crates/app/src/bar/
  mod.rs        # Bar view, bar::init(), global registration bootstrap
  widget.rs     # trait BarWidget + struct BarWidgetRegistry (cx.global)
  sections.rs   # enum BarSection, layout constants (defaults, no TOML)
```

`crates/app/src/main.rs` keeps `mod bar;` and `bar::init(cx)`; no signature change to `init`.

## 4. `BarWidget` contract (object-safe)

```rust
use gpui::{AnyElement, App, Window};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BarSection { Left, Center, Right }

pub trait BarWidget: 'static {
    /// Which section the widget renders into. Default left.
    fn section(&self) -> BarSection { BarSection::Left }

    /// Produce the widget's element. Called every frame.
    /// `&self` (not `Context<Self>`) keeps the trait object-safe and lets the
    /// bar render widgets through a shared `&dyn BarWidget` borrow from the
    /// immutable registry. Stateful reactivity later uses interior mutability
    /// (an `Entity`, `Mutex`, or a global `AppState`), not `&mut self`.
    fn render(&self, window: &mut Window, cx: &mut App) -> AnyElement;
}
```

Rationale:
- `&self` + `AnyElement` (a `Sized` type) → object-safe. `Context<Self>` is excluded on purpose; using `&mut App` for global access keeps dispatch dynamic. `&self` (not `&mut self`) is required so the bar can call `render` through a shared `&dyn BarWidget` borrowed from the immutable registry during `Bar::render`.
- Reactive widgets later hold an `Entity` or read a global `AppState` internally; the contract does not require that today.

## 5. Registry (runtime, global)

```rust
pub struct BarWidgetRegistry {
    widgets: Vec<Box<dyn BarWidget>>,
}

impl BarWidgetRegistry {
    pub fn register(&mut self, widget: Box<dyn BarWidget>);
    pub fn widgets_for(&self, section: BarSection) -> impl Iterator<Item = &dyn BarWidget>;
}

impl Global for BarWidgetRegistry {}
```

- Stored as a GPUI global (`cx.set_global(BarWidgetRegistry::default())` in `bar::init` bootstrap, before windows open). All bar windows read the same registry → one widget set across all monitors.
- `register` pushes onto the `Vec`. `widgets_for` filters by `section()`.
- Default registry is empty → no panic on an empty bar.

## 6. Bar render (3 sections)

`Bar::render` reads `cx.global::<BarWidgetRegistry>()`, lays out a horizontal flex:

- `Left` — `flex-1` container, content `justify_start` (flush-left).
- `Center` — `flex-none` container, content centered (middle flex child).
- `Right` — `flex-1` container, content `justify_end` (flush-right).

Implementation sketch:

```rust
impl Render for Bar {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let registry = cx.global::<BarWidgetRegistry>();
        let left: Vec<AnyElement>    = registry.widgets_for(BarSection::Left).map(|w| w.render(window, cx)).collect();
        let center: Vec<AnyElement>  = registry.widgets_for(BarSection::Center).map(|w| w.render(window, cx)).collect();
        let right: Vec<AnyElement>   = registry.widgets_for(BarSection::Right).map(|w| w.render(window, cx)).collect();

        div().size_full().flex().items_center()
            .child(section_div(BarSection::Left, left))
            .child(section_div(BarSection::Center, center))
            .child(section_div(BarSection::Right, right))
    }
}
```

`section_div` applies the appropriate `flex`/`justify` per section. With an empty registry all three sections are empty → the bar is a plain strip (the target state for this session).

Constants `BAR_HEIGHT`, `BAR_COLOR` move into `sections.rs` as defaults (no config file yet).

## 7. Registration API & LuaU seam

`BarWidgetRegistry::register(Box<dyn BarWidget>)` is the single entry point:

- Future Rust widgets call it at startup after `bar::init`.
- The future LuaU adapter (`crates/luau`) wraps a LuaU render callback in a thin `Box<dyn BarWidget>` and calls `register` — **this crate is NOT built in this session**; only the clean seam is guaranteed.

## 8. Error handling

- Empty registry → valid render, no panic.
- `register` is infallible (push).
- No config parsing yet → no config-error paths.

## 9. Testing

- Unit test (`crates/app/src/bar/widget.rs` `#[cfg(test)]`): within `App::new()`, register a fake `BarWidget` (returns a fixed `div()`), assert `section()` default and that `render()` yields a non-empty `AnyElement`.
- Build check: `cargo build` in worktree must pass (baseline).

## 10. Out of scope (YAGNI)

- Bar config file / TOML hot-reload.
- Blur, borders, transparency tuning, per-monitor theming.
- Concrete widgets (clock, workspaces, tray, battery, mpris, etc.).
- `crates/luau` runtime, per-plugin VM, sandbox, manifest.
- Hot-reload of widgets.
- `dock` / `launcher` / `notifications` / `osd` modules.

## 11. Acceptance criteria

1. `cargo build` passes in the worktree.
2. Bar still renders as a plain strip on every monitor (no visual regression).
3. `BarWidget` trait + `BarWidgetRegistry` global exist; `register` adds a widget that subsequently renders in the correct section.
4. Unit test for registry + fake widget passes.
5. Design doc committed on `feat/bar-scaffold-widget-contract`.
