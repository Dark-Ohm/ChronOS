# T119 — Architect review (acceptance)

**Дата:** 2026-07-24  
**Вердикт:** **ACCEPTED WITH CAVEATS**

## Сверка

| Утверждение | Факт |
|---|---|
| `AurCommand::UpgradeSelected { packages }` | **Да** — `types.rs` |
| `upgrade_selected_command_args` → `yay/pacman -S --noconfirm -- pkgs` (не `-Syu`) | **Да** + 3 unit-теста |
| Empty list no-op (dispatch + UI) | **Да** |
| Shared `run_upgrade_command` for All/Selected | **Да**, stdout null retained |
| Selection `HashSet` on view | **Да** |
| Footer label flip All ↔ Selected | **Да** |
| Header Check → `Refresh` | **Да** |
| Rows toggle; frozen while Running | **Да** |
| `cargo test … aur` 25 pass | **Повторено: 25/25** |
| `cargo check -p chronos` | **Зелёный** |
| Live smoke | **PENDING** (честно; pending packages on host exist) |

## Errata архитектора при приёмке

1. Check control был `.w(px(26.))` с иконкой+текстом «Check» — гарантированный clip. → `flex_none` + `px(6)` + hover.
2. Unused import `UpgradeProgress` снят.

## Caveats

1. **Live e2e** (toggle + Upgrade selected + Check) — не прогнан на release binary; пользователь/архитектор может смоукнуть при наличии pending (kitty/systemd…).
2. Check всё ещё кликабелен во время Running (muted only) — harmless waste.
3. Spinner spin T118 — out of scope (как просили).

## Файлы

Некоммиченный дифф миньона + errata → коммит при приёмке.
`done/T119-…`, `report-log/T119-*-report.md` + review.
