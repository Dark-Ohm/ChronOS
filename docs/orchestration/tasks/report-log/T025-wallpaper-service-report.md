<!-- T025 — migrated 2026-07-22 from docs/orchestration/report-log/hermes-report-8-rework.md — see docs/orchestration/tasks/MIGRATION.md -->

# Hermes — задание №8: wallpaper-сервис (awww MVP, мультибэкенд-каркас)

Дата: 2026-07-17. Исполнитель: Hermes. Статус: ПРИНЯТО (доработка, 1 баг
починен), коммит сделан.

## Приёмка (доработка, 2026-07-17, Архитектор)
Архитектор прогнал живой apply-смок на графической сессии:
- Демон поднялся идемпотентно (`pidof`-guard).
- `Set` РАБОТАЕТ — awww реально применил тестовую картинку на DP-1
  (`awww query` показал её живьём).
- 137 тестов зелёные.
- NOTICE перенесён Архитектором в НАСТОЯЩИЙ ../Source/NOTICE (мой каталог
  ChronOS/Source/ не существовал — Source это соседнее репо; зафиксировано
  отдельно, себе на ус: проверять реальный путь перед созданием).

## ОДИН БАГ (исправлен)
`parse_query` не переваривал живой вывод awww 0.12.1: строки идут с
ведущим `: ` (`: DP-1: 2560x1440, ...`), а `split_once(':')` давал пустое
имя монитора → смок падал на верификации, хотя обои стояли.

Правка:
1. `parse_query`: `line.trim_start_matches([':', ' '])` перед сплитом —
   терпимость к ведущему `: `. Фраза `currently displaying: image: ` ищется
   через `find` (она посередине строки, не prefix) → путь извлекается
   корректно. Ветка `color: RRGGBB` (монитор без картинки) НЕ считается
   изображением → per_monitor без записи, паники нет.
2. Фикстуры в тестах ЗАМЕНЕНЫ на живой вывод awww 0.12.1 (оба варианта:
   `image:` и `color:`), старая выдуманная (`eDP-1: ...`) убрана.

Юнит-тесты (фикстуры = реальный вывод):
- parse_query_fills_per_monitor_and_current (HDMI-A-1 color + DP-1 image →
  только DP-1 в per_monitor, current = image-путь)
- parse_query_handles_no_image (`: HDMI-A-1: ...color: 000000` → current None,
  per_monitor пуст)
- parse_query_handles_spaces_in_path (ведущее `: ` + пробел в пути)
- parse_query_ignores_unrelated_lines (нет `currently displaying` → игнор)

## Верификация (реально прогнано)
- cargo test -p chronos-services --lib wallpaper → 10 passed; 0 failed.
- cargo test --workspace --lib --bins → 137 passed; 0 failed.
- Живой apply-смок: подтверждён Архитектором (демон + Set + query).

## Что сделано (код)
- crates/services/src/wallpaper/types.rs: WallpaperState { current,
  per_monitor: HashMap, backend: Backend }, enum Backend (Awww + заглушки
  Hyprpaper/Swaybg/Mpvpaper/Gslapper), WallpaperCommand::Set{path,monitor,
  transition}.
- crates/services/src/wallpaper/mod.rs: WallpaperSubscriber (§5.1: Handle::
  current, rt.block_on, runtime_guard-тест), идемпотентный демон (pidof +
  spawn + retry query с таймаутом), dispatch → `awww img --resize crop
  [--outputs MON] [--transition-type T] path`, чистые command_to_awww_args /
  parse_query / is_image. Мониторы НЕ энумерируются (passthrough; имена из
  CompositorSubscriber в UI).
- crates/services/src/lib.rs: pub mod wallpaper + реэкспорты + поле Services
  + init_all (только свои строки).
- crates/app/src/state.rs: accessor wallpaper(cx) (своя строка).
- crates/services/examples/wallpaper-smoke.rs: tracing_subscriber::fmt::init()
  обязателен; exit(1) при нуле; сохраняет/восстанавливает обои пользователя;
  генерит картинку в /tmp (не ~/Pictures); финальный pkill awww-daemon.

## Коммит
Один: `services : wallpaper-сервис (awww MVP, мультибэкенд-каркас)`.
Поимённо: crates/services/src/wallpaper/**, crates/services/examples/
wallpaper-smoke.rs, crates/services/src/lib.rs, crates/app/src/state.rs,
hermes-report.md.
Cargo.lock НЕ застейжен: его изменение (удаление crossbeam-channel) —
чужое (Mimo, switch applications на tokio mpsc), мой сервис не добавляет
зависимостей. NOTICE НЕ трогал (сделал Архитектор в ../Source).

## Урок (в копилку)
Фикстура, не снятая с живого вывода — не фикстура, а фантазия. Нет живой
среды — писать в отчёте «фикстура умозрительная, снять живьём при приёмке»,
а не выдумывать формат. (Случалось у OpenCode — теперь зафиксировано.)
