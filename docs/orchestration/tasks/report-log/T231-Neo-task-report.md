# T231-Neo — сводный отчёт: редизайн Bar-вкладки + паттерн на соседние вкладки + инфраструктура proof-ссылок скиллов

**Дата:** 2026-08-04..05. **Роль:** FRONTEND + skills/CI.
**Статус:** выполнено (11 коммитов, `f5b69c5` → `424b45a`).
Это сводный отчёт всей серии работ, начатой тикетом T231; по каждой части
есть свой отчёт (`T231-bar-settings-tab-visual-redesign-report.md`,
`T231-pattern-spread-report.md`) — здесь всё вместе с привязкой к коммитам.

---

## 1. UI — редизайн Bar-вкладки (T231, коммит `f5b69c5`)

Тикет: `docs/orchestration/tasks/active/T231-bar-settings-tab-visual-redesign.md`.
Вердикт архитектора: «Не eye-appealing. Debug-меню, не продукт». В рабочем
дереве при старте лежал **несобранный partial-rewrite** (сломанный
`impl DragMoveEvent`, `new` возвращал `Entity<Self>` из `-> Self`, persist
в обход `apply_patch`) — правка сделана **начисто поверх HEAD-логики**,
поведение не менялось.

Что сделано (все 5 пунктов тикета):

| § | Проблема | Решение |
|---|---|---|
| 1 | Одна колонка на 960px, пустота | Appearance-блок (7 строк) — CSS-подобный grid: **2 колонки от `GRID_BREAKPOINT`**, 1 на дефолтной ширине. Hypr modules — grid 3/2/1 колонки по ширине. Grid форка подтверждён `Source/gpui/src/styled.rs` + `examples/grid_layout.rs` |
| 2 | Нет иерархии | `section_header()` (акцентный тик + semibold + mono-подпись), `setting_label()` (лейбл + mono-путь), секции разнесены `gap(16px)`. Пути `appearance.*` оставлены — System settings это техническая поверхность |
| 3 | Плоские контролы | `-`/`+` — bordered-кнопки 24×24 с hover; трек слайдера 6px (было 4px), thumb 16px с border+drop-shadow; сегменты/onoff — единый accent-язык |
| 4 | Стена строк modules | `module_card()` — компактная карточка (имя mono, путь muted, «Open ▸»), grid |
| 5 | Панель сливается с обоями | Всё содержимое на `theme.bg.elevated`-карточке с `elevation_popup()`-тенями через `elevation_apply_light_chrome` (язык глубины `side_panel_left/panel.rs`) |

**Канон соблюдён:** только токены темы, `font_mono` для путей, persist-логика
(`apply_patch`, preset-ids, drag-математика, `PreviewTarget`) не тронута.

**Верификация:** `cargo build --release -p chronos` чисто; тесты зелёные
(+2 новых: `breakpoint_keeps_default_width_single_column`,
`slider_frac_clamps_and_handles_zero_width`); **живой grim 4/4 кадра**
(320/960 × обе темы, `docs/orchestration/tasks/notes/T231-*.png`, 960px
получен живым drag-ресайзом через `/dev/uinput`); live-клик `+` на Height —
`bar.toml` пишется (35.6 → 38.4).

## 2. UI — паттерн T231 на соседние вкладки (коммит `fc9daa9`)

Followup пользователя: «распространить паттерн». Общие примитивы вынесены в
новый модуль `crates/app/src/side_panel_right/tab/ui.rs`:

- `elevated_card(theme) -> Div` (голый `Div` — `.id()` после вызова даёт `Stateful`),
- `section_header` / `setting_label` / `setting_row`,
- `GRID_BREAKPOINT = 720` / `is_wide(window)` (тест переехал в `ui.rs`).

Приведены к паттерну (логика не тронута): `files.rs` (header + карточка,
список сознательно 1-колоночный), `hypr_binds.rs` (группы через
`section_header`, бинды grid 2/1), `acp_settings.rs` (секции agents/actions,
агенты grid 2/1). `bar_settings.rs` отрефакторен на `ui::*`-хелперы.

**Верификация:** build чисто, **165 passed / 0 failed**; live: `select-tab:files`
→ lazy-create → `switched tab → width=440.0` без `zero size` и падений.
Пользователь подтвердил: «шелл не падает».

**Диагностика ложной тревоги «падающего шелла»:** процессы умирали из-за
моих же `pkill -x chronos` в рестарт-командах (а `$!` после `nohup &` — PID
обёртки, не хроноса). Вывод: трекать через `pgrep -x chronos`, не pkill'ить
живой шелл.

## 3. Skills-обогащение (коммиты `604a2ec`, `135d2cb`)

- `chronos-shell` (`604a2ec`): уроки T231 — правда про grid форка, паттерн
  `tab/ui.rs`, рецепт uinput live-ввода, урок «самонанесённый краш от pkill»
  (в `live-smoke-wayland.md`, `verification-before-completion`).
- `chronos-gpui` (`135d2cb`): fast-answer про grid с точными строками
  `styled.rs` (52/752/780/789/803/817/831, `style.rs:302`), про `.id()` после
  `elevation_apply_light_chrome` (`elevation.rs:172`, эталон `tab/ui.rs`);
  **2 новых eval-вопроса Q9/Q10**; механический прогон proofs — **67/67**.

## 4. Инфраструктура: проверка proof-ссылок скиллов (коммиты `bba9a7a`…`424b45a`)

Эволюция от локального скрипта до CI-гейта:

