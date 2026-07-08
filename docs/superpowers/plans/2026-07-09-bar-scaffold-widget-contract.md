# Bar Scaffold & Widget Contract — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Refactor the single-`div` bar into a 3-section scaffold with an object-safe `BarWidget` runtime contract and a global registry, so future widgets (Rust or LuaU) plug in without recompiling the core. The bar renders as a plain strip (registry empty) today.

**Architecture:** The bar module (`crates/app/src/bar/`) exposes a dyn-safe `trait BarWidget` and a `BarWidgetRegistry` stored as a GPUI global. `Bar::render` reads the registry and lays widgets into `Left`/`Center`/`Right` flex sections. Registration is the single seam; no concrete widgets, config file, or LuaU runtime are built.

**Tech Stack:** Rust, GPUI (gpui-ce, pinned rev `20340e14874a3b55122e5cb2aa0d023874e08b2d`), `tracing`. No new crates.

## Global Constraints

- `panic = "unwind"` — a panic in any listener/thread must not kill the shell (workspace `Cargo.toml` profile).
- gpui is pinned to local path `/home/neo/Projects/SOURCE/gpui/gpui-ce-main/crates/gpui` (rev `20340e14874a3b55122e5cb2aa0d023874e08b2d`) — do NOT bump.
- Object-safe `BarWidget` trait + runtime registry. NO `enum Widget` + `match` dispatch (ARCHITECTURE.md §6).
- Registry is a GPUI global; all bar windows read the same registry (one widget set across monitors).
- `BarWidget::render(&self, window: &mut Window, cx: &App) -> AnyElement` — immutable `cx` (see spec §4 rationale).
- YAGNI: no bar config file, no blur/borders, no concrete widgets, no `crates/luau`, no hot-reload.
- Reconciliation note: ARCHITECTURE.md §6 mentions `HashMap<String, Box<dyn BarWidget>>`; this plan uses `Vec<Box<dyn BarWidget>>` because widget ORDER within a section is significant for layout and `HashMap` is unordered. Name-keyed lookup is not needed at scaffold stage; revisit only if named widget replacement is required.
- Work in the existing worktree `feat/bar-scaffold-widget-contract` (`.worktrees/feat-bar-scaffold-widget-contract`). Commit frequently, one commit per task.

---

## File Structure

```
crates/app/src/bar/
  mod.rs        # Bar view (render 3 sections), bar::init(), section_div(), window_options()
  widget.rs     # trait BarWidget + struct BarWidgetRegistry (register / widgets_for / Global)
  sections.rs   # enum BarSection + BAR_HEIGHT / BAR_COLOR defaults
crates/app/src/bar.rs   # DELETED (replaced by bar/ module dir)
crates/app/src/main.rs  # UNCHANGED (mod bar; + bar::init stay the same)
```

Each file has one responsibility. `mod.rs` owns window/rendering, `widget.rs` owns the contract + registry, `sections.rs` owns layout constants. `main.rs` is untouched (Rust resolves `mod bar;` to `bar/mod.rs` automatically once `bar.rs` is deleted).

---

## Task 1: `sections.rs` — BarSection enum + layout constants

**Files:**
- Create: `crates/app/src/bar/sections.rs`

**Interfaces:**
- Produces: `pub enum BarSection { Left, Center, Right }`, `pub const BAR_HEIGHT: f32 = 32.0`, `pub const BAR_COLOR: u32 = 0x1e1e2e` (consumed by Task 2 `widget.rs` and Task 3 `mod.rs`).

- [ ] **Step 1: Write the module**

```rust
// crates/app/src/bar/sections.rs
//! Bar layout sections and default geometry constants.

/// Which horizontal section a widget renders into.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BarSection {
    Left,
    Center,
    Right,
}

/// Bar thickness in logical pixels. Mirrors the previous hard-coded constant.
pub const BAR_HEIGHT: f32 = 32.0;

/// Bar background color (0xRRGGBB). Mirrors the previous hard-coded constant.
pub const BAR_COLOR: u32 = 0x1e1e2e;
```

- [ ] **Step 2: Build to confirm it compiles in isolation**

Run: `cargo build -p chronos 2>&1 | tail -20`
Expected: compiles (may warn about unused `mod` if not yet wired — that is fixed in Task 3). If `crates/app` is the default member, `cargo build` works too.

- [ ] **Step 3: Commit**

