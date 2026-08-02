# T188 — вкладка Library (Gamer hub)

**Статус:** BLOCKED — стартовать **после приёмки T185+T186+T187**.
**Роль:** FRONTEND.

**Зависит от:** T186 (PanelTab::Library), T187 (`is_game_entry`, games.toml),
T185 не обязателен для list/launch, но pin→scene create — T189.

**Зона (когда снимешь block):**
- `crates/app/src/side_panel_right/tab/library.rs` — **create**
- `tab/mod.rs` — **только** `TabContent::Library` arm (не Scenes)

**НЕ:** scene.rs, applications parse (уже T187), Scenes UI (T189).

## Цель

1. Список игр: `ApplicationsState` entries filtered `is_game_entry`.
2. Секции: Pinned (from games.toml) → Recent → All games.
3. Click row → `launcher::launch::launch(&entry.exec)` + `touch_recent`.
4. Pin/unpin toggles → games_config save.
5. Empty: honest «no games detected» + hint (Categories/Steam shortcuts).
6. **Нет** fake artwork/playtime.
7. Width 480 already from T186.

Образец вьюхи: `tab/files.rs` / `system.rs` — Entity + Render, lazy create.

**Отчёт:** `report/T188-library-tab-report.md`.

Архитектор снимет BLOCKED и перенесёт в `active/` после волны 1.