1. **`bba9a7a`** — `skills/chronos-gpui/evals/check-proofs.sh`: воспроизводимый
   валидатор `file:line`-ссылок в eval-файлах (67/67, exit 0).
2. **`3ad7086`** — перенесён в корень **`skills/check-proofs.sh`**: обход всех
   `SKILL.md` + `references/*.md` + `*.eval.md` (141 файл), резолв путей
   расширен, path-like-фильтр (CSS-ложняк `flex:1` больше не матчится).
3. **`4f9ad94`** — починены устаревшие ссылки в исторических reference:
   live-proof строки (`volume_popup/view.rs:199→531`, `launcher/view.rs:160→161`,
   `desktop_terminal/view.rs:343→204`), дизамбигуация донорских путей
   (`reference/gpui-shell-main/…`), якоря executor'а (оказались в `gpui_scheduler`,
   `spawn`=189, `timer`=248, а не 89/162).
4. **`8e5aad5`** — триаж оставшихся 32: **6 реально устаревших починены**
   (kael-main донор `build.rs`, переехавший форк-файл → `text_system/line_layout.rs:280`,
   gpui-component worktree `Source-wt-component/…`), **26 внешних по дизайну**
   (Zed upstream, Hermes checkout, philip, fable-примеры, плейсхолдеры) —
   EXTERNAL-allowlist, не валят exit. Итог: **265 valid, BROKEN: 0**.
5. **`a14d232`** — **CI-гейт**: job `skill-proofs` в `.github/workflows/ci.yml` +
   **pre-commit хук** `scripts/git-hooks/pre-commit` (активирован `git config
   core.hooksPath scripts/git-hooks`, проверяет staged `SKILL.md`/`*.eval.md`/
   `references/*.md`); деградация `fork-missing` (в CI форка `../Source` нет);
   попутно уточнён якорь `Source/Cargo.toml:126` (дрейф 4 строки).
6. **`fac183f`** — документация в **`CONTRIBUTING.md`**: секция Skills (гейт,
   активация/отключение хука, семантика EXTERNAL-allowlist).
7. **`424b45a`** — **CI клонирует форк ChronOS-GPUI** (`github.com/Dark-Ohm/ChronOS-GPUI`,
   HEAD `57f582f`) в `../Source` для полной строгости fork-ссылок; обобщённая
   деградация **`env-missing`** для gitignored-корней (`reference/` — донорские
   снапшоты, `Source-wt-component/` — worktree), которые физически отсутствуют
   в свежем клоне.

**Ключевой инсайт симуляции:** свежий клон мастера + свежий форк дал бы 8
ложных BROKEN — это были не битые ссылки, а gitignored-корни, которых нет в
клоне; без деградации job был бы красным с первой минуты.

## Верификация итоговая

| Режим | Проверено | EXT | BROKEN |
|---|---|---|---|
| Локальный полный (dev, fork + reference/ + worktree) | 265 | 26 by-design | **0** |
| CI-макет (свежий клон master + свежий форк) | 257 (fork строго) | 26 + 8 env-missing | **0** |
| CI без форка (fallback) | 24 repo-local | 267 | **0** |
| Pre-commit хук | broken→1, clean→0, skip | — | — |

Плюс: `cargo build --release -p chronos` чисто, `cargo test --release -p chronos
--lib -- side_panel_right` — 165 passed / 0 failed, live-шелл стабилен.

## Коммиты серии

```
f5b69c5  ui : bar settings tab responsive grid + visual hierarchy (T231)
fc9daa9  ui : spread T231 tab pattern (elevated card, section headers, responsive grid)
         to Files/Hyprland binds/ACP settings + shared tab/ui.rs + skill notes
604a2ec  skills : enrich with T231 lessons — fork grid truth, tab/ui.rs pattern,
         uinput live-input recipe, self-inflicted crash scare
135d2cb  skills(chronos-gpui) : add grid + elevation/id() eval items Q9-Q10,
         pin exact styled.rs lines; eval proof check 67/67
bba9a7a  skills(chronos-gpui) : reproducible eval proof checker evals/check-proofs.sh (67/67 green)
3ad7086  skills : check-proofs.sh repo-wide — all SKILL.md + references + evals (254 ok, …)
4f9ad94  skills : fix stale proof refs in historical references — live-proof lines,
         donor launcher/mod.rs disambiguation, BackgroundExecutor anchors
8e5aad5  skills : triage remaining broken proofs — fix 6 stale refs, external-by-design
         allowlist (26 EXT, exit 0)
a14d232  ci : gate broken skill proof links — skill-proofs job + pre-commit hook
         (core.hooksPath), fork-missing degradation, donor Cargo.toml:126 pin
fac183f  docs : CONTRIBUTING — skills proof-link gate: check-proofs.sh + pre-commit
         hook activation and CI semantics
424b45a  ci : skill-proofs clones ChronOS-GPUI fork into ../Source for strict
         fork-proof checks; env-missing degradation (fresh clone + fresh fork = 0 broken)
```

## Открытые хвосты

- Полные grim-кадры трёх распространённых вкладок (320/960, обе темы) — не
  сняты (живая сессия пользователя); «после» подтверждено визуально.
- Полный click-through всех контролов — при появлении надёжного
  синтетического ввода (uinput ABS-маппинг плывёт на 4480px-десктопе).
- Дальнейший спред паттерна на остальные вкладки (System/Library/Settings) —
  как и планировалось в T231, отдельной задачей.
