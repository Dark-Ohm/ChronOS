<!-- T025 — SUPERSEDED draft, migrated 2026-07-22 from orchestration/report-log/hermes-report-8.md — canonical version is in orchestration/tasks/report-log/, see orchestration/tasks/MIGRATION.md -->

# Hermes — задание №8: wallpaper-сервис (awww MVP, мультибэкенд-каркас)

Дата: 2026-07-17. Исполнитель: Hermes. Статус: КОД ГОТОВ, ЮНИТ-ТЕСТЫ ЗЕЛЁНЫЕ,
ЖИВОЙ APPLY-СМОК ЗАБЛОКИРОВАН СРЕДОЙ (нет Wayland-композитора в этой сессии).

## Что сделано

### Новый сервис crates/services/src/wallpaper/
- types.rs
  - WallpaperState { current: Option<PathBuf>, per_monitor: HashMap<String,PathBuf>,
    backend: Backend } — Eq, float'ов нет.
  - enum Backend { Awww, Hyprpaper, Swaybg, Mpvpaper, Gslapper } — реализован
    только Awww; остальные ветки `warn!("backend not implemented")`.
  - WallpaperCommand::Set { path, monitor: Option<String>, transition: Option<String> }.
- mod.rs (шаблон §5.1, как AudioSubscriber)
  - WallpaperSubscriber: sync new() внутри rt.block_on, Handle::current(),
    runtime_guard-тест (new() паникует вне tokio, стартует внутри).
  - Демон: `pidof awww-daemon` → нет, спавн `awww-daemon` (null-stdio); ожидание
    сокета через retry `awww query` с таймаутом (не blind sleep).
  - dispatch(Set) → `awww img --resize crop [--outputs MON] [--transition-type T] path`.
  - Чистые функции: command_to_awww_args (юнит-тест), parse_query (юнит-тест на
    фикстурах живого вывода awww query), is_image.
  - Мониторы НЕ энумерируются wayland-client'ом — passthrough; имена для UI берутся
    из CompositorSubscriber (DP-1/HDMI-A-1).
  - Док-комментарий: состояние меняется только нашими командами, внешний `awww img`
    мимо сервиса — известный лимит (не polling).

### Wiring (только свои строки)
- crates/services/src/lib.rs: pub mod wallpaper + реэкспорты (вкл. AWWW_BIN,
  AWWW_DAEMON_BIN, Backend, command_to_awww_args, parse_query) + поле Services.wallpaper
  + строка в init_all().
- crates/app/src/state.rs: добавлен accessor `wallpaper(cx)` (своя строка, рядом
  с applications).
- Source/NOTICE: строка об атрибуции waytrogen (Unlicense, public domain) — по брифу.

### Пример
- crates/services/examples/wallpaper-smoke.rs: tracing_subscriber::fmt::init()
  обязателен; exit(1) при нулевом результате; сохраняет обои пользователя ДО,
  ставит сгенерированную magick-картинку в /tmp на ОДИН монитор, query подтверждает,
  восстанавливает; финальный pkill awww-daemon (чтобы не осиротел).

## Верификация (реально прогнано)
- cargo test -p chronos-services --lib  → 68 passed; 0 failed (в т.ч. 10 wallpaper).
- cargo test --workspace --lib --bins  → 137 passed; 0 failed.
- cargo build -p chronos-services --example wallpaper-smoke → OK.
- Живой прогон примера (headless): дошёл до fail! с EXIT=1
  («awww query did not confirm the test wallpaper on the target monitor»).
  Критерий «падать при нуле» выполнен; НЕ завис (hang починен: daemon child →
  null-stdio + внутренний таймаут 25с + pkill в конце).

## БЛОКЕР — живой apply-смок
awww-daemon требует живой Wayland-композитор. В этой headless-сессии он падает сразу:
  $ awww-daemon
  [WARN] failed to read cache file ...
  [INFO] BumpPool with: 1 buffers. Size: 14400Kb
  [WARN] We failed to find wayland buffer with id: 11. This should be impossible.
  [INFO] Removed socket at /run/user/1000/wayland-1-awww-daemon.sock
  [INFO] Goodbye!
  $ awww query
  Error: "Socket file '/run/user/1000/wayland-1-awww-daemon.sock' not found ..."
Значит применить обои и снять release-смок здесь нельзя. Это тот же класс лимита,
что HANDOFF фиксирует для GUI/display-смоков («только живой прогон»). Логичный
выход: прогнать wallpaper-smoke на графической сессии (реальный Hyprland):
  cargo run -p chronos-services --example wallpaper-smoke

## Что осталось (не блокер кода)
- Коммит отдельный: `services : wallpaper-сервис (awww MVP, мультибэкенд-каркас)`,
  поимённо wallpaper-файлы + lib.rs/state.rs/NOTICE, `git diff --staged` глазами
  перед коммитом.
- Живой apply-смок на графической сессии (после коммита, у Архитектора/релизе).
- Прочие бэкенды (hyprpaper/swaybg/mpvpaper/gslapper) — заглушки enum'а, по брифу
  в этом задании не реализуются.

## Решения (зафиксированы)
- Демон спавним при старте сервиса идемпотентно (pidof-guard), ждём сокет retry'ем,
  не sleep наугад.
- Poll не нужен: состояние меняется только нашими командами.
- Мониторы: имена из CompositorSubscriber, не wayland-client (как у waytrogen).
- NOTICE-строка о waytrogen добавлена (Unlicense — attribution по приличиям).
