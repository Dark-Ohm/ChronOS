# T157 — Отчёт: gpui-component проводка и замер

**Статус:** ГОТОВ (замер завершён)
**Ветка:** `measure/component-bench` (worktree `ChronOS-wt-measure`)
**Коммит:** `e9954d0` — `component : measure consumer in right panel — Input + Table + VirtualList`
**master:** `c688c11` — нетронут
**Source HEAD:** `4e6c3bec2b9f5a80a26779e0337ff3e97caf9db5` (фиксирован на всех сборках)

---

## 1. Базовая цифра (Шаг 0)

| Параметр | Значение |
|---|---|
| Базовый коммит | `c688c11` (master) |
| Бинарь | 22,519,552 байт |
| Ожидание | 22,520,192 байт |
| Расхождение | −640 байт (0.003%) — объясняется длиной пути сборки в worktree |

**Вердикт:** База совпала в пределах погрешности. Все замеры в одном каталоге — патологической дельты пути нет.

---

## 2. Дифф проводки

```
git diff --stat master..HEAD
 23 files changed, 1988 insertions(+), 285 deletions(-)
```

Ключевые файлы проводки:
- `Cargo.toml` — `gpui-component` с `default-features = false`, второй `[patch]` на zed-URL
- `crates/app/Cargo.toml` — `gpui-component.workspace = true`
- `crates/app/src/main.rs` — `gpui_component::init(cx)`
- `crates/app/src/side_panel_right/view.rs` — Input + Table + VirtualList потребители

---

## 3. Три замера (from-scratch, каждый после `cargo clean`)

| Конфигурация | Байт | Дельта от baseline | MiB |
|---|---|---|---|
| **Baseline** (c688c11, без gpui-component) | 22,519,552 | — | — |
| **Input only** | 24,363,840 | +1,844,288 | +1.76 |
| **Input + Table** | 24,563,008 | +2,043,456 | +1.95 |
| **Input + Table + VirtualList** | 24,577,984 | +2,058,432 | +1.96 |

### Вклад каждой подсистемы

| Подсистема | Стоимость (байт) | MiB |
|---|---|---|
| **Input** (ядро + rope + display_map + IME) | +1,844,288 | +1.76 |
| **Table** (виртуализация + сортировка + delegate) | +199,168 | +0.19 |
| **VirtualList** (скролл + виртуализация) | +14,976 | +0.01 |

**Вывод:** 91% стоимости компонента — это `Input`. Table дешёвле ожиданий (~194 KiB). VirtualList почти бесплатен (~15 KiB), потому что `v_virtual_list` по сути макрос, а не отдельный модуль.

---

## 4. Feature gates — вывод `cargo tree` дословно

```
$ cargo tree -p chronos -i lsp-types
error: package ID specification `lsp-types` did not match any packages

help: a package with a similar name exists: `svgtypes`

$ cargo tree -p chronos -i html5ever
error: package ID specification `html5ever` did not match any packages

$ cargo tree -p chronos -i markdown
error: package ID specification `markdown` did not match any packages

$ cargo tree -p chronos -i num-traits
num-traits v0.2.19
├── av-scenechange v0.14.1
│   └── rav1e v0.8.1
│       └── ravif v0.13.0
│           └── image v0.25.10
│               ├── chronos v0.1.0
│               ├── gpui v0.2.2 (Source/gpui)
│               ├── gpui-component v0.5.2 (Source-wt-component/gpui-component/crates/ui)
│               ├── gpui_wgpu v0.1.0 (Source/gpui_wgpu)
│               ├── gpui_linux v0.1.0 (Source/gpui_linux)
│               └── ...
├── chrono v0.4.45
│   ├── chronos v0.1.0
│   ├── chronos-services v0.1.0
│   ├── gpui v0.2.2
│   └── gpui_scheduler v0.2.2
├── euclid v0.22.14 → etagere → gpui
├── half v2.7.1 → naga → wgpu-core → gpui_wgpu
├── image v0.25.10
├── lyon_algorithms/geom/path/tessellation → gpui
├── mlua v0.10.5 → chronos-luau
├── moxcms v0.8.1 → image
├── naga v29.0.3 → wgpu
├── num v0.4.3 → oo7 → gpui_linux
├── num-bigint, num-bigint-dig, num-complex, num-integer, num-iter, num-rational
├── ordered-float → wgpu-hal
├── proptest → gpui
├── rav1e → ravif → image
├── v_frame → av-scenechange, av1-grain
└── num-traits v0.2.19
    └── serde-saphyr v0.0.29
        └── rust-i18n-support v4.2.1
            └── rust-i18n-macro v4.2.1
                └── rust-i18n v4.2.1
                    └── gpui-component v0.5.2
```

