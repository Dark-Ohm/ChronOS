<!-- T028 — migrated 2026-07-22 from docs/orchestration/report-log/grok-report-4.md — see docs/orchestration/tasks/MIGRATION.md -->

# Session: Grok №4 — виджет громкости в баре — 2026-07-17

## Сделано (факт, не намерение)
- `crates/app/src/bar/widgets/volume.rs` (новый): `VolumeWidget` (Right), иконка
  🔇/🔈/🔉/🔊 + `NN%` sink; `on_click` → `AudioCommand::ToggleSinkMute`;
  `on_scroll_wheel` → `SetSinkVolume(current ± 0.05)` через
  `chronos_services::audio::clamp_volume` (0..1.5). Pure helpers:
  `volume_icon`, `format_percent`, `scroll_volume_delta`, `describe` + 8 unit-тестов.
- `crates/app/src/bar/widgets/mod.rs`: `mod volume;` + `volume::register(cx);`
  в конце `register_builtin` (чужие строки не двигались).

## Расхождения со спекой/планом
- **audio не в watch-списке `bar/mod.rs`** (только compositor/network/upower/
  notification). По брифу `bar/mod.rs` НЕ трогал — **нужна одна строка от
  Архитектора**:
  `watch(cx, AppState::audio(cx).subscribe(), |_, _, cx| { cx.notify(); });`
  Сейчас виджет обновляется через 1s clock-ticker бара — для live scroll/mute
  UX лагом до ~1с. Внешний `wpctl` + poll 250ms + тикер: бар-процент меняется
  (см. grim 35%→55%→mute).
- **Клик/скролл мышкой не автоматизированы**: в среде нет ydotool/wtype/dotool.
  Handlers стоят (тот же `dispatch`, что tray/workspaces). Живой путь
  mute/volume подтверждён через `wpctl` + OSD-лог + grim бара. Ручной клик
  Архитектору на живом release-инстансе (pid шелла после смока мог быть убит).

## Не реализовано из acceptance criteria
- Прямой интерактивный клик-mute / scroll ±5% с grim «до/после клика» —
  нет input-automation. Код handlers готов; smoke render+dispatch-эффект
  через wpctl.
- audio-watch в bar — ждёт разрешения Архитектора (зона bar/mod.rs).

## Проверено фактом, не на словах
- `cargo test -p chronos --bin chronos volume` → **8 passed**
- `cargo test -p chronos --bins` → **54 passed**
- `cargo test --workspace` → lib 4 + bins 54 + luau 25 + services 68 + ui 3
  (все ok; services 68 включает audio argv)
- `cargo build --release -p chronos` → ok (~3m)
- Живой release-смок (`pkill -x chronos` → `RUST_LOG=info ./target/release/chronos`):
  - лог: `OSD: subscribed`, `Opening bar on 2 displays`, volume changes
    `0.55` / mute / restore `0.35`
  - grim: `/tmp/chronos-vol-smoke/01..04-*.png`, кропы
    `0*-bar-right-2x.png`: **🔊 35% → 🔊 55% → 🔇 55%** в правой секции
    рядом с eth/tray
  - `wpctl get-volume` совпал: 0.35 → 0.55 → 0.55 [MUTED] → 0.35
- `git diff --staged` (перед коммитом): только volume.rs + 2 строки mod.rs

## Новые риски / известные баги
- **medium**: без audio-watch бар отстаёт до ~1s при dispatch с виджета
  (клик/скролл) — OSD всплывает сразу, процент в баре догоняет по тикеру.
- **low**: scroll-направление (neg y → +vol) — конвенция content-scroll;
  если мышь пользователя инвертирована — одна строка в `scroll_volume_delta`.
- **low**: click/scroll без hitbox-automation не закрыты end-to-end в этом
  отчёте.

## Статус ARCHITECTURE.md / DECISIONS.log
- Не обновлялись (UI-виджет в зоне bar/widgets, канон не менялся).
