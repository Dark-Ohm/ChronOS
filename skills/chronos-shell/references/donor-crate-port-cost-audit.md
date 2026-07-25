# Donor crate port-cost audit — gpui fork ← kael-main

Verified facts + reusable methodology for assessing whether to port a donor
crate from `ChronOS/reference/kael-main/crates/*` into the gpui Rust fork at
`Source/gpui` (the ChronOS UI fork). Read-only by default; produce a cost
report before any port.

> Scope note: this is a SEPARATE track from `chronos-donor-port`, which covers
> the C++ ggml `donors/*` → Chronos-Engine git-apply work under HERMES.md. The
> gpui fork is Rust, uses no git-apply, and has no two-executor protocol. Don't
> cross-apply that skill's modes here.

## Methodology (5 steps, all read-only)
1. **Standalone vs coupled:** read the donor crate's `Cargo.toml`. If there is
   NO `[dependencies]` block, it's standalone (portable as-is). If it lists
   `kael` / `kael-core`, it drags the framework.
2. **Storage format / codegen:** read `build.rs`. Look for `include_str!`
   (assets compiled into the binary), an atlas generator, or `serde` data
   models. This decides assets-only vs whole-crate.
3. **Dependency surface of the target file:** read the `use crate::{…}` block at
   the top of the file. Classify each import: IME/composition? clipboard?
   rope/editor buffer? selection model? `WrappedLine`/text shaping?
   undo/history (`local_history`, `undo_manager`)? Build a fork-primitive gap
   matrix (see below).
4. **Verify each fork primitive exists:** `search_files` the fork for the
   symbol's DECLARATION (not just usage). Record PRESENT / MISSING per symbol.
   MISSING = you must port that donor module too, or reimplement.
5. **Judge + cost:** produce S/M/L/XL per path (assets-only, whole-crate,
   improve-hand-rolled) and a clear recommend. Flag any claim you can't confirm.

## Verified fork (Source/gpui) vs kael facts — 2026-07-16 audit

### `kael_icons` — standalone, assets-only port = S
- `kael_icons/Cargo.toml`: **NO `[dependencies]`** (23-line file, workspace
  inherited only). Confirmed standalone.
- `build.rs` codegens `OUT_DIR/generated_icon_catalog.rs`: an `IconName` enum +
  `include_str!("<abs svg path>")` per icon (`build.rs:157-167`). SVGs are
  compiled into the binary, not loaded at runtime.
- Only **4 Lucide SVGs** ship today: `check`, `chevron_left`, `chevron_right`,
  `close` (icons/*.svg). Each uses `fill="currentColor"` + `viewBox`
  (validated in `build.rs:85-106`).
- `weight.rs` `IconWeight` is a stroke-width enum only (no per-weight SVG) — the
  fork's `paint_svg` has no stroke-width param, so the weight knob can't be
  honored as-is.
- `kael` depends on `kael_icons` only as an OPTIONAL feature
  (`kael/Cargo.toml:46,220-222`); `text_input.rs` does NOT use it.

### Fork `svg()` element — PRESENT, accepts inline bytes (KEY BRIDGE)
- `Source/gpui/src/elements/svg.rs` exists. `svg()` (`:20`) → `path()`/`external_path()`.
- `window.paint_svg(bounds, path: SharedString, data: Option<&[u8]>,
  transformation, color: Hsla, cx)` (`window.rs:4102-4110`).
- When `data = Some(bytes)`, it renders **inline SVG bytes directly**
  (`svg_renderer.rs:221-224` → `render_alpha_mask` → `render_pixmap(bytes)`).
  So you can `svg().external_path(...)` with bundled asset bytes, or add a
  `svg().data(&str)` builder (~10 LOC) to feed Lucide strings without touching
  the filesystem. Confirms assets-only icon port is feasible with ZERO new deps.

### kael `text_input` — 2987 LOC, port = L (drags history module)
- `crates/kael/src/elements/text_input.rs`: **2987 LOC**, single file.
- Imports (`text_input.rs:1-19`): IME via `EntityInputHandler` impl
  (`:1818-1877`, real `marked_text_range`/`replace_text_in_range` — not stubs),
  clipboard `cx.read/write_from_clipboard` (`:1555-1579`), grapheme handling via
  `unicode_segmentation` (`:18`), `WrappedLine`/`TextRun`/`UnderlineStyle`,
  `UTF16Selection`. It is a single-style field, NOT a rich editor (imports
  nothing from `rich_text`).
- **Depends on `local_history::WindowValueHistory`** (`:1,285,622`) and calls
  `window.undo_manager()` (`:614`). **These are MISSING in the gpui fork** —
  porting text_input therefore also requires `local_history.rs` (~243 LOC).

### Fork primitives — PRESENT (verified)
| kael text_input needs | fork location |
|---|---|
| `ElementInputHandler`, `EntityInputHandler`, IME methods | `input.rs:100-117`; `platform.rs:1366,1494,1577` |
| `read/write_from_clipboard` | `app.rs:1334,1349`; `platform.rs:259-260` |
| `WrappedLine` + `wrap_boundaries()` | `text_system/line.rs:257`; `line_layout.rs:266` |
| `unicode-segmentation` | `Cargo.toml:122` (`unicode-segmentation = "1.10"`), `gpui/Cargo.toml:155` |
| `with_element_state` / `current_view` | `window.rs:3562` / `window.rs:4394` |
| `paint_svg` (inline bytes) | `window.rs:4102` |

### Fork primitives — MISSING
- `Window::undo_manager()` — no declaration in `Source/gpui/src`.
- `WindowValueHistory` / `local_history` — kael-internal (`kael/src/elements/local_history.rs:243`);
  not in the fork.

## Launcher hand-rolled input (the comparison target)
- `ChronOS/crates/app/src/launcher/view.rs` — 173 LOC; `handle_key`
  (`:59-107`) on a single root `on_key_down` (`:126`).
- Handles Escape/Enter/Up/Down/Tab/Backspace (`pattern.pop()`) + printable chars
  via `keystroke.key_char` push (`:93-105`, gated on alt/ctrl/platform).
- Lacks: cursor position, in-field editing, selection, IME/multilingual,
  clipboard paste, undo, left/right/home/end. `String::pop` is byte-unsafe on
  multi-byte; never uses `ElementInputHandler` so no OS IME reaches it.
- It's a single-line fuzzy filter — overkill target for a 2987-LOC port.

## Cost taxonomy from this audit
| Path | Cost | Verdict |
|---|---|---|
| `kael_icons` assets-only (copy SVGs + small `svg().data()` wrapper) | **S** | ✅ take assets, skip the crate |
| `kael_icons` whole crate (verbatim, ~350 LOC, 0 deps) | **S** | ❌ pointless — no deps, buys nothing over assets-only |
| `text_input` port (incl. `local_history.rs` + `undo_manager` shim) | **L** | ❌ overkill for a filter box |
| Improve hand-rolled `on_key_down` (grapheme buffer + cursor index, ≤40 LOC) | **S** | ✅ for the launcher |

## Integration points (verified)
- Icons: fork `svg()` + `window.paint_svg(bounds, path, Some(bytes), transform, color, cx)`
  (`window.rs:4102`); `render_alpha_mask` accepts inline `bytes` (`svg_renderer.rs:221`).
- Text: fork `ElementInputHandler`/`EntityInputHandler` + clipboards + `WrappedLine`
  all present; only `undo_manager()`/`WindowValueHistory` missing.