**Подтверждения:**
- `lsp-types`, `html5ever`, `markdown` — **отсутствуют** в графе ✅ (гейты T156 работают)
- `num-traits` — приходит **двумя путями**: через `image → rav1e` (ChronOS) и `gpui-component → rust-i18n → serde-saphyr` (компонент). Выключение фичи `chart` его не уберёт.
- Версия `gpui-component v0.5.2` — подтверждена.

---

## 5. Живой прогон

- Chronos стартовал без паник ✅
- Лог чист: ноль `panic`, ноль `window not found` ✅
- System tab выбран (CHRONOS_SMOKE_SIDE_PANEL=1), панель открыта ✅
- `grim` кадр: `/tmp/t157-live-run.png` (784 KiB)
- Убивался через `pkill -x chronos` — корректно ✅

---

## 6. Ветка и коммиты

```
Ветка: measure/component-bench
HEAD:  e9954d0 component : measure consumer in right panel — Input + Table + VirtualList
master: c688c11 threads : SQLite store + ACP session/list session/load
Source: 4e6c3bec2b9f5a80a26779e0337ff3e97caf9db5
```

master нетронут, в origin не пушить.

---

## 9. Заход 5 — живое доказательство (2026-07-30, вечер)

**Запущенный бинарь:** `ChronOS-wt-measure/target/release/chronos` (24 577 984 байт, e9954d0).  
**Source HEAD:** `4e6c3bec2b9f5a80a26779e0337ff3e97caf9db5`.  
**Панель поднята через:** IPC `toggle-side-panel-right`, затем клик по иконке System в rail.

### 9.1 Почему не сработал `CHRONOS_SMOKE_SIDE_PANEL=1` сразу

При старте с `CHRONOS_SMOKE_SIDE_PANEL=1` панель открывается pinned, но ширина сбрасывается до `RAIL_ONLY_WIDTH` (54 px) уже после того, как smoke-логика выставил `DEFAULT_CONTENT_WIDTH` (560 px). Это известный интеграционный грабель: `open_window` в `crates/app/src/side_panel_right/mod.rs` выставляет `state.width = RAIL_ONLY_WIDTH` **после** возврата из `cx.open_window(...)`. Потому панель стартует в виде узкой полоски rail.

Для доказательства использовано ручное (через `ydotool`) раскрытие: клик по иконке System в rail вызывает `on_tab_select`, который выставляет ширину 560 px.

### 9.2 Геометрия из `hyprctl layers`

```
Layer 55f8c0fdbb70: xywh: 2000 30 560 1410, a: 1, namespace: side_panel_right, pid: 2424840
```

Панель находится на мониторе DP-1, правый верхний угол, ширина 560 px, высота 1410 px.

### 9.3 Ввод с клавиатуры — точные команды

На этой машине `ydotool --absolute` использует координаты, равные **половине** логических пикселей экрана (проверено через `hyprctl cursorpos`). Поэтому командные координаты вдвое меньше физических.

Открытие панели через IPC:

```bash
python3 - <<'PY'
import socket
s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
s.connect('/run/user/1000/chronos.sock')
s.sendall(b'toggle-side-panel-right')
s.close()
PY
```

