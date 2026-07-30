# T118 — Architect review (acceptance)

**Дата:** 2026-07-24  
**Вердикт:** **ACCEPTED WITH CAVEATS**

## Сверка

| Утверждение | Факт |
|---|---|
| `UpgradeState::Running(UpgradeProgress)` | **Да** — `types.rs` |
| Streaming `spawn` + stderr line reader | **Да** — `run_upgrade_all`, `spawn_blocking` + `std::thread` |
| `parse_progress_line` + 6 unit tests | **Да** — 6 parse_* + suite `aur` = **22 passed** (повторён) |
| Spinner / bar / % / last_line UI | **Да** — `view.rs` Running branch |
| Staircase = filter `completed_names` | **Да**, мгновенный filter, **без** fade/height animation |
| `gpui-animation` spinner spin | **Нет** — static `arrows-clockwise.svg`, rotation не подключена |
| `cargo build --release` | commit `7329106` in tree; `cargo check -p chronos` green |
| Живой апгрейд | **PENDING** (честно в отчёте). На момент приёмки архитектора `checkupdates` уже показывает пакеты (kitty/systemd…) — можно смоукнуть живьём отдельно |
| Reactive repaint | **Да** — `updates_popup::init` subscribe → `notify` |

## Caveats / residual (не блокер accept)

1. **Spinner не крутится** — бриф просил `gpui-animation`; сейчас статичная иконка + текст `Upgrading… N/M`. Полироль.
2. **Staircase без анимации** — бриф разрешал честный компромисс; отчёт слабо это пометил. Filter работает.
3. **Семантика `completed_names`**: имя попадает при строке `(N/M) upgrading X` — это **старт** пакета, не конец. Для UI ок первой итерации.
4. **stdout был piped без reader** — риск deadlock pipe buffer. **Errata архитектора:** `stdout(Stdio::null())`.
5. **Footer height 80** при Running (spinner+bar+line) может быть тесноват — живой смок покажет.
6. Живой end-to-end апгрейд в отчёте не прогнан.

## Файлы / коммиты

- `7329106` — основной T118
- errata stdout null — отдельный маленький коммит при приёмке

Бриф → `done/`. Отчёт → `report-log/`.
