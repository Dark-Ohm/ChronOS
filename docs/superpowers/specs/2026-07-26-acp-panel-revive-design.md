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

## Phases

### Phase A — **T137 Chat must work** (P0, ship first)

1. **Diagnose** live: open panel, expand chat, send one prompt, capture log (`composer: send`, `ACP send failed`, hermes stderr).
2. **Session model fix:** one ACP session per UI session (create once; reuse on send). Wire `active_session_id` to real ACP id or map UI session → held `ActiveSession` handle.
3. **Transport stability:** why command channel closes after connect; keep task alive; surface disconnect in UI (status + toast/message), not silent dead send.
4. **Open path:** Super+A opens to **usable chat width** (or last width / dock-on default) — not rail-only that hides composer.
5. **Send path UX:** focus composer on open; Enter-to-send reliable; errors visible in thread.
6. **Accept:** user prompt → agent text in thread; second prompt continues same session; restart panel reconnects cleanly.

### Phase B — **T138 Multi-agent registry** (after A green)

1. Config `~/.config/chronos/agents.toml` (or section in existing): `[[agent]] id, display_name, command, args`.
2. Built-in list: Hermes + **only** backends that pass a unit/smoke handshake helper.
3. UI: switcher lists registry; **Add agent** dialog or “edit config + reload” (prefer config + hot-reload over fake wizard v1).
4. Grok path: research which binary exposes ACP stdio; if none, document “not yet” — **no stub**.
5. Accept: ≥2 real agents switchable; sessions cleared/isolated on switch (already partially there).

### Phase C — **T139 ChronOS character** (parallel-ok after A starts, not before chat works)

1. Align empty/thread density with mockup + right panel language (elevation, gaps, mono accents).
2. Header/status: ChronOS badge language (EDIT-mode caliber), not generic IDE.
3. Composer: caret, density, send affordance — still not full Zed TextInput port unless required.
4. Accept: grim vs mockup; user “feels ChronOS” not “feels Zed port”.

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
