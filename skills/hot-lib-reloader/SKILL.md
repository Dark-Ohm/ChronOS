---
name: hot-lib-reloader
description: >
  Use when setting up Rust hot-reload with hot-lib-reloader in a Cargo
  workspace, or diagnosing "Cannot load library function", "LibraryCopyError",
  "file does not exist", or "expected string literal" from hot-lib-reloader.
---

# Hot-lib-reloader Setup

Hot-reload Rust code at runtime without restart. Separate dylib crate with
pure functions, loaded by the main binary via `hot_module!`.

## Pitfalls (all confirmed)

**1. Dylib name: underscore, not hyphen.** `crate "chronos-hotview"` →
`libchronos_hotview.so`. Use `dylib = "chronos_hotview"`, not `"chronos-hotview"`.

**2. lib_dir must point to workspace target.** Default is
`$CARGO_MANIFEST_DIR/target/debug/` — wrong for workspace members. Specify:
```rust
lib_dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../target/debug")
```

**3. hot_functions_from_file! path is from workspace root**, not from the
file containing the macro.

**4. Generated wrappers need type imports.** Types in function signatures
must be imported inside the `hot_lib` module:
```rust
mod hot_lib {
    use gpui::{AnyElement, Hsla};
    use chronos_ui::Theme;
    hot_functions_from_file!("crates/hotview/src/lib.rs");
}
```

**5. Changing function signature requires binary rebuild.** Hot-reload
only swaps the dylib — call site changes need a restart.

**6. Workspace unsafe_code = "deny" + cdylib.** Give the dylib its own
empty `[lints]` section to skip workspace lints.

## Cargo-watch: --delay 0

Default debounce (500ms) adds ~6s. Use `--delay 0`:
```bash
cargo watch --delay 0 -w crates/hotview \
  -s 'cargo build -p chronos-hotview 2>&1 | tail -3'
```
Measured: **~2 seconds** save→update (1s compile + inotify).

## Minimal setup

**Crate (`crates/hotview/Cargo.toml`):**
```toml
[package]
name = "chronos-hotview"
edition = "2024"
[lib]
crate-type = ["cdylib", "rlib"]
[dependencies]
gpui.workspace = true
chronos-ui = { path = "../ui" }
[lints]  # empty — no workspace lints
```

**Library (`crates/hotview/src/lib.rs`):**
```rust
use gpui::{AnyElement, Hsla, div, prelude::*, px};
use chronos_ui::Theme;

#[unsafe(no_mangle)]
pub fn render_network(
    dl: &str, ul: &str,
    dot_color: Hsla, speed_color: Hsla,
    theme: &Theme,
) -> AnyElement {
    div().flex().items_center()
        .child(div().child(format!("\u{2193} {dl}")))
        .into_any_element()
}
```

**Caller (`crates/app`):**
```rust
#[cfg(feature = "hot-reload")]
#[hot_lib_reloader::hot_module(
    dylib = "chronos_hotview",
    lib_dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../target/debug")
)]
mod hot_lib {
    use gpui::{AnyElement, Hsla};
    use chronos_ui::Theme;
    hot_functions_from_file!("crates/hotview/src/lib.rs");
}

#[cfg(feature = "hot-reload")]
{ hot_lib::render_network(&dl, &ul, dot_color, speed_color, &theme) }
#[cfg(not(feature = "hot-reload"))]
{ /* static fallback */ }
```

## Error reference

| Error | Cause | Fix |
|-------|-------|-----|
| `LibraryCopyError: file "..." does not exist` | Wrong `lib_dir` | Path from `CARGO_MANIFEST_DIR` to workspace target |
| `Cannot load library function: LibraryNotLoaded` | Wrong dylib name | `chronos_hotview` not `chronos-hotview` |
| `expected string literal` in `hot_functions_from_file!` | Used `concat!()` | Plain string, path from workspace root |
| `the name X is defined multiple times` | `pub use` + generated wrapper | Remove `pub use`, keep only `hot_functions_from_file!` |
| Compile error doesn't crash app | Expected | hot-lib-reloader keeps last working .so |
