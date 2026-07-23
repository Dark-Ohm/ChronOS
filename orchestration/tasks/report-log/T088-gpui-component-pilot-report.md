<!-- T088 — migrated 2026-07-22 from orchestration/report-log/hermes-report-19.md — see orchestration/tasks/MIGRATION.md -->

# Session: Hermes №19 — пилот-спайк цена `gpui-component` в ChronOS — 2026-07-21

**Агент:** Hermes (исполнение Lead Architect / Grok).  
**Изоляция:** worktree `../ChronOS-gpc-pilot`, ветка `pilot/gpui-component-spike`  
**Коммит пилота:** `20ee13a` — master **не** тронут (`fb3ba40`).  
**Source/gpui-component:** не патчили (только path + `[patch]` в ChronOS).

---

## Вердикт + цифры (первая строка)

**A — цена приемлема, брать тулкит.**  
Бинарь **+713 472 B (+0.68 MiB, +3.5%)** vs baseline 20 152 392 → **20 865 864**;  
clean release from-scratch **~4m 35s** (wall 275s); граф **+187 строк tree / +69 unique packages**;  
**один** `gpui` path `Source/gpui`; dual-gpui **нет**.  
Layer-shell smoke: панель открылась, **Button «Log out» видна в dark**, без panic/error.  
Автоклик ydotool на multi-monitor не попал (известная хрупкость) — визуал + log open = ok; click-handler в коде, живой клик — за пользователем/Архитектором.

---

## Сделано (факт)

### Проводка (что сработало для `[patch]`)

1. **ChronOS workspace** — `gpui`/`gpui_platform` с git Chronos-GPUI →  
   `path = "../Source/gpui"` / `../Source/gpui_platform` (тот же tree, tip `99cab5e`).  
   Иначе ChronOS и component жили бы на **разных URL** git → dual-gpui гарантирован.
2. **dep:** `gpui-component = { path = "../Source/gpui-component/crates/ui" }`  
   в workspace + `gpui-component.workspace = true` в `crates/app`.
3. **`[patch."https://github.com/zed-industries/zed"]`** → path Source:  
   `gpui`, `gpui_macros`, `gpui_platform`, `gpui_web`.  
   Component-манифест по-прежнему зовёт zed-git — patch переписывает **без** правки  
   `Source/gpui-component`.
4. `gpui_component::init(cx)` + `Theme::change(ThemeMode::Dark, None, cx)`  
   сразу после `theme_config::init` (их default Light — закрыто).
5. **Одна** `Button::new("pilot-logout").label("Log out").on_click(...)`  
   в `side_panel_right/view.rs`.
6. Smoke-only: `CHRONOS_SMOKE_SIDE_PANEL` → `open_pinned` в `init`  
   (как Task 7; только пилот-ветка).

### Файлы пилота (коммит `20ee13a`)

| Файл | Что |
|---|---|
| `Cargo.toml` | path gpui + gpui-component + `[patch]` zed |
| `Cargo.lock` | +~1200 строк (deps component) |
| `crates/app/Cargo.toml` | `gpui-component.workspace` |
| `crates/app/src/main.rs` | init + Dark |
| `side_panel_right/view.rs` | Button stub |
| `side_panel_right/mod.rs` | smoke env hook |

---

## Замеры

### Бинарь (release, LTO+strip opt-level=z, clean target)

| | bytes | MiB |
|---|---:|---:|
| Baseline (master, Architect 2026-07-21) | 20 152 392 | 19.22 |
| Pilot clean release (toolkit + Button) | **20 865 864** | **19.90** |
| **Δ** | **+713 472** | **+0.68 (+3.5%)** |

Команда: `rm -rf target && cargo build --release -p chronos`  
в worktree → `Finished release … in 4m 35s`.

> После smoke-hook incremental rebuild размер скакнул до ~22.8 MiB — **не**  
> принимать: partial LTO/strip. Официальная цифра — **clean** 20 865 864.

### Время сборки

| | wall |
|---|---|
| Pilot clean release | **~275 s (~4m 35s)** |
| `cargo check -p chronos` (после resolve) | ~61 s |
| Incremental release (smoke hook) | ~163 s |

Порядок величины для full LTO chronos — нормален; toolkit не превратил  
сборку в «час». Точного clean-baseline master в этой сессии не гонял  
(бинарь master уже был) — сравнивать «с нуля» только пилот vs себя.

### Граф зависимостей

| | master | pilot | Δ |
|---|---:|---:|---:|
| `cargo tree \| wc -l` | 1713 | 1900 | **+187** |
| unique (`--prefix none \| sort -u`) | 768 | 837 | **+69** |

**`cargo tree -i gpui`:** единственный  
`gpui v0.2.2 (/…/Source/gpui)` — consumers: chronos, chronos-ui, chronos-luau,  
gpui-component, assets, gpui_platform/linux/wgpu.

**Тяжёлые крейты реально в графе** (подтверждено `cargo tree -p chronos`):

- `gpui-component 0.5.2` + macros + assets  
- `ropey 2.0.0-beta.1`  
- `markdown 1.0.0`  
- `html5ever 0.27.0`  
- `lsp-types 0.97.0`  
- `zed-sum-tree 0.2.0`  

