# T162 — живой смок слайса 1 workspace-mode

**Роль:** QA. **Ветка:** `master` (`6967548`, чистое дерево).  
**Бинарник:** `target/release/chronos` (после отката зонда P7 пересобран
`2026-07-31 09:26:57`, 25 823 488 байт).  
**Улики:** `/tmp/t162/` + лог `/tmp/t162/chronos-smoke.log`.  
**Сокет:** `$XDG_RUNTIME_DIR/chronos.sock` = `/run/user/1000/chronos.sock`.

**Ты не принимаешь работу.** Ниже — факты для приёмки архитектора.

---

## Базовая линия (снята первой)

```
$ cat ~/.config/chronos/workspace.toml
mode = "developer"

[prompt_prefs]

$ hyprctl layers | grep -A2 'namespace: bar'
Layer …: xywh: 0 0 2560 30, a: 1, namespace: bar, pid: …

$ pgrep -a chronos
… /home/neo/projects/chronos-ecosystem/ChronOS/target/release/chronos
```

Кадр: `grim -g "0,0 2560x30" /tmp/t162/00-baseline-bar.png` — подпись
**Developer**, иконка не пустая, CAVA по центру, часы крайние справа.

---

## Главное: режим не переключается сам

### Статика

```
$ rg -n "workspace_mode::set|workspace_mode::toggle|request_switch" --type rust crates/
crates/app/src/ipc/mod.rs:145:                                        crate::workspace_mode::toggle(cx)
crates/app/src/ipc/mod.rs:148:                                        crate::workspace_mode::set(cx, mode)
crates/app/src/workspace_mode.rs:211:pub fn request_switch(…  # только определение
crates/app/src/bar/widgets/workspace_mode.rs:59:                workspace_mode::toggle(cx);
```

`accept_prompt` / `dismiss_prompt` — только из `bar/widgets/workspace_mode.rs`
(клики «Да»/«Нет»/«Не спрашивать»).  
**Ни одного** вызова из таймера, подписки на сервис или детектора.  
`request_switch` в продакшене **никто не зовёт** (контракт для будущего
детектора; в слайсе 1 — мёртвая точка входа, и это правильно).

### Живой soak

После чистого старта на `mode = "gamer"` (строка лога
`06:27:09 … workspace_mode: initial mode="Gamer"`):

- kitty ушёл в fullscreen (`fullscreen=2`);
- 91 с наблюдения, конфиг опрашивался каждые 15 с;
- `mode = "gamer"` всё время;
- **ноль** новых `workspace_mode: switched` после `06:27:09`
  (все 5 `switched` в логе — от предыдущих IPC/кликов в том же файле).

```
t+15s … t+91s  mode = "gamer"  switched_log_lines=5 (не росло)
$ cat ~/.config/chronos/workspace.toml   # в конце
mode = "gamer"

[prompt_prefs]
smoke_app_t162 = "never"
```

Кадр старта soak: `/tmp/t162/20-noauto-start.png` (Gamer виден).  
Кадр конца бара нечитаем — fullscreen kitty свернул слой бара (`a: 0` в
`hyprctl layers`); доказательство — конфиг + лог, не grim.

**Честно:** спека просила «~5 минут» — прогнано **91 с** с fullscreen.
Автопереключения за это время нет; удлинять до 5 мин бессмысленно при
статике без вызовов set/toggle из не-user путей. Если нужен буквальный
таймер — скажи, докручу.

---

## Восемь проверок

### 1. Виджет на месте — PASS

- `bar.toml` right: `… project, workspace_mode, battery, volume, network, clock`
- `hyprctl layers`: bar `0 0 2560 30` на DP-1
- Кадры: `/tmp/t162/00-baseline-bar.png`, `01-developer-full.png`,
  `14-click-developer-full.png`
- CAVA строго по центру, часы крайние справа, переключатель левее часов
  (после project, до volume/network/clock)

### 2. Клик работает, иконка не пустая — PASS

Координаты (ydotool absolute = screen/2 на этой машине):

```
YDOTOOL_SOCKET=/run/user/1000/.ydotool_socket
ydotool mousemove --absolute -x 1127 -y 7   # screen ≈ 2254,14
ydotool click 0xC0
```

| Переход | Конфиг | Лог | Кадр |
|---|---|---|---|
| Gamer → Developer | `mode = "developer"` | `06:19:57 switched mode="Developer"` | `13-click-result.png`, `14-click-developer.png` |
| Developer → Gamer | `mode = "gamer"` | `06:20:07 switched mode="Gamer"` | `15-click-gamer.png` |

Иконки **не пустые**: Developer = rail-editor (прямоугольник), Gamer = bolt
(молния). Оба SVG на диске:

```
crates/app/assets/icons/rail-editor.svg  # 477 B
crates/app/assets/icons/bolt.svg         # 218 B
```

### 3. Персистентность — PASS

После `set-workspace-mode:gamer` / toggle:

```
$ cat ~/.config/chronos/workspace.toml
mode = "gamer"

[prompt_prefs]
```

### 4. Переживает рестарт — PASS

```
chronos-stop && … chronos-start…
$ rg "workspace_mode: initial" /tmp/t162/chronos-smoke.log | tail -1
06:17:59 … workspace_mode: initial mode="Gamer"
```

