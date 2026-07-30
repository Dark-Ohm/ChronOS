# Task 3 Review: Hermes ACP Client + Session

**Reviewer:** Lead Architect Agent
**Reviewed commit:** `83f8925` (feat/left-agent-panel branch)
**SDK verified against:** agent-client-protocol 0.11.1 + agent-client-protocol-tokio 0.11.1 (local registry + source)
**Build status:** `cargo build --release` — zero warnings from hermes_acp code, compiles clean.

---

## Verdict: PASS with 3 required fixes

The implementation is architecturally sound and uses the ACP SDK correctly. The deviations from the brief are all justified and well-documented in the report. However, there are real issues that should be addressed before merge.

---

## Spec Compliance

| Brief requirement | Status | Notes |
|---|---|---|
| Create `session.rs` | ✅ Done | Simplified — dropped `Vec<Message>` and `is_draft` (justified) |
| Create `client.rs` | ✅ Done | Command-channel architecture instead of direct ownership (justified) |
| `HermesClient` type | ✅ Done | `_transport` + `cmd_tx` fields |
| `AcpSession` type | ✅ Done | Lightweight wrapper over `SessionId` |
| `send_prompt` implementation | ✅ Done | Brief had `todo!()` — actual code is complete |
| `create_session` implementation | ✅ Done | Uses `start_session()` instead of manual `SessionId::new()` |
| Verify compiles | ✅ Done | Release build clean |
| Commit | ✅ Done | Single atomic commit |

### Justified deviations (all documented in report)

1. **No `Message` / `is_draft` in AcpSession** — Correct. The ACP SDK's `ActiveSession` owns message routing internally via `SessionMessage` enum and `update_rx` channel. There is no standalone `Message` type on the client side. The brief's `Vec<Message>` model was based on a pre-SDK mental model.

2. **Command channel pattern** — Correct. The ACP SDK's `ConnectionTo<Agent>` lives inside the `connect_with` closure and cannot be extracted. The command channel (`mpsc::UnboundedSender<Command>`) + oneshot replies is the standard pattern for this constraint. The brief's direct-ownership model (`transport: HermesTransport`) would have required the connection to outlive the closure.

3. **No client-side session tracking** — Correct. `create_session` and `send_prompt` are stateless from the client's perspective. Sessions are server-side state tracked by the Hermes agent process.

---

## Code Quality Issues

### Fix required (3)

**F1: `AcpAgent::from_str` with unquoted args is fragile** (`transport.rs:43-44`)

```rust
let command_str = format!("{} {}", config.command, config.args.join(" "));
let agent = AcpAgent::from_str(&command_str)...;
```

`AcpAgent::from_str` uses `shell_words::split` internally. If any arg contains spaces (e.g., `--cwd /path with spaces`), the round-trip through `format!` without quoting will break the parsing.

Replace with `AcpAgent::from_args`:
```rust
let mut parts = vec![config.command.clone()];
parts.extend(config.args.iter().cloned());
let agent = AcpAgent::from_args(parts).context("failed to parse ACP agent command")?;
```

This is also one fewer intermediate allocation.

**F2: Error type inconsistency** (`client.rs:112` vs `client.rs:121,132`)

`HermesClient::new()` returns `Result<Self, anyhow::Error>` while `create_session()` and `send_prompt()` return `Result<T, String>`. The command channel encodes errors as `String` (`oneshot::Sender<Result<T, String>>`).

Options:
- (a) Make `HermesClient::new()` also return `Result<Self, String>` — consistent but loses context.
- (b) Propagate `anyhow::Error` through the channel — more ergonomic at call sites.
- (c) Define a `HermesError` enum — most correct but over-engineered at this stage.

Recommendation: (b) — change the channel type to `oneshot::Sender<Result<T, anyhow::Error>>` and map errors at the call site. This matches the crate's existing `anyhow` convention.

**F3: `create_session` spawns infinite sleep loop** (`client.rs:67-73`)

```rust
cx.spawn(async move {
    let _active = active_session;
    loop {
        tokio::time::sleep(std::time::Duration::from_secs(3600)).await;
    }
})
```

This works but is a maintenance hazard. The `ActiveSession` holds `DynamicHandlerRegistration`s — dropping it deregisters the handlers. The infinite loop prevents that drop. But:
- The task never terminates normally — it's killed when the connection drops (via task set abort).
- The sleep duration (3600s) is arbitrary and could mask issues if something expects the session to be responsive.

Consider using `std::future::pending::<()>().await` instead — same effect, clearer intent, no arbitrary timer.

### Advisory (4)

