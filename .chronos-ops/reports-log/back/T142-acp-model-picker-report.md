# T142 — ACP model list + picker + set_model: Report

**Status:** DONE  
**Date:** 2026-07-26  
**Commit:** pending

## Summary

Model picker now populates from the session's model list and sends
`session/set_model` when the user selects a different model.

## What changed

### `crates/services/src/hermes_acp/client.rs`

**Task 1 — Evidence logging (lines ~180–210):**  
Added evidence tracing in `models_from_session` to log what Hermes actually
returns for models on connect:
- `response.models` (if present)
- `config_options` (if present)
- Log target: `chronos::acp::models`

**Task 2 — SetModel command (lines ~65–75, 420–430):**  
Added `Command::SetModel { model_id, reply }` variant to the Command enum.
`execute_command` dispatches to `set_model_on_active()`.

**Task 3 — set_model_on_active (lines ~279–317):**  
```rust
async fn set_model_on_active(
    _cx: &ConnectionTo<Agent>,
    active: &mut Option<ActiveSession<'static, Agent>>,
    model_id: &str,
) -> Result<()>
```
Uses `conn.send_request_to(Agent, SetSessionModelRequest::new(...))`
with `.on_receiving_result()` callback pattern (matches SDK's internal
`send_prompt` pattern — avoids blocking the event loop).

**Public API — HermesClient::set_model (lines ~420–430):**
```rust
pub async fn set_model(&self, model_id: &str) -> Result<()>
```
Sends `Command::SetModel` through the command channel; blocks on
oneshot reply.

### `crates/app/src/side_panel_left/composer.rs`

**Model picker click handler (line ~281):**  
On model option click:
1. Updates `composer_selected_model` (local UI state).
2. Clones `HermesClient` from `clients` map.
3. Spawns background task calling `client.set_model(&model_id)`.
4. Logs warning on error; no toast (future enhancement).

```rust
cx.spawn(async move |this, cx| {
    if let Err(e) = client.set_model(&model).await {
        tracing::warn!("set_model failed: {e}");
    }
    let _ = this.update(cx, |_this, cx| { cx.notify(); });
})
```

## ACP types used

| Type | Crate | Notes |
|------|-------|-------|
| `SetSessionModelRequest` | `acp-schema` | `new(session_id, model_id)` |
| `SetSessionModelResponse` | `acp-schema` | Empty on success |
| `ModelId` | `acp-schema` | `Arc<str>` newtype, `From<String>` |
| `ModelInfo` | `acp-schema` | `{ model_id, name, description? }` |
| `MatchDispatch` | `acp::util` | Typed dispatch from `SessionMessage` |

Feature gate: `unstable_session_model` (enabled in `crates/services/Cargo.toml`).

## Verification

- [x] `cargo check -p chronos` — clean
- [ ] Live smoke: Super+A → model dropdown shows list if Hermes provides
      models; select different model → log "set_model OK"; next prompt
      uses new model (agent-dependent behavior).

**Live smoke** requires running shell — deferred to Architect's manual test.

## Known limitations

- `SetSessionModelResponse` is empty — no confirmation payload; success
  is inferred from no JSON-RPC error.
- If Hermes doesn't send models, picker shows "default" placeholder
  (no fake list).
- No UI feedback on set_model success/failure beyond tracing log.

## Out of scope (unchanged)

- Provider settings UI / API keys
- Multi-agent (T138)
- Model picker visual polish / search

## Architect verdict 2026-07-26T18:07:28+03:00
**Architect: ACCEPTED WITH CAVEATS** (36e8399; live model list depends on Hermes payload).
