ДА (врёт) — Upgrade all стоит под Repos + AUR, но обновляет только Repos.

# T327 — отчёт QA по правой панели и режимам

Дата живой проверки: 2026-08-21. Проверялся свежесобранный ./target/release/chronos на живой Wayland-сессии Hyprland, DP-1 2560×1440. Код продукта не менялся.

cargo build --release -p chronos завершился успешно за 0,36 с (Finished release profile; предупреждения без ошибок). cargo test -p chronos --lib side_panel_right завершился: **197 passed, 0 failed, 413 filtered out**. Live smoke выполнен тем же release artifact; финальный процесс PID **496884** остаётся жив.

## Вердикт

Каркас mode sets работает: Developer и Gamer показывают по 11 пунктов, переход между ними не ломает геометрию, Library реально заполнена, Captures честно сообщает об отсутствии backend, а недоступные id визуально сбрасываются в System. До честного продуктового состояния не хватает точного обещания Updates, исправной narrow-layout в ACP и resolve-before-create для IPC id вне mode set.

## Блокеры — 3

1. **P1 UX truthfulness: Upgrade all обещает обновить видимые AUR-пакеты, но backend их принципиально исключает.** В одном списке показано 23 строки: 20 Repos и 3 AUR; общая кнопка расположена под обеими секциями. Код отправляет только pkexec pacman -Syu --noconfirm (services/src/aur/mod.rs:381-389), AUR объявлен display-only (:413-415). Кадр: frames/right-updates.png. Черновик: reports-fresh/DRAFT-updates-upgrade-all-excludes-visible-aur.md.
2. **P2 UX: основное действие ACP сломано на штатной ширине 320 px.** Open agents.toml переносится на три строки, последняя — одиночная l; рядом отдельные path, View и Reload. Причина — одна узкая flex-row: open-card flex_1/min_w(0), а Reload flex_none (tab/acp_settings.rs:273-346). Кадр: frames/right-acp_settings.png. Черновик: reports-fresh/DRAFT-acp-settings-open-button-wraps-at-default-width.md.
3. **P2 functional/resource lifecycle: id вне mode set создаётся до видимого fallback.** В Developer select-tab:terminal запустил /bin/zsh, затем UI показал System. После sweep child PID 501243 оставался Z<s [zsh] <defunct>. select-tab:build аналогично начал loading tasks до fallback. Код сначала вызывает ensure_tab_view (view.rs:324-350), а разрешение mode set происходит только в следующем render (:431-446). Кадр фактического результата: frames/right-terminal.png; процесс/лог: log/out-of-mode-terminal-process.txt. Черновик: reports-fresh/DRAFT-out-of-mode-select-tab-spawns-hidden-terminal.md.

Известную T309-находку про слабые empty states не переоткрывал: No notifications остаётся одинокой строкой в пустой панели, но это не новый blocker T327.

## Таблица 22 id

| id | Developer sweep: что реально видно | Mode/статус |
|---|---|---|
| system | System: media, power/gaming, CPU/RAM/GPU, network, disks | видно в Developer + Gamer; real |
| updates | Updates (23): Repos + AUR + общая Upgrade all | видно в обоих; real, blocker №1 |
| notifications | No notifications | видно в обоих; honest empty, известный T309 polish |
| files | Files для ChronOS root | видно только Developer; real |
| editor | System | сброс в System; отдельный id не входит в Developer mode set |
| terminal | System | сброс в System; hidden backend side effect, blocker №3 |
| preview | Editor: No file selected, подсказка и Open Files | видно только Developer под label Editor; honest empty |
| inspector | System | сброс в System |
| build | System | сброс в System; скрыто начинает loading tasks |
| source_control | System | сброс в System |
| library | System | в Developer сброс; в Gamer реальный список 5 games (gamer-library.png) |
| scenes | System | сброс в System; нет ни в одном rail |
| captures | System | в Developer сброс; в Gamer честная заглушка Unavailable · no capture backend |
| acp_settings | ACP agents, 1 Hermes, Actions | видно в обоих; real, blocker №2 |
| mcp_settings | System | сброс в System |
| lsp_settings | System | сброс в System |
| api_providers | System | сброс в System |
| editor_settings | System settings: bar/theme/Hypr modules/About | видно в обоих; real, label намеренно System settings |
| hyprland_binds | Hyprland binds · 58 | видно в обоих; real |
| display | Brightness + Wallpapers | видно в обоих; real |
| launcher_settings | Launcher config controls | видно в обоих; real |
| media | MPRIS art/title/progress/controls | видно в обоих; real |