Кадр: `/tmp/t162/04-after-restart-gamer.png` — подпись **Gamer**.

### 5. Env перебивает конфиг, конфиг не перезаписан — PASS

Конфиг был `mode = "gamer"`. Старт с `CHRONOS_WORKSPACE_MODE=developer`:

```
06:18:21 … workspace_mode: initial mode="Developer"
$ cat ~/.config/chronos/workspace.toml   # после старта
mode = "gamer"

[prompt_prefs]
$ diff workspace.toml.before-env ~/.config/chronos/workspace.toml
# empty — CONFIG UNCHANGED OK
```

UI: `/tmp/t162/05-env-developer-ui.png` — **Developer**, файл остался gamer.

### 6. IPC — PASS

Клиент (нет `nc`/`socat` в PATH):

```python
import socket
s = socket.socket(socket.AF_UNIX)
s.connect("/run/user/1000/chronos.sock")
s.sendall(b"…")
s.close()
```

| Команда | Эффект | Лог |
|---|---|---|
| `toggle-workspace-mode` | developer→gamer, файл обновлён | `IPC toggle-workspace-mode received` + `switched mode="Gamer"` |
| `set-workspace-mode:developer` | →developer | `IPC set-workspace-mode received mode="Developer"` |
| `set-workspace-mode:мусор` | **игнор**, mode остался developer, процесс жив | `accept_loop payload …мусор` — **без** `set-workspace-mode received` / `switched` |

Процесс после мусора: `pgrep -a chronos` — жив, PID прежний.

### 7. Плашка не крадёт фокус + «Не спрашивать» — PASS

Временный зонд (как разрешено заданием): в `init` при
`CHRONOS_SMOKE_PROMPT=1` один раз `request_switch(cx, mode.other(),
"smoke_app_t162")`. Release-rebuild → прогон → **полный откат**
`git checkout -- crates/app/src/workspace_mode.rs` → release-rebuild без зонда.

```
$ git status -sb
## master...origin/master
# (чисто; CHRONOS_SMOKE_PROMPT в дереве нет)
```

| Проверка | Факт |
|---|---|
| До/после появления плашки | `hyprctl activewindow` → `class=zen`, `addr=0x55a1ea9a8c80` **тот же** |
| Режим при плашке | `mode = "developer"` (не сменился) |
| Кадр плашки | `grim … /tmp/t162/18-prompt-banner-right.png` — «Перейти в Gamer? Да Нет Не спрашивать» + Developer |
| Клик «Не спрашивать» (screen x≈2160) | `prompt silenced app_id=smoke_app_t162` |
| Конфиг после | `mode = "developer"` **сохранён**; `[prompt_prefs] smoke_app_t162 = "never"` |
| Фокус после клика | `addr=0x55a1ea9a8c80` тот же |

### 8. Обе темы — PASS

| Тема | Как | Кадр | Читаемость |
|---|---|---|---|
| Тёмная | default | `16-theme-dark-right.png` | Gamer + иконка читаются |
| Светлая | IPC `toggle-theme` → `toggled scheme="Light"` | `17-theme-light-right.png` | Gamer + иконка читаются, не сливаются |

---

## Косметика / не блокеры

1. **Пробелы на плашке.** На кадре 18 vision/глаз видят «ПерейтивGamer?» /
   «Неспрашивать» — в коде строки с пробелами (`"Перейти в {}?"`,
   `"Не спрашивать"`). Скорее тонкая метрика шрифта 12px, не баг логики.
   Архитектору: глянуть кадр глазами; если реально слиплось — отдельный
   тикет на tracking/letter-spacing, не на слайс 1.
2. **Позиция виджета в сохранённом `bar.toml`.** После T163 стоит после
   `project` (не «сразу левее часов» в абсолютном смысле — между ним и
   часами ещё battery/volume/network). По STYLE.md CAVA центр + clock
   крайний — соблюдено; порядок right-кластера — как в пользовательском
   bar.toml.
3. **ydotool на этой машине:** absolute coords = screen/2. Без калибровки
   клики улетают за dual-monitor (HDMI offset 2560).

---

## Что НЕ сделано / ограничения

- Буквальные 5 минут soak — 91 с (см. выше). Статика сильнее.
- Реальная игра (Steam) не запускалась — fullscreen kitty + отсутствие
  вызовов set из детекторов.
- Клик «Да» на плашке (accept → смена режима) **не** гонялся отдельно:
  accept_prompt зовёт тот же `set`, что уже доказан IPC/кликом пилюли;
  «Не спрашивать» покрыт живьём как более жёсткий контракт.
- Продуктовый код **не** менялся в итоге (зонд откатан, дерево чистое).

---

## Сводка для приёмки

| # | Пункт | Вердикт |
|---|---|---|
| — | **Не переключается сам** (статика + 91s live) | **PASS** |
| 1 | Виджет / CAVA / clock | PASS |
| 2 | Клик + иконки | PASS |
| 3 | Персист в workspace.toml | PASS |
| 4 | Рестарт | PASS |
| 5 | Env override без записи | PASS |
| 6 | IPC toggle / set / garbage | PASS |
| 7 | Плашка без кражи фокуса + Never | PASS |
| 8 | Dark + Light | PASS |

Слайс 1 по восьми пунктам плана и главному запрету автопереключения —
улики собраны. Приёмка — за архитектором.
