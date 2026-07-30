# T138 report — Multi-agent registry + shared .env

**Status:** implementer complete — infrastructure green; live smoke PENDING second agent binary.

## What changed

| Area | Change |
|------|--------|
| registry.rs | `AgentDescriptor` owns `String` fields (was `&'static str`); new `load_shared_env()`, `parse_env_file()`, `load_config_agents()` |
| registry.rs | `known_agents()` merges builtin Hermes + config entries (config with same id overrides builtin) |
| transport.rs | `spawn()` accepts `shared_env: HashMap<String, String>` — passed as leading `KEY=value` args |
| client.rs | `AcpClient::new()` accepts `shared_env`, forwards to `spawn()` |
| mod.rs | re-exports `load_shared_env` |
| side_panel_left/mod.rs | loads shared env once at panel creation |
| Cargo.toml | added `dirs = "6"`, `toml = "0.8"` |

## Design decisions

1. **Additive registry.** Hermes always present; config entries add more. A config
   entry with `id = "hermes"` overrides the builtin — no separate `builtin` flag.
2. **Single shared `.env`** at `~/.config/chronos/.env`. All API keys in one place.
   Passed as leading env args to every agent spawn, not stored on `HermesConfig`.
3. **Config lives on disk** — no in-app editor for v1. Users edit
   `~/.config/chronos/agents.toml` directly. Reload is panel re-open.
4. **`AgentDescriptor` owns Strings** — config-loaded agents need heap-allocated
   `id`/`display_name`; `&'static str` is wrong for this.

## agents.toml schema

```toml
[[agents]]
id = "vibe-acp"
display_name = "Vibe"
command = "vibe-acp"
args = ["--headless"]

[[agents]]
id = "claude-code"
display_name = "Claude Code"
command = "claude"
args = ["--acp"]
```

## Verify

```text
cargo check -p chronos          # clean (T138 + T140–T142)
cargo check -p chronos-services # clean

# Unit tests for parse_env_file:
cargo test -p chronos-services -- hermes_acp::registry

# Live smoke (requires second ACP binary):
# 1. Create ~/.config/chronos/.env with ANTHROPIC_API_KEY=sk-...
# 2. Create ~/.config/chronos/agents.toml with a real entry
# 3. Reopen panel → should show "Hermes" + configured agent
# 4. Switch → spawns correct command
```

## Commit

`acp : multi-agent registry with agents.toml + shared .env (T138)`

## Still open (not T138)

- Live smoke with second ACP binary (Architect must provide)
- UI agent menu / Super+A polish
- Visual identity (T139)

## Architect verdict 2026-07-26T18:07:28+03:00
**Architect: ACCEPTED WITH CAVEATS** (82405c3; second-agent live PENDING). Smoke test fixed post-review.
