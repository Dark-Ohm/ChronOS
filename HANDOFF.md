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

## ГДЕ МЫ: все три задания приняты, очередь свободна

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

## СЕЙЧАС В ПОЛЕ: 5 агентов, bar-виджеты (задания от 2026-07-17)

- **Cline** (CLINE.md №5) — refresh-мост Bar + Clock. Владеет bar/mod.rs
  и bar/widgets/mod.rs — фундамент остальных.
- **Hermes** (HERMES.md №5) — Workspaces (widgets/workspaces.rs, может
  тронуть compositor dispatch отдельным коммитом).
- **Mimo** (MIMO.md №1, новичок) — Battery (widgets/battery.rs; десктоп —
  без паники при отсутствии батареи).
- **Autohand** (AUTOHAND.md №1, новичок) — Network (widgets/network.rs).
- **OpenCode** (OPENCODE.md №1, XL) — tray: StatusNotifierWatcher-демон
  (crates/services/src/tray/, zbus 5.17) + widgets/tray.rs.
Правила у всех: свой файл + ровно 2 строки в widgets/mod.rs; поимённый
git add; git checkout чужого запрещён.

## Сделано ранее (все приняты)

- Демон нотификаций + попапы UI (Hermes №3/№4), theme-крейт crates/ui.
- Kael-порты в Source/: easing+spring (Cline), backdrop blur (OMP,
  визуал подтверждён скриншотом).
- Launcher: XDG toplevel + Critical focus trap снят (см. секцию ниже).

## Очередь после виджетов

1. Полировка попапов («выглядит криво» — уточнить у пользователя) и
   лаунчера (остаточная «баговынность», монитор DP-1 vs HDMI).
2. applications + wallpaper сервисы (S).
3. Gradient borders (Source, после блюра), OSD (нужны audio+brightness), dock.
4. Отложено (DECISIONS.log): FLIP/transitions (нет transform в Style),
   8-stop градиенты, effect layers, color filter.

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
