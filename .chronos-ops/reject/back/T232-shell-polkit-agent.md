# T232 — ChronOS как свой polkit-агент (шелловский попап вместо системного)

> **ОТКЛОНЁН, 2026-08-04.** Пользователь: «я не конкурент hyprland, их
> мелкие модули под это и существуют — использовать, не изобретать
> заново» (память `use-hyprland-ecosystem-modules`). Вместо своего
> D-Bus/PAM агента на GPUI — собрали апстримный `hyprwm/hyprpolkitagent`
> из git HEAD (переписан на `hyprtoolkit`, живая тема через
> `~/.config/hypr/hyprtoolkit.conf`) поверх системного Qt-пакета
> (`prefix=/usr`, точь-в-точь пути pacman-версии — откат
> `sudo pacman -S hyprtoolkit hyprpolkitagent`). Заодно собрали
> `hyprtoolkit` из git HEAD (packaged 0.5.4 отставал API от HEAD
> hyprpolkitagent). `~/.config/hypr/hyprtoolkit.conf` заполнен токенами
> ChronOS (`crates/ui/src/theme/schemes.rs` DEFAULT_BASE16): accent
> `#007acc`, bg `#1e1e2e`/`#313244`/`#25253b`, JetBrains Mono, rounding
> 12/6. Живьём подтверждено (`pkexec true`, grim): попап рисуется на
> ChronOS-палитре, сервис стабилен (не крашится). Ловушка на будущее:
> старый агент, запущенный ДО замены бинаря, держит polkit-регистрацию
> и не освобождает её сам даже после `systemctl restart` на новом
> юните — нужно `pkill -9` живого старого процесса (найти через
> `readlink /proc/<pid>/exe` → `(deleted)`) перед стартом нового,
> иначе новый инстанс падает SIGSEGV на "agent already exists".
> Ноль изменений в ChronOS-репо — вся работа вне git-репозитория
> (система + `~/.config/hypr/`).

