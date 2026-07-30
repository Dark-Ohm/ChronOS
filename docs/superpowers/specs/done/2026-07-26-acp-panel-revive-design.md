# ACP Left Panel Revive — design

_2026-07-26. Product feedback: chat dead; cannot add agents (e.g. Grok);
UI is Zed-shaped without ChronOS character; more TBD._

## Problem (evidence, not vibes)

### 1. Chat “doesn’t work”

| Layer | Reality in tree |
|-------|-----------------|
| Connect | `HermesClient::new` + `create_session` on panel open — often **OK** (`ACP client connected` in log). |
| Sessions UI | Local UUIDs only — **not** ACP `session_id`. Switch/create session does **not** talk to the agent. |
| Send | `HermesClient::send_prompt` **creates a new ACP session every prompt** (`client.rs` comment: “stateless”). No thread continuity. |
| Streaming | Blocking `read_to_string` — no token stream, no tool live updates. |
| Transport | `Hermes ACP command channel closed` after open in several log bursts — process/task dies; send then fails with `command channel closed`. |
| UX open | Super+A opens **sidebar strip only** (`SIDEBAR_MIN_WIDTH` ~46px). Chat needs pull-open or Dock. Easy to think “panel broken” when composer isn’t even on screen. |

### 2. “Can’t add other agents”

- `known_agents()` = **only Hermes** (`registry.rs`).
- Switcher UI exists (T108) but list length 1 → useless.
- Registry comment correctly forbids “declared but untested” entries — so Grok/Claude/Cline need **handshake smoke**, not a label.
- Host has: `hermes`, `vibe-acp`, `claude`, `agent` under `~/.local/bin` — candidates after verify.
- “Add me (Grok)” is product: either a real ACP-capable Grok bridge **or** config row that points at a verified command — not a fake entry.

### 3. “View stolen from Zed, no character”

- Layout/chrome follow Zed agent panel patterns (sessions rail + thread + composer) by design skill path — functional copy without ChronOS identity.
- Mockups exist: `design/Agent Panel.dc.html`, `design/Agent Thread.dc.html` — not fully reflected; empty thread is “huge void” (TBD).
- Wanted: shell chrome (theme/surfaces/elevation already on left), denser ChronOS language, not second Zed.

## Non-goals (this front)

- Full IDE (Files tab T115 stays PAUSE).
- Subagents / @mentions / inline diffs.
- Replacing Hermes as default.
- Bar drag (T135) / hotview (T136).
- Plasma multi-applet editor.

## Phases (updated 2026-07-26 after live chat)

| Phase | T | Status | What |
|-------|---|--------|------|
| A | **T137** | **DONE** (`af54fb0`) | Session reuse, usage_update, open width — **chat works** |
| A2 | **T140** | OPEN P0 | `session/request_permission` auto-approve — tools run |
| A3 | **T141** | OPEN | Parse thought + tool updates → ToolCard / reasoning UI |
| A4 | **T142** | OPEN | Model list + set_model (if Hermes provides models) |
| B | **T138** | OPEN | Multi-agent registry (verified ACP only) |
| C | **T139** | OPEN | ChronOS visual character |

### Live follow-ups (user 2026-07-26, post-T137)

- Chat OK multi-turn.
- Tools fail: `Edit approval denied by ACP client` — **no permission handler** on Client builder (see SDK `yolo_one_shot_client.rs`).
- UI YOLO mode ≠ ACP permission (session mode only).
- `read_to_string` drops `AgentThoughtChunk` / `ToolCall` / `ToolCallUpdate`.
- Model picker empty when `available_models` empty.

### Phase A — T137 Chat (shipped)

Session hold in transport loop; `unstable_session_usage`; Super+A chat width;
`--accept-hooks`. Smoke: two prompts same `session_id`.

### Phase A2 — T140 Permissions (next implementer)

Register `on_receive_request(RequestPermissionRequest)` → prefer
`AllowAlways` / `AllowOnce`. Brief: `active/T140-acp-permission-auto-approve.md`.

### Phase A3 — T141 Stream UI

Custom `read_turn` → text + thought + tools; wire `ToolCard` + reasoning block.
Brief: `active/T141-acp-tools-and-reasoning-ui.md`.

### Phase A4 — T142 Models

Evidence what Hermes returns; picker + `session/set_model`.
Brief: `active/T142-acp-model-picker.md`.

### Phase B — T138 Multi-agent

`agents.toml`, handshake-only second agent. No fake Grok.
Brief: `active/T138-acp-multi-agent-registry.md`.

### Phase C — T139 Character

Density/mockup, not protocol. Brief: `active/T139-acp-chronos-character.md`.

## Architecture notes

- Keep `HermesClient` name debt or rename later to `AcpClient` — **not** Phase A blocker.
- GPL: steal **patterns** from Zed skills only; zero line copy.
- Streaming (token/tool events) can be Phase A.1 if blocking send already works; otherwise A first without stream.

## Verification

```bash
chronos-rebuild && chronos-stop && chronos-start
# Super+A → chat column visible without archaeology
# type + send → reply in thread (log + grim)
# second message → same session (agent context)
# agents.toml / switcher — Phase B
```

## Open questions (defaults if silent)

1. Super+A default: **open chat ~420px** (Recommended) vs keep rail-only.
2. Grok: **config slot after ACP exists** vs block Phase B until Grok ACP found.
3. Streaming: **after first green round-trip** (Recommended) vs same PR as session fix.
