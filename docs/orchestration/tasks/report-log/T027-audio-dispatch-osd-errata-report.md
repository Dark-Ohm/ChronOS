<!-- T027 — migrated 2026-07-22 from docs/orchestration/report-log/grok-report-3.md — see docs/orchestration/tasks/MIGRATION.md -->

<!-- ⚠ АНОМАЛИЯ (не разобрана, см. HANDOFF.md "АНОМАЛИЯ" 2026-07-17): исходный
     grok-report-3.md был обнаружен молча перезаписанным неизвестным источником —
     содержимое ниже НЕ гарантированно то, что архитектор изначально принял.
     Не доверять деталям без сверки с git-историей коммитов 6f24bb3/f4edb88. -->


# Session: audio dispatch + OSD эрратумы — 2026-07-17

## Сделано (факт, не намерение)
- `crates/services/src/audio/wpctl.rs`: pure `format_set_volume_args` / `format_set_mute_toggle_args` → `set-volume -l 1.5 <id> N%` / `set-mute <id> toggle`; unit-тесты argv
- `crates/services/src/audio/mod.rs`: `command_to_wpctl_args` + `apply_command` через pure map; `dispatch` по-прежнему fire-and-forget + **немедленный re-read** (не ждёт poll 250мс); тесты на mapping
- `crates/services/examples/audio-dispatch-smoke.rs`: SetSinkVolume(0.40) <300мс + hold 600мс (чтобы shell/OSD успели), ToggleSourceMute + hold, restore; `wpctl get-volume` cross-check
- `crates/app/src/osd/mod.rs`:
  1. **Стартовый флэш:** ждать первый **недефолтный** `AudioState`, сидировать baseline без show; seed из `service.get()` в `init` если poll уже выиграл; только последующий diff → OSD
  2. **window-not-found:** soft-hide — `display=None` + `set_input_region(Some(&[]))` + notify, **без** `remove_window` (destroy гоняет Wayland frame callbacks с `.log_err()` → пара `ERROR : window not found`)

## Расхождения со спекой/планом
- Hide = soft-reuse surface, не destroy. Причина: gpui platform frame path после `remove_window` логирует Err через `log_err` (пустой target: путь Source/gpui без `crates/`). Source/ не трогали (зона запрещена).
- Dispatch smoke держит 0.40/mute 600мс — иначе set→restore nets to zero между poll-тиками шелла, OSD не видит. Это прогон, не прод-API.

## Не реализовано из acceptance criteria
- In-process `AppState::audio().dispatch()` из UI ползунков — вне зоны (следующая очередь)
- Brightness OSD — вне зоны

## Проверено фактом, не на словах
- `cargo test -p chronos-services --lib audio` → **11 passed** (новые format/command tests)
- `cargo test --workspace` (чужой WIP network/upower/tray временно set-aside stash) → **130 passed** (4+44+25+54+3)
- 5 рестартов release chronos: **0** `OSD: audio change` в первые 2.2с, **0** `window not found` (`/tmp/chronos-osd-smoke3/restart-*.log`)
- `audio-dispatch-smoke` (release): sink 0.40 за **42ms**, wpctl `Volume: 0.40`, ToggleSourceMute ok
- Живой shell log (`clean.log`):
  - `OSD: audio change … volume=0.4`
  - `OSD: audio change is_source=true volume=0.33 muted=true`
  - `window not found` count = **0** после hide
- grim: `/tmp/chronos-osd-smoke3/hold-40.png` (Громкость 40%), `hold-mute.png` (Микрофон mute)

## Новые риски / известные баги
- **low:** soft-hide оставляет layer-shell surface живым (input region пустой → клики pass-through). Если когда-нибудь понадобится полный destroy — чинить frame `.log_err` в Source/gpui.
- **process:** параллельный WIP (Hermes/Mimo/OpenCode) периодически ломал `cargo test --workspace` (network/upower if-let, tray menu fields). Верификация — на изолированном дереве; их stash: `tmp-foreign-wip-*` (если ещё лежат — `git stash list`).

## Статус ARCHITECTURE.md / DECISIONS.log
- Не трогал (dispatch уже подразумевался audio MVP; OSD soft-hide — UI-деталь)

## Коммиты
- `audio : dispatch-команды (wpctl)` — audio/** + example
- `osd : эрратумы — стартовый флэш и window-not-found` — osd/mod.rs