```bash
git add crates/app/src/bar/sections.rs
git commit -m "feat(bar): add BarSection enum and default geometry constants"
```

---

## Task 2: `widget.rs` — BarWidget trait + registry (TDD)

**Files:**
- Create: `crates/app/src/bar/widget.rs`
- Test: inline `#[cfg(test)] mod tests` in `widget.rs`

**Interfaces:**
- Produces: `pub trait BarWidget: 'static { fn section(&self) -> BarSection { Left } fn render(&self, window: &mut Window, cx: &App) -> AnyElement; }`, `pub struct BarWidgetRegistry { widgets: Vec<Box<dyn BarWidget>> }`, `impl Global for BarWidgetRegistry {}`, `BarWidgetRegistry::register(&mut self, widget: Box<dyn BarWidget>)`, `BarWidgetRegistry::widgets_for(&self, section: BarSection) -> impl Iterator<Item = &dyn BarWidget>`. Consumed by Task 3 `mod.rs`.

- [ ] **Step 1: Write the failing test**

```rust
// crates/app/src/bar/widget.rs
use gpui::{AnyElement, App, Global, Window};

use crate::bar::sections::BarSection;

pub trait BarWidget: 'static {
    fn section(&self) -> BarSection {
        BarSection::Left
    }
    fn render(&self, _window: &mut Window, _cx: &App) -> AnyElement;
}

pub struct BarWidgetRegistry {
    widgets: Vec<Box<dyn BarWidget>>,
}

impl Default for BarWidgetRegistry {
    fn default() -> Self {
        Self {
            widgets: Vec::new(),
        }
    }
}

impl Global for BarWidgetRegistry {}

impl BarWidgetRegistry {
    pub fn register(&mut self, widget: Box<dyn BarWidget>) {
        self.widgets.push(widget);
    }

    pub fn widgets_for(&self, section: BarSection) -> impl Iterator<Item = &dyn BarWidget> {
        self.widgets
            .iter()
            .filter(move |w| w.section() == section)
            .map(|w| w.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::div;

    struct FakeWidget {
        section: BarSection,
    }

    impl BarWidget for FakeWidget {
        fn section(&self) -> BarSection {
            self.section
        }
        fn render(&self, _window: &mut Window, _cx: &App) -> AnyElement {
            div().into_any_element()
        }
    }

    #[test]
    fn register_then_filter_by_section() {
        let mut registry = BarWidgetRegistry::default();
        registry.register(Box::new(FakeWidget { section: BarSection::Left }));
        registry.register(Box::new(FakeWidget { section: BarSection::Right }));

        let left: Vec<&dyn BarWidget> = registry.widgets_for(BarSection::Left).collect();
        assert_eq!(left.len(), 1);
        assert_eq!(left[0].section(), BarSection::Left);

        let right: Vec<&dyn BarWidget> = registry.widgets_for(BarSection::Right).collect();
        assert_eq!(right.len(), 1);

        let center: Vec<&dyn BarWidget> = registry.widgets_for(BarSection::Center).collect();
        assert_eq!(center.len(), 0);
    }

    #[test]
    fn default_section_is_left() {
        struct Plain;
        impl BarWidget for Plain {
            fn render(&self, _w: &mut Window, _c: &App) -> AnyElement {
                div().into_any_element()
            }
        }
        let mut registry = BarWidgetRegistry::default();
        registry.register(Box::new(Plain));
        // Plain relies on the default trait method -> Left
        assert_eq!(registry.widgets_for(BarSection::Left).count(), 1);
        assert_eq!(registry.widgets_for(BarSection::Center).count(), 0);
    }
}
```

- [ ] **Step 2: Run the test to verify it compiles & passes**

Run: `cargo test -p chronos widget 2>&1 | tail -30`
Expected: PASS (2 tests). If `bar` module is not yet wired into `main.rs`/`mod.rs`, the file may not be compiled — that is wired in Task 3; for now compile-check with `cargo build -p chronos` and the test will run once Task 3 connects the module. If you need to run the test before Task 3, temporarily add `mod widget; mod sections;` to `crates/app/src/main.rs` under a `#[cfg(test)]` or just proceed to Task 3 and run there.

- [ ] **Step 3: Commit**

```bash
git add crates/app/src/bar/widget.rs
git commit -m "feat(bar): add object-safe BarWidget trait and global registry"
```

---

