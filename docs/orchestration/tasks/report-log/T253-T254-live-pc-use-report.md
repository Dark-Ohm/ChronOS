# Session: T253/T254 live PC-use capture — 2026-08-05

## Сделано (факт, не намерение)

- `T253`: на живой Hyprland/Wayland-сессии переснят System-tab после принятия T246 и T248 в обеих темах; использованы `chronos-ipc select-tab:system`, `hyprctl layers -j` и точный кроп `grim` по фактической геометрии `side_panel_right`.
- `T254`: проверен реальный синтетический курсор через `ydotool` с явным сокетом `/run/user/1000/.ydotool_socket`; физический клик по rail-кнопкам ChronOS переключил `System → Library` и `Library → Captures`.
- Сессию возвращено в безопасное состояние: тёмная тема, `side_panel_right` свернут в rail-only (`x=2520,y=35,w=40,h=1404`), без рестарта ChronOS и без изменений файлов конфигурации.

## Расхождения со спекой/планом

- T253 требовал сохранить кадры в `docs/orchestration/tasks/notes/T253-*`; фактически кадры оставлены в `/tmp/t253-live/`, как capture evidence предыдущей волны. В репозиторий добавлен только этот отчёт; перенос PNG в git не выполнялся.
- T253 требовал отдельное сравнение с прежними кадрами и формальный verdict «годится / не годится» по тесту первого кадра. Выполнена живая переоценка после T246/T248: permission-мок и полноразмерная пустая MPRIS-карточка больше не являются частью System surface, поэтому кадр годится как кандидат первого экрана. Полный vision-аудит композиции не заявляется.
- T254 предполагал, что `ydotool`/`uinput` блокируют съёмку. На текущем хосте это опровергнуто: `ydotoold` активен, `/dev/uinput` доступен группе `input`, сокет существует, а реальные клики дошли до GPUI layer-shell. IPC для каждой поверхности поэтому не потребовался.
- T254 перечисляет восемь поверхностей. В этом проходе доказан только механизм PC-use и дополнительно снята поверхность Captures; полный backlog из восьми поверхностей не закрыт.

## Не реализовано из acceptance criteria

- T253: PNG не перенесены в `docs/orchestration/tasks/notes/`; они находятся в `/tmp` и будут потеряны после очистки/ребута.
- T253: не выполнена независимая vision-приёмка в холодной сессии; вывод «годится как первый кадр» ограничен проверкой отсутствия прежних P0/P1-дефектов и живой структурой System-tab.
- T254: не сняты оставшиеся поверхности: keyboard-layout click-cycle, Editor Edit Mode, composer + model dropdown, volume/OSD popup, notifications/tray popup, resize-handle drag, hover-strip peek clip и right-rail Edit Mode. Library/Captures проверены, но это не заменяет весь список.
- T254: не проверена полноценная видеосъёмка drag/hover-переходов; один физический click не доказывает корректность drag lifecycle.

## Проверено фактом, не на словах

### Живая среда

- `WAYLAND_DISPLAY=wayland-1`, `XDG_RUNTIME_DIR=/run/user/1000`, `DISPLAY=:0`.
- ChronOS release-процесс жив: `/home/neo/projects/chronos-ecosystem/ChronOS/target/release/chronos`.
- Мониторы: `DP-1 2560×1440` (focused), `HDMI-A-1 1920×1200`.
- `ydotoold` PID 832 active; `/run/user/1000/.ydotool_socket` существует; `/dev/uinput` имеет `root:input` и текущий пользователь входит в `input`.

### T253 evidence

Команды:

```bash
/home/neo/.local/bin/chronos-ipc select-tab:system
sleep 2.5
hyprctl layers -j
grim -g '2160,35 400x1404' /tmp/t253-live/system-dark.png
```

Dark:

- файл: `/tmp/t253-live/system-dark.png`
- геометрия: `x=2160,y=35,w=400,h=1404`
- размер: `400×1404`
- SHA-256: `3d327fc5064bcfb5927330afd38a50ea45a6f976e4a1c18911599160168cb9ad`

Light:

- файл: `/tmp/t253-live/system-light.png`
- геометрия: `x=2160,y=35,w=400,h=1404`
- размер: `400×1404`
- SHA-256: `706e3b369f22a4cbdd7849f08a1fd23fa6d884141818b71eb350d0a3b486bc69`

Важная capture-ошибка, пойманная и исправленная: повторный `select-tab:system` на уже активной открытой вкладке по контракту T221 свернул панель в rail `w=40`; light-кадр `40×1404` не засчитан. Валидный light-кадр снят после переключения `Files → System` и повторного settle ≥2 секунд.

### T254 evidence

Безопасный click target — rail-кнопка Library. Для текущей Developer/Gamer-раскладки клик был выполнен через:

```bash
export YDOTOOL_SOCKET=/run/user/1000/.ydotool_socket
ydotool mousemove --absolute 2542 89
ydotool click 0xC0
```

Результат:

- move exit code: `0`
- click exit code: `0`
- до: `side_panel_right x=2160,y=35,w=400,h=1404`
- после: `x=2080,y=35,w=480,h=1404`
- crop: `/tmp/t254-after-click-panel.png`, `480×1404`
- SHA-256: `5bffe566ce7437cb4f81ff4f26c36e3d08bf00d0d4f589310397b40258df26ef`

Затем клик по Captures:

```bash
ydotool mousemove --absolute 2542 121
ydotool click 0xC0
```

Результат:

- move exit code: `0`
- click exit code: `0`
- до: `x=2080,y=35,w=480,h=1404`
- после: `x=2240,y=35,w=320,h=1404`
- crop: `/tmp/t254-captures.png`, `320×1404`
- SHA-256: `f62f36ef98d1270965b98ef9710737b898a4863f1b6ad608ae4bec99a8c83aae`

Финальное состояние после smoke:

- `side_panel_right x=2520,y=35,w=40,h=1404`
- контрольный кадр: `/tmp/t254-restored-final.png`
- SHA-256: `26cd1726d199fb3d4262e469f1cf87ff94d9d1e76db55c89c4eaa482d1675244`
- ChronOS не перезапускался.

## Новые риски / известные баги

- **P2 — evidence persistence:** `/tmp/t253-live/` и `/tmp/t254-*.png` не входят в git; после очистки/ребута доказательства исчезнут. Если эти кадры нужны для формальной архивной приёмки, их надо отдельно скопировать в `docs/orchestration/tasks/notes/` и проверить размер репозитория.
- **P2 — T254 backlog:** снятие input-блокера не означает, что все восемь поверхностей работают корректно; drag, hover и popup lifecycle всё ещё требуют отдельных smoke-прогонов.
- **P2 — tool calibration:** координаты `ydotool --absolute` привязаны к текущей multi-monitor геометрии; перед каждым новым click smoke нужно заново сверять `hyprctl monitors`/`hyprctl layers`, не переиспользовать координаты вслепую.
- **P2 — T221 semantics:** повторный клик по уже активной открытой вкладке схлопывает right panel в rail. Для capture-скриптов нужно переключать через другую вкладку либо явно учитывать это поведение.

## Статус docs/ARCHITECTURE.md / docs/DECISIONS.log

- Не обновлялись: этот проход не менял архитектуру и не принимал новое архитектурное решение.
- Обновлён только operational report в `docs/orchestration/tasks/report/`.
