---
name: chronos-llm-backends
description: Use when wiring or debugging an LLM backend for this machine — the local Chronos-Engine llama-server (turbo3/kvarn KV cache, reasoning off, VRAM fitting) or the OmniRoute gateway on :20128 (combo models, streaming default). Covers what Hindsight and other consumers must be pointed at, and the traps measured on 2026-07-29.
---

# LLM-бэкенды на этой машине

Два пути, оба живые. Выбор — по задаче, не по вкусу.

| Бэкенд | Адрес | Когда |
|---|---|---|
| **OmniRoute** (шлюз) | `http://localhost:20128/v1`, модель `hindsight-combo` | по умолчанию: быстро, не жрёт VRAM, фолбэк по провайдерам |
| **Chronos-Engine** (локально) | `http://localhost:11435/v1`, модель `ornith` | офлайн, приватность, эксперименты с KV-кэшем |

Запуск локального: `infra/hindsight/run-llm.sh ornith` (ключи, env-override
`PORT`/`NGL`/`CTX`/`CTK`/`CTV`/`REASONING`).

## OmniRoute (`:20128`)

- Ставится глобально (`/usr/lib/node_modules/omniroute`, симлинк
  `/usr/bin/omniroute`), поднимается `omniroute serve --daemon --no-open`.
- **Отдаёт SSE-поток по умолчанию.** Без `"stream": false` ответ приходит
  чанками `data: {...}`, и наивный `json.load` молчит. Если «нет ответа» —
  первым делом добавь `stream:false`, а не ищи поломку.
- **`hindsight-combo` есть в `/v1/models`, но `omniroute combo list` пуст**
  и `omniroute simulate` говорит «No matching combo found». Тем не менее
  вызов с `stream:false` работает и маршрутизируется (замер 29.07:
  `stepfun/step-3.7-flash`). Не пугаться расхождения CLI и API.
- Другие рабочие алиасы из списка: `llm`
  (`nvidia/nemotron-3-ultra-550b-a55b:free`), `coding`
  (`cohere/north-mini-code:free`), плюс прямые
  `gemini/gemini-3.1-flash-lite-preview` и ещё ~80.
- **Подхватывает `.env` из посторонних каталогов** — при запуске из
  `infra/hindsight` затянул наши переменные вместе с ключом Jina. Не
  запускать из каталогов с секретами.

## Chronos-Engine llama-server (`:11435`)

Форк умеет то, чего нет в стоке: **свои типы KV-кэша**.

- `turbo2/3/4`, `turbo*_tcq` — TurboQuant (WHT-ротация + PolarQuant/QJL).
  `turbo3` живой, по скорости ≈ f16 (`docs/ARCHITECTURE.md:132` форка).
- `kvarn2…8` — **не брать**: незакрытый Z-4, hot-window ломает retrieval
  внутри окна (включается только при `--kv-hot-size`, но не рисковать).
- **`q3` для KV не существует** ни в стоке, ни в форке. Стоковый набор:
  `f32, f16, bf16, q8_0, q4_0, q4_1, iq4_nl, q5_0, q5_1`.
- **Ollama даёт одну ручку** `OLLAMA_KV_CACHE_TYPE` на K и V сразу.
  Раздельные `-ctk`/`-ctv` — только прямым запуском llama-server.

Рабочая конфигурация на 3070 8GB + 64GB RAM (замерено 29.07):

```
-c 32768 -ctk q8_0 -ctv turbo3 --flash-attn on -ngl 24
```

- **`-ngl 999` не влезает**: 5.4 ГБ весов + 32k кэша > 8 ГБ,
  `failed to fit model parameters to device memory`. 24 слоя ≈ 5.5 ГБ на
  GPU, остальное на CPU — 6–8 t/s.
- `GGML_CUDA_REGISTER_HOST=1` при частичном выгрузе (как в
  `Chronos-Engine/models/main/run-server.sh`).

### Reasoning: главная ловушка

`ornith` — reasoning-модель, пишет `<think>` и **съедает бюджет вывода**,
из-за чего JSON обрывается на `finish_reason=length`. Именно это ломало
консолидацию Hindsight, а не размер контекста.

- Выключается `--reasoning off`, **но только вместе с `--jinja`**
  (родной шаблон модели). С `--no-jinja --chat-template chatml` шаблон
  подменяется, тумблера в нём нет, и флаг молча не действует — я потерял
  на этом час.
- `--chat-template-kwargs '{"enable_thinking":false}'` — deprecated (сервер
  сам предупреждает), а в паре с `--reasoning-budget 0` отдаёт **500
  Invalid input batch**. Не использовать.
- Замер эффекта: «2+2?» → **2 токена вместо 700+**.

## Что выбрать для Hindsight

Шлюз. Цифры одного и того же документа (29.07):

| Бэкенд | retain одного документа |
|---|---|
| Ollama, контекст 4096 | падал: `finish_reason=length`, битый JSON |
| Chronos-Engine, ornith, reasoning off | ~500 c, часто таймаут |
| OmniRoute `hindsight-combo` | **117 c, success** |

Локальный держим как запасной путь и как площадку для turbo3/kvarn.

## Ловушки процесса

1. **`pkill -f "llama-server --model …"` убивает твою же оболочку** —
   строка совпадает с командной строкой `pkill`. Только `pgrep -x
   llama-server` → `kill` по PID.
2. **Не перезапускать бэкенд, пока по нему идёт запись.** Три раза за
   вечер убил сервер в момент фонового `retain` — пачка терялась целиком.
3. **`kill -TERM` может не сработать сразу**, если сервер занят запросом:
   он держит VRAM, и новый инстанс падает на `insufficient memory`.
   Ждать освобождения (`nvidia-smi`) или добивать `-KILL`.
4. Модель лежит блобом ollama (`~/.ollama/models/blobs/sha256-…`), имя
   файла = хеш. Путь зашит в `run-llm.sh`.
