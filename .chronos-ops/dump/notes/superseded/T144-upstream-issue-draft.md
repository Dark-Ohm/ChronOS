> **ВЫТЕСНЕНО. НЕ ОТПРАВЛЯТЬ.** Черновик тела апстрим-issue из T144, захода 1.
> Issue отправлен архитектором 2026-07-28 с переписанным телом:
> **https://github.com/agentclientprotocol/rust-sdk/issues/301**
>
> Почему переписан — две неточности, обе ниже по тексту:
> 1. сигнатура `NewSessionResponse<'a, ModuleId>` выдумана, дженериков в
>    крейте нет;
> 2. «with_debug you can observe `configOptions`» — мы их не наблюдали ни
>    разу, Hermes 0.18.2 шлёт старый `models`.
>
> Лежит здесь как след метода, а не как исходник для отправки.

**Repo:** https://github.com/agentclientprotocol/rust-sdk
**Title:** ActiveSession.response() discards config_options from NewSessionResponse

`ActiveSession.response()` rebuilds `NewSessionResponse` from internal fields but omits `config_options`. Any client that relies on `response()` (e.g. to find model selectors or other session config) gets `config_options: None` regardless of what the agent originally sent.

## Versions

- `agent-client-protocol` 2.0.0
- `agent-client-protocol-schema` 1.5.0

## Evidence

`NewSessionResponse` (in `v1/agent.rs:1087-1112`) has four fields:

```rust
pub struct NewSessionResponse {
    pub session_id: SessionId,
    pub modes: Option<SessionModeState>,
    pub config_options: Option<Vec<SessionConfigOption>>,
    pub meta: Option<Meta>,
}
```

But `ActiveSession::response()` (in `session.rs:570-574`) only returns three:

```rust
pub fn response(&self) -> NewSessionResponse<'a, ModuleId> {
    NewSessionResponse::new(self.session_id.clone())
        .modes(self.modes.clone())
        .meta(self.meta.clone())
}
```

Note: no `.config_options(...)` call. The field is never persisted — `config_options` is not stored in the `ActiveSession` struct at all (grepping `session.rs` for `config_options` returns zero matches).

## Impact

Any client that works through `ActiveSession` — the primary high-level API — cannot display or interact with session config options (model selectors, etc.). The client must either:

1. Intercept and parse the raw JSON-RPC wire traffic (brittle, duplicates protocol parsing), or
2. Make a separate `session/get` request and maintain its own cache.

Neither is desirable.

## Expected

`ActiveSession` should store `config_options` from the original `NewSessionResponse` and include them in `response()`, same as it already does for `modes` and `meta`.

## Minimal reproduction

```rust
// Assuming a connected session:
let session: ActiveSession<'static, Agent> = /* ... */;
let resp = session.response();
assert!(resp.config_options.is_none(), "config_options must be preserved");

// But the agent may have sent them. With with_debug you can observe:
// {"jsonrpc":"2.0","result":{"sessionId":"...","configOptions":[...]}}
```
