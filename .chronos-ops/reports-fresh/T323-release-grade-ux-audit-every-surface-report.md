НЕТ — в текущем виде я бы не заплатил за ChronOS $100: основные поверхности уже выглядят цельно, но продукт всё ещё показывает покупателю незавершённые разделы, молча отклоняет часть собственных IPC-команд и ненадёжно управляет popup/wallpaper lifecycle.

# T323 — release-grade UX audit: every surface

## Итог для Design Authority

Проверка выполнена на живой Wayland-сессии из release-бинарника `91f9a9e4` (`cargo build --release -p chronos`, exit 0, 0.42 s; 81 warning). Процесс аудита: `3375920 ./target/release/chronos`. Собрано 67 полноэкранных кадров в `/tmp/t323/run-91f9a9e4/frames/`.

Общее впечатление заметно лучше, чем следует из списка блокеров: shell визуально узнаваем, launcher, notification toast, OSD, tray menu, Files, Updates, System и игровая Library выглядят как части одного продукта. Но paid-release gate не пройден. Главная причина не в косметике, а в том, что несколько доступных пользователю путей заканчиваются явным placeholder, тихим возвратом в другой раздел либо popup, который продолжает жить поверх несвязанных поверхностей.

Известные пункты T309/T313/TBD не переоткрывались как новые: отсутствие pinned dock apps, перегруженность bar, generic empty states, тяжёлый Light, stepped controls/card layering в System и ограничение `toggle-theme` уже находятся в каноне.

## Пять главных причин не покупать релиз

1. **Покупатель видит незавершённые production-разделы.** В левой панели `Plan` показывает `Coming in Slice B`, `Tools` — `Coming in Slice C`; в Gamer mode `Captures` сообщает `Unavailable — no capture backend`. Это не edge case и не пустое состояние данных, а прямое признание незавершённости функции в основной навигации.
   - Evidence: `/tmp/t323/run-91f9a9e4/frames/left-rail-y154.png`
   - Evidence: `/tmp/t323/run-91f9a9e4/frames/left-rail-y180.png`
   - Evidence: `/tmp/t323/run-91f9a9e4/frames/right-captures-gamer.png`

2. **Девять принятых IPC target IDs недостижимы и молча превращаются в System.** `editor`, `terminal`, `inspector`, `build`, `source_control`, `scenes`, `mcp_settings`, `lsp_settings`, `api_providers` принимаются `select-right-tab:<id>`, но затем текущий mode set сбрасывает вкладку на System без обратной связи пользователю или вызывающей стороне. Например, после `select-right-tab:terminal` виден System, а лог содержит `active tab not in mode set → System was="Terminal"`. Это делает публично принимаемую команду ложно успешной.
   - Evidence: `/tmp/t323/run-91f9a9e4/frames/right-terminal.png`
   - Evidence: `/tmp/t323/run-91f9a9e4/frames/right-inspector.png`
   - Evidence: `/tmp/t323/run-91f9a9e4/frames/right-build.png`
   - Evidence: `/tmp/t323/run-91f9a9e4/frames/right-source_control.png`

3. **Sound popup не соблюдает ожидаемый lifecycle.** После открытия Sound запуск Calendar не закрыл предыдущий popup: поверхности наложились. Затем Sound продолжал отображаться поверх Start Menu и Edit Mode; клик по свободной области `(1400, 100)` также не закрыл его. Для shell это высокий риск случайного ввода и визуально выглядит как сломанная композиция окон.
   - Evidence: `/tmp/t323/run-91f9a9e4/frames/bar-calendar-click.png`
   - Evidence: `/tmp/t323/run-91f9a9e4/frames/start-menu.png`
   - Evidence: `/tmp/t323/run-91f9a9e4/frames/edit-mode.png`
   - Evidence: `/tmp/t323/run-91f9a9e4/frames/popup-click-catcher-after.png`

4. **Wallpaper restore path валит wallpaper daemon.** После открытия gallery и обязательного `waytrogen --restore` команда завершилась `Connection refused`; `awww query` также потерял socket. Повтор с новым `awww-daemon --no-cache` снова был убит restore-path. Закрытие запущенного Waytrogen повторно оставило daemon недоступным. Исходный GIF пришлось возвращать вручную на оба output. Это release-blocker для функции, меняющей пользовательский desktop state.
   - Surface evidence: `/tmp/t323/run-91f9a9e4/frames/ipc-wallpaper-gallery.png`
   - Post-action evidence: два независимых `awww query` вернули `failed to connect to socket: Connection refused (os error 111)`.
   - Recovery caveat: исходный wallpaper восстановлен на `DP-1` и `HDMI-A-1` через управляемую живую сессию `awww-daemon --no-cache`; detached daemon на этом animated GIF также завершался через несколько секунд, поэтому долговременную стабильность восстановления подтвердить нельзя.

