---
name: hermes-acp-tool-completed
description: Use when ACP tool cards in the ChronOS agent panel stay pending/stale after a turn finishes successfully, or after running `hermes update` (which reverts this patch), or before editing anything under ~/.hermes/hermes-agent — the checkout is a detached-HEAD upstream release, not a fork.
---

# Hermes ACP: tools of the last step never complete

**Measured against:** hermes-agent 0.18.2 (upstream commit `9de9c25f6`,
2026-07-07), agent-client-protocol 0.11.1, ChronOS ACP client at protocol v1,
2026-07-27. Upstream `main` still carried the defect when checked that day.

## Symptom

The turn finishes fine — text streams, `composer: turn END (reason=ok)` —
but the tool cards for that turn's last batch stay `pending` and the panel
marks them `stale`. Cards from *earlier* steps of the same turn show `Done`.

Not a ChronOS bug. Measured before blaming anyone: over one live turn the
wire carried **10** `"sessionUpdate":"tool_call"` and **1**
`"sessionUpdate":"tool_call_update"`, and ChronOS parsed 10 and 1 — nothing
was dropped on our side. The agent simply never sent the other nine.

## Cause

`~/.hermes/hermes-agent/acp_adapter/events.py`:

- `make_tool_progress_cb` emitted `ToolCallStart` on `tool.started` and
  **silently returned** on `tool.completed`;
- completion was emitted only from `make_step_cb`, which walks the
  `prev_tools` list of the **next** agent step.

Tools that run in the final step before the answer have no next step, so
their terminal update never existed. Zed doesn't surface this as loudly, so
"it works in Zed" was not evidence against it.

The payload was there all along: `agent/tool_executor.py:878` and `:1546`
already pass `result=`, `is_error=`, `duration=` with the `tool.completed`
event, and `acp_adapter/tools.py:1249` `build_tool_complete()` already
derives `failed` vs `completed` from the result.

## The patch

`0001-fix-emit-ACP-tool-completion-on-tool-completed.patch` in this
directory. It adds a `tool.completed` branch to `make_tool_progress_cb`
that pops the existing FIFO entry and reuses `build_tool_complete`, plus
three tests in `tests/acp/test_events.py`.

Both completion paths consume the same FIFO entry, so whichever fires first
wins and the other becomes a no-op — the dedup is free, and the test
`test_tool_completed_and_step_cb_do_not_double_report` proves it rather
than asserting it.

## Applying (and re-applying after `hermes update`)

**`~/.hermes/hermes-agent` is a detached HEAD on an upstream release
(`NousResearch/hermes-agent`), not a fork.** `hermes update` moves it and
discards local commits without asking. That is why the patch lives here.

```bash
cd ~/.hermes/hermes-agent
git checkout -b fix/acp-tool-completed          # never commit onto detached HEAD
git am /home/neo/projects/chronos-ecosystem/ChronOS/skills/hermes-acp-tool-completed/*.patch
```

Restart matters: code changes are picked up only by a new agent process.
ChronOS spawns the agent per session, so `chronos-stop && chronos-start`
is enough.

## Running the tests

The bundled `venv/` is stripped — it has no pytest, so `scripts/run_tests.sh`
reports "0 tests passed" with `No module named pytest` and looks like a
disaster. It isn't; it's a harness gap. Layer pytest over their interpreter
without touching their install:

```bash
cd ~/.hermes/hermes-agent
PYTHONPATH="$(pwd):$(pwd)/venv/lib/python3.11/site-packages" \
  uv run --python venv/bin/python --with pytest --with pytest-xdist \
  python -m pytest tests/acp/test_events.py -q -p no:cacheprovider -o addopts=""
```

`PYTHONPATH` must include the venv's `site-packages` — the `acp` module
lives there and `uv run` builds an isolated env that won't see it otherwise.

**Always take a baseline before judging failures.** On 2026-07-27
`tests/acp/ tests/acp_adapter/` gave **83 failed / 238 passed on the clean
tree** — pre-existing, caused by this ad-hoc runner rather than by any
change. With the patch: 83 failed / 241 passed. Same failures, +3 green.
Without the baseline run those 83 read as "you broke everything".

## Upstream

Submitted: **https://github.com/NousResearch/hermes-agent/pull/72964**
(2026-07-28, from `Dark-Ohm:acp-tool-completed`, rebased onto their `main`).

If it lands, drop the local patch and this skill's apply step — check the PR
state before re-applying after a `hermes update`.
