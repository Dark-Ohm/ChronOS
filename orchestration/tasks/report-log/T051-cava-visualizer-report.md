<!-- T051 — migrated 2026-07-22 from orchestration/report-log/grok-report-13.md — see orchestration/tasks/MIGRATION.md -->

# Session: cava visualizer — 2026-07-19

## Сделано (факт, не намерение)
- `crates/services/src/cava/{mod,types}.rs`: `CavaSubscriber` — long-lived `cava -p <runtime-config>` (pipewire → ascii raw stdout), line-parse → `Mutable<Vec<u8>>` (24 bars, 0..=100). Soft-fail + exponential backoff on spawn/exit. Config written to `$XDG_RUNTIME_DIR/chronos-cava.conf`.
- `parse_cava_ascii_line` — pure parser; unit-test на **живой** строке с этой машины (`speaker-test` + cava 0.10.7).
- `crates/services/src/lib.rs`: `pub mod cava`, re-export, `Services.cava`, `init_all`.
- `crates/app/src/state.rs`: `AppState::cava(cx)`.
- `crates/app/src/bar/widgets/cava.rs`: `BarSection::Center`, 24 полоски, `theme.accent.primary`, height 0..18px.
- `bar/widgets/mod.rs`: `mod cava` + `cava::register(cx)` в конце.
- `Cargo.toml`: tokio feature `process`.
- cava 0.10.7 уже был в системе (`pacman`); конфиг raw/ascii сверен с реальным `~/.config/cava/config` + live stdout.
- Коммит: `c519e2e` — `bar/services : cava-визуализатор звука (реальный процесс, soft-fail без бинаря)`.

## Расхождения со спекой/планом
- **`bar/mod.rs` watch НЕ добавлен** (зона запрещена явно). Бар репаинтится по 1s-тикеру + чужим watch'ам → визуализатор **дергается ~1 Гц**, не 30. Нужна однострочная эррата Архитектора (как audio/mpris раньше):
  ```rust
  watch(cx, AppState::cava(cx).subscribe(), |_, _, cx| { cx.notify(); });
  ```
- `CavaState = Vec<u8>` (type alias), не отдельный struct — достаточно для Service::Data.
- При мгновенном exit cava (fake binary) backoff **растёт** (после фикса), не спамит 1/s forever.

## Не реализовано из acceptance criteria
- Плавный 30fps UI без `bar/mod.rs` watch — ждёт Архитектора (не зона).
- Soft-fail «снести пакет pacman» — не делал (нужен sudo); эквивалент: `PATH` с stub `cava` exit 127 → шелл жив, log warn+restart.

## Проверено фактом, не на словах
- `cargo test -p chronos-services --lib cava` → 6 ok.
- `cargo test --workspace --lib --bins` → **222** ok (4+83+25+107+3).
- `cargo build --release -p chronos` → green.
- Live release:
  - log: `cava: process started (bars=24, ascii raw)`
  - grim center bar with `speaker-test`: `bar-sound-2/3` md5 **≠** idle (`/tmp/g13-smoke/`) — spike visible next to mpris
  - after silence: back toward flat
  - soft-fail PATH stub: process ends → warn restart, `pgrep chronos` alive, no panic
- Real fixture line: `0;0;1;1;1;2;2;4;7;19;100;12;…` (24 values).

## Новые риски / известные баги
- **Med (блокирует UX-частоту):** нет `watch` на cava в `bar/mod.rs` — только 1s tick. Архитектор: одна строка.
- **Low:** cava child CPU ~ continuously; framerate=30 в конфиге.
- **Low:** dual-monitor bar crops; visualizer center per-bar window.

## Статус ARCHITECTURE.md / DECISIONS.log
- Не трогал. Решение «shell real cava» уже в DECISIONS.log 2026-07-19 Top Bar redesign wave.