5. **`surface_alpha` применяется не к общей surface-системе.** При Mocha и `surface_alpha = 0.7` bar стал полупрозрачным, но frame ring и Start Menu остались практически непрозрачными. Pixel probes на белом фоне: blur off — `bar=srgb(73,70,74)`, `ring=srgb(24,19,26)`, `start=srgb(29,25,31)`; при alpha 1.0 bar и ring оба `srgb(24,20,26)`. Пользователь получает несогласованную композицию от одного глобально звучащего параметра.
   - Evidence: `/tmp/t323/run-91f9a9e4/frames/mocha-alpha07-blur-off-start.png`
   - Evidence: `/tmp/t323/run-91f9a9e4/frames/theme-mocha-white.png`

## Остальные findings, по серьёзности

### High

- **Dock context action отсутствует либо не даёт feedback.** Правый клик по видимой dock-зоне `(40, 20)` не открыл меню и не показал никакой реакции. Для элемента, который выглядит интерактивным и используется как основной launcher affordance, silent no-op воспринимается как поломка.
  - Evidence: `/tmp/t323/run-91f9a9e4/frames/dock-context-menu.png`

### Medium

- **Левая rail-навигация требует угадывания.** Project и Chat открываются и пригодны к работе, но соседние icon-only destinations не имеют постоянных labels, а результат клика иногда является placeholder. Это усиливает эффект незавершённости Plan/Tools и затрудняет первичное знакомство без hover-discovery.
  - Evidence: `/tmp/t323/run-91f9a9e4/frames/left-rail-only.png`
  - Evidence: `/tmp/t323/run-91f9a9e4/frames/left-rail-y50.png`

- **Blur control не даёт проверяемого результата на flat background.** При Mocha/alpha 0.7 pixel probes blur off/on различались несущественно: bar `73,70,74` в обоих случаях, ring `24,19,26` против `23,20,26`, Start `29,25,31` против `28,25,30`. На однородном белом фоне это ожидаемо не доказывает отсутствие blur; требуется отдельный patterned-background test, которого в этом проходе не было.
  - Evidence: `/tmp/t323/run-91f9a9e4/frames/mocha-alpha07-blur-off-start.png`
  - Evidence: `/tmp/t323/run-91f9a9e4/frames/mocha-alpha07-blur-on-start.png`

## Что уже выглядит готовым к продаже

- Launcher открывается быстро, центрирован, имеет ясную иерархию и не выглядит техническим прототипом: `/tmp/t323/run-91f9a9e4/frames/launcher.png`.
- Notification toast визуально собран и читаем: `/tmp/t323/run-91f9a9e4/frames/notification-toast.png`.
- Volume OSD даёт немедленную, аккуратную обратную связь: `/tmp/t323/run-91f9a9e4/frames/osd-volume.png`.
- Steam tray context menu корректно привязан к bar и читаем: `/tmp/t323/run-91f9a9e4/frames/tray-menu.png`.
- Project/Chat/Preview paths реально работают; `preview-target` показал README, compose/send добавил сообщение и честно обозначил connecting state: `/tmp/t323/run-91f9a9e4/frames/ipc-preview-target.png`, `/tmp/t323/run-91f9a9e4/frames/ipc-compose-send.png`.
- Files, Updates, Notifications, System, ACP settings, Hyprland binds, Display, Launcher settings и Media имеют различимые, последовательно оформленные поверхности. Полный sweep: `/tmp/t323/run-91f9a9e4/frames/right-*.png`.
- Gamer Library выглядит как настоящая продуктовая поверхность и отобразила пять игр: `/tmp/t323/run-91f9a9e4/frames/right-library-gamer.png`.
- Wrapped и Normal geometry на белом и исходном wallpaper не показали ранее виденного 1px seam: `/tmp/t323/run-91f9a9e4/frames/frame-wrapped-white.png`, `/tmp/t323/run-91f9a9e4/frames/frame-normal-white.png`.
- Default, Light, Solarized Dark и Mocha реально применяются и остаются читаемыми на белом и чёрном backgrounds: `/tmp/t323/run-91f9a9e4/frames/theme-*-white.png`, `/tmp/t323/run-91f9a9e4/frames/theme-*-black.png`.

## Coverage matrix

