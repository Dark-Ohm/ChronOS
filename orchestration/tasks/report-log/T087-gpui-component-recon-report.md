# Session: Hermes №18 — разведка `gpui-component` (Longbridge) об наш форк — 2026-07-21

**Агент:** Hermes (исполнение Lead Architect / Grok по запросу пользователя).
**Режим:** read-only разведка компиляцией. Коммит не требуется.
**Throwaway:** `/home/neo/scratch/gpui-component-recon` (только манифест).
**Source/ChronOS код:** не трогались (Source `gpui-component/` без RECON-патча).

---

## Вердикт (первая строка)

**БРИФ, не месяц.** `gpui-component 0.5.2` (`crates/ui`) **компилируется
об наш форк `Source/gpui` @ `99cab5e` с нулём ошибок** — достаточно path-deps
в workspace-манифесте. Это **rsx-паттерн** (чистый check), **не** animation/ccf
(58×дельта). Полная интеграция Longbridge как path-зависимости — реалистичный
интеграционный бриф (дни–неделя: wiring + theme-адаптер + smoke layer-shell),
а не месяц ABI-правок. Рекомендация: **брать Longbridge dependency'ем**,
не переписывать Button/Slider руками и не вендорить с патч-сеткой как ccf.

---

## Сделано (факт, не намерение)

- rsync `Source/gpui-component/` → `/home/neo/scratch/gpui-component-recon`
  (без `target/`, без `.git`).
- Манифест-патч (только throwaway):
  - members сужены до `crates/{macros,ui,assets}` (story/webview/examples
    отрезаны — они тянут `reqwest_client`/`gpui_web` для галереи, не для UI).
  - `gpui` / `gpui_macros` / `gpui_platform` / `gpui_web` →
    `path = "/home/neo/projects/chronos-ecosystem/Source/<crate>"`.
  - `reqwest_client` оставлен git pin `876ec5a8` (как в Source workspace);
    **crates/ui его не depends'ит**.
  - `Cargo.lock` удалён → чистый резолв.
- `cargo check -p gpui-component` → **Finished, 0 error[E…], 0 `error:`**.
- `cargo tree`: **один** `gpui v0.2.2` (path Source). Двойного gpui нет.
- Theme: `gpui_component::Theme` = `Global` + ~141 `Hsla` в `ThemeColor`;
  chronos = другой тип `Global` (~26 `Hsla`-листьев) — type-keyed, оба
  сосуществуют; нужен **адаптер цветов**, не rename.
- Минимальный набор виджетов под панель (Tasks 9–11) — см. ниже.
- `Source/gpui-component/Cargo.toml` — без RECON-патча (проверено).

---

## Гистограмма ошибок

| Код | Кол-во | Класс |
|---|---|---|
| *(пусто)* | **0** | — |

Сравнение с предыдущими recon:

| Цель | Ошибки | Паттерн |
|---|---|---|
| `gpui-rsx` (Cline) | 0 | чистый check |
| `gpui-animation` (Grok) | >0, API-дельты | animation |
| `ccf-gpui-widgets` (Hermes) | **58** (E0061×43 …) | animation / вендор |
| **`gpui-component` (это)** | **0** | **rsx-паттерн** |

Лог: `/tmp/gpui-component-check.log` (895 строк, `Finished … 40.66s`).
Единственные warning'и — `missing documentation` **внутри нашего**
`Source/gpui` (spring.rs и т.п.), не из component.

---

## Метод и доказательная база

```
# throwaway only
rsync -a --exclude target --exclude .git \
  Source/gpui-component/ → /home/neo/scratch/gpui-component-recon/

# workspace.dependencies (фрагмент после патча):
gpui = { path = "…/Source/gpui" }
gpui_macros = { path = "…/Source/gpui_macros" }
# platform/web — path, но ui-крейту не нужны; story выкинут из members

rm Cargo.lock
cargo check -p gpui-component   # exit 0, 40.66s
```

