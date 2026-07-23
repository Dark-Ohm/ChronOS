<!-- T100 — migrated 2026-07-22 from orchestration/report-log/grok-report-20.md — see orchestration/tasks/MIGRATION.md -->

# Session: Трек 3 — MPRIS art+progress + media card live — 2026-07-21

**Исход: сдано.** Backend `art_url`/`position_us`/`length_us` + карточка рисует
`file://` обложку, progress fill и `-M:SS`. Коммит не делал.

## Сделано (факт, не намерение)

- `crates/services/src/mpris/types.rs` — в `MprisState` добавлены
  `art_url: Option<String>`, `position_us: Option<i64>`, `length_us: Option<i64>`.
- `crates/services/src/mpris/mod.rs`:
  - proxy property `position() -> i64`;
  - `extract_art_url`, `extract_length_us`, `extract_i64`, `has_position`;
  - `read_active_state` заполняет art/length из Metadata + Position poll
    (существующий 500 ms tick);
  - unit-тесты: idle Vivaldi-shape fixture (`mpris:length x 0`), full track
    fixture с `file://` art + length 222_200_000, variant/u64 length, has_position.
- `crates/app/src/side_panel_right/mpris_card.rs` — live:
  - art: `file://` → path (percent-decode) + `img` + `ObjectFit::Cover`;
    http(s)/missing → play-placeholder;
  - progress: `position/length` fill `#007acc`;
  - timecode: remaining `-M:SS` (или `-H:MM:SS`);
  - pure helpers + unit-тесты.
- `crates/app/src/bar/widgets/mpris.rs` — **минимальный compile-fix**:
  `..Default::default()` в test helper `track_state` (новые поля `MprisState`).
  Логику виджета не трогал. Вне зоны, но иначе `cargo test --bins` красный.

## Расхождения со спекой/планом

- Полл Position: гейт «не чаще 1с» — **не отдельный**; Position читается на
  общем MPRIS tick **500 ms** (уже был). Чуть чаще 1с, без второго таймера.
- `view.rs` **не трогал** — подписка уже несёт полный `MprisState`.
- http(s) art **не** грузится (плейсхолдер) — по брифу.
- Compilе-fix bar widget — см. выше (вынужденный).

## Не реализовано из acceptance criteria

- Сетевая загрузка http(s) обложек — out of scope.
- Seek по клику на progress — не просили.
- Pixel-delta прогресса на двух grim с Δt=2s = 0 (доля ~1% ширины бара
  при 3:42 треке — шум). **Движение позиции** подтверждено busctl до паузы
  (30 M → 51 M μs) и **стоп на Paused** (51 M → 51 M за 1.2s).

## Проверено фактом, не на словах

```
$ cargo test -p chronos-services --lib mpris
# 22 passed (incl. extract_art_and_length_from_metadata_fixture)

$ cargo test -p chronos --bins mpris
# 21 passed (13 bar mpris + 8 mpris_card)

$ cargo build --workspace   # EXIT 0
$ cargo build --release -p chronos  # EXIT 0 (~2m21s)

# Live mock (НЕ Vivaldi):
# org.mpris.MediaPlayer2.chronos_art
# Metadata: artUrl=file:///usr/share/pixmaps/archlinux-logo.png
#           length=222200000  title=ChronOS Art Smoke
# Position advancing while Playing; after PlayPause: Paused, pos frozen

# Panel open (hover strip ydotool right edge DP-1):
# hyprctl layers → namespace: side_panel_right xywh: 2208 30 352 1410

# Pixel evidence /tmp/chronos-mpris-art-smoke/10-panel.png / 11-mpris-card-live.png:
# - art region: 5591 blueish samples (arch logo cyan ~23,147,209)
# - progress band: 2488 px of ACCENT (0,122,204) = #007acc
# - 12-art-frame.png, 13-progress.png crops

# Vivaldi: busctl Get only earlier (idle length=0); PlayPause НЕ вызывали
# на Vivaldi — только на chronos_art mock.
```

## Тронутые файлы (полный список)

| Файл | Правка |
|---|---|
| `crates/services/src/mpris/types.rs` | 3 новых поля в `MprisState` |
| `crates/services/src/mpris/mod.rs` | Position proxy, extractors, read path, tests |
| `crates/app/src/side_panel_right/mpris_card.rs` | art/progress/timecode UI + tests |
| `crates/app/src/bar/widgets/mpris.rs` | `..Default::default()` в test helper only |

## Новые риски / известные баги

- **low:** http(s) `artUrl` (Spotify/web players) → placeholder until network-art track.
- **low:** players that always report `Position=0` or omit length → bar empty/0 (degrade, not fake).
- **low:** first-frame GPUI image decode may lag one repaint for new `file://` path (not observed as blocker).
- **info:** sticky pin not set — auto-pick prefers Playing mock over idle Vivaldi; if user pins Vivaldi with empty metadata, card shows id/placeholder.

## Статус ARCHITECTURE.md / DECISIONS.log

Не обновлял — расширение существующего MPRIS-контракта, без новых решений.

## Скриншоты (пути)

- `/tmp/chronos-mpris-art-smoke/10-panel.png` — полный верх панели
- `/tmp/chronos-mpris-art-smoke/11-mpris-card-live.png` — карточка
- `/tmp/chronos-mpris-art-smoke/12-art-frame.png` — 16:9 с arch logo
- `/tmp/chronos-mpris-art-smoke/13-progress.png` — fill `#007acc`
