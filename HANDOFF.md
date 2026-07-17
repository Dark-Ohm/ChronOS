# HANDOFF — контекст для новой сессии Архитектора

**Обновлено: 2026-07-17. Читать сверху вниз — самое актуальное первым.
При расхождении с ARCHITECTURE.md/DECISIONS.log побеждают они.**

## Кто ты и как работаешь

Lead Architect Agent проекта **ChronOS** — Rust/GPUI desktop shell для
Hyprland 0.55.4+ (bar/dock/launcher/notifications/osd, плагины Luau,
144 FPS, hot-reload). Сам НЕ кодишь (исключение: документы/однострочная
механика — прецедент: правка NOTICE). НЕ спавнишь своих субагентов
(прецедент: сжёг 30% сессии, был остановлен). Задания миньонам — через их
файлы (CLINE.md, HERMES.md, OMP.md), пользователь скармливает их своим
локальным агентам, отчёты возвращаются в `<имя>-report.md`. Приёмку
делаешь сам грепами/диффами — КАЖДОЕ утверждение отчёта сверяй с деревом
(миньоны врут регулярно, счёт 4+). Канон — ARCHITECTURE.md, отклонённое —
DECISIONS.log. Отвечать по-русски, коммиты БЕЗ AI-трейлеров, стиль
`область : что сделано`.

## Два репозитория

- **ChronOS** (`~/projects/chronos-ecosystem/ChronOS`) — сам шелл.
  Git с 2026-07-17, ветка master. `reference/` в .gitignore
  (нелицензированный gpui-shell НЕ коммитить — юридически критично).