- `Compiling gpui v0.2.2 (/home/neo/…/Source/gpui)` — note: defined/resolved
  **об наш форк**, не zed HEAD.
- `Compiling gpui-component v0.5.2 (…/scratch/…/crates/ui)` → Finished.
- Единственность gpui: `cargo tree -p gpui-component -i gpui` → один path.

---

## Классы дельт (форк-vs-апстрим)

**Нет compile-time дельт.** Код `gpui-component` 0.5.2 на том API-поверхности,
что он зовёт, совместим с нашим `gpui 0.2.2` (база Zed pin `876ec5a8` +
gpui-ce). Это опровергает ожидание брифа «разрыв, вероятно, БОЛЬШОЙ /
сотни ошибок» — для **этой** версии component против **этого** форка
разрыв по компиляции = 0.

Оговорка (честная): check = typecheck default features `crates/ui`.
**Не** прогнаны: `story`, examples, runtime на layer-shell, feature
`tree-sitter*`, `inspector`. Runtime-баги возможны; ABI-разлома compile
не показал.

---

## Транзитив / dual packages (не блокеры dual-gpui)

| Пакет | Источники | Вердикт |
|---|---|---|
| `gpui` | **1×** path `Source/gpui` | OK |
| `gpui_util` | path `Source/gpui_util` **и** git zed@`876ec5a8` (через `http_client`, dep **нашего** `gpui`) | ambiguous tree, но **не** dual-gpui; уже есть в Source workspace, component не вводил |
| `sum_tree` | `gpui_sum_tree` (форк) + `zed-sum-tree 0.2.0` (crates.io, dep component) | разные package name — OK |
| `reqwest_client` | **не** в графе `gpui-component` (ui) | story-only; эскалация dual-gpui **не сработала** |

`cargo tree -p gpui-component | rg reqwest` — пусто для component-пакета
(reqwest-стек живёт внутри Source/gpui → http_client, как всегда).

---

## Тема: `gpui_component::Theme` vs `chronos_ui::Theme`

| | Longbridge | ChronOS |
|---|---|---|
| Тип | `gpui_component::theme::Theme` | `chronos_ui::theme::Theme` |
| `Global` | да (`impl Global for Theme`, `theme/mod.rs:111`) | да (`mod.rs:224`) |
| Цвета | `ThemeColor` ≈ **141 `Hsla`** (плоский shadcn-набор: button_*, danger_*, chart_*, …) | группы `bg/text/border/accent/status/interactive` ≈ **26 `Hsla`** |
| Шрифты | `font_family`, `mono_font_family: SharedString`, sizes `Pixels` | `font_ui`/`font_mono: &'static str` (+ `FontSizes`) |
| Radius | `radius`, `radius_lg` | `radius`, `radius_lg` (есть) |
| Init | `gpui_component::init(cx)` → `Theme::change(Light, …)` + registry | `Theme::init(cx)` / `Theme::set` |

**Коллизия Global:** type-keyed map → **два разных типа спокойно живут
рядом**. Конфликта «второй Theme затрёт первый» **нет**.

**Реальная работа — синхронизация палитры:**

1. Адаптер `fn apply_chronos_to_component(cx)`: map ~20–30 семантических
   ролей chronos → subset `ThemeColor` (background←bg.primary/tertiary,
   border←border.subtle, primary/accent←accent.primary, danger←status.error,
   button_*←interactive/accent, …). Остальные ~100 полей — дефолты
   Longbridge / Catppuccin-ish из их dark theme JSON.
2. Либо панель живёт **только** на `gpui_component::Theme` (хронос-токены
   в панели не читаются) — быстрее, но два визуальных языка.
3. **Не** «копия структуры 1:1» — модели разные (как ccf, но у ccf ещё
   `u32` vs `Hsla`; здесь обе стороны `Hsla` — проще).

Объём адаптера: **~1 файл, десятки присваиваний**, не месяц. Переоценка
ccf «~40 полей Hsla→u32» здесь мягче (Hsla→Hsla).

---

## Минимальный набор виджетов для правой панели