## Task 3: `mod.rs` — Bar view renders 3 sections + global bootstrap

**Files:**
- Create: `crates/app/src/bar/mod.rs`
- Modify: (implicit) `crates/app/src/main.rs` already has `mod bar;` and calls `bar::init(cx)` — no change required; `mod bar;` now resolves to `bar/mod.rs`.

**Interfaces:**
- Consumes: `BarSection`, `BAR_HEIGHT`, `BAR_COLOR` (Task 1); `BarWidget`, `BarWidgetRegistry` (Task 2).
- Produces: `pub fn init(cx: &mut App)` (signature unchanged from original `bar.rs`), `struct Bar` implementing `Render`.

- [ ] **Step 1: Delete legacy `crates/app/src/bar.rs`**

Rust cannot resolve `mod bar;` to both `bar.rs` and `bar/mod.rs` — having both is a compile error. Remove the old file before creating the module dir.

Run: `git rm crates/app/src/bar.rs`

(The deletion was originally scheduled for Task 4; it must happen here so `bar/mod.rs` can exist. Task 4 becomes verification-only.)

- [ ] **Step 2: Write `bar/mod.rs`**

```rust
// crates/app/src/bar/mod.rs
mod sections;
mod widget;

use std::time::Duration;

use gpui::{
    div, px, rgb, AnyElement, Bounds, Context, DisplayId, Global, Layer, LayerShellOptions,
    Anchor, KeyboardInteractivity, Render, Size, Window, WindowBackgroundAppearance,
    WindowBounds, WindowKind, WindowOptions, point, prelude::*,
};
use gpui::layer_shell::*;

use sections::{BarSection, BAR_COLOR, BAR_HEIGHT};
use widget::{BarWidget, BarWidgetRegistry};

struct Bar;

impl Render for Bar {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let registry = cx.global::<BarWidgetRegistry>();
        let left: Vec<AnyElement> = registry
            .widgets_for(BarSection::Left)
            .map(|w| w.render(window, cx))
            .collect();
        let center: Vec<AnyElement> = registry
            .widgets_for(BarSection::Center)
            .map(|w| w.render(window, cx))
            .collect();
        let right: Vec<AnyElement> = registry
            .widgets_for(BarSection::Right)
            .map(|w| w.render(window, cx))
            .collect();

        div()
            .size_full()
            .bg(rgb(BAR_COLOR))
            .flex()
            .items_center()
            .child(section_div(BarSection::Left, left))
            .child(section_div(BarSection::Center, center))
            .child(section_div(BarSection::Right, right))
    }
}

/// Wrap a section's widgets in a flex container aligned per section.
fn section_div(section: BarSection, widgets: Vec<AnyElement>) -> AnyElement {
    match section {
        BarSection::Left => div()
            .flex()
            .flex_1()
            .justify_start()
            .gap(px(8.))
            .children(widgets)
            .into_any_element(),
        BarSection::Center => div()
            .flex()
            .flex_none()
            .justify_center()
            .gap(px(8.))
            .children(widgets)
            .into_any_element(),
        BarSection::Right => div()
            .flex()
            .flex_1()
            .justify_end()
            .gap(px(8.))
            .children(widgets)
            .into_any_element(),
    }
}

/// Returns window options for a top-anchored bar on the given display.
fn window_options(display_id: Option<DisplayId>, cx: &App) -> WindowOptions {
    let display_size = display_id
        .and_then(|id| cx.find_display(id))
        .or_else(|| cx.primary_display())
        .map(|display| display.bounds().size)
        .unwrap_or_else(|| Size::new(px(1920.), px(1080.)));

    WindowOptions {
        display_id,
        titlebar: None,
        window_bounds: Some(WindowBounds::Windowed(Bounds {
            origin: point(px(0.), px(0.)),
            size: Size::new(display_size.width, px(BAR_HEIGHT)),
        })),
        app_id: Some("chronos-bar".to_string()),
        window_background: WindowBackgroundAppearance::Transparent,
        kind: WindowKind::LayerShell(LayerShellOptions {
            namespace: "bar".to_string(),
            layer: Layer::Top,
            anchor: Anchor::LEFT | Anchor::RIGHT | Anchor::TOP,
            exclusive_zone: Some(px(BAR_HEIGHT)),
            margin: None,
            keyboard_interactivity: KeyboardInteractivity::None,
            ..Default::default()
        }),
        ..Default::default()
    }
}

fn open_on_display(display_id: Option<DisplayId>, cx: &mut App) -> bool {
    match cx.open_window(window_options(display_id, cx), move |_, cx| cx.new(|_| Bar)) {
        Ok(_) => true,
        Err(err) => {
            tracing::warn!("Failed to open bar window: {}", err);
            false
        }
    }
}

/// Opens one bar window per display and installs the empty widget registry.
/// Called once at startup from `main.rs`.
pub fn init(cx: &mut App) {
    cx.set_global(BarWidgetRegistry::default());

    cx.spawn(async move |cx| {
        // Small delay to allow Wayland to enumerate displays.
        cx.background_executor()
            .timer(Duration::from_millis(100))
            .await;

        let _ = cx.update(|cx: &mut App| {
            let displays = cx.displays();
            if displays.is_empty() {
                tracing::info!("No displays found, opening bar on default display");
                open_on_display(None, cx);
            } else {
                tracing::info!("Opening bar on {} displays", displays.len());
                for d in displays {
                    open_on_display(Some(d.id()), cx);
                }
            }
        });
    })
    .detach();
}
```

