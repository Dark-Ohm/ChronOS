# T138 — Multi-agent registry + add (verified ACP only)

**Статус: OPEN. После T140–T142 green on Hermes (tools usable).**  
**Канон:** revive design Phase B; DECISIONS 2026-07-23 multi-agent switcher.

| | |
|---|---|
| **Skills** | `chronos-shell` |
| **Зоны** | `hermes_acp/registry.rs`, optional `~/.config/chronos/agents.toml`,  
| | `side_panel_left` switcher only if needed |
| **Отчёт** | `orchestration/tasks/report/T138-acp-multi-agent-registry-report.md` |
| **Коммит** | `acp : multi-agent registry from config (T138)` |

## Контекст

- `known_agents()` = **Hermes only**. Switcher UI exists (T108) but useless.
- Host candidates (verify, don't assume): `hermes`, `vibe-acp`, `claude`, …
- Grok: **only** if real ACP stdio binary exists — no stub entry.
- T107/T108 not reopened; this extends registry + config.

## Цель

≥2 **handshake-verified** agents in list; switch spawns correct command;
config-backed add path (file edit + reload OK for v1).

## Задачи

1. `agents.toml` schema: `[[agent]] id, display_name, command, args[]`.
2. Builtin Hermes default; merge config.
3. Optional: `chronos agents smoke <id>` or unit that runs Initialize only.
4. Switch agent clears UI sessions + new client (already partial).
5. Document Super+A agent menu.

## Accept

- [ ] Second real agent after live handshake (name + binary in report).
- [ ] Switch Hermes ↔ other works; chat still sends (T137 path).
- [ ] Unknown binary → Disconnected + error, not crash.
- [ ] No fake "Grok" without ACP.

## Out of scope

- Visual ChronOS character (T139).
- Permission UI (T140 already auto).
