# Session: Zed №6 — per-app stream mute в audio-сервисе (Task 6, правая панель) — 2026-07-21

**Исход: DONE** (unit-зелень + workspace build; UI не в зоне; live mute-клик не гонялся — это Task 9).

## Сделано (факт, не намерение)

- `crates/services/src/audio/types.rs` — `AudioStream { id, application_name, node_name }` + `AudioCommand::ToggleStreamMute(u32)`.
- `crates/services/src/audio/pw_dump.rs` — `parse_pw_dump_streams` (только `media.class == "Stream/Output/Audio"`), `find_stream_for_player` (case-insensitive substring по `application_name` / `node_name`; empty hint → `None`), 3 unit-теста `stream_tests::*`.
- `crates/services/src/audio/mod.rs` — match-arm `ToggleStreamMute` → `format_set_mute_toggle_args(&id.to_string())`; `AudioSubscriber::toggle_stream_mute_for_player(player_hint)` (spawn: `pw-dump` → parse → find → dispatch или `info!` no-op); реэкспорт `AudioStream`; тест `command_to_wpctl_args_stream_mute_targets_the_given_id`.
- `crates/services/src/lib.rs` — публичный реэкспорт `AudioStream` (1 строка рядом с остальными audio-типами; иначе панель Task 9 не увидит тип без `chronos_services::audio::…` path-хака).
- `wpctl.rs` — **не менялся** (сигнатура `format_set_mute_toggle_args(id: &str)` уже годится).

## Step 1 — живая фикстура (НЕ умозрительная)

Прогон на этой машине 2026-07-21, Vivaldi с вкладками (звук/медиа в браузере):

```
pw-dump | python3 -c '… Stream/Output/Audio …'
```

Факт:
- `media.class` **точно** `Stream/Output/Audio` (не префикс-вариант).
- `application.name` заполнен (`Vivaldi`).
- `node.name` = `Vivaldi`.
- Одновременно **49** playback-стримов (мульти-таб) — подтверждает, что `find_stream_for_player` берёт **первый** match, а `None` — нормальный исход.
- Эвристика `hint=vivaldi` на живых данных → first match id=200, names Vivaldi/Vivaldi.

Схема фикстуры плана (Vivaldi id=142 + Spotify id=143 + sink id=55) **совпадает** с живой схемой props; числа id в тесте синтетические (как в плане), структура — с реального `pw-dump`.

## Расхождения со спекой/планом

- План Step 10 (commit) — **не сделан** по брифу ZED.md («Не коммить. Приёмку и коммит — Архитектор»).
- TDD «fail first» formal: реализация+тесты внесены одним заходом (Архитектор выполнял за Zed); логика тестов и API — 1:1 с планом, fail-red не прогонялся отдельным коммитом.
- `lib.rs` реэкспорт `AudioStream` — план формально не перечислял, но без него публичный surface services неполный; зона минимальна (1 имя в existing re-export).

## Не реализовано из acceptance criteria

- UI кнопки mute на MPRIS-карточке — **Task 9**, вне зоны.
- Живой клик `wpctl set-mute <stream_id> toggle` на реальном плеере — не гонялся (необратимый UX-шум + нет UI). Команда проверена unit-ом: `ToggleStreamMute(142)` → `["set-mute","142","toggle"]`.
- Выбор «правильного» стрима среди N вкладок одного app — не в MVP; берётся первый match (документировано в doc-comment).

## Проверено фактом, не на словах

```
cargo test -p chronos-services --lib audio::
# 24 passed (в т.ч. stream_tests×3 + command_to_wpctl_args_stream_mute_targets_the_given_id)

cargo test -p chronos-services --lib
# 143 passed; 0 failed

cargo build --workspace
# Finished dev profile, exit 0 (предсуществующие warnings в чужих файлах)
```

Живой `pw-dump`: 49× `Stream/Output/Audio`, match `vivaldi` → id 200.

## Новые риски / известные баги

- **Severity medium / by design:** multi-tab browser → first-match mute может замьютить «не ту» вкладку. Caller (панель) должен деградировать на no-op/`None` без паники; UX «mute this tab» без `media.name`/pid — follow-up.
- **Severity low:** MPRIS `player_id` вида `org.mpris.MediaPlayer2.spotify` vs PipeWire `application.name=Spotify` — substring `"spotify"` сработает; полный bus-name без усечения может **не** матчить. Панель должна передавать short hint (как план: player identity, не обязательно full bus path).
- Чужой WIP в том же working tree (`net_stats`, `side_panel_right`, `network.rs`, …) — **не тронут**.

## Статус ARCHITECTURE.md / DECISIONS.log

Не обновлялись — это расширение существующего audio MVP (wpctl), не новый архитектурный контур. При желании Архитектора: одна строка в ARCHITECTURE §services audio про `ToggleStreamMute` + name-match.

## Коммит (предложение Архитектору)

Поимённо только:

```
crates/services/src/audio/types.rs
crates/services/src/audio/pw_dump.rs
crates/services/src/audio/mod.rs
crates/services/src/lib.rs
```

Сообщение: `services : audio — per-app stream mute (ToggleStreamMute + pw-dump streams + name-match)`
