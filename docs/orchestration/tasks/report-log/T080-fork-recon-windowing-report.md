<!-- T080 — migrated 2026-07-22 from docs/orchestration/report-log/hermes-report-17.md — see docs/orchestration/tasks/MIGRATION.md -->

# Documentation Audit — windowing-platform.md (AI-drafted, re-verified)

**Auditor:** Philip (skill) + fable-judge cross-check. **Date:** 2026-07-20.
**Target:** `ChronOS/skills/chronos-gpui/references/windowing-platform.md` (commit f7099e5) +
`evals/windowing-platform.eval.md`. **Source of truth:** `../Source/` (gpui fork).
**Mode:** audit (findings only; fixes applied in place per maintain mode, since the doc is mine).

## Executive Summary

Health: **good after one correction**. The doc is AI-drafted (rule #11 — all citations guilty
until verified). I re-grepped every cited symbol across Source and **compile-tested** the
interactive-method claims. 38 of 39 citations resolve correctly to `file:LINE`/symbols.
**One real error** remained even after the fable-judge pass: §6 asserted `on_scroll_wheel`
requires `.id()` like `on_click`/`overflow_y_scroll`. It does **not** — `on_scroll_wheel` is a
default on `InteractiveElement`, so it compiles on bare `div()`. Fixed. Severity: **Medium**
(wrong `.id()` guidance → a reader would add pointless `.id()` wrappers; not a security/compile
breakage, since the "with .id()" form also compiles). Effort to fix: ~15 min.

## Findings

### Critical
- None.

### High
- None.

### Medium
- **M-1: `on_scroll_wheel` misattributed to `StatefulInteractiveElement` / wrong line.**
  - Problem: §6 claimed `StatefulInteractiveElement::on_scroll_wheel` (div.rs:357-371) and that
    it "lives on StatefulInteractiveElement, reachable on div() only after .id()", lumping it
    with `on_click`. Also said bare `div().on_click(..)`/`div().overflow_y_scroll()` FAIL while
    `div().id("x").on_click(...).overflow_y_scroll()` compiles — implying `on_scroll_wheel`
    likewise needs `.id()`. The line 357-371 is the imperative `Interactivity::on_scroll_wheel`
    (not the `Div` fluent method); the fluent default is at **div.rs:969 on `InteractiveElement`**.
  - Evidence: `grep` div.rs → `on_scroll_wheel` at 361 (Interactivity impl) + 969 (InteractiveElement
    default). `impl InteractiveElement for Div` at 1695; `impl<E> StatefulInteractiveElement for
    Stateful<E>` at 3775. **Compile test:** threwaway `gpui/examples/_audit_inp.rs` built the
    expression `div().overflow_y_scroll()` + `div().on_click(..)` → E0599 (both fail, bare div),
    while `div().on_scroll_wheel(..)` (bare) compiled with **no error**. Probe removed after.
  - Impact: A reader wiring scroll-wheel on a layer-shell bar would wrap the element in a
    needless `.id()` (harmless but wrong guidance) or, worse, believe scroll needs state they
    don't have. The factual claim "scroll reaches layer-shell" was correct; the *method
    placement* was wrong.
  - Fix: Rewrote §6 — `on_scroll_wheel` = InteractiveElement (bare div OK, div.rs:969 / 357
    imperative); `on_click` + `overflow_y_scroll`/`overflow_x_scroll`/`track_scroll` =
    StatefulInteractiveElement (need `.id()`, div.rs:1475 / 1416-1435). Also corrected §7.1
    trailing line ("scroll/click/hover/cursor need .id()" → split: scroll-wheel bare OK;
    click/scroll-clip/stateful-hover need `.id()`; `cursor`/`cursor_pointer` are *style* methods
    styles.rs:164/178, applied on the styled element directly).
  - Verification: **verified** (grep + targeted compile test, both redone this session).
  - Confidence: High.

### Low
- **L-1: `on_hover` placement not stated.** `on_hover` fluent default is at div.rs:1524 — inside
  `StatefulInteractiveElement` (1213-1694), so it needs `.id()` like `on_click`. Doc only mentions
  `on_click` by name for the StatefulInteractiveElement bucket; `on_hover` left implicit. Minor.
  - Fix: added "stateful-hover" to the §7.1 corrected line.
  - Verification: verified (grep div.rs:1524; trait bracket confirmed via div.rs:1213/1695).
  - Confidence: High.

## Coverage Map

| Area | Doc section | Code evidence | Status |
|---|---|---|---|
| LayerShell open | §0 | window.rs:151-195, layer_shell.rs:9-77 | ✅ verified |
| LayerShellOptions fields | §1 | layer_shell.rs:59-77, bitflags Anchor:24-39 | ✅ verified |
| resize path | §2 | window.rs:2318→1340→1306→set_geometry 418-431 | ✅ verified |
| f32::from(Pixels) | §2 | geometry.rs:2677/2909 | ✅ verified |
| max_size clamp | §2 | style.rs:234, styles.rs:884/892/900, 135 | ✅ verified |
| displays None | §3 | client.rs:826-828, window.rs:605/660/1345/1570/2293 | ✅ verified |
| uuid/displays | §3 | platform.rs:288, display.rs:27-31, client.rs:795 | ✅ verified |
| keyboard/focus | §4 | layer_shell.rs:43-55, window.rs:1910/5296, 1616-1633 | ✅ verified |
| lifecycle/Drop | §5 | window.rs:1899/1728, wayland window.rs:680-750 | ✅ verified |
| input/scroll | §6 | client.rs:2179-2206, div.rs:357/969/1475/1416-1435 | ✅ verified (M-1 fixed) |
| reframes §7 | §7 | div.rs:1213/1695/710/3752, window.rs:1468/1340 | ✅ verified |

## Recommended Plan
1. ✅ (done) Split §6: scroll-wheel = bare-div OK; click + scroll-clip = `.id()` required.
2. ✅ (done) Fix §7.1 trailing over-claim; note `cursor` is a style method.
3. Optional: add `on_hover` explicitly to the StatefulInteractiveElement bucket in §6 prose
   (currently only implied). — done via §7.1 note.

## Unknowns
- `Exclusive` freezes Hyprland — cited as "HANDOFF" blood fact, not re-derived from Hyprland
  source this session (out of fork scope). Labeled as such in doc (§4).
- §8 ChronOS one-liners reference brief №12 — not re-checked against brief text this session.

## Verification Notes
- Every cited symbol grepped across `Source/` (div.rs, window.rs, app.rs, geometry.rs, style.rs,
  gpui_macros/src/styles.rs, wayland/{client,display,window,layer_shell}.rs, platform.rs,
  platform/layer_shell.rs). All resolve.
- Targeted compile tests run as throwaway examples under `gpui/examples/`, removed after:
  `_audit_inp.rs` (confirmed bare `on_scroll_wheel` compiles, bare `overflow_y_scroll`/`on_click`
  fail E0599). `layer_shell` + `scrollable` examples `cargo check` green (exit 0).
- No Orbit context used (unavailable); not required — all evidence is local fork source.
- Source tree left clean of probe files (git status: only unrelated untracked .mimocode/,
  brief.md, findings/, plan.json, reflect.json, REPORT.md).
