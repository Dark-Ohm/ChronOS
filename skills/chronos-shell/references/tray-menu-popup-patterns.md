# Tray context-menu popup (DBusMenu) — gpui-fork patterns (2026-07-17)

Verified facts from building `crates/app/src/tray_menu/` (right-click tray icon
→ layer-shell `com.canonical.dbusmenu` tree popup). Use when wiring ANY
clickable layer-shell popup against `../Source/gpui` in this repo. Most of the
pain was gpui-FORK-specific element/closure traps, not tray logic.

## Tray data model recap (crates/services/src/tray/)
- `TrayItem.menu: Option<Vec<MenuNode>>` — populated by
  `TraySubscriber::dispatch(TrayCommand::FetchMenu { service })`. Before that it
  is `None`. Read it back with `AppState::tray(cx).get().find(&svc).menu`.
- `MenuNode { id: i32, label: String, enabled, visible, separator: bool,
  toggle: Option<(MenuToggleType,bool)>, children: Vec<MenuNode> }`. `label`
  is ALREADY mnemonic-stripped by the service (`strip_mnemonic`) — do NOT strip
  again in the view.
- Dispatch is SYNC here: `TraySubscriber::dispatch(TrayCommand::MenuClicked{..})`
  and `FetchMenu{..}` take `&self` (unlike `NotificationSubscriber`, whose
  `dispatch` is async and needs `background_spawn`). So in the click handler just
  call `AppState::tray(cx).dispatch(...)` directly — no spawn needed.

## Working module shape
- `TrayMenuState` : `Global` (open_service, nodes, handle, close_generation).
- `TrayMenuWatcher` entity hosts `state::watch(cx, signal, |_this, state, cx| …)`
  so a late `FetchMenu` arrival repaints the open popup.
- `open(cx, svc)`: dispatch FetchMenu, snapshot current `item.menu`, bump
  `close_generation`, open-or-repaint the window, `schedule_autoclose`.
- `toggle(cx, svc)`: re-click same service → `close`; different → `open`.
- `close(cx)`: clear state, `handle.update(cx, |_, w, _| w.remove_window())`.
- Window: `Anchor::TOP | Anchor::RIGHT`, `Layer::Overlay`,
  `exclusive_zone: None`, `keyboard_interactivity: None`, margin ~36px top.
- 15s auto-close via a generation token (see spawn trap below).

## gpui-FORK element/closure traps (all hit & fixed this session)

### 1. `on_click` / `on_mouse_down` flip `Div` → `Stateful<Div>`
`div()....on_click(move |_,_,cx| …)` has type `Stateful<Div>`, NOT `Div`. You
cannot reassign it back into a `let mut row: Div` or chain more `Div` methods
after. FIX: build the clickable and non-clickable branch each as an
`AnyElement` (via `.into_any_element()`) and select between them — collapse to
`AnyElement` exactly once at the end of the row builder. Collecting
`Vec<AnyElement>` for the menu rows also works. `use gpui::AnyElement`.

```rust
let row_elem: AnyElement = if node.enabled && !node.children.is_empty() {
    let id = node.id;
    div().w_full().flex().items_center()….cursor_pointer()
        .id(format!("tray-menu-item-{id}"))
        .on_click(move |_e, _w, cx: &mut App| click_item(cx, id))
        .child(label_div)
        .into_any_element()
} else {
    div().w_full()….child(label_div).into_any_element()
};
```

### 2. `cx.spawn` async-closure form for `App::spawn`
`App::spawn` takes a SINGLE-arg async closure: `cx.spawn(async move |app_cx:
&mut AsyncApp| { … })`. The double-closure form used by `Context<T>::spawn`
(`|this, cx| { let x = x.clone(); async move {…} }`) does NOT apply here — it
captures by borrow and yields `lifetime may not live long enough`. Move
captures in (they are `u64`/`String`, all `Copy`/owned) and use the direct
`async move |app_cx|` form. Inside, get a timer with
`app_cx.background_executor().timer(Duration::from_secs(15)).await`.

### 3. `on_mouse_down` signature + `Fn` (not `FnOnce`)
`InteractiveElement::on_mouse_down(button: MouseButton, listener: impl
Fn(&MouseDownEvent, &mut Window, &mut App) + 'static)`. Because it is `Fn` (may
be called many times), you CANNOT `move` a `String` into it — `.clone()` the
capture: `let id_right = id.clone(); … move |_,_,cx| toggle(cx, id_right.clone())`.
Import `gpui::{InteractiveElement, MouseButton}`. The handler only fires on
`DispatchPhase::Bubble` + hovered, so a right-click over the tray badge is safe.

### 4. `rounded()` takes `gpui::Pixels`, not `f32`
`theme.radius` / `theme.radius_lg` are `Pixels` (from `chronos-ui`). Passing an
`f32` to `.rounded(..)` → `mismatched types: expected f32, found Pixels`. Pass
`theme.radius` directly. Same for `px(..)` (a `const fn`, returns `Pixels`).

### 5. `Context<T>` derefs to `App`
The `watch` callback receives `&mut Context<TrayMenuWatcher>`; you can pass it
where `&mut App` is expected (e.g. `handle.update(cx, …)`, `AppState::tray(cx)`)
without reborrowing — `Context` implements `Deref<DerefMut> for App`.

### 6. `Service` trait must be in scope
`.get()` / `.subscribe()` on any subscriber require `use
chronos_services::Service;` in the file — they are trait methods, not inherent.

### 7. Inline linter is edition-2015-blind
The in-editor patch/lint harness parses async without `--edition 2024` and
reports `async move blocks are only allowed in Rust 2018 or later` on otherwise
valid code. TRUST `cargo build`/`cargo test`; ignore those lint red-lines.

## Multi-agent isolation nuance (refines the existing escape hatch)
`cargo build --workspace` builds TEST targets too. A peer's broken
`#[cfg(test)]` in `crates/services` (e.g. an in-flight `tray/menu.rs` test
fixture with an unclosed delimiter) breaks the WHOLE `--workspace` build even
though the SERVICES LIBRARY itself compiles. Verify YOUR app-crate slice with
`cargo build -p chronos` (lib-only) — it links the services lib and surfaces
YOUR real errors, skipping peers' test code. (`cargo build -p chronos-services`
likewise builds the lib and skips its own broken tests.) Only escalate to
`--workspace` at the very end; if it's still red on a peer's test, that's their
WIP — do NOT edit their `services/**` file (zone-forbidden), report to the
Architect. The skill's "Peer `examples/` breakage escape hatch" already covers
`cargo test -p chronos-services --lib`; this adds the `cargo build -p chronos`
(lib) analog for build-time.

## 8. Same fixed-size disease as notifications — apply the resize recipe
The tray-menu popup opened with a FIXED `Bounds` (e.g. 240×40) at
`open_window` time. The Lead Architect flagged this is the SAME clipping bug as
the notifications popup (HERMES №9): a deep submenu or a wide label overflows a
hardcoded surface and gets clipped by the compositor. The fix is the rubber-band
resize recipe in `references/notifications-module-patterns.md` **§8** — layer-shell
surfaces do NOT auto-size to children, so call `window.resize(Size)` from the
`watch` callback (after a `FetchMenu` repaint) to fit the rendered `MenuNode`
tree. Reuse the same `estimate_content_height`-style walk (count visible nodes,
sum per-row heights + submenu indentation) and the same display-fraction cap.
The `close_generation` token + window lifecycle here are independent of the
resize; just add the `window.resize(...)` call alongside the existing
`view_cx.notify()` in the watcher's update branch.