**A1: `send_prompt` creates a new session per prompt** — Documented in report as a known simplification. For production use, persistent sessions will be needed. The command channel architecture supports this — just change `SendPrompt` to maintain a session ID. Not blocking for Task 3, but should be tracked.

**A2: No unit tests** — The brief's Step 3 only required compile verification. But `AcpSession::new()` and `Display` impl are trivially testable. Consider adding at least:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use agent_client_protocol::schema::SessionId;

    #[test]
    fn session_display() {
        let s = AcpSession::new(SessionId::new("test-123"));
        assert_eq!(format!("{s}"), "AcpSession(test-123)");
    }
}
```

**A3: `_transport` field convention** — The underscore prefix suppresses `dead_code` warnings but implies the field is unused. It's not — dropping it kills the connection. Rename to `transport` (the warning doesn't fire because the field *is* read via `Drop`).

**A4: `execute_command` silently drops reply errors** (`client.rs:44,47`) — `let _ = reply.send(result)` is correct for oneshot (caller dropped = don't care), but a `debug!` log on the Err path would help during debugging.

---

## Architecture Assessment

The command-channel pattern is the right call for this SDK. The key insight — that `ConnectionTo<Agent>` is owned by the `connect_with` closure and cannot be extracted — forces the background-task-with-commands architecture. The implementation handles this cleanly:

1. `HermesTransport::spawn` creates the channel pair and waits for the connection via oneshot before returning.
2. `execute_command` runs in the background task where `cx` is alive.
3. `HermesClient` holds only the sender side — safe to clone, send across threads.

The oneshot-per-request pattern (create_session, send_prompt) is slightly wasteful but correct for v1. The `start_session()` call internally spawns the session handshake into the connection's task set, which is the SDK's intended usage.

---

## SDK API Verification (v0.11.1)

| API used in code | Exists in SDK v0.11.1 | Correct usage |
|---|---|---|
| `Client.builder().name().connect_with()` | ✅ `jsonrpc.rs:599, connect_with` | Correct — `AsyncFnOnce(ConnectionTo<Agent>) -> Result<R, Error>` |
| `cx.send_request(InitializeRequest::new(ProtocolVersion::V1)).block_task().await` | ✅ `session.rs:32-33`, `jsonrpc.rs` | Correct — returns `Result<InitializeResponse, Error>` |
| `cx.build_session_cwd()?.block_task().start_session().await` | ✅ `session.rs:51, 346, 409` | Correct — returns `ActiveSession<'static, Agent>` |
| `active_session.session_id().clone()` | ✅ `session.rs:530` | Correct — `&SessionId`, cloneable |
| `active_session.send_prompt(prompt)` | ✅ `session.rs:560` | Correct — `&mut self`, returns `Result<(), Error>` |
| `active_session.read_to_string().await` | ✅ `session.rs:591` | Correct — collects text chunks, returns `Result<String, Error>` |
| `cx.spawn(async { ... })` | ✅ `jsonrpc.rs:1498` | Correct — `IntoFuture<Output = Result<(), Error>>` |
| `AcpAgent::from_str(...)` | ✅ `acp_agent.rs` (tokio crate) | Correct — shell_words parsing |
| `SessionId::new(...)` | ✅ `schema/lib.rs:104` | Correct — `impl Into<Arc<str>>` |
| `AcpError::internal_error().data(...)` | ✅ `schema` + `util.rs` | Correct — error construction |

All SDK APIs are used correctly against the actual v0.11.1 source.

---

## Transport.rs changes (diff from Task 2 baseline)

The transport refactoring is well-structured:
- `spawn()` now returns `(Self, UnboundedSender<Command>)` — clean API expansion.
- `pub(crate)` visibility is correct — only `HermesClient` needs to call it.
- The oneshot handoff (`conn_tx.send(cx.clone())`) ensures `spawn` doesn't return until the connection is established.
- `Shutdown` and `Drop` both abort the handle — redundant but harmless (idempotent).

One minor nit: `spawn()` is `async` but the `conn_rx.await` could hang indefinitely if the connection fails before sending. The outer `tokio::spawn` catches connection errors via `error!` log, but the oneshot sender is dropped without sending on error, which causes `conn_rx.await` to return `Err(RecvError)`. The `.context("failed to receive ACP connection handle")` wraps this correctly.

---

## Summary

| Category | Count |
|---|---|
| Required fixes | 3 (F1: from_args, F2: error types, F3: pending) |
| Advisory items | 4 (A1: persistent sessions, A2: tests, A3: field naming, A4: debug log) |
| Spec deviations | All justified and documented |
| SDK API correctness | All verified against v0.11.1 source |
| Build | Clean |

The 3 required fixes are low-risk and can be addressed in a follow-up commit without changing the architecture.
