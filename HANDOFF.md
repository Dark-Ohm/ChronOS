# HANDOFF — контекст для новой сессии Архитектора

**Обновлено: 2026-07-17 (поздний вечер; волна №2 принята, №3 роздана).
Читать сверху вниз. При расхождении с ARCHITECTURE.md/DECISIONS.log побеждают они.**

## Кто ты и как работаешь

Lead Architect Agent проекта **ChronOS** — Rust/GPUI desktop shell для
Lua-Hyprland 0.55.4+. Сам НЕ кодишь (исключения: документы, однострочные
эрраты после приёмки, живой дебаг). НЕ спавнишь субагентов. Задания
миньонам — через их файлы (CLINE.md, HERMES.md, OMP.md, MIMO.md,
AUTOHAND.md, OPENCODE.md, GROK.md), отчёты — `<имя>-report.md` В КОРНЕ
репо; report-log/ — архив отработанного (перенос делает пользователь).
Приёмка — сам: грепы, диффы, build/test, живой release-смок.

## ВНИМАНИЕ: два стэша с чужим WIP (2026-07-17)

Grok при верификации №3 стэшил чужой WIP. НЕ дропать, НЕ pop вслепую:
- `stash@{0}` (tmp-foreign-wip-2): **mpsc-переделка debounce-лупа
  applications/mod.rs — рабочий код Mimo №4** + tray/types.rs (+63,
  OpenCode MenuNode) + lib.rs/Cargo.toml.
- `stash@{1}` (tmp-foreign-wip-for-grok-verify): live-интеграционные
  тесты network/upower (Hermes) + applications-правки.
Разрулить, когда владельцы доделают свои задания: сверить с их свежим
деревом, лишнее дропнуть, недостающее отдать владельцу. Урок в задания:
СТЭШИТЬ ЧУЖОЙ WIP ЗАПРЕЩЕНО (изоляция — только git worktree).

## СЕЙЧАС В ПОЛЕ

**Волна №2 принята целиком** (живые release-смоки Архитектора):
- Grok №2 OSD ✅ 653ae57; Hermes №7 services ✅ 1d7e285+60c09c7+a22b53f;
  Cline №6 tray-иконки ✅ b25dc97+8e7052a; Mimo №3 applications ✅ 0352e2a;
  эррата Архитектора b4c72a8 (UPower-интерфейс).
**Волна №3 роздана** (брифы в файлах миньонов, самодостаточные):
- **Grok №3** ✅ СДАН (f4edb88+6f24bb3, приёмка в GROK.md) — audio
  dispatch + эрратумы OSD (стартовый флэш, window-not-found via soft-hide).
- **Mimo №4** — миграция лаунчера на applications-сервис + mpsc-луп +
  strip_field_codes. В ПОЛЕ (WIP в дереве: launcher/*, main.rs; его
  mpsc-код заперт в stash@{0}!).
- **OpenCode №3** — DBusMenu сервисная часть. В ПОЛЕ (WIP: tray/*).
- **Hermes №8** — wallpaper-сервис (swww MVP). Ждём (его агент один раз
  выплюнул старый отчёт №4-6 вместо работы — скормить HERMES.md заново).
- Autohand, Cline — резерв (кандидаты: UI-попап DBusMenu, ReplaceExisting,
  полировка попапов/лаунчера — СНАЧАЛА спросить пользователя, что криво).

## Git

Свежее сверху: f4edb88 (osd эрратумы) ← 6f24bb3 (audio dispatch) ←
8e7052a ← b25dc97 (tray-иконки) ← b4c72a8 (upower эррата) ← 0352e2a
(applications) ← a22b53f ← 60c09c7 ← 1d7e285 ← 653ae57 (OSD) ← b2ad267.
`git log --oneline` — истина. **Identity (оба репо): dark-ohm /
dohm.labs@proton.me** (орг dohm-labs; системный юзер neo). Коммиты дня
за neo/mishabcbb — НЕ переписывать (пользователь так решил). Без
AI-трейлеров, стиль `область : что сделано`, поимённый add.

## Очередь после волны №3

UI-попап DBusMenu (по данным OpenCode №3), ползунки громкости (dispatch
готов), dock, gradient borders (Source), popups polish. Отложено
(DECISIONS.log): FLIP/transitions, 8-stop градиенты.

## Пользовательское окружение (не ломать)

- hyprland.lua: SUPER+equal/minus → wpctl микрофон ±5%; автостарт
  easyeffects (дефолтный source = easyeffects_source);
  kb_layout = "us,ru,il" (Alt+Shift).
- Пользователь работает в Vivaldi — процессы не трогать.
- Память-инфра после ребута НЕ автостартует: 9router
  (`systemctl --user start app-9router@autostart.service`, порт :20128)
  → podman start hindsight-embeddings hindsight-reranker hindsight →
  health :8888. hindsight-контейнер склонен к OOM (exit 137) — рестарт.
  401 в ретейне = протух ключ провайдера в 9router (чинит пользователь).

## Ключевые технические факты (кровью)

- Lua-Hyprland: диспатчи ТОЛЬКО Lua-формой в сокет
  (`hl.dsp.focus({ workspace = N })`); `hl.dsp.move` нет — есть
  `hl.dsp.window.move`. Истина — живой сокет, не wiki.
- zbus-прокси сверять с `busctl introspect` живого объекта (кейс
  UPower.DisplayDevice → org.freedesktop.UPower.Device, эррата b4c72a8).
- UX-смоки ТОЛЬКО release; gpui-оконный код — только живой прогон
  (RUST_LOG=info + grim; кропы magick -crop … -resize).
- KeyboardInteractivity::Exclusive ЗАПРЕЩЁН навсегда.
- gpui BGRA: сырой RGBA-пиксмап свапать (0,2) перед RenderImage.
- remove_window на layer-shell гоняет frame callbacks с log_err → пары
  «window not found»; для часто скрываемых окон (OSD) — soft-hide
  (display=None + пустой input region), destroy не звать (f4edb88).
- Иконки: тема — /usr/share/icons/default/index.theme (Inherits=Adwaita);
  hicolor/devices ПУСТ; цепочка наследования обязательна.
- Бар перерисовывается ежесекундно — в render() виджетов никаких
  аллокаций/IO без кэша.
- Float в Service::Data → НЕ derive Eq. zbus-хендлеры не на tokio.
- Деп-политика bleeding edge; reference/ НЕ коммитить; Kael=Apache-2.0.
- Hindsight: REST :8888, POST /v1/default/banks/chronos-ecosystem/memories,
  items[] с уникальными document_id; MCP игнорирует bank_id.

## Смоки: чем и как

- Бар/OSD/tray: `cargo build --release -p chronos` → `RUST_LOG=info
  ./target/release/chronos` → wpctl/notify-send/udiskie → grim.
- audio dispatch: `cargo run -p chronos-services --example
  audio-dispatch-smoke` (release).
- applications: example applications-smoke.
- Тесты: `cargo test --workspace --lib --bins` (130 зелёных на 2026-07-17).
