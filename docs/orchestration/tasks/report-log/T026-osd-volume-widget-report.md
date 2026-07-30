<!-- T026 — migrated 2026-07-22 from docs/orchestration/report-log/grok-report.md — see docs/orchestration/tasks/MIGRATION.md -->

# Session: OSD громкости (поверх audio) — 2026-07-17

## Сделано (факт, не намерение)
- `crates/app/src/osd/mod.rs`: layer-shell Overlay, namespace `osd`, anchor BOTTOM|LEFT|RIGHT, margin bottom 48px, `KeyboardInteractivity::None`, transparent; `OsdPopupState` + `OsdWatcher` + `state::watch` на `AppState::audio`; первый снапшот подавлен; hide-таймер 1.5с через `cx.spawn` + `background_executor().timer` с generation token
- `crates/app/src/osd/view.rs`: карточка ~320×80, прогресс-бар (ширина от volume 0–100%), иконка/лейбл sink («Громкость») vs source («Микрофон»), mute → muted-цвета + «mute»
- `crates/app/src/main.rs`: `mod osd;` + `osd::init(cx);` (только эти две строки)

## Расхождения со спекой/планом
- Anchor: задание «BOTTOM (низ-центр)» → реализовано `BOTTOM|LEFT|RIGHT` + центрирование карточки во view (тот же приём, что notifications TOP|RIGHT; чистый BOTTOM без left/right на Hyprland даёт непредсказуемую ширину surface)
- Иконка mute: юникод 🔇/🎤 + тусклый цвет Theme, не отдельная «перечёркнутая» SVG-иконка (MVP, достаточно для смока)
- Язык UI: «Громкость» / «Микрофон» (рус.) — в задании не зафиксировано; ок для текущего дефолта шелла

## Не реализовано из acceptance criteria
- Brightness OSD — вне зоны (следующая очередь)
- Multi-monitor: OSD только на primary (как notifications `pick_display`) — не требовалось явно
- Native PipeWire backend — уже deferred в audio-сервисе; лаг poll ~250мс остаётся

## Проверено фактом, не на словах
- `cargo build -p chronos` → ok
- `cargo test --workspace` → **104 passed** (4+35 app + 25 luau + 37 services + 3 ui)
- `cargo build --release -p chronos` → Finished release ~1m48s
- Живой release-смок (`RUST_LOG=info ./target/release/chronos`, pid 28565):
  - log: `OSD: subscribed to audio service` при старте
  - `wpctl set-volume @DEFAULT_AUDIO_SINK@ 5%+` → `OSD: audio change is_source=false volume=0.45`
  - grim `/tmp/chronos-osd-smoke/osd-sink.png` — плашка «Громкость 45%» низ-центр
  - grim `osd-after-hide.png` (~1.8с) — OSD нет
  - source `5%+` → `is_source=true volume=1.05`, grim `osd-source.png` — «Микрофон 105%» / Easy Effects Source
  - `notify-send test` + sink change → grim `osd-with-notify.png` — оба слоя одновременно (notifications top-right + OSD bottom)
  - mute toggle → `muted=true` в логе
- `hyprctl layers` после hide: namespace `osd` отсутствует (окно закрыто), notifications жив

## Новые риски / известные баги
- **low:** fill-bar ширина через фиксированный px-расчёт от 320 (не relative %), при смене padding/font может чуть врать
- **low/known:** poll audio 250мс → OSD отстаёт ~¼с (HANDOFF/DECISIONS, MVP)
- **low:** dual-monitor — OSD только primary; second display без плашки

## Статус docs/ARCHITECTURE.md / docs/DECISIONS.log
- Не трогал: OSD — UI-модуль поверх уже задокументированного audio-сервиса; новых отклонённых альтернатив нет
- Коммит: `osd : плашка громкости поверх audio-сервиса` (только `osd/` + `main.rs`)
