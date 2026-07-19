# Contributing to ChronOS

ChronOS is developed as an **AI-orchestrated project**: a Lead Architect agent
coordinates a set of task-specific coding agents, does the acceptance review
(greps, diffs, build/test, live smokes), and owns the canonical docs. The
method, agent briefs and archived reports live under [`orchestration/`](orchestration/).

This document covers what any contributor — human or agent — is expected to
follow.

## Ground rules

- **Read the docs first, in this order:** `HANDOFF.md` (current state) →
  `AGENTS.md` (house rules) → `ARCHITECTURE.md` (accepted decisions) →
  `DECISIONS.log` (rejected alternatives). On conflict, these win over any
  remembered context.
- **Verification is not "it compiles."** For window/UX code, a release binary
  run against a live Wayland session (`RUST_LOG=info`, `grim` screenshots) is
  required. Unit-green alone does not count. See `HANDOFF.md` for smoke recipes.
- **Don't silently swallow errors.** No `let _ = fallible_call()`. Propagate with
  `?`, log with `.log_err()` when intentionally ignored, or `match`/`if let Err`
  for specific handling. (This exact pattern was the root of a ghost-window bug.)

## Code style

- Workspace lints are on (`Cargo.toml`): `unsafe_code = deny`,
  `clippy::unwrap_used` / `expect_used = warn`. Every new crate must opt in with
  `[lints] workspace = true`.
- Match the surrounding code — comment density, naming, idiom. Comment the
  *why* when it is not obvious; do not narrate the *what*.
- Format with `cargo fmt`; keep `cargo clippy` clean for new code.

## Commits

- Message form: `area : what changed` (concise). No AI trailers
  (`Co-Authored-By` / `Assisted-by`).
- **Stage by name** (`git add <path>`), never `git add -A`. Review
  `git diff --staged` before committing — self-contained commits only, never
  sweep unrelated working-tree changes into a commit.

## Building & testing

```sh
cargo build --release -p chronos          # release binary
cargo test  --workspace --lib --bins      # tests
RUST_LOG=info ./target/release/chronos     # live run
```

## Plugins

Luau plugins live in `crates/plugins/` (data, not a Rust crate). Each plugin has
a `manifest.toml` and a Luau entry point (e.g. `init.luau`) and is hot-reloaded
by the `chronos-luau` runtime.
