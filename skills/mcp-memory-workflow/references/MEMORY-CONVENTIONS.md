# Memory Layer Conventions — Chronos Project

Project-specific правила использования трёх слоёв памяти (lean-ctx, engram, codebase-tools), чтобы не разводить бардак.
Написано после того, как память пришлось чистить с нуля: 38 записей, 5 имён проекта, дубли, устаревшие пути.

> Базовый протокол — в `SKILL.md` (mcp-memory-workflow). Этот файл — дополнение под конкретные project-скопы Chronos.

## Три слоя — что куда класть

| Слой | Инструменты | Что хранит | Когда писать |
|---|---|---|---|
| **lean-ctx** | `ctx_session`, `ctx_*` | Состояние сессии, прогресс задач, находки, решения | В начале/конце каждой сессии, после каждой задачи |
| **engram** | `mem_save`, `mem_search`, `mem_*` | Долгосрочные факты: решения, багфиксы, паттерны, конфиг | После завершения задачи, при важных находках |
| **codebase-tools** | `index_*`, `search_*`, `trace_*` | Граф кода: функции, классы, вызовы | При необходимости структурного поиска |

**Золотое правило:** lean-ctx = рабочее состояние сессии, engram = долгосрочное знание, codebase-tools = структура кода. Не мешать.

**Важно:** `ctx_knowledge` в этой среде НЕДОСТУПЕН — не вызывать. Используй `ctx_session` + `mem_save`.

---

## Project-скопы (ОБЯЗАТЕЛЬНО)

В engram каждая запись ДОЛЖНА иметь правильный `project`. Для этого репозитория — только эти значения:

| `project` | Что | Путь |
|---|---|---|
| `chronos` | Desktop shell + shell-layer (bar/dock/launcher/notifications/osd) | `/home/neo/Projects/chronos` |
| `Chronos-IDE` | AI-native IDE (исторический workspace, Vessel Engine) | `/home/neo/Projects/Chronos-IDE` |
| `chronos-fm` | File manager (launcher × explorer) | `/home/neo/Projects/chronos-fm` |
| `hyprland` | System-level Lua-конфиг композитора | `~/.config/hypr/` |

**Запрещено:**
- `chronos-shell` как отдельный project — shell-layer это часть `chronos`
- `Chronos-IDE` / `chronos-ide` / `chronos` как одно и то же — это РАЗНЫЕ проекты
- Любые другие вариации регистра/названия

---

## topic_key конвенция (engram)

Каждая запись в engram ДОЛЖНА иметь `topic_key` в формате `<project>/<topic>`.
Это даёт upsert-поведение: повторное `mem_save` с тем же `topic_key` обновляет запись, а не плодит дубли.

Примеры:
- `chronos/bar-widget-contract`
- `chronos/luau-plugin-layer`
- `chronos/shell-gpui-fixes`
- `Chronos-IDE/vessel-core-dna`
- `chronos-fm/identity`
- `hyprland/config`

**Не используй** свободные topic_key типа `architecture/skeleton` — всегда с префиксом project.

---

## Что сохранять в engram (и что НЕТ)

**Сохранять:**
- Архитектурные решения (`type="decision"` или `type="architecture"`)
- Багфиксы с root cause (`type="bugfix"`)
- Подтверждённые API-паттерны (`type="pattern"` или `type="learning"`)
- Изменения конфига/окружения (`type="config"`)
- Identity проекта (стек, приоритеты, module scope)

**НЕ сохранять:**
- Промежуточные шаги задачи (только результат)
- Trivial-правки (опечатки, форматирование)
- Состояние прогресса задачи (это в `ctx_session`, не в engram)
- То, что легко переоткрыть (`git log`, чтение кода)

---

## lean-ctx протокол (для этой среды)

### Session Start (каждый запуск)
```
ctx_session(action="load")          # восстановить состояние
mem_session_start(id="chronos-YYYY-MM-DD", project="chronos")
mem_search(query="chronos")         # освежить контекст
index_status(project="chronos")     # если stale → index_repository
```

### После каждой задачи
```
ctx_session(action="task", value="<что сделано>")
# если неочевидное открытие:
ctx_session(action="finding", value="<что узнал>")
# если принято решение:
ctx_session(action="decision", value="<что и почему>")
```

### Session End (обязательно)
```
ctx_session(action="save")
```

---

## codebase-tools протокол

- `index_repository` — один раз при первом структурном запросе
- Перед `search_graph` / `trace_path` — всегда `index_status`
- Переиндексировать только если структура crates сильно изменилась

---

## Анти-бардак чеклист

Перед `mem_save` спроси себя:
1. **Project-скоп правильный?** (только `chronos` / `Chronos-IDE` / `chronos-fm` / `hyprland`)
2. **Есть topic_key?** (формат `<project>/<topic>`)
3. **Это дубль?** (поискал ли `mem_search` с тем же topic_key перед записью?)
4. **Пути актуальны?** (текущий `chronos` workspace использует `crates/`, НЕ плоский layout)
5. **Это долгосрочный факт, а не прогресс задачи?** (прогресс → в `ctx_session`)

Перед удалением записи:
- Только soft-delete (`mem_delete`) — не физически
- Не удаляй чужие project-скопы без причины
- Если сомневаешься — оставь, пометь `topic_key` и обнови, не удаляй

---

## История (почему это написано)

2026-07-10 память была в бардаке: 38 записей, 5 имён проекта (`chronos`, `Chronos-IDE`, `chronos-ide`, `chronos-shell`, `chronos-fm`), дубли (#26/#28 embeddings, #29/#30 autocomplete), устаревшие плоские пути (`chronos-memory/src/` вместо `crates/chronos-memory/src/`).

Чистили с нуля: удалили все 38, пересоздали 18 чистых записей по 4 project-скопам.
Этот гайд — чтобы не повторять.