- **Source/** (`~/projects/chronos-ecosystem/Source`) — наш форк GPUI
  «gpui-ce chronos edition», отдельный git. 18 крейтов (9 базовых + 9
  форкнутых zed-internal), хвост zed-зависимостей на git rev 876ec5a8.
  ChronOS зависит от него path-деps'ами.

## Приёмки первой волны (детали; актуальное — секция «СЕЙЧАС В ПОЛЕ» ниже)

- **Hermes ✅ ПРИНЯТО** — демон `org.freedesktop.Notifications`
  (server-side zbus 5.17, rewrite-по-паттерну, донор нелицензирован — 0
  копипаста) + launcher per-frame focus re-assert. 15/15 тестов, живой
  smoke (notify-send/busctl/gdbus). Коммиты ChronOS: 0316de6 + 3b1a473.
- **Cline ✅ ПРИНЯТО с эрратой** — easing (28 кривых, `EasingCurve`) +
  spring-интегратор из Kael (Apache-2.0) в Source/gpui. 21/21 тестов.
  Коммит Source: ef6b4bd. Эррата: закоммитил чужой NOTICE не глядя
  (битый URL, без easing/spring) — исправлено Архитектором: 8881d4d.
- **OMP ✅ ПРИНЯТО** — 2-pass backdrop blur из Kael: `BlurRect` в scene.rs,
  `paint_blur` в window.rs, пайплайны в gpui_wgpu. Коммит Source: 9c9b6f5.
  Визуал подтверждён grim-скриншотом (пример `gpui/examples/blur.rs`).
  Все трое приняты — очередь свободна.

## Состояние git (на 2026-07-17 вечер)

- ChronOS master (свежее сверху): 4aa3c10 (задания 5 агентам) ← 790554d ←
  0bc770d ← bfb1503 (launcher focus trap снят) ← 7eaf6e1 ← 05ea4d1 ←
  e40a4e6 (попапы+theme-миграция) ← 1387999 (crates/ui) ← 0cd18c1 ←
  7af364e (toplevel) ← … ← 03b0c87 (baseline). `git log --oneline` — истина.
- Source master: `9c9b6f5` (blur) ← `8881d4d` (NOTICE) ← `ef6b4bd`
  (easing+spring) ← `3ce3466` (skeleton).
- git identity локально: neo / mishabcbb@gmail.com.

## СЕЙЧАС В ПОЛЕ (обновлено 2026-07-17, перед ребутом/yay -Syyu)

**Bar-волна ПОЛНОСТЬЮ ЗАВЕРШЕНА и принята** (все виджеты живьём на release):
clock ✅ (Cline, e415718+эррата e2845bd), workspaces ✅ (Hermes, cfcef99, клик
работает после Lua-socket фикса), network ✅ (Autohand, 1f508d6), battery ✅
(Mimo, ba78b70), tray ✅ (OpenCode, 435af47+5b31628+75a1061 — ayatana-фикс,
udiskie-бейдж виден живьём). **Audio-сервис ✅** (Grok №1, 079f1d4 — MVP wpctl
+ 250ms poll, внешние изменения доезжают за ~400мс; DECISIONS.log 2026-07-17).
104 теста workspace зелёные. Всё закоммичено, дерево чистое.

Раздача НОВОЙ волны (задания в файлах, самодостаточные для свежих сессий):
- **Grok** (GROK.md №2) — OSD громкости (crates/app/src/osd/).
- **Cline** (CLINE.md №6) — иконки tray: icon-theme lookup + pixmap.
- **Hermes** (HERMES.md №7) — services follow-ups: wired:bool, has_battery,
  network-флап.
- **Mimo** (MIMO.md №3) — wallpaper-сервис.
- Autohand, OpenCode — резерв (кандидаты: DBusMenu контекст-меню трея,
  полировка лаунчера после уточнений пользователя).

Пользовательские времянки в hyprland.lua (НЕ ломать, потом заменим на ChronOS):
- SUPER+equal/minus → wpctl @DEFAULT_AUDIO_SOURCE@ ±5% (уйдёт в OSD).
- Автостарт easyeffects --gapplication-service (шумодав микрофона; дефолтный
  source = easyeffects_source, записан в WirePlumber state).
- 2026-07-17 запущен полный апгрейд системы (yay -Syyu с --overwrite по npm) —
  после ребута возможны новые версии ядра/тулчейна; если сборка странно падает,
  сначала `cargo clean -p <crate>` и проверь rustc --version.

## Сделано ранее (все приняты)

- Демон нотификаций + попапы UI (Hermes №3/№4), theme-крейт crates/ui.
- Kael-порты в Source/: easing+spring (Cline), backdrop blur (OMP,
  визуал подтверждён скриншотом).
- Launcher: XDG toplevel + Critical focus trap снят (см. секцию ниже).

## Очередь после tray-хвоста и audio

1. OSD + ползунки громкости (после audio-сервиса Grok; brightness — потом).
2. Полировка попапов («выглядит криво» — уточнить у пользователя) и
   лаунчера (остаточная «баговынность», монитор DP-1 vs HDMI).
3. applications + wallpaper сервисы (S). Мелкие follow-ups: has_battery в
   UPowerData, wired:bool в NetworkData, network «signal timeout» флап
   раз в минуту, SVG/icon-theme иконки виджетов вместо юникода.
4. Gradient borders (Source, после блюра), dock.
5. Отложено (DECISIONS.log): FLIP/transitions (нет transform в Style),
   8-stop градиенты, effect layers, color filter.

## Lua-Hyprland: диспатчи (кровью, 2026-07-17)

hyprland-rs `Dispatch::call` НЕСОВМЕСТИМ с Lua-Hyprland — сервер заворачивает
всё из сокета в Lua и падает парсером. Чтение (events/data) работает. Команды —
только Lua-формой: `/dispatch hl.dsp.focus({ workspace = N })` в
`$XDG_RUNTIME_DIR/hypr/$HYPRLAND_INSTANCE_SIGNATURE/.socket.sock`
(compositor/hyprland.rs::command_to_socket_line, 2a076a3 + эррата df65f42:
`hl.dsp.move` не существует, надо `hl.dsp.window.move`). Wiki описывает
классический Hyprland — истина для форка ТОЛЬКО живой сокет
(`hyprctl dispatch '<lua>'` для проверки). Полный разбор — DECISIONS.log
2026-07-17.

## Launcher Critical — ЗАКРЫТ (bfb1503, «немножко баговынно, но работает»)

XDG toplevel + windowrules (Lua). Пять фиксов Архитектора поверх хотфикса
OMP — все найдены ТОЛЬКО живым прогоном с RUST_LOG=info (полный разбор —
приёмка в OMP.md): detach() на Subscription; focus_input на активацию;
track_focus на корневом div; lowercase-имена клавиш; предикат «открыт» =
handle.is_some() + close_this() со сверкой окна (призраки, кража хендла,
was_active-гейт против ложного active=false первого Wayland-configure).
Остаточная «баговынность» — полировка отдельным заданием (спросить
пользователя, что именно). Бинд: SUPER+L (hyprland.lua пользователя).

## Ключевые технические факты (кровью заработанные)

- **Смоки производительности/UX — ТОЛЬКО release-сборка**: debug-gpui даёт
  ощущение «пинг 300» на вводе (живой случай 2026-07-17).
- **gpui-оконный код не верифицируется тестами** — только живой прогон
  (RUST_LOG=info, tracing в observer'ы, hyprctl clients/activewindow).
- gpui: имена клавиш — lowercase ("escape"); `.track_focus(&handle)`
  обязателен, иначе клавиши мимо; Subscription — `#[must_use]`, дроп =
  отмена; фокус до активации окна — no-op; первый Wayland-configure шлёт
  ложный active=false; события активации от умерших окон приходят
  асинхронно ПОСЛЕ открытия нового — глобальный close() ворует хендлы.

- **KeyboardInteractivity::Exclusive ЗАПРЕЩЁН** — фризит input-стек
  Hyprland/Niri. Только OnDemand + per-frame re-assert.
- **zbus 5.17 object server диспатчит хендлеры на СВОЁМ executor-потоке,
  не на tokio** — `tokio::spawn` внутри хендлера паникует. Рецепт:
  std::sync::Mutex для состояния, `Handle::current()` захватить в `new()`
  для таймеров. (hermes-report №3, секция «Ключевые решения».)
- Деп-политика: **bleeding edge** (CachyOS/Arch) — новейшие версии,
  донорский код адаптируем к текущим API, пины не наследуем (MEMORY.md §Rules).
- Kael = Apache-2.0 (можно портировать с атрибуцией в NOTICE);
  gpui-shell = БЕЗ лицензии (только rewrite-по-паттерну, 0 строк).
- Float в Service::Data → НЕ derive Eq (трижды наступали).
- Сервисы: sync `new()` внутри `init_all()`/`rt.block_on`, паника вне
  runtime покрывается runtime_guard-тестом (network/upower — образцы).
- Source/ активно шуршит и вне сессий (крейты появлялись, adk-rust
  исчезал) — провенанс июльского форка gpui-ce у пользователя НЕ выяснен
  (открытый вопрос).
- Hindsight (банк chronos-ecosystem): с хоста живой порт — **:8888** (REST
  /v1/default/banks/...); :8080 (nginx) может молчать при живых контейнерах
  (pasta). Ретейн — POST .../memories, items[] с уникальными document_id.
- Тесты Source: 3 падения svg_renderer — pre-existing (нет ассетов
  шрифтов), не блокер.

## Смоки: чем и как

- Демон нотификаций: погасить mako/dunst/swaync →
  `cargo run -p chronos-services --example notification-smoke` →
  `notify-send test body`, `busctl --user status org.freedesktop.Notifications`.
- Тесты сервисов: `cargo test -p chronos-services` (15 зелёных).
- Source: `cargo test --package "path+file:///home/neo/projects/chronos-ecosystem/Source/gpui#0.2.2" --lib`
  (спека `-p gpui` неоднозначна: path-форк vs zed-git).

## Уроки процесса

- Субагентов НЕ спавнить — только файлы-задания миньонам.
- Отчёты сверять с деревом построчно; «✅ выполнено» ≠ выполнено.
- Параллельные миньоны — только непересекающиеся зоны файлов; в задания
  вписывать чужие зоны явно («эти файлы НЕ трогай / НЕ откатывай»).
- Жалоба миньона на «сломанный workspace» — сначала проверь, не WIP ли
  это второго миньона (случай Hermes vs OMP).
- Побочные артефакты субагентов миньонов (аудиты *.md в корне) — не в
  репо, требовать в заданиях.
