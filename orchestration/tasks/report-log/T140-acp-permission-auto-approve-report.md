# T140 — ACP permission auto-approve: Report

**Status:** DONE  
**Date:** 2026-07-26  
**Commit:** pending

## Summary

Registered `on_receive_request(RequestPermissionRequest)` handler on the
`Client.builder()` in `transport.rs` so that every `session/request_permission`
from the Hermes agent receives an auto-approved response.

## What changed

**File:** `crates/services/src/hermes_acp/transport.rs`

### Before (lines 59–87)

```rust
let result = Client
    .builder()
    .name("chronos-shell")
    .connect_with(agent, async |cx| {
```

No `on_receive_request` — permission requests from the agent (e.g. tool
write/edit) had no handler, causing the agent to receive an implicit denial
("Edit approval denied by ACP client").

### After (lines 65–112)

```rust
let result = Client
    .builder()
    .name("chronos-shell")
    .on_receive_request(
        async move |request: RequestPermissionRequest, responder, _cx: ConnectionTo<Agent>| {
            // Auto-approve: prefer AllowAlways > AllowOnce > first option.
            let chosen = request.options.iter()
                .find(|o| o.kind == PermissionOptionKind::AllowAlways)
                .or_else(|| request.options.iter().find(|o| o.kind == PermissionOptionKind::AllowOnce))
                .or_else(|| request.options.first());

            match chosen {
                Some(opt) => {
                    info!(target: TARGET, tool = ..., option = %opt.name,
                          "ACP permission auto-approved");
                    let _ = responder.respond(RequestPermissionResponse::new(
                        RequestPermissionOutcome::Selected(
                            SelectedPermissionOutcome::new(opt.option_id.clone()),
                        ),
                    ));
                }
                None => {
                    warn!(target: TARGET, tool = ...,
                          "ACP permission request has no options — cancelling");
                    let _ = responder.respond(RequestPermissionResponse::new(
                        RequestPermissionOutcome::Cancelled,
                    ));
                }
            }
            Ok::<(), AcpError>(())
        },
        agent_client_protocol::on_receive_request!(),
    )
    .connect_with(agent, async |cx| { ... });
```

### Imports added

```rust
use agent_client_protocol::schema::{
    InitializeRequest, PermissionOptionKind, ProtocolVersion, RequestPermissionOutcome,
    RequestPermissionRequest, RequestPermissionResponse, SelectedPermissionOutcome,
};
```

## Policy v1

| | |
|---|---|
| **Default** | Always auto-approve (AllowAlways > AllowOnce > first option) |
| **Empty options** | Cancelled + warn (no panic) |
| **Log target** | `chronos::acp::permission` |
| **Not in v1** | UI prompt per tool, per-path ACL, deny list |

## Pattern source

`agent-client-protocol-0.11.1/examples/yolo_one_shot_client.rs` lines 99–116.
Our version adds preference ordering (AllowAlways > AllowOnce) and structured
tracing with the log target.

## Verification

- [x] `cargo build --release -p chronos` — clean, zero errors from `hermes_acp`
- [ ] Live smoke: restart shell, Super+A, prompt Hermes to create/delete a file
  — expected: no "Edit approval denied", file operations succeed, log line
  "ACP permission auto-approved" in `RUST_LOG=info` output.

**Live smoke** requires running shell — deferred to Architect's manual test.

## Out of scope (unchanged)

- Tool card / reasoning UI (T141)
- Model picker (T142)
- Multi-agent (T138)
- `auto_approve_permissions` config toggle — trivial future addition

## Architect verdict 2026-07-26T18:07:28+03:00
**Architect: ACCEPTED WITH CAVEATS** (code in tree via 36e8399; live write_file smoke PENDING). No separate T140 commit.