Note: `cx.global()` in `render` borrows `cx` immutably; `w.render(window, cx)` takes `&App` (immutable), so the two immutable borrows coexist — no borrow conflict. `window` is passed through as `&mut Window`, which is fine across sequential widget calls.

- [ ] **Step 3: Run the registry unit tests now that the module is wired**

Run: `cargo test -p chronos widget 2>&1 | tail -30`
Expected: PASS (2 tests from Task 2).

- [ ] **Step 3: Commit**

```bash
git add crates/app/src/bar/mod.rs
git commit -m "feat(bar): render 3 sections from global registry, bootstrap in init()"
```

---

## Task 4: Remove legacy `bar.rs`, verify build + tests green

**Files:**
- Delete: `crates/app/src/bar.rs`

**Interfaces:**
- Consumes: everything from Tasks 1–3. No new public API.

- [ ] **Step 1: Confirm legacy `bar.rs` is gone**

The old `crates/app/src/bar.rs` was deleted in Task 3 (Rust forbids `bar.rs` and `bar/mod.rs` coexisting). Verify it is absent:

Run: `ls crates/app/src/bar.rs 2>&1 || echo "bar.rs absent (good)"`
Expected: `bar.rs absent (good)`.

- [ ] **Step 2: Full build**

Run: `cargo build -p chronos 2>&1 | tail -30`
Expected: compiles with no errors. Warnings allowed.

- [ ] **Step 3: Full test run**

Run: `cargo test -p chronos 2>&1 | tail -30`
Expected: all tests PASS (the 2 registry tests). No panics.

- [ ] **Step 4: Sanity-check the bar still opens (optional, needs Wayland display)**

Run: `cargo run -p chronos` (under Hyprland). Expected: a plain bar strip on every monitor, no crash. If no display is available, rely on the build + test steps above.

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "refactor(bar): drop legacy bar.rs, module scaffold complete"
```

---

## Self-Review Notes (run against spec)

- **Spec coverage:** §3 file structure → Tasks 1–3 (+ Task 4 deletion). §4 `BarWidget` (`&self`, `section`, `render(&self, &mut Window, &App)`) → Task 2. §5 registry (`register`, `widgets_for`, `Global`, default empty) → Task 2. §6 3-section render + `section_div` + constants moved → Task 3. §7 `register` seam → Task 2/3. §8 error handling (empty registry safe) → Task 3 default global. §9 test → Task 2. §10/§11 YAGNI + acceptance → all tasks avoid those scopes; acceptance criteria 1–5 satisfied across tasks.
- **Placeholder scan:** no TBD/TODO; all steps contain concrete code or exact commands.
- **Type consistency:** `BarSection` (Task 1) used identically in Task 2 (`widgets_for` param, `section()` return) and Task 3 (`section_div`, `widgets_for` calls). `BarWidgetRegistry::register(Box<dyn BarWidget>)` and `widgets_for(...) -> impl Iterator<Item = &dyn BarWidget>` stable across tasks. `Bar::render` passes `cx` as `&App` to `render` — matches the trait signature fixed in the spec.
- **One deliberate deviation recorded:** `Vec` instead of `HashMap` (see Global Constraints) — justified by layout ordering.
