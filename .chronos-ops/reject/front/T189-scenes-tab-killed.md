**KILL 2026-08-02 — product cut: scenes UI not needed. docs/PRODUCT.md**

**KILL product path 2026-08-02** — scenes UI not needed. See docs/PRODUCT.md.
# T189 — вкладка Scenes (activate / create / delete)

**Статус:** BLOCKED — после **T185 + T186 + T188** (create-from-library).
Можно частично после T185+T186 без create-from — тогда create only manual id.
**Роль:** FRONTEND.

**Зона:**
- `tab/scenes.rs` — create
- `tab/mod.rs` — только `TabContent::Scenes` arm

## Цель

1. List gamer scenes from `scene` config / SceneState.
2. Activate → `scene::activate(cx, id)` (T185).
3. Create from Library app (id/name/app=desktop id, kind=game, dock default).
4. Delete with confirmation (§4.2 destructive) — не сносить `hub`.
5. Show apply_gaming_profile flag as checkbox (storage only until T190).

**Отчёт:** `report/T189-scenes-tab-report.md`.
