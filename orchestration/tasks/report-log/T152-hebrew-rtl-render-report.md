# T152 — Hebrew / RTL in agent panel — report

**Executor:** Hermes (terminal agent — no GUI / Wayland session access).
**Date:** 2026-07-28.
**Status:** IMPLEMENTED. Defect A (alignment) and Defect B (overflow) both patched in `crates/app`.

## Done (code)

### Defect A — base-direction-aware alignment (implemented, compiles)
- `crates/app/src/side_panel_left/mod.rs`: added `is_rtl_text(text: &str) -> bool`.
  Scans for the first strong character — Hebrew `U+05D0..U+05EA` and Arabic
  blocks → RTL; Latin `A..Z`/`a..z` short-circuits to LTR. No allocation.
- `chat_view.rs::render_message`: content div now does
  `.when(is_rtl_text(&msg.content), |el| el.text_right())`. Also dropped a dead
  `if is_user { … } else { … }` ternary whose both branches were
  `theme.text.primary`.
- `composer.rs::render_composer`: the `text_input` canvas gets the same
  `.when(is_rtl_text(text), |el| el.text_right())`.
- Both files got `use super::is_rtl_text;`.

Matches the task's amended plan: the fork has **no `text_direction` API** (only
`text_align` / `text_right`), so RTL content is right-aligned; intra-paragraph
bidi is delegated to the shaper (cosmic-text 0.19), already confirmed working by
the architect's live frame.

### Defect B — text overflowing the bubble (patched)
The architect's live `grim` measurement confirmed that long RTL strings
overflow bubble boundaries when they wrap — fragments paint outside the
bubble, over the panel background. The root cause is in our bubble markup
(`chat_view.rs`), not in the fork's `text_system`.

**Fix applied** (`chat_view.rs`):
- Added `.overflow_hidden()` to the user bubble inner div (the
  `bg(theme.bg.elevated)` container).
- Added `.overflow_hidden()` to the agent bubble inner div (the
  `bg(theme.bg.secondary)` container).

This clips any text fragments that would otherwise render outside the
bubble bounds. The `hebrew_wrap_test.rs` example in `Source/gpui/examples/`
remains as a regression harness for this class of issue.

The architect's measurement also confirmed:
- P0 (Noto Sans Hebrew font) — cancelled. Glyphs render fine via system
  fallback (`DejaVuSans`), no tofu.
- Bidi inside strings — works via cosmic-text 0.19, no fork changes needed.
- `text_right()` alignment — sufficient; no `text_direction` API exists in
  the fork.

## Build verification (real, not claimed)
- `cargo check -p chronos` (the app crate is named `chronos`): **Finished**,
  only pre-existing warnings, 0 errors from T152 edits.
- `cargo check --example hebrew_wrap_test` (from `Source/gpui`): **Finished**,
  0 errors. Example is runnable via `cargo run --example hebrew_wrap_test`.

Both artifacts are compile-correct. Runtime / visual correctness still needs
the architect's eye for final sign-off (see blockers).

## Blockers (honest)
- **No GUI session.** This agent runs in a terminal with no Wayland/Hyprland
  display. The task requires a live `grim` frame of the panel with real Hebrew
  input (`ydotool` ruled out by architect — layout issue). I cannot produce or
  inspect that frame. Therefore Defect A's *visual* result and Defect B's
  *runtime behaviour* are unverified by me — only the compile path is proven.

## Acceptance criteria — status
1. Pure Hebrew right-aligned + readable glyphs — code done, **visual TBD**.
2. Mixed `שלום world` (Hebrew RTL, `world` LTR) — bidi confirmed by architect;
   alignment code done, overflow fix applied, **visual TBD**.
3. Pure Latin unchanged — conditional on `is_rtl_text`, so Latin is untouched by
   construction, **visual TBD**.
4. Composer Hebrew natural — code done, **visual TBD**.

## Next steps for architect
1. `cargo run -p chronos` (or your `chronos-rebuild && chronos-start`), open
   chat, send a long Hebrew message, `grim` it. Confirm Defect A visually and
   that Defect B (text outside bubble) no longer reproduces.
2. `cd Source/gpui && cargo run --example hebrew_wrap_test`, `grim` the 280px
   box. Confirm the red-bordered box contains all text (no fragments left of
   the border).
3. Report back if any regressions or remaining issues.
