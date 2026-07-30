<!-- T050a — migrated 2026-07-22 from orchestration/report-log/grok-report-12.md — see orchestration/tasks/MIGRATION.md -->

# Session: volume_popup + device picker — 2026-07-19

## Сделано (факт, не намерение)
- `crates/services/src/audio/types.rs`: `AudioDevice { id, name, node_name, is_default }`; `EndpointState.available`; `AudioCommand::SetDefaultSink/Source(u32)`.
- `crates/services/src/audio/pw_dump.rs`: `run_pw_dump` + pure `parse_pw_dump_devices` (sinks/sources + Metadata defaults). Учитывает `Audio/Source/Virtual` (Easy Effects).
- `crates/services/src/audio/wpctl.rs`: `format_set_default_args`.
- `crates/services/src/audio/mod.rs`: poll `read_state` подмешивает device list; soft-fail если pw-dump упал.
- Фикстура `audio/fixtures/pw_dump_sample.json` — срез **живого** `pw-dump` с этой машины.
- `crates/app/src/volume_popup/{mod,view}.rs`: layer-shell popup, fill-bar + ±5%/mute, клик по заголовку Speakers/Microphone → in-window device list (●/○), `window.resize` при expand.
- `bar/widgets/volume.rs`: click → `volume_popup::toggle`; scroll без изменений.
- `main.rs`: `mod` + `init`.
- `serde_json` в workspace + services.
- Коммит: `66d66c3` — message per GROK.md.

## Расхождения со спекой/планом
- `available` — `Vec<AudioDevice>` (структура), не tuple `(u32, String, bool)`: нужен `node_name` для match default; `name` — description. Семантика та же.
- Sources: фильтр `Audio/Source` **и** `Audio/Source/*` (иначе Easy Effects Source пропадает — media.class Virtual).
- ydotool-клики по mic-device rows нестабильны (dual-monitor drift); sink switch + source `wpctl set-default` path доказаны. Код set-default для source = sink.

## Не реализовано из acceptance criteria
- Drag-слайдер — **out of scope** (задание запретило).
- Авто-клик ydotool по каждой source-строке — не закрыт (drift); UI source-list expand (height 304) + `SetDefaultSource` argv/path verified.

## Проверено фактом, не на словах
- `cargo test -p chronos-services --lib audio` → 21 ok (pw-dump fixture, set-default argv, volume).
- `cargo test --workspace --lib --bins` → **213** ok (4+80+25+101+3).
- `cargo build --release -p chronos` → green.
- Живой release:
  - `volume_popup: subscribed to audio service`
  - open bar-volume → namespace `volume-popup`
  - grim `/tmp/g12-smoke/picker-sources-open.png`: expanded Speakers list =
    Easy Effects Sink / GA104 HDMI / **● Built-in Audio Analog Stereo** — совпадает с `wpctl status` sinks
  - title shows live device names (Built-in / Easy Effects Source)
  - `wpctl` default sink: 70 → **45** (EE) → **69** (HDMI) → 70 via UI clicks; log `set default sink id=…`
  - Speakers −5%: 0.55 → **0.50**
  - bar toggle close → layers clean
  - parser live: sinks [45,69,70] sources [46,71] match `wpctl status` names/ids
  - `wpctl set-default 71/46` confirms source switch command used by UI

## Новые риски / известные баги
- **Low:** `pw-dump` на каждом 250ms poll — тяжеловато; MVP ok, native PipeWire backend потом.
- **Low:** ydotool dual-monitor hitbox drift — automation, не UI.
- **Info:** предыдущий orphan-коммит `8ad3da8` (popup без picker) не на master — этот коммит полный (popup+picker) поверх `2522018`.

## Статус ARCHITECTURE.md / DECISIONS.log
- Не трогал. Расширение MVP audio backend (pw-dump list + set-default) укладывается в существующий wpctl-путь; DECISIONS про native pipewire backend по-прежнему в силе.
