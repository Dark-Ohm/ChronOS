<!-- T039 — migrated 2026-07-22 from orchestration/report-log/zed-report-1.md — see orchestration/tasks/MIGRATION.md -->

# Session: Zed №1 — AUR/pacman update-виджет (порт логики Alloy, MVP) — 2026-07-18

## Сделано (факт, не намерение)

- `crates/services/src/aur/mod.rs`, `crates/services/src/aur/types.rs`
  (новые) — `AurSubscriber` (async template, как `AudioSubscriber`):
  - Детект официальных обновлений — `checkupdates` (pacman-contrib), если
    установлен (проверка через `which`, не хардкод пути); фолбэк —
    `pacman -Qu`, если `checkupdates` нет.
  - Детект AUR-обновлений — `yay -Qua`, только если `yay` есть на PATH
    (best-effort: ошибка `yay` не роняет весь цикл чтения).
  - Опрос раз в 15 минут (сеть/db-sync тяжелее, чем 250мс у audio),
    экспоненциальный backoff на реальный отказ (не путать с «нет
    обновлений» — это нормальный успешный результат с пустым списком).
  - `AurCommand::Refresh` (форс-перечитывание, дёргается при открытии
    попапа — паттерн `TrayCommand::FetchMenu`) и `AurCommand::UpgradeAll`
    (единственная привилегированная операция — `pkexec yay -Syu
    --noconfirm` / `pkexec pacman -Syu --noconfirm` без yay; тот же
    подход, что уже в Alloy `upgrade_stream_script`/`upgrade_script`,
    без обёртки в fish+zenity, которая тут не нужна).
  - Чистые функции `parse_update_line`/`parse_updates`/
    `upgrade_command_args`/`binary_available` — юнит-тестируемы без
    живой системы.
- `crates/services/src/lib.rs` — регистрация (`pub mod aur;`, ре-экспорт,
  поле `Services.aur`, строка в `init_all()`) — только свои строки.
- `crates/app/src/state.rs` — `AppState::aur(cx)` — один аксессор,
  по образцу остальных.
- `crates/app/src/bar/widgets/updates.rs` (новый) — иконка `⬆` +
  счётчик (жёлтый/warning при >0, приглушённый при 0), клик →
  `updates_popup::toggle`. Регистрация — 2 строки в
  `bar/widgets/mod.rs` (в конце списка, как велит комментарий).
- `crates/app/src/updates_popup/{mod.rs,view.rs}` (новый) — layer-shell
  попап (TOP|RIGHT, `Layer::Overlay`, `KeyboardInteractivity::None`),
  список пакетов (имя [+ `(AUR)`], `старая → новая`), кнопка
  «Upgrade all», кнопка «✕». **Сознательно НЕ закрывается по потере
  клавиатурного фокуса** (никакого `observe_window_activation`) —
  единственные пути закрытия: повторный клик по виджету бара, кнопка
  «✕», клик по «Upgrade all». Это прямое следствие урока
  `follow_mouse=1` (MEMORY.md 2026-07-18, отказ Cline №9) — решил не
  наступать на те же грабли вообще, а не дебаунсить.
  Реентерабельность `remove_window()` — `close_this()` скопирован 1:1
  по паттерну `tray_menu::close_this`/`launcher::close_this` (прямой
  вызов на живой `&mut Window`, без повторного `handle.update` на тот
  же id).
- `crates/app/src/main.rs` — `mod updates_popup;` + `updates_popup::init(cx);`
  (после `tray_menu::init(cx)`, 2 строки).

## Расхождения со спекой/планом

- Имя модуля сервиса — `aur` (бриф явно разрешил `aur` ИЛИ `pkg_updates`
  и явно указал путь `services/src/aur/**` в разделе «Зоны» — взял его,
  конфликтов в дереве не было).
- Бриф просил «залогируй команду / выведи в тестовом моке» для кнопки
  апгрейда, не уточняя, должен ли реальный клик реально запускать
  `pkexec`. Решил: да, реальный клик должен реально запускать (иначе
  фича не фича) — только Я САМ не жал эту кнопку живьём (см. «Проверено
  фактом» ниже). Юнит-тест покрывает исключительно построение argv
  (`upgrade_command_args`), которое и есть «правильная команда» из
  брифа.
- Попап не скроллится и не имеет 15с auto-close таймера как у
  `tray_menu` — осознанное упрощение MVP (список обновлений читают
  дольше 15с; высота попапа кэпается на 520px, как у tray_menu). Не
  acceptance-критерий брифа, но стоит знать при ревью.

## Не реализовано из acceptance criteria

Всё из MVP-скоупа брифа реализовано (детект + бейдж + список + кнопка
апгрейда). Явно НЕ в этом заходе (по брифу и не тронуто):
`deb.rs`/`rpm.rs`/`appimage.rs`/`pkg_tar.rs`, `malware_check.rs`,
`pkg_analyze.rs`, `pkg_build.rs`, произвольный поиск/установка пакетов,
drag&drop конвертация форматов — ничего из этого не начато, как и
просили.

## Проверено фактом, не на словах