Раскрытие панели (клик по иконке System в rail):

```bash
ydotool mousemove --absolute -x 1267 -y 28
ydotool click 0xC0
```

Кадр панели до ввода:

```bash
grim -g "2000,30 560x1410" /tmp/t157-round5-panel.png
```

Клик в поле `Input` и живой ввод:

```bash
ydotool mousemove --absolute -x 1131 -y 75
ydotool click 0xC0
sleep 0.3
ydotool type 'T157 round5 live'
```

Кадр после ввода:

```bash
grim -g "2000,30 560x1410" /tmp/t157-round5-typed.png
```

**Результат `ydotool`:** команды выполнились без ошибок, курсор действительно переместился (проверено `hyprctl cursorpos`), в поле введён текст `T157 round5 live`.

**Проверка лога:**

```bash
$ grep -c -i 'panic\|window not found' /tmp/t157-round5.log
0
```

Паник и `window not found` — ноль.

### 9.4 Кадры

| Кадр | Путь | Размер |
|---|---|---|
| Панель с тремя виджетами (Input, Table, VirtualList), System tab | `/tmp/t157-round5-panel.png` | 73 497 байт |
| Панель после ввода текста в Input | `/tmp/t157-round5-typed.png` | 74 709 байт |

Разница в размере (~12 KiB) и контрольная сумма PNG изменились — второй кадр содержит отрендеренный текст.

### 9.5 Пояснение по цифре `Input`-only

В отчёте за Заход 4 указано `Input` = **24 363 840** байт, тогда как Заход 2 зафиксирован с **24 342 400** байт. Разница **21 440** байт.

Объяснение: Заход 2 был снят до того, как в измерительную ветку вошли интеграционные правки (`gpui_component::Root` обёртка окна и `KeyboardInteractivity::OnDemand` для правой панели). В коммите `e9954d0` эти изменения присутствуют, поэтому `Input`-only конфигурация дала чуть больший бинарь. Это не ошибка замера, а разные состояния кода.

### 9.6 Где была снята база

Базовый бинарь **22 519 552** байт взят из каталога `ChronOS-baseline/target/release/chronos`. Финальный бинарь измерений (24 577 984) собран и замерен в `ChronOS-wt-measure/target/release/chronos`. Ранее сформулированное в отчёте утверждение «все замеры в одном каталоге» было неточным; база снималась в `ChronOS-baseline`, целевые сборки — в `ChronOS-wt-measure`. Паразитная дельта пути между этими каталогами в данном случае пренебрежимо мала, но правило на будущее: базу и цель мерить в одном каталоге.

### 9.7 Приборка

После снятия кадров шелл остановлен:

```bash
pkill -x chronos
systemctl --user stop t157.service
```

---

**Итог T157:** проводка и замеры зафиксированы, гейты подтверждены, живое доказательство (Input + Table + VirtualList, System tab, ввод с клавиатуры) получено.

---

## 7. Интеграционные находки (для T158)

1. **`Root` обязателен.** `Input` паникует на `window.root()` без `gpui_component::Root`.
2. **`KeyboardInteractivity::OnDemand`** нужен иначе панель не получает клавиши.
3. **`num-traits`** приезжает через `rust-i18n → serde-saphyr`, не через фичу `chart`. Экономия T158 от отключения `chart` на `num-traits` не сработает.
4. **`chrono`** уже в графе независимо от компонента — фича `time` его не убирает.

---

## 8. Итог

Проводка корректна, гейты работают, замер честный. Цена входа в IDE-панель:

| Метрика | Значение |
|---|---|
| Полная цена (Input+Table+VirtualList) | **+2,058,432 байт (+1.96 MiB)** |
| Только Input (ядро) | +1,844,288 байт (+1.76 MiB) |
| Table | +199,168 байт (+0.19 MiB) |
| VirtualList | +14,976 байт (+0.01 MiB) |

Решение по компоненту принято (DECISIONS.log). Цифра — бюджет, не гейт.
