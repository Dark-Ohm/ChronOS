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
not decide scope, and they do not self-merge. That role is not delegable and
not claimable: whatever an agent's own configuration calls it, inside this
repository it is a worker.

**The canon below is read-only for everyone but the owner.** It is the
owner's working instrument, not a shared wiki — a PR that edits any file
under `.chronos-ops/checkpoint/` is rejected on sight, no matter how correct
the edit is. Disagree with something in it? Say so in the report or the
issue; the owner makes the edit. The same applies to `.rules` and
`CLAUDE.md`.

`CLAUDE.md` is the owner's own architect-side agent config. You are welcome
to read it, and to run the same process on your own machine — but it
describes a role you do not hold here, and it is never edited by a
contributor.

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
.chronos-ops/
├── active/<role>/   open tickets by role (back/front/qa/recon/design)
│   ├── <ROLE>.md    role entry point — points at the current ticket
│   └── ../hold/     blocked or intentionally deferred (any role)
├── reports-fresh/   your report goes here when a ticket is done (shared inbox)
├── reports-log/<role>/  accepted reports (archive)
├── done/<role>/     accepted tickets (archive)
├── rework/<role>/   sent back for fixes, reason stated inline
└── reject/<role>/   not to be continued, reason stated inline
```

Full rules: `.chronos-ops/RULES.md`. Ledger of every T-ID:
`.chronos-ops/MIGRATION.md`.

**Picking up a ticket:** read the brief in `active/<role>/TNNN-slug.md` in full —
it's written to be self-sufficient (exact file paths, zone boundaries,
what's already in the tree, what "done" means, how to verify) because your
session may have no memory of how it was written. If a brief is ambiguous
or contradicts `.chronos-ops/checkpoint/ARCHITECTURE.md` /
`.chronos-ops/checkpoint/REJECTED.md`, stop and ask — don't
guess and don't silently expand scope.

**Reporting:** write `.chronos-ops/reports-fresh/TNNN-slug-report.md`
before you're done. State what you actually did (not what the brief asked
for), what you verified and how (command output, not "should work"), and
what you did *not* do. An honest "not sure, didn't verify" is worth more
than a confident claim that turns out wrong on review.

**Acceptance:** the Architect re-runs your verification independently
against the tree — grep, diff, build, test, and for anything touching a
window or user-visible behavior, a live release-binary smoke test. Claims
in a report are treated as claims, not facts, until reproduced. Accepted
work moves brief + report to `done/<role>/` / `reports-log/<role>/`;
work sent back moves to `rework/<role>/` and rejected work to
`reject/<role>/`, with the reason recorded inline.

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