План (`2026-07-20-right-side-panel.md`): power-row (Log out / Restart /
Shutdown / Switch user disabled), MPRIS-карточка, spectrum/meters.

| Нужда панели | Виджет Longbridge | LOC (ориентир) | Тянет |
|---|---|---|---|
| Power buttons | `button::Button` (+ variants) | button.rs ~1396 + group/icon/toggle | `Icon`, `Tooltip`, `theme`, `Sizable`, `h_flex`, `StyledExt` |
| Transport / mute | `Button` / `button::Toggle` | см. выше | то же |
| Volume-like | `slider::Slider` (+ state entity) | slider.rs ~770 | `ActiveTheme`, `StyledExt` |
| Meters (CPU/…) | `progress::Progress` / `ProgressCircle` | ~400 | theme |
| Track meta | `label::Label`, `separator::Separator` | мелкие | theme |
| Cover art | `avatar::Avatar` **или** наш `RenderImage` | avatar/* | — |
| Badge/unread-style | `badge::Badge` | badge.rs | — |

**Хирургия «только Button»:** монокрейт `crates/ui` (~**89k LOC**, 238 `.rs`,
~50 top-level modules). Feature-флагов «без table/dock/input» **нет** —
в `lib.rs` все `pub mod` всегда в сборке. Default deps **всегда** тянут
`ropey`, `markdown`, `html5ever`/`markup5ever_rcdom`, `lsp-types`,
`notify`, `sum-tree` (zed-sum-tree), `chrono` — даже если панель table/LSP
не трогает.

**Вывод по подмножеству:** компиляционно «взять 3 файла Button» = вендор +
отрезать зависимости вручную (недели работы на граф). Практический путь:
**весь `gpui-component` path-dep** (раз он уже зелёный), в UI панели
использовать 5–8 типов. Не тащить `webview`/`story` (уже отрезаны members).

Assets: `crates/assets` ~420K, ~106 файлов (иконки Lucide-набор для
`IconName`) — обязательный соседний member.

---

## Рекомендация с цифрами

| Вариант | Оценка | Зачем |
|---|---|---|
| **A. Path-dep Longbridge + theme-адаптер** | **бриф, 3–7 чел·дней** | 0 compile-дельт; wiring workspace + `init` + map Theme + smoke panel |
| B. Вендор + вырезать Button/Slider | 2–4 недели | 89k монолит, граф `use crate` широкий; бессмысленно при A=зелёный |
| C. Ручные div'ы + rsx/animation | уже доступно | дешевле по binary, но power-row/slider/progress писать самим; дублировать Longbridge |
| D. Как ccf — «не брать» | **отклонён** | у ccf было 58 ошибок; здесь 0 |

**Итоговая рекомендация: A.** Писать интеграционный бриф:

1. Внести `gpui-component` (+ macros, assets) path из `Source/gpui-component`
   **или** git-pin + `[patch]` на наш `gpui` (как в recon).
2. `gpui_component::init(cx)` рядом с `chronos_ui::Theme::init`.
3. Адаптер dark Catppuccin chronos → `ThemeColor` (минимум ролей кнопки/
   фона/бордера/danger).
4. Task 9–11: Button / Slider / Progress / Label; MPRIS cover — Avatar
   или существующий image pipeline.
5. Smoke: layer-shell `side_panel_right` + клики power (без Exclusive
   keyboard — HANDOFF).
6. **Не** подключать story/webview/tree-sitter-languages без нужды.

Остаточный риск (severity, не compile): layer-shell + `Root`/`WindowExt`/
tooltip/focus_trap component'а рассчитаны на обычные окна; на overlay
панели возможны focus/hover сюрпризы — чинятся в smoke, не блокируют
«брать ли либу».

Binary/compile cost: +монолит ~89k LOC UI + тяжёлые default deps —
осознанная плата; для shell-капстоуна приемлемо, если не тащить story.

---

## Расхождения со спекой/планом

- Бриф ожидал «разрыв Zed-базы, вероятно большой / сотни ошибок» →
  **факт: 0 ошибок** на 0.5.2 vs Source@99cab5e. Вердикт «месяц» не
  подтвердился.
- `Source/Cargo.toml` exclude'ит gpui-component — верно, он **отдельный
  workspace**; recon собрал его снаружи через path на gpui, не вшивая
  в Source members.
- members урезаны (story out) — иначе `reqwest_client`/story тянут
  галерею; на ui-крейт не влияет. Это осознанное сужение scope разведки,
  не «полный monorepo check».

---

## Не реализовано из acceptance criteria

- Живой runtime / grim панели с Button — **не делалось** (разведка
  compile-only; headless/ recon mandate).
- Полный map 141 поля ThemeColor — только оценка объёма, не код адаптера.
- `cargo check -p story` / examples — не гонялись (вне минимального UI).
- Интеграция в ChronOS workspace — **не** (запрещено брифом).

---

## Проверено фактом, не на словах

```
$ cargo check -p gpui-component
# … Compiling gpui v0.2.2 (…/Source/gpui)
# … Compiling gpui-component v0.5.2 (…/scratch/…/crates/ui)
# Finished `dev` profile … in 40.66s
# exit 0

$ rg 'error\[|error:' /tmp/gpui-component-check.log
# (пусто)

$ cargo tree -p gpui-component -i gpui
# gpui v0.2.2 (…/Source/gpui)  ← единственный

$ rg 'RECON PATCH|path = "/home/neo' Source/gpui-component/Cargo.toml
# (пусто) — оригинал чист

$ find crates/ui/src -name '*.rs' | wc -l   # в recon
# 238
$ find … -name '*.rs' | xargs wc -l | tail -1
# 88897 total
```

---

## Новые риски / известные баги

| Severity | Риск |
|---|---|
| Med | Монолит + ropey/markdown/html/lsp всегда в линке — раздувает `chronos` binary; feature-strip нет |
| Med | Runtime на layer-shell не измерен (Root/tooltip/focus) |
| Low | Dual `gpui_util` (path vs zed-git via http_client) — pre-existing Source, `cargo tree -i` ambiguous |
| Low | Dual sum-tree names (`gpui_sum_tree` vs `zed-sum-tree`) — OK, но путает при debug |
| Low | Component `init` ставит **Light** по умолчанию (`Theme::change(Light,…)`) — для dark shell надо сразу Dark/change после init |
| Info | Icon pipeline = SVG assets crate; наши bar-icons (`include_bytes`) — другой мир; не смешивать без нужды |

---

## Статус ARCHITECTURE.md / DECISIONS.log

Не обновлялись (разведка; решение Архитектора после приёмки отчёта).
Кандидат в DECISIONS после принятия:

> `gpui-component` 0.5.2 check-clean vs Source/gpui; интеграция path-dep
> предпочтительнее ручных виджетов / ccf-style вендора; theme-адаптер
> Hsla→Hsla, dual Global OK.

---

## Ловушки и опровержения

1. **Думали:** unpinned zed HEAD у component vs pin `876ec5a8` = сотни
   ошибок. **Оказалось:** 0. API, которое зовёт 0.5.2, пересекается с
   нашим форком.
2. **Думали:** `reqwest_client` утащит dual-gpui. **Оказалось:** ui-крейт
   его не depends'ит; dual-gpui не возник.
3. **Думали (ccf-урок):** Global Theme = коллизия. **Оказалось:** разные
   типы → оба Global; боль = sync палитры, не type clash.
4. **Не путать** с ccf-recon: там вендор обязателен из-за 58 дельт; здесь
   **вендор не обязателен** для compile — достаточно path + patch gpui.

---

## Как разбил работу (батчи)

1. Копия + patch манифеста + `cargo check` + histogram.
2. `cargo tree` dual-gpui / util / reqwest; Theme vs chronos.
3. Graph Button/Slider, LOC, min set для панели, вердикт, отчёт.

Под-агентов не поднимал — check уложился в ~41s, параллель не нужна.
