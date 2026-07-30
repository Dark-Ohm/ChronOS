<!-- T029 — migrated 2026-07-22 from orchestration/report-log/grok-report-5.md — see orchestration/tasks/MIGRATION.md -->

# Session: Grok №5 — MPRIS сервис + виджет медиаплеера — 2026-07-17

## Сделано (факт, не намерение)
- `crates/services/src/mpris/types.rs` — `MprisState { title, artist, playing, has_player }`,
  `MprisCommand { PlayPause, Next, Previous }`.
- `crates/services/src/mpris/mod.rs` — `MprisSubscriber`: session bus,
  `DBusProxy::list_names` + `NameOwnerChanged` (префикс
  `org.mpris.MediaPlayer2.`), zbus proxy
  `org.mpris.MediaPlayer2.Player` (playback_status, metadata a{sv},
  play_pause/next/previous), dispatch + немедленный re-read, poll 500ms
  fallback. MVP pick: first Playing, else first. `unwrap_variant` для
  metadata (рецепт tray/menu). Pure: `is_mpris_name`, `select_active_player`,
  `parse_metadata` + unit-тесты.
- `crates/services/src/lib.rs` — `pub mod mpris`, re-export, поле в
  `Services`/`init_all`, runtime-guard тест.
- `crates/app/src/state.rs` — `AppState::mpris(cx)`.
- `crates/app/src/bar/widgets/mpris.rs` — `BarSection::Center` (трек длинный;
  Right уже crowded), hidden если `!has_player`, ▶/⏸ + `title — artist`
  (truncate 40), click → `PlayPause`. 8 unit-тестов.
- `crates/app/src/bar/widgets/mod.rs` — `mod mpris;` + `mpris::register(cx);`
  в конец.

## Расхождения со спекой/планом
- **Секция Center, не Right** — обоснование: long track label vs crowded
  Right (eth/tray/volume). В отчёте по брифу.
- **mpv без MPRIS** на этой машине (нет mpv-mpris). Smoke: одноразовый
  Python mock `org.mpris.MediaPlayer2.chronos_smoke` через `python-dbus`
  (`/tmp/chronos-mpris-mock.py`). Vivaldi/firefox MPRIS **не** дёргали
  PlayPause (firefox/zen-bin висел Paused — только read).
- **`bar/mod.rs` watch на mpris не добавлял** (зона не моя; как в №4 с
  audio). Обновление виджета: 1s clock-ticker + 500ms poll сервиса
  (publish только при diff state). Для мгновенного repaint после
  dispatch — Архитектору одна строка watch, как для audio.
- next/prev кнопки не делал (MVP: один hitbox play/pause).

## Не реализовано из acceptance criteria
- Прямой клик по виджету мышью — нет ydotool. Эквивалент: `busctl call
  … PlayPause` на mock → иконка ⏸→▶ (grim). Handler клика = тот же
  `dispatch(PlayPause)`.
- mpris-watch в bar/mod.rs — ждёт Архитектора.

## Проверено фактом, не на словах
- `cargo test -p chronos-services mpris` → 10 ok (module + runtime guard)
- `cargo test -p chronos --bins mpris` → 8 ok
- `cargo test --workspace --lib --bins` → 4 + 62 + 25 + 80 + 3, все ok
- `cargo build --release -p chronos` → ok (~2m)
- Живой release-смок:
  - лог: `MprisSubscriber connected (session bus)`
  - mock Playing + title/artist → бар Center: **⏸ ChronOS Smoke Track — Grok Mock**
    (`/tmp/chronos-mpris-smoke/01-bar*.png`)
  - busctl PlayPause → **▶** + Paused (`02-bar-center-2x.png`);
    PlayPause → **⏸** Playing (`03-…`)
  - firefox MPRIS: только status/metadata read; PlayPause не вызывали
  - mock убит после смока (PID file)
- staged: только mpris/**, lib.rs (свои строки), state.rs, widgets/mpris.rs,
  widgets/mod.rs (+2), grok-report.md

## Новые риски / известные баги
- **medium**: без bar watch repaint после клика ≤1s (clock). Сервисный
  state обновляется сразу.
- **low**: при нескольких Playing — «первый в list_names», порядок
  нестабилен. MVP задокументирован.
- **low**: metadata без title/artist → label `"Unknown"`.
- **low**: poll 500ms — лишние D-Bus Get при idle; заменить property
  streams в follow-up.

## Статус ARCHITECTURE.md / DECISIONS.log
- Не обновлялись (новый сервис по шаблону Service trait; канон не
  менялся).
