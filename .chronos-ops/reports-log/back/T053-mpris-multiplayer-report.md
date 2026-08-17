<!-- T053 — migrated 2026-07-22 from docs/orchestration/report-log/grok-report-14.md — see docs/orchestration/tasks/MIGRATION.md -->

# SESSION_REPORT — Grok №14: MPRIS multi-player

**Дата:** 2026-07-20  
**Коммит:** `a3d36ba`  
**Зоны:** `crates/services/src/mpris/**`, `crates/services/src/lib.rs` (1 строка re-export), `crates/app/src/bar/widgets/mpris.rs`  
**Не трогал:** попапы, `bar/mod.rs`, другие виджеты, WIP Mimo (`monitor.rs` и др.)

## Сделано

### Сервис
- `MprisState`: `player_count`, `player_index` (1-based), `player_id` (suffix после prefix).
- `user_pinned: Mutable<Option<String>>` — sticky override.
- `resolve_active_player(players, pin)`: pin жив → pin; иначе auto (`Playing` first / first) + `clear_pin`.
- `MprisCommand::CyclePlayer(CycleDirection::{Next,Prev})` — wrap-around по `list_mpris_names`; 0–1 плееров → no-op; immediate re-read.
- Pin чистится, когда имя ушло с шины (не воскресает при reappear без нового cycle).
- Новый Playing-плеер **не** крадёт фокус при живом pin.

### Виджет
- `on_click` = `PlayPause` (как было).
- `on_scroll_wheel` → `CyclePlayer` (scroll up = Next, down = Prev; маппинг 1:1 volume).
- При `player_count > 1` — тонкий `‹i/n›` (`text_xs` + muted); при ≤1 скрыт.

### Тесты (unit)
- sticky hold / pin gone → auto+clear / empty clears pin
- cycle next/prev wrap / noop 0–1 / unknown current from 0
- scroll delta → Next/Prev/None
- multi indicator show/hide
- прежние metadata/select/name-prefix

## Верификация

| Проверка | Результат |
|---|---|
| `cargo test --workspace --lib --bins` | **256 green** (изолированный worktree `ChronOS-grok14` — в master-дереве чужой WIP Mimo `monitor.rs` ломал test-сборку) |
| `cargo build --release -p chronos` | green (worktree) |
| Живой смок release | см. ниже |

### Живой смок (`/tmp/g14-smoke/`)

Бинар: `ChronOS-grok14/target/release/chronos`  
Моки: `/tmp/chronos-mpris-mock.py chronos_a` + `chronos_b` (Vivaldi на шине **не** трогали).

1. **2 мока + Vivaldi → 3 плеера:** grim `mpris-crop.png` — `Alpha Track — Alpha Artist ‹1/3›`.
2. **Убили active mock_a:** bar → `Beta Track — Beta Artist ‹1/2›` (auto-fallback + count) — `mpris-after-kill-a.png` / `bar-after-kill-a.png`.
3. **PlayPause** через `gdbus` на mock_a — mock log: `PlayPause → Paused` (мок жив).
4. **Scroll-cycle живьём:** не автоматизирован — `ydotool` dual-monitor drift (курсор улетает на HDMI), wheel в ydotool API нет; uinput-wheel без точного hit на bar surface. **Unit-тесты cycle + scroll-mapping зелёные.** Тот же класс, что приёмка №5 (клик ydotool).

Лог: `MprisSubscriber connected`; без `error`/`panic` по mpris. Soft-fail Vivaldi: только в списке, методы не звали.

## Не реализовано / оговорки

- Scroll-cycle **не** прожат живым input (см. выше) — только unit + косвенный multi-list/fallback смок.
- Порядок `ListNames` не стабилен → индекс `‹i/n›` зависит от D-Bus enumeration order; для UX достаточно.
- `player_id` экспонирован в state, в UI сейчас `‹i/n›` (не app-имя) — компактнее в bar center.

## Файлы

- `crates/services/src/mpris/types.rs`
- `crates/services/src/mpris/mod.rs`
- `crates/services/src/lib.rs` — `CycleDirection` re-export
- `crates/app/src/bar/widgets/mpris.rs`

Артефакты смока (не в git): `/tmp/g14-smoke/*.png`, mock logs.
