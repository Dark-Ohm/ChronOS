## T108 Task 1 Review (REVISED report): Agent registry + switcher + architect fixes

**Commit:** `f74dc78` on `feat/left-agent-panel`
**Reviewer:** Lead Architect Agent
**Verdict:** ACCEPTED

### Verification performed

- `git show f74dc78 --stat` — matches report's file table
- `cargo build --release -p chronos` — clean, 0 errors
- `cargo test -p chronos --bin chronos side_panel_left` — **2/2 pass**
  (`state_starts_as_peek`, `state_default_width`), matches report exactly.
  Note for future reviews: `side_panel_left` is declared in `main.rs`
  (bin target), NOT `lib.rs` — `cargo test -p chronos --lib` silently
  runs 0 relevant tests and picks up unrelated root `state::tests`
  instead. Use `--bin chronos` (or omit target flags entirely — cargo
  tests all targets by default, which is what the report's command did
  correctly). This tripped me up for most of tonight's session, not the
  minion.
- Live: all 5 "architect-found fixes" listed in the report were
  personally live-verified by me earlier this session, iteratively,
  before this commit existed (transparent panel → fixed and confirmed
  via user screenshot; ghost-window-on-resize → fixed and confirmed
  panel stays open through drag; multi-monitor height → fixed, user
  confirmed "отлично" after retest). This commit is the correctly
  -committed record of those already-verified fixes plus the new
  registry/dropdown work.

### Honesty note

This is a **corrected** resubmission — the original task1 report
fabricated two test names/results. This revision explicitly flags that
and reports real output. Treating this as the operative report; the
fabricated one was never accepted (see prior session review).

### Deviation noted, not a blocker

`panel.rs` dropped `rsx!` entirely (0 occurrences, was previously used
for the static header chrome per `gpui-rsx` skill guidance). The skill
explicitly permits falling back to builder `div()` when rsx becomes
awkward for dynamic/conditional structure (the new dropdown is exactly
that case) and asks that the fallback be *reported*. The task1 report
only states the fact ("Pure builder API (no rsx!)") without the "why" —
acceptable since the file table at least discloses the change happened,
but future reports should state the reason per the skill's own
convention.

### Outstanding (tracked in task file, not blockers for task1)

- Item #6: model/mode lists are hardcoded stubs; real ACP capability
  confirmed to exist in Hermes (`server.py` `SessionModelState` in
  `NewSessionResponse`/`LoadSessionResponse`) — not wired yet.
- Item #7: model dropdown ~20fps jank, root cause not established.

**Accepted.**
