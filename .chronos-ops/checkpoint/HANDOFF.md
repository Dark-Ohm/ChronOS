# HANDOFF — состояние проекта для новой сессии Архитектора

**Это файл СОСТОЯНИЯ, не журнал.** Здесь только то, что верно прямо
сейчас. История сессий, разборы приёмок по дням, старые чекпоинты —
`checkpoint/log/` (разложен по месяцам 2026-08-19, T306: `handoff-2026-07.log`,
`handoff-2026-08.log`, точка входа `log/INDEX.md`) — грепать по
T-ID/дате/симптому, не читать подряд.

**Правило гигиены:** этот файл живёт в пределах ~200 строк. Всё, что
устарело — переносится в `checkpoint/log/`, а не копится тут. Разрез
2026-08-18 случился потому, что файл дорос до 3973 строк и его перестали
читать целиком — включая меня.

**Обновлено: 2026-08-19.** HEAD — `git log -1` (на момент записи
`f2cacee`).

---

## Где стоим

Инбокс `.chronos-ops/reports-fresh/` **пуст**. Очереди `back`, `recon`,
`design` пусты. `front` пуста. Живая работа — **QA: T309** (живой
клиентский UX/визуальный критик-аудит шелла, обе темы, все поверхности —
не код, только клики + честная оценка. Владелец прямо просил
профессионального некомплиментарного критика, см. `active/qa/QA.md`).

**Волна 2026-08-18/19 принята полностью:** T305 (`f326fc7`, control-center
popup), T303 (`c6df21a`, wrap matte-геометрия базовая), T301 (`96f713a`,
composer Select эллипсис), T307 (`601f8f0`, wrap hot-reload), T308
(`f2cacee`, wrap-матте `exclusive_zone: Some(px(-1.))` opt-out — не
съезжает при открытой панели), T306 (RECON, `fc1945e`, журнал разрезан
по месяцам). Детали — `MIGRATION.md` и `active/front/FRONTEND.md`.

**Припарковано (`active/hold/`):** T281 (левая workspace Slice A — код
принят, открыт ровно один гейт: живой рестарт шелла, лента И сессия
агента должны восстановиться; это за владельцем, миньону не выдавать),
T191, T224, T277, T282.

**T285 (ACP `load_session`) — STOP по апстриму**, ветка
`feat/t285-acp-load-session` парковая. Причина — в `REJECTED.md`.

## Открытые долги

- **T281 гейт 8** — живой рестарт (владелец).
- **T266** — попапы (volume / OSD / notifications / tray / dock) живьём
  не сняты; покрытие пока аналитическое через `bg.elevated`. Смок:
  `surface_alpha` 0.7, попапы на светлых и тёмных обоях.
- **T303** — живой grim после правки.
- Память-инфра после ребута не автостартует (см. «Окружение»).

## Кухня

`.chronos-ops/` под git — контрибьюторы берут задания отсюда.
`docs/orchestration/` закрыт и физически удалён 2026-08-18.

- Тикеты: `active/<role>/TNNN-slug.md` (+ `active/hold/`), точка входа
  роли — `active/<role>/<ROLE>.md` (тонкий указатель, не журнал).
- Отчёты: инбокс `reports-fresh/` → принятые `reports-log/<role>/`.
- Тикеты после приёмки: `done/<role>/`, на доработку `rework/<role>/`,
  отклонённые `reject/<role>/`.
- Реестр всех T-ID с вердиктами приёмки — `.chronos-ops/MIGRATION.md`.
- Что в какой каталог класть — `.chronos-ops/RULES.md`.
- Заметки и кадры-улики — `dump/notes/`, старые точки входа ролей —
  `dump/legacy-agents/`.

`skills/` вынесен из репо: единый вольт
`/home/neo/projects/chronos-ecosystem/skills/`, вне git, общий на всю
экосистему. В ChronOS каталога `skills/` нет; CI-job `skill-proofs` и
`scripts/git-hooks/pre-commit` удалены вместе с ним.

`docs/` — только продукт и сайт: `product/`/`style/`/`guides/` плюс
`index.html`/`.nojekyll`/`landing/` (GitHub Pages). Hyprland-правила
живут в `packaging/hyprland/` — **путь
`packaging/hyprland/40-windowrules-chronos.lua` менять нельзя**, живой
конфиг `~/.config/hypr/modules/40-windowrules.lua:4` жёстко его
`dofile`'ит, переезд сломает reload молча.