Developer sweep дал 11 реальных mode-tab и 11 визуальных fallback в System. Двенадцатый active tab not in mode set → System в логе появился при возврате Gamer → Developer, потому что активным был Gamer-only Captures.

## Mode sets и рельсы

- **Developer:** System, Updates, Notifications, Media, Files, Preview под label Editor, Hyprland binds, ACP agents, Display, Launcher, System settings — 11.
- **Gamer:** System, Updates, Notifications, Media, Library, Captures, ACP agents, Hyprland binds, Display, Launcher, System settings — 11.
- toggle-workspace-mode дал два подтверждённых перехода: Developer → Gamer → Developer.
- Gamer Library показала 5 записей; Captures показала требуемое честное Unavailable · no capture backend.
- После возврата /home/neo/.config/chronos/workspace.toml содержит mode = developer.

Кадры уникального Gamer-среза: frames/gamer-rail-media.png, frames/gamer-library.png, frames/gamer-captures.png. Финальный подтверждающий Developer-кадр: frames/final-developer-system.png.

## Геометрия, IPC и финальное состояние

- Открытая панель: fixed content canvas x=1600, w=920, rail x=2520, w=40, оба y=20, h=1404 (log/layers-open.json).
- Gamer сохранил ту же layer geometry (log/layers-gamer.json); менялись rail/content, не surface footprint.
- В sweep отправлено 25 select-tab событий: 22 Developer id, Gamer Library/Captures и финальный System.
- Финальный toggle-side-panel-right удалил namespaces side_panel_right_content и side_panel_right_rail; frame_wrap_excl_right вернулся на x=2544 (log/layers-final.json, frames/final-right-closed.png).
- PID 496884 жив после smoke.

## Panic / protocol

- Rust panic: **0**.
- Protocol/JSONRPC error: **0**.
- ERROR: **0**.
- WARN: **3**, все — известный T309 dock config (firefox, code, vivaldi: no matching AppEntry), вне зоны T327.
- Полный валидный launch/sweep log: log/chronos.log.

## Конфиги

До запуска скопированы 11 файлов ~/.config/chronos/*.toml в config-backup/. SHA-256 текущих файлов до/после и backup совпадают **11/11**; полный список — log/config-before.sha256 и log/config-after.sha256. В частности:

- panels.toml: bba9070546180194f418cef712483d6cbb18767c9ad6f9edb612ece60fd6d433
- workspace.toml: d05510d5d6393c39a1ca6047b1e8428ad42b9d82cc22adbd62ae0188599351ae

## ls frames

~~~text
final-developer-system.png
final-right-closed.png
gamer-captures.png
gamer-library.png
gamer-rail-media.png
right-acp_settings.png
right-api_providers.png
right-build.png
right-captures.png
right-display.png
right-editor.png
right-editor_settings.png
right-files.png
right-hyprland_binds.png
right-inspector.png
right-launcher_settings.png
right-library.png
right-lsp_settings.png
right-mcp_settings.png
right-media.png
right-notifications.png
right-preview.png
right-scenes.png
right-source_control.png
right-system.png
right-terminal.png
right-updates.png
~~~

Всего: **27** полных кадров 2560×1440, **25** crops, **7** log/evidence files и **11** config backups — 70 файлов в .chronos-ops/dump/qa-ux/T327/. Все PNG непустые; размеры полных кадров 1,77–2,33 MB. SHA-256 всех 27 кадров проверен.

## Что не делал

- Не менял Rust-код, конфиги, обои, раму, схему, бар или левую панель.
- Не запускал привилегированный Upgrade all, mount/unmount/eject и power actions.
- Не переоткрывал как новые баги известные T309 empty-state/dock находки.
- Не делал commit и не редактировал .chronos-ops/checkpoint/, .rules или CLAUDE.md.
