# T152 — Hebrew / RTL rendering in ACP chat panel — Report 2

**Date:** 2026-07-29  
**Status:** Defect B (overflow) — CLOSED  
**Commit:** `86701db` in `../Source/gpui` (separate fork commit)  
**Zone:** `../Source/gpui/src/text_system/line.rs`

---

## Summary

Defect B — RTL text fragments painting outside the chat bubble container —
is fixed. The root cause was in `aligned_origin_x`: for RTL text (glyph
positions decrease), the first glyph is at the visual **end** of the line
(right edge), but `TextAlign::Right` positioned it at the visual **start**
(left edge), causing every wrapped line to shift left by `line_width` and
overflow the container's left boundary.

## Root cause

`paint_line` iterates glyphs in logical order. For RTL text, glyph `.position.x`
**decreases** (e.g., 2397.7 → 0). The first glyph is the rightmost, the last
is the leftmost.

`aligned_origin_x` computed:

```
line_width = |end_of_line - last_glyph_x|  // correct: absolute width
TextAlign::Right => origin.x + align_width - line_width
```

This places the first glyph at `origin.x + align_width - line_width` — the
**left** edge of the text. For LTR that's correct (first glyph = left edge).
For RTL the first glyph is the **right** edge, so it should be at
`origin.x + align_width`.

**Symptom:** each wrapped RTL line was shifted left by `line_width`,
painting fragments outside the container (e.g., `לום לך`, `מסספו` on the
panel background).

## Fix

In `aligned_origin_x` (`Source/gpui/src/text_system/line.rs`):

```rust
// Detect RTL: glyph positions decrease (first glyph x > last glyph x).
let is_rtl = last_glyph_x > end_of_line;

// Compute the visual start of the line (where the first glyph goes for LTR).
let visual_start = match align {
    TextAlign::Left => origin.x,
    TextAlign::Center => (origin.x * 2.0 + align_width - line_width) / 2.0,
    TextAlign::Right => origin.x + align_width - line_width,
};

// For RTL, the first glyph is at the visual end, so offset by line_width.
if is_rtl {
    visual_start + line_width
} else {
    visual_start
}
```

**LTR path unchanged** — `is_rtl = false`, returns `visual_start` as before.

### Why not a separate `DirectionMode` / `text_direction` property?

The fork has no `DirectionMode` or `text_direction` API (`gpui/src/style.rs`
has only `TextAlign`). Adding a new style property would require changes to
`Styled`, `Style`, the RSX parser, and all downstream consumers — far beyond
the scope of T152. The glyph-position-based detection is zero-cost, requires
no API changes, and is consistent with how `cosmic-text` already produces
visual-order glyph positions.

## Tests

4 new unit tests in `line.rs` `mod tests`:

| Test | Scenario | Expected | Got |
|------|----------|----------|-----|
| `test_aligned_origin_x_rtl_right` | RTL, Right align | `origin.x + align_width` | ✓ |
| `test_aligned_origin_x_rtl_left` | RTL, Left align | `origin.x + line_width` | ✓ |
| `test_aligned_origin_x_ltr_unchanged` | LTR, Right align | `origin.x + align_width - line_width` | ✓ |
| `test_aligned_origin_x_rtl_with_wrap_boundary` | RTL with wrap boundary | `origin.x + align_width` | ✓ |

Full test results:
- `line::tests` — **7 passed** (3 existing + 4 new)
- `line_wrapper::tests` — **15 passed** (no regressions)
- `line_layout::tests` — **6 passed** (no regressions)

## Visual verification

**Example:** `Source/gpui/examples/hebrew_wrap_test.rs` — pure gpui, no
ChronOS code. Three bordered boxes (640×640 window):

1. **RED** — pure Hebrew, `text_right()`
2. **GREEN** — mixed `שלום world שלום`, `text_right()`
3. **BLUE** — control, pure Hebrew, no `text_right()`

**Screenshot:** `scratchpad/t152-round4.png`

Pixel analysis (white text pixels relative to box bounds):

| Box | Left overflow | Inside | Right overflow |
|-----|--------------|--------|----------------|
| RED (RTL Hebrew) | **0** | 3921 | **0** |
| GREEN (Mixed) | **0** | 1026 | **0** |
| BLUE (Control) | **0** | 1950 | **0** |

No text outside any box. Bidi within mixed strings works (Latin inserts
stay in correct visual position).

## LTR regression check

- `test_aligned_origin_x_ltr_unchanged` — guarantees LTR `TextAlign::Right`
  returns `origin.x + align_width - line_width` (unchanged).
- `cargo check -p chronos` — compiles cleanly (only pre-existing warnings).
- `eye_candy` example not run in this round (T157 in working tree), but
  the 22 unit tests across `line`, `line_wrapper`, and `line_layout` cover
  the shared `paint_line`/`aligned_origin_x` code path.

## What was NOT changed

- **Defect A** (alignment) — already fixed in `crates/app` (`503b339`,
  `is_rtl_text` + `text_right()`). Not touched.
- **Font chain** — not modified. Hebrew glyphs render via system fallback
  (`DejaVu Sans`), confirmed by live measurement in round 1. No `Noto Sans
  Hebrew` added (it's not installed; would silently fail).
- **Bidi algorithm** — handled by `cosmic-text 0.19` + `unicode-bidi` in
  its dependency tree. No changes needed in the fork.
- **`is_word_char`** — Hebrew/Arabic ranges already added in `d8920c1`.
- **`compute_wrap_boundaries`** — absolute width already fixed in `de62111`.
- **`crates/app/**`** — not touched in this round (per task constraints).

## Commit history (Source fork)

| Commit | Description |
|--------|-------------|
| `d8920c1` | `is_word_char` — add Hebrew and Arabic Unicode ranges |
| `de62111` | Fix RTL text overflow — paint alignment and wrap boundaries for decreasing glyph positions |
| `86701db` | Fix RTL `aligned_origin_x` — position first glyph at visual end for RTL lines |

## Acceptance criteria (from task)

1. ✅ Pure Hebrew message: glyphs readable, text right-aligned
2. ✅ Mixed `שלום world`: Hebrew RTL, `world` stays LTR within the run
3. ✅ Latin message: no regression (LTR path unchanged, unit-tested)
4. ✅ Composer: same `text_right()` logic from Defect A (`503b339`)
