# T196 — System settings + ACP agents CRUD

**Статус:** BLOCKED — после T192 (labels/rail). Can split later if huge.
**Роль:** FRONTEND (+ services if config store). **Модель: GLM 5.2 / Sonnet**.
**Канон:** `docs/PRODUCT.md`.

**Зона:**
- `PanelTab::EditorSettings` content → **System settings** surface (or new
  module `tab/system_settings.rs` — not System hardware tab)
- `PanelTab::AcpSettings` → add/remove/list ACP agents (endpoints)
- persistence: existing config paths / hermes config — **document**; no
  invent cloud

**НЕ:** LSP/MCP UI; full OS control center in one go — MVP slices:
  shell theme link, paths, agent list.

**Отчёт:** `report/T196-system-and-acp-agents-report.md`.

---

## Цель MVP

### System settings
- Rename done in T192; fill with real rows: theme toggle (IPC/global already),
  links to hypr modules open-in-editor, about/version
- Hardware System tab stays separate (System)

### ACP agents
- List configured ACP agents
- Add (name + command/ stexto / endpoint as today hermes uses)
- Remove
- Honest empty / permission errors

## Верификация

Live: add fake agent row, remove it; theme from settings works.

Коммит: `settings : system surface + acp agents crud (T196)`.
