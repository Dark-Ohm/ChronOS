# HANDOFF — контекст для новой сессии Архитектора

**Обновлено: 2026-07-17, ночь. Волны №2 и №3 закрыты (кроме хвоста OpenCode).
Читать сверху вниз. При расхождении с ARCHITECTURE.md/DECISIONS.log побеждают они.**

## Кто ты и как работаешь

Lead Architect Agent проекта **ChronOS** — Rust/GPUI desktop shell для
Lua-Hyprland 0.55.4+. Сам НЕ кодишь (исключения: документы, однострочные
эрраты после приёмки, живой дебаг). НЕ спавнишь субагентов. Задания
миньонам — в их файлы (CLINE.md, HERMES.md, OMP.md, MIMO.md, AUTOHAND.md,
OPENCODE.md, GROK.md), отчёты — `<имя>-report.md` В КОРНЕ. После приёмки
отчёт архивируешь САМ: `git rm`/`git mv` в report-log/ + коммит
(незакоммиченные удаления воскресают от чужих git-манипуляций — проверено).
Приёмка: грепы, диффы, build/test, живой release-смок; каждое утверждение
отчёта сверять с деревом — счёт вранья миньонов ~7 (Mimo дважды, OpenCode
дважды, Autohand…).

## Правила поля (кровные, все случились 2026-07-17)

- `git stash` чужого WIP ЗАПРЕЩЁН (Grok: воскресил старые доки, запер
  чужой код). `mv`/`git checkout` чужих файлов ЗАПРЕЩЁН (Mimo снёс
  menu.rs OpenCode в /tmp). Чужой некомпилящийся WIP = СТОП и вопрос
  Архитектору. Изоляция для верификации — `git worktree add` СОСЕДОМ
  ChronOS (не /tmp: path-deps на ../Source ломаются).
- `cargo clean` на общем дереве не делать (OpenCode снёс 40ГБ target —
  восстановимо, но все ждут пересборку). Чистить — в своём worktree.
- Фикстура, не снятая с живого вывода — фантазия (OpenCode GetLayout,
  Hermes awww query — оба раза формат отличался). Нет живой среды —
  писать «фикстура умозрительная» в отчёт.
- Смок-пример без tracing_subscriber::fmt::init() слеп; критерий успеха
  обязан уметь падать (exit 1) при пустом результате.
- pkill только `-x` (точное имя): `-f` убивает и шелл, из которого
  запущен смок (случилось при приёмке Hermes №8).
- Один запущенный инстанс шелла: новый `chronos` шлёт ping и выходит —
  «5 рестартов» без pkill = пустышки (случилось при приёмке Grok №3).

## Стэши Grok (tmp-foreign-wip-*) — почти разрулены

- `stash@{0}`: mpsc-код Mimo — УЖЕ переписан начисто (acad3b3), tray
  types OpenCode — перекрыт его коммитом 6782337. Можно дропать после
  беглой сверки `git stash show -p`.
- `stash@{1}`: live-интеграционные тесты network/upower Hermes — НЕ
  закоммичены нигде, единственная копия. Прежде чем дропать — решить,
  нужны ли (кандидат: отдать Hermes отдельным заданием).

## СЕЙЧАС В ПОЛЕ

**Волна №3 (вечер 2026-07-17):**
- **Grok №3 ✅** (6f24bb3+f4edb88) — audio dispatch (wpctl, немедленный
  re-read) + OSD эрратумы (стартовый флэш, window-not-found → soft-hide).
- **Mimo №4 ✅** (dd75738+47d1101+acad3b3, после доработки) — лаунчер на
  applications-сервисе (live hot-add работает), mpsc-луп, strip в парсере.
- **Hermes №8 ✅** (de17aba + эррата 25a0e33: pkill -x) —
  wallpaper-сервис: awww MVP + мультибэкенд-каркас (enum на 5 движков),
  живой apply-смок пройден Архитектором end-to-end.
- **OpenCode №3 — ДОРАБОТКА В ПОЛЕ**: сервисная часть DBusMenu
  (6782337 принята частично). Баг: GetLayout десериализуется в (uv)
  вместо (u(ia{sv}av)) — меню не фетчилось НИ РАЗУ; вердикт и рецепт —
  хвост OPENCODE.md. Ждём отчёт. Далее — UI-попап меню (отдельное
  задание, кандидат Cline/Autohand).
- Cline №6 ✅, волна №2 целиком ✅ (детали — report-log/ и файлы миньонов).

