# T137 report — ACP chat must work

**Status:** implementer complete — live Hermes smoke green; UI smoke PENDING user.

## Root cause

1. **Stateless send:** every `send_prompt` called `start_session` → no multi-turn.
2. **usage_update:** Hermes 0.18 sends `sessionUpdate: usage_update`; ACP schema
   without `unstable_session_usage` failed deserialize → `read_to_string` error
   → chat looked dead even when agent replied.
3. **Open UX:** Super+A used `SIDEBAR_MIN_WIDTH` (~46px) — composer often invisible.

## Fix

| Area | Change |
|------|--------|
| client | Hold `ActiveSession` in transport command loop; CreateSession replaces; SendPrompt reuses |
| Cargo | `unstable_session_usage` + existing `unstable_session_model` |
| transport | hermes args `acp --accept-hooks` |
| UI open | `DEFAULT_CHAT_WIDTH` (352) on Super+A |
| New session | clears chat + `create_session` on agent |
| errors | disconnect status on channel closed |

## Verify

```text
CHRONOS_SMOKE_HERMES_ACP=1 cargo test -p chronos-services --release -- --ignored smoke_hermes
# ok: same session_id, r1_chars=5 r2_chars=5

cargo test -p chronos --bin chronos -- side_panel_left::tests  # 7 ok
release build + open Super+A:
  layer w≈372, log "ACP session started", "ACP client connected"
```

## Commit

`acp : stable session + send path for left panel (T137)`

## Still open (not T137)

- Streaming tokens / tool cards live
- Multi-agent (T138)
- Visual identity (T139)
- User click-confirm composer Enter in UI
