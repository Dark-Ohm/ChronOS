---
name: zed-acp-stdio
description: >
  Use when studying how Zed spawns and talks to external ACP agents over
  stdio — AcpConnection::stdio, initialize handshake, session list, JSON-RPC
  dispatch, or designing ChronOS hermes_acp subprocess wiring.
---

# Zed ACP stdio connection

**Source:** `crates/agent_servers/src/acp.rs` (~5k), trait glue in
`agent_servers.rs`, connection trait in `crates/acp_thread/src/connection.rs`  
**License:** GPL-3.0-or-later  
**Entry:** `zed-ai`

## Layers

```text
AgentServer::connect(delegate, project, cx)
    → Task<Rc<dyn AgentConnection>>
AcpConnection::stdio(agent_id, project, command, …)
    → spawn child (stdin/stdout/stderr piped)
    → line JSON-RPC + foreground dispatch bridge
    → initialize request/response
AgentConnection::new_session / load_session
    → Entity<AcpThread>
```

## `AgentServer` trait (`agent_servers.rs`)

- `connect` → `Rc<dyn AgentConnection>`
- optional defaults: mode, config options, favorites
- concrete servers: built-in + `custom` external commands

## `AcpConnection::stdio` (~L796)

1. Resolve command path/args/env (local vs remote client template).
2. `Child::spawn(…, Stdio::piped ×3)`.
3. Wrap stdout lines + stdin writer; stderr background task.
4. Debug message log channel.
5. **Initialize** handshake; fail if process exits first
   (`futures::select` initialize vs status ~L984).
6. Hold IO + dispatch tasks for connection lifetime.
7. Sessions map: `Rc<RefCell<HashMap<…>>>`.

Schema crate: workspace `agent-client-protocol` (`schema::v1 as acp`).

## `AgentConnection` trait (`connection.rs`)

Minimum ChronOS-relevant surface:

| Method | Purpose |
|---|---|
| `new_session` | create `AcpThread` |
| `load_session` | restore by `SessionId` (opt-in `supports_load_session`) |
| `agent_id` / `telemetry_id` | identity |
| auth helpers | `auth_methods`, logout, etc. |

Also: model list, cancel, prompt, permission responses — follow trait body
beyond L120 when implementing a client.

## Process model (Zed)

- **One connection per agent server type** (via connection store), not
  necessarily one OS process forever — reconnect paths exist on error.
- Multiple **ACP sessions** multiplexed on one connection.
- Project/work_dirs passed into session creation (`PathList`).

## ChronOS target (T107 shipped, T108 in progress — supersedes the
2026-07-22 single-process decision below)

`crates/services/src/hermes_acp/` (T107, accepted): `HermesTransport`
spawns via `HermesConfig { command, args }` (already parameterizable,
default `"hermes" ["acp"]`), `HermesClient` wraps one connection,
`#[derive(Clone)]`, sessions held per-client. Matches Zed's model above:
one connection per agent server, sessions multiplexed on it.

**Revised 2026-07-23 (`docs/DECISIONS.log`, T108):** left panel gets a
multi-agent switcher (Hermes/Cline/OpenCode/... — whichever actually
speak ACP stdio, verified not assumed). This replaces the older
"one process per ChronOS shell" model below with Zed's actual pattern —
**one connection per agent server *type*, lazily spawned**, not one
process total:

```text
services/hermes_acp
  registry: AgentDescriptor { id, display_name, config: HermesConfig }
  SidePanelLeft: HashMap<agent_id, HermesClient>, lazy-spawned on first use
  multi-session per HermesClient (unchanged from T107)
```

Superseded text (kept for history, do not follow): ~~one process per
ChronOS shell (user decision 2026-07-22)~~ — this was the original T107
scope before the switcher was restored to v1.

Do **not** depend on Zed's `Project` / remote SSH scaffolding — local spawn +
cwd from config is enough for shell v1.

## Common mistakes

- HTTP chat API ≠ ACP (different product).
- Forgetting initialize before new_session.
- Dropping child handles → agent dies mid-turn.
- Copying `acp.rs` into Apache tree — GPL; reimplement against public ACP
  schema / `agent-client-protocol` crate if license-compatible, or hand-roll
  JSON-RPC to the published protocol.
