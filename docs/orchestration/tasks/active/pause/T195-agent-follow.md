# T195 — «Следить за агентом» + live activity справа

**Статус:** BLOCKED — после T194 желательно (open file in Editor); можно
частично раньше (activity only).
**Роль:** FRONTEND (+ thin BACKEND if event bus). **Модель: Sonnet 5**.
**Канон:** `docs/PRODUCT.md` §2 Agent.

**Зона:**
- left panel agent UI (toolbar button Follow)
- right panel: activity strip / auto-open Editor path
- hermes_acp / tool events — **read** stream; minimal new global
  `AgentFollowState` or similar

**НЕ:** new agent backend; multi-agent registry; scenes.

**Отчёт:** `report/T195-agent-follow-report.md`.

---

## Цель

1. Кнопка **Follow** на левой панели (toggle).
2. Когда Follow on и агент трогает файл / tool path — правая панель:
   - показывает **lightweight activity** (last tool / path / status)
   - открывает path в Editor (T194) если text
3. Follow off — агент работает, right не прыгает.

UX «как везде» — не R&D science project. Empty when idle.

## Верификация

Live: turn Follow on → agent edits config → right shows path + content.

Коммит: `agent : follow mode + right activity (T195)`.