| Surface / path | Покрытие | Результат / ограничение |
|---|---|---|
| Top bar + wrapped frame | Да | Baseline и финальный restored frame сняты |
| Dock | Частично | Right-click проверен; left-click не выполнялся, чтобы не запускать пользовательское приложение |
| Workspaces | Визуально | Переключение рабочих пространств не выполнялось, чтобы не менять текущую рабочую раскладку |
| Cava / MPRIS | Визуально | Отрисованы; playback не менялся, чтобы не вмешиваться в пользовательское media state |
| Volume | Да | Popup + hardware-key OSD, уровень возвращён volume-up |
| Tray | Да | Steam DBus menu через right-click |
| Keyboard layout | Визуально | Не переключался, чтобы не менять раскладку ввода владельца |
| Notifications | Да | Реальный `notify-send` toast; history click не дал убедимого подтверждения отдельной history surface |
| Battery | N/A | На тестовой desktop-машине battery surface отсутствует |
| Updates | Да | Right panel surface |
| Clock / calendar | Да | Calendar открылся; выявлено overlap с Sound |
| Left project / chat | Да | Rail clicks, expand, compose/send, preview-target |
| Left Plan / Tools | Да | Оба explicit placeholders зафиксированы |
| Right Developer mode | Да | Полный sweep всех 22 принимаемых IDs; 9 reset-to-System |
| Right Gamer mode | Да | Rail, Library, Captures; затем Developer mode восстановлен |
| Start menu | Да | Open/close, overlap с Sound зафиксирован |
| Launcher | Да | Open/close |
| Edit mode | Да | Open/close, bar zones видимы |
| Theme toggle | Да | Два toggles, исходный theme hash восстановлен |
| Four schemes | Да | Default baseline + Light/Solarized Dark/Mocha на white/black |
| Alpha / blur | Частично | Alpha inconsistency подтверждена; blur на flat background не является достаточным тестом |
| Frame normal / wrapped | Да | White + original wallpaper; исходный wrapped восстановлен |
| Wallpaper next / refresh / gallery | Да | Команды отправлены; gallery открылась; restore-path уронил daemon |
| Popup click catcher | Да | Свободный клик не закрыл Sound |
| Hover states | Частично | Cursor-positioned bar/rail interactions были, но систематического screenshot sweep каждого hover state не было |

## IPC coverage

Успешно упражнялись: `ping`, `toggle-side-panel`, `expand-side-panel`, `compose-and-send`, `preview-target`, `select-right-tab` для всех известных IDs, `toggle-workspace-mode`, `toggle-start-menu`, `toggle-launcher`, `toggle-edit-mode`, `toggle-theme`, `wallpaper-next`, `wallpaper-refresh`, `wallpaper-gallery`.

`select-right-tab` sweep:

- Доступны и различимы: `system`, `updates`, `notifications`, `files`, `preview`, `library`, `captures`, `acp_settings`, `hyprland_binds`, `display`, `launcher_settings`, `media`.
- Молча сбрасываются на System: `editor`, `terminal`, `inspector`, `build`, `source_control`, `scenes`, `mcp_settings`, `lsp_settings`, `api_providers`.

## Верификация и состояние после теста

Fresh log: `/tmp/t323/run-91f9a9e4/chronos.log`.

```text
panic=0
protocol_error=0
```

Исходные и финальные конфиги совпадают побайтно по SHA-256.

```text
frame before = 7617c40630d6f6ac1e179c34f80b6352159e65032fbee5712d67ea1b53f94e42
frame after  = 7617c40630d6f6ac1e179c34f80b6352159e65032fbee5712d67ea1b53f94e42
theme before = 3841c70c58d9bf1faa48617a0a88e3c431c339c2e345decbe9769d2bf2be524f
theme after  = 3841c70c58d9bf1faa48617a0a88e3c431c339c2e345decbe9769d2bf2be524f
```

Before/after `frame.toml`:

```toml
style = "wrapped"

[bottom_strip]
enabled = true
height = 4.0
junction = "break"
```

Before/after `theme.toml`:

```toml
blur_enabled = true
scheme = "Default"
surface_alpha = 1.0
```

Финальный кадр после восстановления: `/tmp/t323/run-91f9a9e4/frames/99-final-restored.png`.

Product code не изменялся. В рабочем дереве до начала уже находился untracked `.chronos-ops/reports-fresh/T323-full-ui-audit-report.md`; он сохранён без изменений и не является результатом этого прохода. Единственный созданный repository-файл — этот отчёт. Все screenshots и временные изображения находятся под `/tmp/t323/run-91f9a9e4/`.

## Что не проверено

- Не проводился systematic hover screenshot sweep каждого элемента.
- Не менялись workspace, keyboard layout и media playback, чтобы не нарушать текущую пользовательскую сессию.
- Не было аппаратной battery surface.
- Blur проверялся на flat white background; этого недостаточно для окончательного вывода о blur algorithm.
- Не выполнялся долгий soak test. После отказа Waytrogen restore исходный animated wallpaper возвращён, но долговременная устойчивость восстановленного `awww-daemon --no-cache` не доказана.
