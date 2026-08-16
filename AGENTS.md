# AGENTS.md

How AI coding agents should work in this repository.

## Project

ChronOS is a Wayland desktop shell for Hyprland, written in Rust on
[GPUI](https://www.gpui.rs/). See [`README.md`](README.md) for what it is and
[`.chronos-ops/checkpoint/ARCHITECTURE.md`](.chronos-ops/checkpoint/ARCHITECTURE.md) for how it's built.

## Governance

There is one Design Authority for this project — **[Dark-Ohm](https://github.com/Dark-Ohm)**
(the repository owner). Every architectural decision, every accepted change,
and every merge goes through them. Agents implement against a brief; they do
not decide scope, and they do not self-merge.

The canonical docs, in priority order when they disagree with anything else
(chat history, an agent's own prior output, a stale brief):

1. [`.chronos-ops/checkpoint/HANDOFF.md`](.chronos-ops/checkpoint/HANDOFF.md) — current state of the project and
   the active work queue. Read this first.
2. [`.chronos-ops/checkpoint/ARCHITECTURE.md`](.chronos-ops/checkpoint/ARCHITECTURE.md) — accepted design
   decisions and why.
3. [`.chronos-ops/checkpoint/REJECTED.md`](.chronos-ops/checkpoint/REJECTED.md) — options that were
   considered and rejected, and why. Read before re-proposing something;
   it may already have a documented answer.

## Task workflow

Work is tracked as numbered tickets (`TNNN`), not as ad-hoc chat requests:

```
docs/orchestration/tasks/
├── active/       open tickets, ready to pick up (self-contained briefs)
│   └── pause/    blocked or intentionally deferred
├── report/       your report goes here when a ticket is done
├── report-log/   accepted reports (archive)
├── done/         accepted tickets (archive)
└── rejected/     rejected briefs/reports, with the reason stated inline
```

**Picking up a ticket:** read the brief in `active/TNNN-slug.md` in full —
it's written to be self-sufficient (exact file paths, zone boundaries,
what's already in the tree, what "done" means, how to verify) because your
session may have no memory of how it was written. If a brief is ambiguous
or contradicts `ARCHITECTURE.md`/`DECISIONS.log`, stop and ask — don't
guess and don't silently expand scope.

**Reporting:** write `docs/orchestration/tasks/report/TNNN-slug-report.md`
before you're done. State what you actually did (not what the brief asked
for), what you verified and how (command output, not "should work"), and
what you did *not* do. An honest "not sure, didn't verify" is worth more
than a confident claim that turns out wrong on review.

**Acceptance:** the Architect re-runs your verification independently
against the tree — grep, diff, build, test, and for anything touching a
window or user-visible behavior, a live release-binary smoke test. Claims
in a report are treated as claims, not facts, until reproduced. Accepted
work moves brief + report to `done/` / `report-log/`; rejected work moves
to `rejected/` with the reason recorded, and stays open for another pass.

## Working rules

- **Zone discipline.** A brief states which files are in scope. Don't touch
  files outside it, especially ones another ticket is actively using —
  check `git status` and recent commits before editing a shared file.
- **`let _ = fallible_call()` is not acceptable.** Propagate with `?`, log
  with `.log_err()` if the result is intentionally ignored, or handle the
  error explicitly. A silently swallowed error has caused real, hard-to-find
  bugs in this codebase.
- **Dependency policy is bleeding-edge.** Newest versions; don't inherit
  pins from other projects or reference material.
- **Comments explain *why*, not *what*.** If the code already says what it
  does, a comment repeating that is noise.
- **New crates** need `[lints] workspace = true` — the workspace-level
  lints don't apply automatically otherwise.
- **`reference/` is never committed.** It holds unlicensed study material
  (see [`NOTICE`](NOTICE)); look at it, don't copy from it verbatim, don't
  `git add` it.
- **Commits:** `area : what changed`, no AI-authorship trailers, `git diff
  --staged` reviewed (by name, not `git add -A`) before committing —
  sweeping an unrelated file into someone else's in-progress commit is a
  repeat failure mode in a multi-agent tree.
- **"Compiles, tests pass" is not "done" for window/UX code.** This
  codebase has shipped visually broken changes behind a green test suite
  more than once. Anything that touches layout, a popup, or user input
  needs a release build and a live Wayland smoke test — see
  [`.chronos-ops/checkpoint/HANDOFF.md`](.chronos-ops/checkpoint/HANDOFF.md) for the current smoke recipes.

## License

Apache-2.0 — see [`LICENSE`](LICENSE) and [`NOTICE`](NOTICE) for
attribution of ported/derived code.
