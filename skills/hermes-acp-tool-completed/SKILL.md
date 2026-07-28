---
name: hermes-acp-tool-completed
description: Use when ACP tool cards in the ChronOS agent panel stay pending/stale after a turn finishes successfully, when an ACP request to Hermes returns Ok but nothing changes (model switch in particular — `session/set_mode` swallows anything), or after running `hermes update` (which reverts this patch), or before editing anything under ~/.hermes/hermes-agent — the checkout is a detached-HEAD upstream release, not a fork.
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

---

# Hermes ACP: `session/set_mode` answers `Ok` to anything

**Measured 2026-07-28** against hermes-agent 0.18.2, agent-client-protocol
2.0.0, ChronOS `set_model_on_active`. Found during T144 acceptance; the code
had shipped weeks earlier and looked healthy in every log.

## Symptom

A request succeeds and nothing happens. Specifically: switching the model
returned `Ok(())`, the log said `set_model OK` with the requested id — and
the next turn ran on the *old* model.

## Cause

Two different agent methods, and only one of them switches models:

- `set_session_model` (`acp_adapter/server.py:1995`, method
  `session/set_model`) — the real thing: rebuilds `state.agent` with the new
  model and provider, persists the session.
- `set_session_mode` (`server.py:2029`, method `session/set_mode`) —
  documented verbatim as *"persist the editor-requested mode so ACP clients
  do not fail on mode switches"*. It stores whatever string arrives and
  answers success. **It validates nothing.**

ChronOS was calling the second one with a model id in the `mode_id` field,
because `SetSessionModelRequest` was deleted from agent-client-protocol
2.0.0 along with the entire `models` concept, and `SetSessionModeRequest`
was the type that still compiled. It compiled, it ran, it logged `OK`, and
it did nothing.

## The lesson that generalizes

**`Ok` from an ACP agent is not evidence that the intent was carried out.**
This agent has at least one handler whose stated purpose is to absorb
requests so clients don't error. Verify effects, not return codes:

```bash
# what actually went on the wire
grep -aoE '"method":"[a-z/_]+"' <log> | sort | uniq -c
# what the agent actually did with it
grep -aoE 'provider=\S+ base_url=\S+ model=\S+' <log> | sort | uniq -c
```

Before the fix: `1 "method":"session/set_mode"`, and `model=tencent/hy3:free`
throughout. After: `1 "method":"session/set_model"`, then 12 lines of
`model=anthropic/claude-opus-4`.

## The fix on our side

`session/set_model` has no type in the 2.0.0 crate, so it is sent as
`UntypedMessage::new("session/set_model", json!({"sessionId", "modelId"}))`
through the normal `conn.send_request_to(Agent, …)` path
(`crates/services/src/hermes_acp/client.rs`, commit `b5116ee`, marked
DELETE). Two dialects meet here: the crate dropped `models` in favour of
`configOptions`, the agent has not adopted them yet — see upstream issue
[#301](https://github.com/agentclientprotocol/rust-sdk/issues/301) for the
other half of the same split.

## How to prove a switch works, in 18 seconds, without the GUI

An ignored smoke test against the service layer — no Hyprland, no panel:
`create_session` → pick any model from `models.available` other than the
current one → `set_model(target)` → send a one-word prompt → grep the two
lines above. That is how this defect was found and how the fix was accepted.
