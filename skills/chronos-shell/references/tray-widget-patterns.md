# Tray bar widget patterns (icon list)

Covers `crates/app/src/bar/widgets/tray.rs` — the bar's live icon list of
`StatusNotifierItem`s (NOT the `tray_menu/` right-click popup; that's a
different module, see `tray-menu-popup-patterns.md`).

## Render purity — hard line

`BarWidget::render()` fires **every frame** (bar redraws each second via the
clock ticker AND on every service signal). Therefore:

- ZERO side effects inside `render()` — no state mutation, no IO, no cache
  writes that aren't already thread-local read-through.
- ZERO per-call allocations without a cache — icon-theme resolution and pixmap
  `RenderImage` builds MUST go through the existing `thread_local!` caches
  (`ICON_CACHE`, `PIXMAP_CACHE`), which are already in the file.
- Factor ALL list-shaping logic (filter / dedupe / cap / transform) into PURE
  `pub fn`s that take `&TrayState` (or `&[&TrayItem]`) and return data. `render()`
  only calls them and maps to elements. This makes the logic unit-testable on
  fixtures WITHOUT a live D-Bus / Wayland session.

Concrete shape (Hermes №16, 2026-07-20):

```rust
pub fn is_useful(item: &TrayItem) -> bool { /* icon name/pixmap OR non-empty title */ }
pub fn bus_name(id: &str) -> &str { /* owner key for dedupe */ }
pub fn dedupe_by_bus<'a>(items: &[&'a TrayItem]) -> Vec<&'a TrayItem> { /* first per owner wins */ }
pub fn apply_cap<'a>(items: &[&'a TrayItem], max: usize) -> (Vec<&'a TrayItem>, usize) { /* keep max, return overflow */ }
pub fn prepare_tray_items<'a>(state: &'a TrayState, max: usize) -> PreparedTray<'a> {
    let useful: Vec<&TrayItem> = state.items.iter().filter(|i| is_useful(i)).collect();
    let deduped = dedupe_by_bus(&useful);
    let (visible, overflow) = apply_cap(&deduped, max);
    PreparedTray { visible, overflow }
}
```

`render()` then does `let prepared = prepare_tray_items(&state, MAX_TRAY_ITEMS);`
and iterates `prepared.visible`. Test with a `mk_item(id, title, icon)` helper
building `TrayItem` + `TrayState` fixtures — no bus needed.

## Clutter defence (filter → dedupe → cap, in that order)

Symptom this fixed: Vivaldi/Chromium registers many anonymous
`StatusNotifierItem`s (`:1.75/org/chromium/StatusNotifierItem/15`, …) with
`icon=None` + `title=""` — they render as blank glyphs and the bar becomes a
"picket fence" of microphones. Root cause is upstream (Chromium doesn't
unregister); the fix is on OUR side (per user decision).

1. **Filter anonymous** (`is_useful`): keep an item only if it has an icon
   (`icon_name` non-empty OR `icon_pixmap` present) OR a non-empty `title`.
   The SERVICE keeps the full bus truth (needed for menus/debug) — filter only
   in the WIDGET.
2. **Dedupe by bus owner** (`bus_name` + `dedupe_by_bus`): multiple items from
   one bus name (`:1.75`) → one icon. First useful item per owner wins; order
   preserved (newest last, per `TrayState`).
3. **Cap** (`MAX_TRAY_ITEMS = 8`): keep at most N; show a compact `+N` overflow
   badge (same visual language as bell/updates: `theme.font_mono`,
   `theme.font_sizes.sm`, `theme.text.muted`) ONLY when `overflow > 0`.

### `bus_name` split rule (VERIFIED)

```rust
pub fn bus_name(id: &str) -> &str {
    let no_path = id.split('/').next().unwrap_or(id);   // strip /Menu etc.
    if let Some(idx) = no_path.find('-') {
        if no_path.starts_with(':') { no_path }         // ":1.75" -> ":1.75" (unique name, never split)
        else { &no_path[..idx] }                         // "org.kde.StatusNotifierItem-1234-1" -> "org.kde.StatusNotifierItem"
    } else { no_path }
}
```

- Unique bus names `:N.M` have NO dash → kept whole (correct: each is one owner).
- Well-known names `org.kde.StatusNotifierItem-1234-1` → prefix before the
  dash-instance suffix, so multiple instances of one app collapse together.

### Escalation guard

If the "anonymous" items ACTUALLY have an icon (i.e. `icon=None` in our log was
OUR resolver bug, not Chromium's silence), then filtering masks a real resolver
defect. STOP, report the finding, don't paper over it with the filter. In №16
the `icon=None` was confirmed fact (Architect's busctl dump), so filtering was
correct — but verify the fact before trusting the filter.

## GPUI gotcha — `Vec<AnyElement>` needs `.into_any_element()`

When pushing a `div()` into a `Vec<AnyElement>` (overflow badge, extra chip),
the element MUST end with `.into_any_element()`:

```rust
badges.push(
    div()
        .id("tray-overflow")
        .font_family(theme.font_mono)
        .text_color(theme.text.muted)
        .child(format!("+{}", prepared.overflow))
        .into_any_element(),   // REQUIRED — without it: E0308 expected `AnyElement`, found `Stateful<Div>`
);
```

`on_click`/`on_mouse_down` already flip `Div`→`Stateful<Div>`; if you collect
those into `AnyElement` via `.into_any_element()` at the end of the `.map()`,
keep the overflow/extra pushes consistent (also `.into_any_element()`).

## Isolation for verification

`render()` and the pure fns need no live session — unit-test them directly. But
the WHOLE workspace build is frequently red from a PEER's concurrent WIP (another
agent's broken `services`/`examples`/`mod.rs`). Verify YOUR slice in a
`git worktree` sibling at clean HEAD (see the "Variant B" multi-agent isolation
section in SKILL.md) — copy ONLY your `tray.rs` in, `cargo test -p chronos tray`
there. Don't patch peers' files to make the build green.
