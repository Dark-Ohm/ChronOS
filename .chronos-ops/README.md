# ChronOS kitchen

Кухня **репозитория ChronOS**. Не копия экосистемной. Под git — контрибьюторы
берут задания отсюда, история не теряется.

- Эта: `ChronOS/.chronos-ops/`
- Экосистема (кросс-репо): `/home/neo/projects/chronos-ecosystem/.chronos-ops/`
  (вне git — там корень вообще не репо, другая нумерация, не путать)

Правила — `RULES.md`. Снимок очереди:

```
bash .chronos-ops/bin/kitchen-status.sh
```

## Что здесь живёт (с 2026-08-17)

- `active/` `done/` `reports-fresh/` `reports-log/` `rework/` `reject/` —
  очередь тикетов, по ролям.
- `checkpoint/` — канон, реальное содержимое (не указатели на `docs/`):
  `HANDOFF.md`, `ARCHITECTURE.md`, `ARCHITECT.md`, `TBD.md`, `SOUL.md`,
  `MEMORY.md`, `REJECTED.md` (бывший `docs/DECISIONS.log`, переименован —
  файл только про отклонённое, имя было двусмысленным).
- `design/` — макеты (`.dc.html` и т.п.), бывший `docs/design/`, перенесён
  as-is.
- `superpowers/` — planning-скиллы (`plans/`, `specs/`), бывший
  `docs/superpowers/`, перенесён as-is.

`docs/` остаётся продуктовой документацией: `PRODUCT.md`, `STYLE.md` и
остальное — то, что контрибьютору нужно читать как обычный проектный докс,
не как рабочее состояние архитектора.

## Cutover из `docs/orchestration/tasks/` — частичный

- **Новые тикеты** заводятся сразу в `active/<role>/`, точка входа роли —
  `active/<role>/<ROLE>.md` (не старый `docs/orchestration/agents/<ROLE>.md`,
  тот помечен как архив со ссылкой сюда).
- **Живые тикеты**, заведённые до cutover (T266, T271, T284, T285, T287,
  T298), остаются в `docs/orchestration/tasks/active/` до закрытия —
  не переносить на живой очереди, это race condition, который уже ловили.
- **Архив** (`done/`, `report-log/`, `rejected/`) переносится по мере
  разбора: 334 тикета уже в `.chronos-ops/{done,reports-log,reject}/<role>/`,
  167 непонятых эвристикой — на разборе в T299 (RECON), пока в
  `docs/orchestration/tasks/{done,report-log,rejected}/`.

## Известный долг

Кросс-ссылки на старые пути (`docs/HANDOFF.md`, `docs/ARCHITECTURE.md`,
`docs/DECISIONS.log`, `docs/ARCHITECT.md`) ещё не везде поправлены —
в первую очередь корневой `CLAUDE.md` и `docs/orchestration/agents/*.md`.