- `cargo build -p chronos-services` / `cargo build -p chronos` —
  зелёные (только предсуществующие warning'и в чужих файлах,
  0 warning'ов в новом коде).
- `cargo test -p chronos-services aur::` — 12/12 зелёных, включая
  `parse_updates_matches_live_pacman_qu_fixture` — фикстура снята с
  РЕАЛЬНОГО `pacman -Qu` этой машины (не выдумана), покрывает
  epoch-версии (`discord 1:1.0.148-1 -> 1:1.0.149-1`).
- `cargo test --workspace --lib --bins` — финальный прогон: 193/193
  зелёных (4+69+25+92+3 по крейтам). Один прогон ранее показал
  `ipc::service::tests::second_acquire_on_same_path_becomes_secondary`
  падающим 2 раза подряд (в т.ч. с `--test-threads=1`), затем на
  финальном прогоне — зелёный. Файл `crates/app/src/ipc/service.rs` я
  НЕ трогал (`git diff` подтверждает — там только чужие
  косметические изменения форматирования, уже в дереве до меня, не
  мои). Похоже на нестабильный тест, не связанный с моей задачей —
  фиксирую здесь как находку, не чиню (не моя зона).
- **Живой смок бейджа (release-сборка, реальный Wayland/Hyprland,
  RUST_LOG=info):** `cargo build --release -p chronos` →
  `RUST_LOG=info ./target/release/chronos` → лог
  `AurSubscriber connected (checkupdates/pacman -Qu MVP backend)` →
  grim-скрин бара DP-1 показал `⬆ 17` жёлтым. Параллельно вручную
  запустил `checkupdates` (unsandboxed, тот же уровень привилегий, что
  у живого процесса шелла) — **17 строк, ПОЛНОЕ совпадение** по числу
  с бейджем (имена пакетов совпадают построчно). `pkill -x chronos`
  после смока, `pgrep -x chronos` — пусто.
- **Попытка живого клика по попапу — НЕ удалась технически, не
  код.** `hyprctl dispatch movecursor`/`hl.dsp.movecursor` — нет такого
  диспатчера в этой версии Lua-Hyprland. `ydotoold` не запущен
  (`ydotool` ругается `No such file or directory` на
  `/run/user/1000/.ydotool_socket`, а стартовать его нужно `sudo`
  руками — не стал делать это сам без пользователя за компом, по
  аналогии с ограничением на кнопку апгрейда). Итог: сама открывалка/
  закрывалка попапа проверена только сборкой+компилятором+паттерном
  (1:1 скопирован уже живьём проверенный `tray_menu::close_this`), НЕ
  живым кликом мыши — та же ситуация, что была у Grok №4/№5 (клик
  принят «по аналогии», ydotool-нестабильность — не код).
- **Кнопка «Upgrade all» — НЕ нажималась мной вживую** (по прямому
  требованию брифа). Проверено только: `upgrade_command_args(true)` →
  `("pkexec", ["yay","-Syu","--noconfirm"])`,
  `upgrade_command_args(false)` → `("pkexec", ["pacman","-Syu","--noconfirm"])`
  — юнит-тестами.

## Новые риски / известные баги

- **`checkupdates` не работает в сэндбоксе агент-терминала (severity:
  low, не влияет на пользователя).** Причина — `fakeroot -- pacman -Sy`
  внутри `checkupdates` падает с `failed to chown temporary download
  directory ... Invalid argument` именно в сэндбоксированном режиме
  инструмента `terminal` (без `unsandboxed`); с `unsandboxed=true`
  (= реальный процесс пользователя, как и сам `chronos`) всё работает
  штатно — подтверждено смоком выше (17/17 совпало). Не баг ChronOS,
  фиксирую как артефакт МОЕЙ верификационной среды, чтобы не повторяли
  расследование.
- **Попап не скроллится, кэп высоты 520px** — при экстремально большом
  списке (сотни пакетов) низ списка окажется недоступен. MVP-уровень
  риска, тот же, что у `tray_menu`.
- **Живой клик по попапу и по кнопке «Upgrade all» не проверены
  синтетическим вводом** (см. выше) — тот же класс риска, что уже
  принимался у Grok №4/№5 без возражений Архитектора; здесь дополнительно
  сама кнопка апгрейда прямо запрещена мне брифом для живого запуска.
  Рекомендую живой клик мышью + осознанное решение нажать «Upgrade all»
  — Архитектору с пользователем, как и написано в брифе.
- **`pkexec` предполагается установленным и рабочим (polkit-агент в
  сессии)** — на этой машине есть (`/usr/bin/pkexec`), но не проверялось,
  поднят ли polkit auth agent визуально (для реального клика это
  вскроется сразу — если агента нет, `pkexec` откажет с понятной
  ошибкой в лог, ничего скрытого).

## Статус ARCHITECTURE.md / DECISIONS.log

Не трогал — MVP укладывается в существующие паттерны (`Service` trait,
`BarWidget`, layer-shell popup lifecycle), новых архитектурных решений
не потребовалось. Если Архитектор решит, что новая политика
(«привилегированная команда через `dispatch`, а не через отдельный
IPC/askpass-слой») достойна записи в `DECISIONS.log` — не стал сам,
оставляю на его решение.
