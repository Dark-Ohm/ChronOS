# T108 — Multi-agent switcher: Task 2 Report

**Date:** 2026-07-23
**Status:** PARTIAL — code compiles, 2/2 tests pass, live verification pending

---

## What was done

### Real modes/models from ACP session response (item #6)

Replaced hardcoded `MODELS`/`MODES` constants in composer with live data
extracted from ACP `NewSessionResponse`.

**Changes:**

1. **Cargo.toml** — enabled `unstable_session_model` feature on
   `agent-client-protocol` (required for `SessionModelState`/`ModelInfo`
   types in the Rust SDK)

2. **`session.rs`** — new types: `SessionMode`, `SessionModes`,
   `ModelInfo`, `SessionModels`. `AcpSession` now holds optional
   modes + models extracted from the session response.

3. **`client.rs`** — `send_prompt()` returns `PromptResponse` (text +
   modes + models) instead of bare `String`. Modes are extracted from
   `ActiveSession::response().modes`, models from
   `response.models` (cfg-gated).

4. **`composer.rs`** — removed hardcoded `MODELS`/`MODES` constants.
   Model picker iterates `panel.available_models`, mode picker iterates
   `panel.available_modes`. Display name falls back to ID if `name` is
   empty.

5. **`mod.rs`** — `SidePanelLeft` gains `available_modes: Vec<SessionMode>`
   and `available_models: Vec<ModelInfo>`. Populated on first prompt
   response. Selected model/mode initialized from `current_id` on first
   response (empty-string guard).

**Data flow:**
```
User sends prompt → HermesClient::send_prompt()
  → ACP: start_session() → NewSessionResponse { modes, models }
  → PromptResponse { text, modes, models }
  → SidePanelLeft stores available_modes/available_models
  → Composer renders real picker lists
```

**Hermes server support confirmed** (live, from server.py):
- `_session_modes()` returns 3 modes: "default", "accept_edits", "dont_ask"
- `_build_model_state()` returns models from curated list per provider
- Both fields present in `NewSessionResponse`, `LoadSessionResponse`,
  `ResumeSessionResponse`, `ForkSessionResponse`

---

## Files touched

| File | Change |
|---|---|
| `crates/services/Cargo.toml` | Added `unstable_session_model` feature |
| `crates/services/src/hermes_acp/session.rs` | New types: SessionMode, SessionModes, ModelInfo, SessionModels |
| `crates/services/src/hermes_acp/client.rs` | send_prompt returns PromptResponse with modes/models |
| `crates/services/src/hermes_acp/mod.rs` | Re-export PromptResponse |
| `crates/services/src/lib.rs` | Re-export ModelInfo, SessionMode |
| `crates/app/src/side_panel_left/mod.rs` | Store available_modes/available_models |
| `crates/app/src/side_panel_left/composer.rs` | Use real modes/models instead of hardcoded constants |

---

## Verification

- `cargo check -p chronos` — green (0 errors)
- `cargo test -p chronos side_panel_left` — 2/2 pass:
  - `state_starts_as_peek` ✓
  - `state_default_width` ✓

---

## Live verification — NOT DONE

1. Build release binary
2. Open left panel, send first prompt
3. Verify model picker shows real Hermes models (not "claude-sonnet-4-...")
4. Verify mode picker shows "default", "accept_edits", "dont_ask"
5. Verify switching model/mode works (if Hermes supports
   `set_session_model`/`set_session_mode`)

---

## What's NOT in this task

- Item #7/#8 (ghost trail during resize + dropdown jank) — separate issue,
  hypothesis: throttle `window.resize()` + `cx.notify()` in `update_resize`
- Hermes `set_session_model`/`set_session_mode` commands not wired yet
  (picker is display-only for now)
