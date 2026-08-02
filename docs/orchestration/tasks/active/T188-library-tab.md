# T188 — вкладка Library (Gamer hub)

**Статус:** active (волна 1 закрыта: T185/T186/T187 приняты).
**Роль:** FRONTEND. **Модель: GLM 5.2** (или Mimo 2.5 — запас).
**Правила:** `docs/orchestration/agents/RULES.md`.

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

**Отчёт только** `report/T188-library-tab-report.md`. Не `done/`, не «принята».

## Зависимости (готовы)

- `PanelTab::Library` + width 480 — T186 `102fef4`
- `chronos_services::applications::is_game_entry` — T187 `7a99116`
- `crate::games_config::GamesConfig` — T187 `af66b58`
- launch: `crate::launcher::launch::launch` (dock уже образец)
- Applications: `AppState` / subscriber — как launcher читает entries

## Живой смок (обязателен насколько возможно)

Gamer mode → rail Library → expand content → список CS2/PUBG/SCUM, **не**
steam client. Launch + pin. Кадр grim. Нет compositor — `НЕ ПРОВЕРЕНО`.
