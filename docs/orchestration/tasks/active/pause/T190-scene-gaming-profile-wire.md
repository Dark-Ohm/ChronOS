# T190 — apply_gaming_profile при scene::activate

**Статус:** BLOCKED — после T185 (+ желательно T189 UI flag).
**Роль:** BACKEND.

**Зона:**
- `system_popup/gaming_mode.rs` — `pub(crate) fn apply/revert` (не менять hyprctl payload)
- `scene.rs` — в `activate`: если `apply_gaming_profile` → apply; при уходе
  на сцену с false / hub → revert if was applied by scene

**Правила §5:** `workspace_mode::set` **не** трогать. Только scene activate path.

Не врать UI: индикатор только `GamingModeState::is_active`.

**Отчёт:** `report/T190-scene-gaming-profile-wire-report.md`.
