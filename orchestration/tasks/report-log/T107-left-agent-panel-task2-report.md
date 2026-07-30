# Task 2: Hermes ACP Service — Transport — Report

## Status: DONE

## Files created/modified

| File | Action |
|---|---|
| `crates/services/src/hermes_acp/transport.rs` | Created — `HermesTransport` wrapping ACP `AcpAgent` subprocess spawn |
| `crates/services/src/hermes_acp/mod.rs` | Created — module re-exports |
| `crates/services/src/lib.rs` | Modified — added `pub mod hermes_acp;` |
| `crates/services/Cargo.toml` | Modified — added `agent-client-protocol` + `agent-client-protocol-tokio` deps |

## Deviations from task brief

1. **SDK versions:** Task brief assumed `agent-client-protocol = "0.5"` and `agent-client-protocol-tokio = "0.5"`. Actual available versions are `0.11.1` / `0.11.1`. Used real versions.

2. **SDK API is version-gated:** The tokio crate (`agent-client-protocol-tokio` v0.11.1) depends on `agent-client-protocol ^0.11.1`, not v1.3.0 (the latest core). Using the latest core would cause a version conflict. Pinned to 0.11.1.

3. **No `StdioTransport` type:** Task brief referenced `agent_client_protocol_tokio::StdioTransport::spawn()`. The actual type is `agent_client_protocol_tokio::AcpAgent` which parses command strings and handles subprocess spawning. The `Stdio` type is for wrapping current-process stdin/stdout, not spawning.

4. **`Client` is a unit struct:** Task brief used `Client::builder()` (associated function syntax). The actual API is `Client.builder()` (instance method on a unit struct). Both work since `Client` is zero-sized, but the compiler required instance syntax.

5. **Error type mismatch:** The ACP SDK uses its own `agent_client_protocol::Error`, not `anyhow::Error`. The `connect_with` closure must return `Result<_, AcpError>`. Added explicit error conversion.

6. **`schema::v1` doesn't exist in 0.11.x:** The 0.11.x schema module is flat — `InitializeRequest` and `ProtocolVersion` are directly in `schema`, not under `schema::v1`.

7. **Module structure simplified:** Task brief suggested `hermes_acp/client.rs` and `hermes_acp/session.rs`. These are deferred to later tasks — only `transport.rs` is implemented here.

## Architecture decisions

- **Background tokio task:** The ACP connection runs in `tokio::spawn` so it doesn't block the shell's main thread. The `HermesTransport` struct holds the join handle and aborts on drop.

- **`std::future::pending` keepalive:** After initialization, the `connect_with` closure blocks on `pending()` to keep the ACP event loop alive indefinitely. The connection stays open until the transport is dropped.

- **ACP 0.11.x over 1.3.0:** The tokio integration crate hasn't been updated for 1.x yet. Using 0.11.1 is the only viable path with tokio.

## Test results

```
cargo check -p chronos-services   — OK
cargo check -p chronos             — OK (30 pre-existing warnings)
```

## Commit

```
e33ff4f feat(hermes_acp): transport layer with stdio spawn
```

## Concerns

1. **ACP SDK async runtime:** The ACP core SDK uses `async-io` internally while ChronOS uses tokio. These coexist (smol/async-io can work alongside tokio), but if issues arise at runtime (e.g., blocking the tokio reactor), we may need to run the ACP event loop on a separate thread with its own smol runtime.

2. **No message forwarding yet:** The transport establishes the connection but doesn't yet bridge shell requests to ACP messages. That's the session/client layer's job (next tasks).

3. **ACP 0.11.x is not latest:** The core SDK is at 1.3.0 but we're pinned to 0.11.1 for tokio compatibility. If/when `agent-client-protocol-tokio` gets updated to 1.x, we should upgrade.