**Роль:** BACKEND (D-Bus/PAM протокол) + FRONTEND (попап). Может быть один
минион, но зона файлов чёткая — см. ниже.
**Источник:** прямой запрос пользователя, живой скриншот 2026-08-04 —
клик на updates-виджете (`bar/widgets/updates.rs` → `pkexec yay -Syu
--noconfirm`) поднимает GTK-диалог "Authenticating for unix-user:neo",
явно чужеродный визуально ("это должен быть шелловский попап, не
системный").
**Канон:** `docs/DECISIONS.log` 2026-07-21 запись «Chronos-AUR: отдельное
приложение... shell-polkit-агентом системно (ловит любой pkexec без
per-app IPC)» — эта задача **реализует** тот отложенный трек.
**Приоритет:** P1 — не косметика, реальная фича (auth UX), но не блокер.

## Текущее состояние (проверено живьём)

- Кто спрашивает пароль сейчас: **`hyprpolkitagent`**, внешний бинарь,
  автостартуется из `~/.config/hypr/hyprland.lua` (`startPolkitAgent()`
  в `hl.on("hyprland.start", ...)` и `config.reloaded`) — этот файл ВНЕ
  git-репо ChronOS (личный конфиг машины), править можно свободно, но
  **координировать с архитектором** перед выключением автостарта (см.
  §"Порядок выката" ниже — без агента `pkexec` сломан целиком).
- Кто вызывает `pkexec`: `crates/services/src/aur/mod.rs` —
  `AurCommand::UpgradeAll`/`UpgradeSelected` (строки ~382-429,
  `upgrade_command_args`/`upgrade_selected_command_args`), плюс
  `crates/app/src/updates_popup/mod.rs` (кнопка "Upgrade all" в
  футере попапа). Это **единственный** живой pkexec-путь в шелле сейчас
  — но задача должна ловить **любой** pkexec в сессии (agent
  регистрируется системно, не per-command), не только этот.
- `zbus = "5"` уже воркспейс-зависимость (workspace `Cargo.toml:18`,
  `crates/services/Cargo.toml:11`) — используется для tray
  (`StatusNotifierItem`). Не тащи новую D-Bus библиотеку, реюзай.
- `/usr/lib/polkit-1/polkit-agent-helper-1` есть на машине (setuid
  PAM-хелпер, тот же, которым пользуются polkit-gnome/lxqt-policykit/
  hyprpolkitagent под капотом — избавляет от прямой линковки PAM в Rust).

## Что нужно сделать

### 1. Регистрация как polkit authentication agent (D-Bus, system bus)

- Подключиться к **system bus** (не session — polkitd там).
- Вызвать `RegisterAuthenticationAgent(subject, locale, object_path)` на
  `org.freedesktop.PolicyKit1.Authority` (`/org/freedesktop/PolicyKit1/
  Authority`, интерфейс `org.freedesktop.PolicyKit1.Authority`).
  `subject` — structure `(sa{sv})` вида `("unix-session", {"session-id":
  Str(XDG_SESSION_ID)})`. `XDG_SESSION_ID` берётся из env (или
  `org.freedesktop.login1` `GetSessionByPID` если env пуст — свериться,
  что надёжнее в реальном хопрленд-сеансе).
- Экспортировать **свой** D-Bus объект на `object_path`, реализующий
  интерфейс `org.freedesktop.PolicyKit1.AuthenticationAgent` с методами:
  - `BeginAuthentication(action_id, message, icon_name, details,
    cookie, identities)` — вызывается polkitd, когда кому-то нужна
    авторизация. Метод **блокирующий/долгий** — возвращается (пустым),
    когда диалог закрыт (успех/отмена/провал); polkitd в это время
    держит вызывающего (`pkexec`) в ожидании.
  - `CancelAuthentication(cookie)` — polkitd просит отменить (напр.
    вызывающий процесс убит) — закрыть попап без действия.
- **Протокол `BeginAuthentication`/PAM-хелпера уточнён приблизительно**
  по документации polkit и открытым агентам (polkit-gnome,
  lxqt-policykit) — НЕ считать точным без сверки с реальным поведением:
  1. Взять первую identity из списка `identities` (обычно
     `unix-user:<uid>`), достать `uid`/имя пользователя.
  2. Спавнить `/usr/lib/polkit-1/polkit-agent-helper-1 <username>` с
     захваченными stdin/stdout (НЕ pty — хелпер общается построчным
     текстовым протоколом).
  3. Записать `cookie` первой строкой в stdin хелпера.
  4. Читать строки из stdout хелпера — префиксы вида
     `PAM_PROMPT_ECHO_OFF <текст>` (запрос пароля, скрытый ввод),
     `PAM_PROMPT_ECHO_ON <текст>` (видимый ввод), `PAM_ERROR_MSG`,
     `PAM_TEXT_INFO`, `PAM_SUCCESS`, `PAM_FAILURE`, `PAM_MAXTRIES` —
     точные имена/формат сверить с исходником `polkit-agent-helper-1`
     (часть pkg `polkit`, может быть в `/usr/share/doc/polkit` или
     апстрим-репо `https://gitlab.freedesktop.org/polkit/polkit`) или
     логами `hyprpolkitagent`, если у него есть debug-режим.
  5. На `PAM_PROMPT_ECHO_OFF` — показать шелловский попап (см. §2),
     дождаться ввода пароля, записать пароль + `\n` в stdin хелпера.
  6. На `PAM_SUCCESS`/`PAM_FAILURE` — закрыть попап, дать
     `BeginAuthentication` вернуться (разблокирует polkitd/`pkexec`).

### 2. Шелловский попап (UI)

Зона: новый модуль `crates/app/src/polkit_popup/` (по образцу
`crates/app/src/system_popup/` или `updates_popup/` — Layer::Overlay,
`KeyboardInteractivity`, паттерн `close_this` reentrancy guard,
`ARCHITECTURE.md §4.1`).

- Заголовок: `message` из `BeginAuthentication` (напр. "Authentication
  is needed to run `/usr/bin/yay -Syu --noconfirm` as the super user").
- Поле пароля — **скрытый ввод**, `text_input.rs` (`side_panel_left/
  text_input.rs`) сейчас **не поддерживает маскирование** (проверено,
  0 совпадений `mask`) — либо добавь маскирование туда (реюз), либо
  сделай минимальное локальное текстовое поле в `polkit_popup/` без
  общего компонента (проще, меньше риска сломать существующие места
  использования `text_input.rs`). Точки/звёздочки вместо символов при
  рендере, сам буфер пароля — plain String в памяти (это то же самое,
  что делает `polkit-agent-helper-1` — приемлемо, не логировать это
  значение НИКУДА, включая `tracing::info!`/`debug!`).
- Кнопки Cancel/Authenticate — стиль как в `updates_popup`/
  `volume_popup` (акцент `theme.accent.primary`, не hardcoded hex).
- На Cancel — записать `CancelAuthentication`-эквивалент в протокол
  хелпера (или просто убить процесс хелпера) и закрыть попап.
- Ошибка (`PAM_FAILURE`/неверный пароль) — показать inline (как error
  в `bar_settings.rs:289`, `theme.status.error`), НЕ закрывать попап
  сразу, дать повторить (полкит обычно даёт несколько попыток —
  `PAM_MAXTRIES` уточнит лимит).

### 3. Инициализация

- `crates/app/src/main.rs` — вызвать `polkit_agent::init(cx)` (по
  образцу других `*::init(cx)` в этом файле) один раз при старте шелла.
- Агент должен пережить перезапуск шелла корректно:
  `UnregisterAuthenticationAgent` при чистом shutdown (если такой есть
  в API — свериться), иначе polkitd может держать мёртвую регистрацию
  до следующего сеанса. Если жёсткого unregister нет в API — хотя бы не
  падать при повторной регистрации на живом сокете (`chronos-stop` +
  `chronos-start` — частый паттерн в этом репо, см. HANDOFF).

## Порядок выката (важно — не оставить пользователя без auth agent)

1. Собрать и живьём проверить новый агент **рядом** с работающим
   `hyprpolkitagent` — если оба зарегистрированы, polkitd обычно
   использует последнего зарегистрировавшегося (uncertain — свериться
   на практике) — тестировать через `pkexec true` или похожий безопасный
   вызов, не сразу `yay -Syu`.
2. Только после того, как несколько реальных `pkexec`-вызовов (включая
   AUR-апгрейд) прошли чисто через шелловский попап — закомментировать/
   убрать `startPolkitAgent()` в `~/.config/hypr/hyprland.lua` (машинный
   файл, вне git, координировать с архитектором перед правкой — см.
   правило в `CLAUDE.md` про Write-перезапись нетрекаемых файлов:
   **сначала прочитать целиком**, `hyprpolkitagent` — не единственная
   строка там).
3. Если шелловский агент падает/зависает — `hyprpolkitagent` должен
   остаться рабочим фоллбэком, пока новый код не прожил хотя бы один
   полный "живой" AUR-апгрейд без сюрпризов.

## Канон, который нельзя нарушать

- НЕ логировать пароль ни в каком виде (`tracing`, `println!`, файлы).
- Только токены темы для UI, паттерн `close_this`/reentrancy guard из
  `ARCHITECTURE.md §4.1` (тот же класс бага, что ловили в
  `wayland-window-lifecycle` skill — не звать `handle.update` рекурсивно
  из колбэка того же окна).
- `unsafe_code = deny` (workspace lint) — весь D-Bus/subprocess код
  безопасен без unsafe, zbus и `std::process::Command` этого не требуют.

## Верификация

```bash
cargo build --release -p chronos
cargo test --release -p chronos-services --lib -- polkit
cargo test --release -p chronos --lib -- polkit_popup
```

**Live — обязателен, это privileged-путь:**

1. `pkexec true` (безопасный тест-вызов, ничего не делает) с шелловским
   агентом — попап появляется, пустой пароль/неверный пароль → ошибка
   inline, верный пароль → попап закрывается, `pkexec true` вернул 0.
2. Реальный клик "Upgrade all" в updates-попапе — весь путь целиком
   (пароль → `pkexec yay -Syu --noconfirm` реально выполняется,
   `pacman -Qu` после — пусто/меньше пакетов).
3. Cancel — `pkexec` возвращает ненулевой код, ничего не выполняется.
4. Второй запрос подряд (два клика Upgrade all с отменой первого) —
   агент не залипает, не дублирует попапы, не крашит шелл.
5. Обе темы — попап читаем в dark и light.

## Отчёт

`docs/orchestration/tasks/report/T232-shell-polkit-agent-report.md` —
приложи живой grim попапа + лог успешного `pkexec` через новый агент.
Коммит: `services+ui : ChronOS as its own polkit authentication agent (T232)`.
