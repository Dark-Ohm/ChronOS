# Hindsight + llama-server local infra (verified commands)

Companion to the `chronos-shell` SKILL.md "Local AI infrastructure" section.
Concrete, re-runnable commands — not a mirror of upstream docs.

## Podman stack (Hindsight)
- Containers: `hindsight-db` (pgvector pg18), `hindsight-embeddings`,
  `hindsight-reranker` (TEI), `hindsight-api` (dataplane :8888),
  `hindsight-cp` (Next.js :9999), `hindsight-nginx` (proxy :8080→:8081).
- From host, only `:8080` (nginx) is reachable. `hindsight-api` etc. resolve
  ONLY inside the podman network. Dataplane is reached via nginx.

## Health probes (host side)
```bash
curl -s -o /dev/null -w "%{http_code}\n" http://localhost:8080/health      # 200
curl -s http://localhost:8080/version                                     # {"api_version":"0.8.4",...}
curl -s -o /dev/null -w "%{http_code}\n" http://localhost:8085/v1/models   # llama-server
```

## Hermes memory-provider wiring
```bash
# 1. install client in Hermes venv
cd ~/.hermes/hermes-agent && source venv/bin/activate
pip install "hindsight-client>=0.6.1"

# 2. config: ~/.hermes/profiles/<profile>/hindsight/config.json
#    {"mode":"local_external","api_url":"http://localhost:8080",
#     "bank_id":"chronos-ecosystem","memory_mode":"hybrid",
#     "recall_budget":"mid","auto_retain":true,"auto_recall":true}

# 3. enable (direct config.yaml edits are blocked)
hermes config set memory.provider hindsight
hermes memory status      # expect "Provider:  hindsight"

# 4. round-trip test
hermes -p <profile> chat -q "recall Chronos; retain: 'Chronos is a Hyprland/Niri shell in Rust+Kael 0.3'."
```

## Bank management API (via nginx :8080)
```bash
# list
curl -s http://localhost:8080/v1/default/banks
# create
curl -s -X PUT http://localhost:8080/v1/default/banks/chronos-ecosystem -d '{"bank_id":"chronos-ecosystem"}'
# mission (top-level)
curl -s -X PATCH http://localhost:8080/v1/default/banks/chronos-ecosystem \
  -d '{"mission":"shared memory for Chronos ecosystem","background":"..."}'
# mission (config-level — check api log to confirm it landed)
curl -s -X PATCH http://localhost:8080/v1/default/banks/chronos-ecosystem/config \
  -d '{"retain_mission":"...","observations_mission":"...","reflect_mission":"..."}'
# export -> import (migrate data between banks)
curl -s -X GET "http://localhost:8080/v1/default/banks/zed/document-transfer?include_observations=true" -o zed.zip
curl -s -X POST "http://localhost:8080/v1/default/banks/chronos-ecosystem/document-transfer?on_conflict=skip" -F "file=@zed.zip"
# counts
curl -s "http://localhost:8080/v1/default/banks/chronos-ecosystem/memories/list?limit=1" # -> {"total": N}
```

## llama-server (beellama) — known-good flags
```bash
llama-server \
  --model ~/Projects/chronos-ecosystem/llama-engine/models/Agents-A1-Q4_K_M.gguf \
  --chat-template-file .../models/chat_template.jinja --jinja \
  --n-gpu-layers 99 --n-cpu-moe 35 \
  --threads 6 --flash-attn on --mlock --no-mmap \
  --ctx-size 32768 \
  --cache-type-k q4_0 --cache-type-v q4_0 \
  --kv-unified --cache-ram 16384 --cache-idle-slots \
  --host 0.0.0.0 --port 8085 \
  --parallel 4 --cont-batching --timeout 300 \
  --metrics --sleep-idle-seconds 300 \
  --reasoning off --reasoning-budget 0
```
- DO NOT use `--cache-type-v turbo2` → CUDA illegal memory access crash.
- MoE on GPU (99/35) = 40–45 t/s; on CPU (40) = 5–13 t/s.

## Failure triage
- Retain tasks `STUCK?` with `claimable=0`, `stage=llm.openai.retain_extract_facts`
  → LLM endpoint (`:8085`) down or slow. Probe `/v1/models` + `/v1/chat/completions`.
- `hermes mcp add` of Hindsight is WRONG — it's a memory provider, not an MCP server.
- Zed connects via MCP: `npx -y mcp-remote http://localhost:8080/mcp/chronos-ecosystem/`.
