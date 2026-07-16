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

## Состояние git

- ChronOS master: `3b1a473` ← `0316de6` ← `03b0c87` (baseline).
- Source master: `9c9b6f5` (blur) ← `8881d4d` (NOTICE-фикс) ← `ef6b4bd` (easing+spring) ←
  `3ce3466` (skeleton). Дерево чистое.
- git identity настроен локально (neo / mishabcbb@gmail.com).

## Следующие шаги (по порядку)

1. ~~Отчёт OMP → приёмка~~ СДЕЛАНО (9c9b6f5, визуал подтверждён). Следующий шаг —
   пункт 2.
2. **Launcher Critical focus fix** — вариант (c): миграция XDG toplevel
   вместо layer-shell (рекомендация omp-report + моя; оверлейность через
   windowrule). ЖДЁТ ПОДТВЕРЖДЕНИЯ пользователя, потом задание миньону.
3. **Theme-вопрос → попапы UI нотификаций** (Hermes, задание №4):
   ui-крейта нет — сначала мини-спека theme-API (паттерн color-math из
   gpui-shell, rewrite-only).
4. applications + wallpaper сервисы (оба S, без рисков).
5. Bar-виджеты (clock/workspaces/battery/network через watch()) +
   gradient borders (S-M, после блюра).
6. Дальше: OSD (нужны audio+brightness), dock, tray (XL — в конце).
7. Отложено (DECISIONS.log): FLIP/implicit transitions (нет transform-полей
   в Style — XL-блокер), 8-stop градиенты, effect layers, color filter.

## Миньоны

- **Hermes** — 3 чистых задания (recon, 2 аудита port-cost, демон).
  Умеет делегировать субагентам. Свободен, следующий — попапы UI.
- **Cline** — сделал хирургию форка (62/62) + easing/spring. Минус:
  4 эрраты за историю (не читает что коммитит; откатывал чужой WIP —
  запрещено в приёмке CLINE.md). Свободен.
- **OMP** — исследование фокуса + backdrop blur (оба приняты, blur с первого захода под обстрелом Cline). Свободен.

## Ключевые технические факты (кровью заработанные)

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
