# T142 — ACP model list + picker + set_model

**Статус: OPEN. После T140; можно параллельно с T141 если зоны не дерутся.**  
**Канон:** revive design; composer already has model dropdown UI.

| | |
|---|---|
| **Skills** | `chronos-shell` |
| **Зоны** | `hermes_acp/{client,session}.rs`, `side_panel_left/composer.rs` |
| **Отчёт** | `docs/orchestration/tasks/report/T142-acp-model-picker-report.md` |
| **Коммит** | `acp : wire model list and set_model (T142)` |

## Контекст

- Composer `model_picker`: shows only if `!available_models.is_empty()`  
  (`composer.rs` ~245). Currently empty → muted placeholder, no choice.
- T137 extracts `session.response().models` with feature
  `unstable_session_model` — Hermes may still return `None` and put
  models only in `config_options` / later `ConfigOptionUpdate`.
- User: «выбора модели нет».

## Цель

1. На connect / after session: **non-empty model list when agent provides any**.
2. Dropdown works; selecting model calls ACP **`session/set_model`**.
3. If agent never sends models — honest UI («Model: default») + report, **no fake list**.

## Задачи

### Task 1 — Discover source of models (evidence first)

On live Hermes create_session, log full:
- `response.models`
- `response.config_options` (if present on NewSessionResponse)
- any `ConfigOptionUpdate` mid-session

Report table: which path Hermes actually fills (2026-07).

### Task 2 — Fill `available_models`

- Keep `SessionModels` path if present.
- Else map config_options whose id/name looks like model (document heuristic)
  **or** skip if not reliable.
- On prompt response, refresh models if present in response.

### Task 3 — `set_model` command

```rust
// client Command + execute on active session connection
// schema: SetSessionModelRequest (feature unstable_session_model)
HermesClient::set_model(&self, model_id: &str) -> Result<()>
```

Composer on model option click: call set_model; on Err show agent error message
in thread or toast log; don't clear list.

### Task 4 — Verify

```bash
# Super+A → Model dropdown clickable if list non-empty
# Select other model → log set_model OK; next prompt uses it (agent-dependent)
# If Hermes has no models API: report ACCEPTED WITH CAVEAT + UI string
```

## Accept

- [ ] Evidence log of what Hermes returns for models.
- [ ] If models exist: picker works + set_model called.
- [ ] If not: no fake models; UX explains; report caveats.

## Out of scope

- Provider settings UI / API keys.
- Multi-agent (T138).
