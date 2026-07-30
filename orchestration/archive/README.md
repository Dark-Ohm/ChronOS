# Orchestration

ChronOS is built by a **Lead Architect agent** coordinating a set of
task-specific coding agents ("minions"). This directory holds that working
method — the agent briefs, their reports, and the archive. It is process, not
product; the shell itself lives under `crates/`.

## How it works

1. The Architect writes a **self-contained brief** into an agent's file under
   `agents/` (cold-session safe: full context, exact paths, non-overlapping file
   zones, verification steps, commit style).
2. The agent executes and returns a report to `reports/<name>-report.md`.
3. The Architect does acceptance **personally** — greps, diffs, `build`/`test`,
   and live release smokes — verifying every claim against the tree. Minion
   reports are not trusted on their word.
4. Accepted reports are archived to `report-log/` with an explicit commit.

The Architect does **not** spawn its own subagents and does **not** write feature
code (exceptions: docs, one-line errata after acceptance, live interactive
debugging).

## Layout

```
orchestration/
├── agents/        Per-agent briefs (CLINE, HERMES, MIMO, OMP, OPENCODE,
│                  GROK, ZED, AUTOHAND) + shared ruleset (rules.md) and
│                  persona (SOUL.md). Last section of each file = current
│                  assignment / acceptance state for that track.
├── reports/       Active, not-yet-accepted agent reports.
└── report-log/    Archived reports (accepted or superseded).
```

Per-agent runtime state (`.cline/`, `.autohand/`, `.mimocode/`, `.clinerules/`)
and context caches are git-ignored — only the human-readable briefs and reports
are tracked.

## Authority

Project canon lives at the repo root and wins over anything here on conflict:
`HANDOFF.md` (current state) → `ARCHITECTURE.md` (accepted decisions) →
`DECISIONS.log` (rejected alternatives). See also `AGENTS.md` and `CLAUDE.md` for
house rules.
