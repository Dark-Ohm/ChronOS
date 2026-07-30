# Task 3: Hermes ACP Client + Session — Report

## Status: DONE

## Files created/modified

| File | Action |
|---|---|
| `crates/services/src/hermes_acp/client.rs` | Created — `HermesClient` wrapping transport with command channel |
| `crates/services/src/hermes_acp/session.rs` | Created — `AcpSession` wrapper around `SessionId` |
| `crates/services/src/hermes_acp/transport.rs` | Modified — added command channel, `ConnectionTo<Agent>` exposure |
| `crates/services/src/hermes_acp/mod.rs` | Modified — added re-exports for new types |

## Deviations from task brief

1. **No `Message` type:** The brief's `AcpSession` held `Vec<Message>` and `is_draft`. The ACP SDK doesn't expose a `Message` type for client-side session management — messages flow through `ActiveSession`'s internal channels. The `AcpSession` is a lightweight wrapper over `SessionId` only.

2. **Channel-based architecture:** The brief's `HermesClient` directly owned `HermesTransport` and called `SessionId::new()`. The actual ACP SDK creates sessions via `cx.build_session_cwd()` on a live connection context. The transport was refactored to use a command channel (`mpsc::UnboundedSender<Command>`) so the client can send commands to the background task where `ConnectionTo<Agent>` lives.

3. **No `Clone` on `AcpSession`:** The brief's `create_session` called `.clone()` on the session. Sessions in ACP are server-side state — cloning a session ID doesn't clone the session. `AcpSession` is `Clone` (wraps `SessionId` which is `Arc<str>`), but this is for convenience, not semantic cloning.

4. **Stateless prompt mode:** `send_prompt` creates a new session per prompt. This avoids the complexity of session lifetime management across the command channel boundary. Can be changed to persistent sessions later if needed.

5. **No `todo!()` in production code:** The brief's `send_prompt` had `todo!("Implement ACP prompt sending")`. The actual implementation creates a session, sends the prompt via `active_session.send_prompt()`, and reads the full response via `active_session.read_to_string()`.

## Architecture decisions

- **Command channel pattern:** `HermesTransport::spawn` returns `(Self, UnboundedSender<Command>)`. The transport's background task owns the `ConnectionTo<Agent>` (received via oneshot from the `connect_with` closure) and processes commands sequentially. This avoids lifetime issues with the connection context.

- **`cx.spawn` for session keepalive:** `create_session` spawns the `ActiveSession` into a background task via `cx.spawn()` to keep it alive. The task sleeps indefinitely — session cleanup happens when the connection drops.

- **Per-prompt sessions:** `send_prompt` creates a fresh session for each prompt. The session is dropped after reading the response. This is simple and avoids cross-command session state.

- **`ConnectionTo<Agent>` is `Clone`:** The ACP SDK's connection context derives `Clone`, allowing it to be sent through a oneshot channel from the `connect_with` closure to the command processing loop.

## Test results

```
cargo build --release -p chronos-services  — OK (0 hermes warnings)
cargo build --release                      — OK (1 unrelated mpris warning)
```

## Commit

```
83f8925 feat(hermes_acp): client and session management
```

## Concerns

1. **Session lifetime in `create_session`:** The spawned task keeps the session alive via an infinite sleep loop. If the connection drops, the task is aborted. This is correct behavior but means sessions don't survive transport restarts.

2. **Per-prompt session overhead:** Each `send_prompt` call creates a new ACP session (subprocess round-trip). For high-frequency prompting, consider maintaining a persistent session. The command channel architecture supports this — just change `SendPrompt` to reuse an existing session.

3. **Error type ergonomics:** All errors are `String`. The command channel uses `oneshot::Sender<Result<T, String>>`. For better error handling, consider using `anyhow::Error` or a dedicated error enum.

## Fix Report
- F1: Replaced `AcpAgent::from_str(&format!("{} {}", command, args))` with `AcpAgent::from_args([command, ...args])` in `transport.rs`. This avoids shell-word splitting bugs on args with spaces. Removed unused `use std::str::FromStr`.
- F2: Unified error types to `anyhow::Result` across `client.rs`. `Command` variants now carry `oneshot::Sender<anyhow::Result<T>>`, internal functions return `anyhow::Result`, and public `HermesClient::create_session`/`send_prompt` return `anyhow::Result`. All `map_err(|e| format!(...))` replaced with `.context(...)`.
- F3: Replaced `loop { tokio::time::sleep(3600s).await; }` with `std::future::pending::<()>().await` inside `Ok(())` wrapper (required by `cx.spawn` return type). Same effect — task lives forever — clearer intent.
- Build: `cargo build --release -p chronos` — OK
- Commit: `0748914`