## Канон (при расхождении побеждают они)

| Файл | Что в нём |
|---|---|
| `checkpoint/ARCHITECT.md` | роль архитектора + живой список дисциплины |
| `checkpoint/ARCHITECTURE.md` | принятые архитектурные решения |
| `checkpoint/REJECTED.md` | что рассматривали и отклонили, почему |
| `checkpoint/HANDOFF.md` | **этот файл** — состояние сейчас |
| `checkpoint/log/INDEX.md` | журнал сессий (архив, по месяцам) |
| `checkpoint/TBD.md` | оперативные хвосты |
| `checkpoint/MEMORY.md`, `SOUL.md` | память и журнал сессий |
| `.chronos-ops/MIGRATION.md` | сквозная история T-ID |

## Кровные технические факты

Каждый оплачен живым багом. Не «стиль» — правила.

- **`cx.background_spawn(...)` без `.detach()` — баг.**
  `gpui_scheduler::Task` — `#[must_use]`, drop = cancel. Голый вызов или
  `let _ =` роняет Task сразу: быстрые футуры проскакивают, медленные
  (zbus/hyprctl/subprocess) дохнут молча. Симптом: `on_click` в логе
  есть, эффекта нет.
- **`AsyncApp::update` в форке возвращает `R`, не `Result`**
  (`Source/gpui/src/app/async_context.rs:163`). Значит `let _ =
  cx.update(...)` ничего не глотает — правка тут «снять `let _ =`», а не
  заворачивать в `if let Err`. Не переоткрывать (урок T271).
- **Никаких побочных эффектов в `render()`.** Он зовётся сотни раз в
  секунду (замер/лэйаут/пейнт + каждый сервисный сигнал, cava шлёт
  30 fps). Любое накопление — с гейтом по времени (≥1 с) и кэшем
  показанного; считать СКОРОСТЬ, а не дельту за неизвестный интервал.
  Юниты этого не ловят: они кормят синтетические дельты.
- **`tokio::task::spawn_blocking` вне tokio-runtime виснет.**
  `cx.background_spawn` — GPUI executor, не tokio: не паникует, просто
  не завершается. Для subprocess из GPUI-таски — `std::thread::spawn` +
  `tokio::sync::oneshot`. (zbus работает — сам поднимает runtime.)
- **`window.display(cx)` == None для layer-shell окон** (форк,
  `Source/gpui/src/window.rs:2293`): wayland-backend не заполняет
  `display_id`. «Попап на дисплее кликнутого окна» так не сделать;
  `display_id` в `WindowOptions` при `open_window` честен — роутить
  через конфиг пультового монитора.
- **`window.bounds()` для `center=true` окон врёт.** Wayland не отдаёт
  позицию toplevel'а: origin заморожен на запрошенном `(0,0)`. Живой
  `hyprctl` — единственный источник истины.
- **`KeyboardInteractivity::Exclusive` ЗАПРЕЩЁН** — вешает input-стек
  композитора.
- **Призыв левой панели = rail-only, и это дизайн, а не баг** (T220,
  подтверждено T302 2026-08-18). `toggle-side-panel-left` / `Super+A`
  открывают ОБА окна (`rail` + `content`), но `panel_width = RAIL_WIDTH`
  → `visible_content_width == 0`: видна только 40px полоса рейла, чат не
  выезжает. Контентное окно при этом висит прозрачным 920px `Overlay` над
  рабочим столом — клики сквозь него проходят, `content_input_region` при
  нулевой ширине пуст (`side_panel_left/state.rs:73`). Чат раскрывает
  клик по иконке таба на рейле или `chronos-ipc expand-left`. Так сделано
  потому, что призыв «сразу с чатом» прятал композер (баг T137). Плюс
  enter-анимация 260 мс стартует с `opacity 0` — `grim` в первые полсекунды
  даёт ровно ту же картину «пустая зона, сквозь неё обои».
- **`remove_window` на часто скрываемых layer-shell окнах шумит
  «window not found»** — soft-hide (display=None + пустой input region).
  Полный разбор двух причин — `checkpoint/log/handoff-2026-07.log`, греп
  `СИСТЕМНЫЙ БАГ`.
- **Композиторные события: «сменился» ≠ «список изменился».** Имена
  хендлеров в крейте `hyprland` генерируются макросом `events!` из
  вариантов enum — грепом `pub fn add_` не находятся, сверяться со
  списком в `event_listener/shared.rs`.