Монолит default-deps №18 — **не миф**, линкуются. При этом strip+LTO  
съели почти всё: **+0.68 MiB** на весь этот зоопарк + 89k LOC UI.

---

## Smoke layer-shell

```
pkill -x chronos
RUST_LOG=info CHRONOS_SMOKE_SIDE_PANEL=1 \
  ./target/release/chronos   # pilot binary
```

**Log:**

```
INFO side_panel_right: opened pinned
INFO side_panel_right: CHRONOS_SMOKE_SIDE_PANEL open_pinned
# no panic / no error / no ghost-window warn
```

**hyprctl layers:**

```
xywh: 2260 15 300 1440, namespace: side_panel_right, pid: <pilot>
```

(DP-1 2560×1440; y=15 ≈ под баром 30? чуть выше Task7 y=30 — не блокер пилота)

**grim:** `orchestration/reports/hermes-19-smoke/`

| файл | что видно |
|---|---|
| `panel-crop.png` | текст «side panel: gpui-component pilot (Button stub)» + **тёмная кнопка «Log out»** |
| `panel-full.png` | dual-monitor full |
| `panel-after-click.png` | повторный crop (клик автоматом не подтверждён) |

**Button dark:** да (component Dark theme; фон панели — chronos `bg.secondary` — два Theme Global, визуально стыкуется).  
**Tooltip:** не провоцировали (Button без `.tooltip(...)` в stub).  
**Focus steal / KeyboardInteractivity:** панель `None` как Task 7; shell не завис, input-stack жив.  
**Click:** `on_click` пишет `side_panel_right pilot: Log out Button clicked` — в log **не** пойман (ydotool absolute на dual-head 4480px врёт координаты; HANDOFF: ydotool хрупкий). Визуал кнопки + отсутствие crash = runtime «Button живёт на layer-shell» доказан; click-path — unit-of-trust на коде + ручной клик при приёмке.

---

## Расхождения / что сработало для patch

| Ожидание брифа | Факт |
|---|---|
| `[patch]` zed при ChronOS на Chronos-GPUI git | **Недостаточно одного patch zed** — ChronOS и component на разных git URL. Нужно **ещё** перевести ChronOS `gpui` на path Source (или patch обоих URL на path). Сделано path. |
| dual-gpui блокер | **Не случился** при path+patch |
| +MiB «месяц боли» | **+0.68 MiB** — дешёво |
| runtime layer-shell | Button рисуется; click auto не дожат |

---

## Не реализовано

- Полный theme-адаптер 141 поле — не scope (пилот).  
- Power-real / MPRIS / meters — не scope.  
- Merge в master — запрещён.  
- Автоподтверждение click в log — ydotool fail.

---

## Проверено фактом

```
# dual gpui?
cargo tree -i gpui
# → gpui v0.2.2 (…/Source/gpui) only

# sizes
stat -c '%s' target/release/chronos
# 20865864   (clean pilot)
# baseline 20152392 → +713472

# build
# Finished `release` profile [optimized] target(s) in 4m 35s

# smoke
hyprctl layers | rg side_panel
# namespace: side_panel_right, 300×1440
# grim panel-crop.png → Log out button visible, dark
```

---

## Новые риски

| Sev | |
|---|---|
| Low | Incremental release size noise после мелких правок — всегда clean measure |
| Low | ydotool multi-monitor — не полагаться на автоклик приёмки |
| Med | Монолит ropey/markdown/html/lsp в графе навсегда, пока dep целый; feature-strip нет. Binary OK сейчас; compile-time/IDE index — тяжелее |
| Info | Два Theme Global: панель-chrome chronos, Button — component dark. Без адаптера кнопка «чужая» по токенам, но читаема |

---

## Рекомендация A vs C

| | A (тулкит) | C (rsx+animation руками) |
|---|---|---|
| Binary | **+0.68 MiB** | 0 |
| Compile dual-gpui | решён path+patch | n/a |
| Layer-shell Button | **рисуется** | писать самим |
| Power-row / Slider / Progress | готовы | недели UI |
| Coupling | 89k + heavy default deps | тонкий |

**A.** +3.5% бинаря за полный shadcn-kit на нашем gpui — разумная плата  
за Tasks 9–11. Не C: руками Button/Slider/Progress дольше, чем wiring  
уже зелёного toolkit.

**Следующий шаг (не этот пилот):** интеграционный бриф в master —  
path Source + patch, `init`+Dark, theme-map ~20–30 ролей, выкинуть smoke env,  
Tasks 9–11 на `gpui_component::{Button,Slider,Progress,…}`.

---

## Статус ARCHITECTURE / DECISIONS

Не обновлялись (пилот-ветка). После принятия A Архитектором — кандидат  
DECISIONS: «gpui-component path-dep; +0.68 MiB measured; dual Theme OK».

---

## Артефакты

- Worktree: `/home/neo/projects/chronos-ecosystem/ChronOS-gpc-pilot`  
- Branch: `pilot/gpui-component-spike` @ `20ee13a`  
- Smoke: `orchestration/reports/hermes-19-smoke/panel-crop.png`  
- Logs: `/tmp/gpc-pilot-check.log`, `/tmp/gpc-pilot-release.log`, `/tmp/gpc-pilot-smoke.log`