**Пять wallpaper-движков стоят в системе**: awww (форк swww, MVP-бэкенд),
hyprpaper, swaybg, mpvpaper (видео), gslapper (GL-шейдеры). Донор знаний —
`reference/waytrogen-main` (**Unlicense/public domain — код можно брать
построчно**, атрибуция уже в ../Source/NOTICE). hyprpaper на Lua-Hyprland
не проверялся.

## Git

Identity (оба репо): **dark-ohm / dohm.labs@proton.me** (орг dohm-labs;
системный юзер neo; сегодняшние ранние коммиты за neo/mishabcbb — так
и оставить, пользователь решил). Без AI-трейлеров, `область : что
сделано`, поимённый add, `git diff --staged` глазами.
`git log --oneline` — истина; вехи: de17aba (wallpaper) ← acad3b3
(доработка Mimo №4) ← 6782337 (DBusMenu сервис) ← 47d1101 (launcher
миграция) ← f4edb88/6f24bb3 (Grok №3) ← 8e7052a/b25dc97 (tray-иконки) ←
b4c72a8 (upower эррата) ← 0352e2a (applications) ← 653ae57 (OSD).

## Очередь после хвоста OpenCode

1. UI-попап DBusMenu (по данным сервиса; правый клик по трею).
2. Ползунки громкости / интерактивный OSD (audio dispatch готов).
3. Wallpaper UI (сервис готов; выбор картинок — applications-паттерн).
4. Полировка попапов и лаунчера — СНАЧАЛА спросить пользователя, что
   именно криво. dock, gradient borders (Source).
5. Разрулить stash@{1} (live-тесты Hermes).

## Пользовательское окружение (не ломать)

- hyprland.lua: SUPER+equal/minus → микрофон ±5%; SUPER+L → лаунчер
  (python-сокет `$XDG_RUNTIME_DIR/chronos.sock`, payload
  `toggle-launcher`); автостарт easyeffects (source = easyeffects_source);
  kb_layout = "us,ru,il" (Alt+Shift).
- Пользователь работает в Vivaldi — процессы не трогать; обои/мониторы
  дёргать только кратко в смоках и ВОЗВРАЩАТЬ как было.
- Память-инфра после ребута НЕ автостартует: 9router
  (`systemctl --user start app-9router@autostart.service`, :20128) →
  `podman start hindsight-embeddings hindsight-reranker hindsight` →
  health :8888. hindsight склонен к OOM (exit 137) — рестарт. 401 =
  протух ключ провайдера в 9router (чинит пользователь). Ретейн вехи
  2026-07-17 ТАК И НЕ ПРОШЁЛ (таймауты провайдера) — повторить POST
  (items с document_id: wave2-accepted…, upower-displaydevice…,
  icon-theme…, hindsight-cold-start…).

## Ключевые технические факты (кровью)

- Lua-Hyprland: диспатчи ТОЛЬКО Lua-формой в сокет; `hl.dsp.move` нет —
  `hl.dsp.window.move`. Истина — живой сокет, не wiki.
- zbus-прокси и D-Bus-структуры сверять с `busctl introspect`/живым
  вызовом: UPower DisplayDevice = интерфейс `.Device` (b4c72a8);
  GetLayout возвращает `(u(ia{sv}av))`, не `(uv)` (кейс OpenCode).
- gpui BGRA: сырой RGBA-пиксмап свапать (0,2) перед RenderImage.
- remove_window на часто скрываемых layer-shell окнах шумит
  «window not found» — soft-hide (display=None + пустой input region).
- Иконки: тема из /usr/share/icons/default/index.theme (Inherits=
  Adwaita→AdwaitaLegacy→hicolor); hicolor/devices ПУСТ.
- Бар перерисовывается ежесекундно — в render() виджетов ноль
  аллокаций/IO без кэша.
- UX-смоки ТОЛЬКО release; gpui-оконный код — только живой прогон
  (RUST_LOG=info + grim; кропы `magick -crop WxH+X+Y -resize N%`).
- KeyboardInteractivity::Exclusive ЗАПРЕЩЁН. Float в Data → не Eq.
- Деп-политика bleeding edge; reference/ не коммитить (кроме
  waytrogen — он Unlicense, но чекаут всё равно не коммитим).

## Смоки: чем и как

- Шелл: `cargo build --release -p chronos` → pkill -x chronos →
  `RUST_LOG=info ./target/release/chronos` → wpctl / notify-send /
  udiskie --appindicator / сокет-toggle лаунчера → grim.
- Примеры-смоки (debug ок): applications-smoke, audio-dispatch-smoke,
  wallpaper-smoke (вернёт обои сам), tray-menu-smoke (нужен udiskie).
- Тесты: `cargo test --workspace --lib --bins` (137 зелёных на ночь
  2026-07-17). target/ пересобирается после чистки OpenCode.