- **Lua-Hyprland:** диспатчи только Lua-формой в сокет (`hl.dsp.window.move`,
  не `hl.dsp.move`); голый `hyprctl dispatch` мёртв. Истина — живой
  сокет, не wiki.
- **Блюр (Lua-Hyprland 0.56.2):** глобальный `decoration.blur.enabled`
  обязателен, иначе корректный layer-rule не рендерит ничего;
  `ignore_alpha` в layer rule молча убивает блюр; `hl.layer_rule`
  идемпотентен по имени — правка файла требует рестарта Hyprland
  (`hyprctl reload` сбрасывает eval-глобалы).
- **Контент поверх заливки — через `chronos_ui::on_fill()`**, не
  `theme.text.*`: токены текста переворачиваются со схемой, заливки нет.
  `status.*` у схем РАЗНЫЕ (Mocha vs Latte).
- **zbus/D-Bus сверять с `busctl introspect`,** не с докой: UPower
  DisplayDevice = интерфейс `.Device`; `GetLayout` возвращает
  `(u(ia{sv}av))`, не `(uv)`.
- **gpui BGRA:** сырой RGBA-пиксмап свапать (0,2) перед `RenderImage`.
- Трей забивается безымянными `StatusNotifierItem` от Chromium/Vivaldi —
  фильтр безымянных + дедуп по bus-имени на нашей стороне.
- Бар перерисовывается ежесекундно: в `render()` виджетов ноль
  аллокаций и IO без кэша.

## Смоки: чем и как

**«Компилируется и тесты зелёные» для оконного/UX-кода — ничто.**

- Шелл: `cargo build --release -p chronos` → `pkill -x chronos` →
  `RUST_LOG=info ./target/release/chronos` → wpctl / notify-send /
  `chronos-ipc` → `grim`.
- UX-смоки ТОЛЬКО release. Кропы: `magick -crop WxH+X+Y -resize N%`.
- **Левая панель (T220):** призыв (`Super+A` / `chronos-ipc
  toggle-side-panel-left`) → `hyprctl layers -j | grep side_panel_left`:
  ждать `rail` 40px И `content` 920px, при этом видимая доля контента = 0.
  Раскрыть чат — клик по иконке таба на рейле или `chronos-ipc expand-left`.
  Снимать `grim` не раньше чем через ~0.5 с после раскрытия, иначе поймаешь
  enter-фейд и решишь, что панель пустая.
- `hyprctl clients -j` — обычные toplevel (лаунчер); `hyprctl layers -j`
  — layer-shell (bar/dock/osd/notifications/tray_menu). Проверять
  «открылось ли окно» надёжнее через `layers -j`, чем через grim-кроп.
- Тесты: `cargo test --workspace --lib --bins`; точечно —
  `cargo test -p chronos --bins <модуль>`.
- **ydotool** для кликов по попапам: юнита нет, `sudo ydotoold` руками +
  `chmod 666 /tmp/.ydotool_socket`; калибровка `hyprctl cursorpos` ⇄
  `ydotool mousemove -a` заново каждую сессию, формула плавает
  (на этой машине absolute ≈ экран/2), только одношаговые прыжки.
- `wf-recorder` для процессных багов (geometry-рецепт как у `grim -g`,
  останавливать `kill -INT`, не `-9`).

## Пользовательское окружение (не ломать)

- **DP-1** (Samsung, главный, слева) 2560×1440 @ 0,0; **HDMI-A-1**
  (Dell, справа) 1920×1200 @ 2560,0. Бар 2560×32.
- `hyprland.lua`: SUPER+equal/minus → микрофон ±5%; SUPER+L → лаунчер
  (сокет `$XDG_RUNTIME_DIR/chronos.sock`, payload `toggle-launcher`);
  автостарт easyeffects; `kb_layout = "us,ru,il"` (Alt+Shift).
- Пользователь работает в Vivaldi — процессы не трогать. Обои/мониторы
  дёргать только кратко в смоках и ВОЗВРАЩАТЬ как было.
- Память-инфра после ребута НЕ автостартует:
  `systemctl --user start app-9router@autostart.service` (:20128) →
  `podman start hindsight-embeddings hindsight-reranker hindsight` →
  health :8888. Hindsight склонен к OOM (exit 137) — рестарт. 401 =
  протух ключ провайдера в 9router, чинит владелец.
- Ядро CachyOS после обновления живёт без модулей до ребута — ломает
  podman-сеть и ydotool.
