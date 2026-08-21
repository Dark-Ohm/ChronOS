# T338 — отчёт BACKEND: не сносить стол пользователя при старте

**Роль:** BACKEND. **Дата:** 2026-08-21.
**Зона изменена:** только `crates/services/src/wallpaper/mod.rs` (+97/−7).
`wallpaper_ctl.rs`, `ipc/mod.rs`, `display.rs` — НЕ трогал (см. «Чего не делал»).

## Что сделал

Один файл, четыре правки.

1. **Детекция чужого бэкенда.** `FOREIGN_BACKEND_BINS = ["hyprpaper",
   "swaybg", "mpvpaper", "gslapper"]` + `process_alive()` (тот же `pidof`,
   что и старый `daemon_alive`) + `foreign_backend_alive()`. awww в список
   НЕ входит — он общий с waytrogen, живой демон надо спрашивать, а не
   переспавнивать поверх. Сам GUI `waytrogen` тоже намеренно не детектим:
   он держит слой только опосредованно через эти демоны.

2. **Старт больше не спавнит пустой awww поверх чужого.** `ensure_daemon()`
   разнесён: теперь это охраняемая стартовая точка (`daemon_alive` → ничего;
   `foreign_backend_alive` → skip + `StartOutcome::SkippedForeignBackend`;
   иначе `spawn_daemon()`), а реальный спавн вынесен в `spawn_daemon()`.
   Стартовый путь `WallpaperSubscriber::new()` при `SkippedForeignBackend`
   ставит статус `Degraded("wallpaper managed externally")` и НЕ дёргает
   `awww query` (заведомо Connection refused). Явный пользовательский
   `Set`/`Next` идёт через новый `ensure_daemon_forced()` — там спавн как
   раньше, без оглядки на чужой бэкенд (пользователь выбрал awww).

3. **`refresh()` стал честным.** На ошибке (`awww query` Connection refused)
   теперь ставит `Degraded("awww daemon dead")` и сбрасывает
   `WallpaperState::default()` — UI не держит протухший путь обоев, которые
   awww уже не отдаёт (сценарий «кнопка Open waytrogen + mpvpaper убил awww»).

4. **Регресс-тест.** `foreign_backend_bins_excludes_awww_and_covers_foreign_backends`
   — awww не считается чужим, все четыре чужых бэкенда в списке.

## Как проверил

### Сборка и тесты (своим прогоном)

```
cargo test -p chronos-services wallpaper
  → 11 passed; 0 failed  (включая новый тест)
cargo check -p chronos
  → компилируется; только предсуществующие warning'и
cargo build --release -p chronos
  → Finished release в 3m47s
```

mtime-сверка (урок ARCHITECT.md): `stat -c '%Y %n'` →
`mod.rs` = 1787342127, `target/release/chronos` = 1787342515 — бинарь
свежее исходника, кадры сняты с текущей сборки, не с чужого артефакта.

### Живой smoke (grim + hyprctl layers + лог)

Реальный Hyprland-сеанс владельца (`$HYPRLAND_INSTANCE_SIGNATURE` задан),
`awww`/`mpvpaper`/`gslapper`/`swaybg`/`hyprpaper` на PATH. Артефакты:
`.chronos-ops/dump/qa-ux/T338/` (фреймы + `case1/case2-*.txt`).

**CASE 1 — рестарт ChronOS при живом mpvpaper (приёмка п.1 «Готово когда»):**
убил awww-daemon, поднял `mpvpaper -o "loop no-audio" -f DP-1
<видео из ~/Pictures/Wallpapers>`, перезапустил chronos. После:

```
chronos    : 2933764   (жив)
mpvpaper   : 2933624   (жив — не убит стартом)
awww-daemon: none      (НЕ стартовал)
hyprctl layers (DP-1): mpvpaper a:1  + bar  — слоя awww-daemon НЕТ
лог: INFO chronos_services::wallpaper: foreign backend owns the wallpaper
     layer; not starting awww-daemon
     (строки "starting awww-daemon" НЕТ)
```

То есть ровно три приёмных пункта: mpvpaper жив, `starting awww-daemon`
отсутствует, стол не Hyprland-сплэш (mpvpaper остался верхним).

**CASE 2 — чистый сеанс без чужого демона (приёмка п.2):** убил mpvpaper,
перезапустил chronos. После:

```
awww-daemon: 2934770   (спавнится как раньше)
лог: INFO chronos_services::wallpaper: starting awww-daemon
hyprctl layers: awww-daemon a:0 (прозрачный, НЕ чёрный поверх — T244 не регресснул)
```

Конечное состояние машины восстановлено к исходному: chronos + awww-daemon
живы, mpvpaper/gslapper/swaybg/hyprpaper отсутствуют.

## Чего НЕ делал (и почему)

- **Пункт 3 брифа трактую как guardrail, не как действие.** «не оставлять
  `color: 000000` без явного действия пользователя» — реализую как: стартовый
  путь никогда не авто-restore'ит кэш (это T244, осталось как было), а пустой
  awww меняется только явным пользовательским `Set`/`Next`. Сознательно НЕ
  стал убивать «пустой живой awww-daemon»: этого пункта нет в приёмке
  «Готово когда», а такой kill рискует прибить awww, который waytrogen сам
  поднял и ещё не успел заполнить (гонка на старте). Если нужно именно
  «убить пустой awww» — скажи, это отдельная маленькая правка.
- **`open_waytrogen_gallery` не трогал.** Он и так неблокирующий
  (`Command::spawn()`, не `wait()`). «После закрытия → refresh или честный
  daemon dead» закрыл веткой «честный daemon dead» через правку `refresh()`
  выше; задержанный refresh (3с) остался на существующих call-site'ах
  (`ipc/mod.rs`, `display.rs`) — это вне зоны брифа (и вне «Не трогать» я туда
  не лез).
- UI Display-карточки (T339), theme, frame — не трогал.
- Свой video-wallpaper не писал, `awww restore` не вызывал.
- Не коммитил, тикет из `active/` не двигал (приёмка за архитектором).

## Оговорки

- Детекция — по `pidof <имя процесса>`. Если чужой бэкенд запущен под другим
  именем процесса (кастомный скрипт/обёртка), его не увидим. Для названных в
  брифе `mpvpaper`/`gslapper` и соседей `swaybg`/`hyprpaper` — покрыто.
- `awww` как бэкенд waytrogen остаётся «не чужим»: живой awww спрашиваем,
  а не переспавниваем — это и есть мирное сосуществование (T244).
