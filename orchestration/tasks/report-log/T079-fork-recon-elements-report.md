<!-- T079 — migrated 2026-07-22 from orchestration/report-log/grok-report-17.md — see orchestration/tasks/MIGRATION.md -->

# Session: Grok №17 — разведка форка (elements/styling/layout/scroll) — 2026-07-20

_Обновлено после post-submit: fable-judge, fable-method audit, Philip docs audit._

## Сделано (факт, не намерение)

- `skills/chronos-gpui/references/elements-styling-layout.md` (396 строк) — production-скилл: трейты, скролл, Style/`max_h`, lists, text, examples, «Ловушки и опровержения» с file:line.
- `skills/chronos-gpui/evals/elements-styling-layout.eval.md` (67 строк) — 8 Q/A с проверяемыми proofs.
- `SKILL.md` **не тронут** (зона Архитектора / общая волна).
- `Source/` **не изменён** (read-only + `cargo check --example` only).
- Коммит ChronOS: **`f4d2ebc`** (`f4d2ebc220a69b749f1fa788f90184bb31f4fc5e`) — ровно 2 файла, +463 строки, без AI-trailer.

## Расхождения со спекой/планом

- Батчи: 3 параллельных explore-агента (A traits+scroll, B style/layout, C lists+text) + сверка main-thread. Спека просила ≥3 партии — выполнено.
- Живой `cargo run --example` под Wayland **не** гонялся — только `cargo check` (зона recon; runtime UX не обязателен, но честно не «доказан»).
- Скилл `gpui-layer-shell` **не** правился (вне зоны №17) — долг подтверждён Philip-аудитом как High (см. ниже).
- Step 1 fable-method (явное «done = …» пользователю в начале) **не** озвучен user-facing — process soft-gap (audit); deliverable от этого не пострадал.

## Не реализовано из acceptance criteria

- Обязательное из брифа №17: **закрыто** (2 skill-файла, цитаты, traps, cargo check ключевых examples, отчёт, коммит в ChronOS).
- Не делалось (осознанно / вне зоны):
  - полный `cargo run` всех examples зоны под Wayland;
  - правка `gpui-layer-shell` / `chronos-gpui/SKILL.md` nav (другие владельцы / follow-up);
  - `ARCHITECTURE.md` / `DECISIONS.log` (recon-карта, не архитектурный контракт ChronOS).

## Проверено фактом, не на словах

### Батчи (работа)

| Партия | Зона | Метод |
|---|---|---|
| A | traits + scroll + ScrollHandle | subagent explore + re-read `div.rs` 699+, 1213+, 3752+, 3880–4100 |
| B | Style / macros / taffy / units | subagent + `style.rs` 180+, `styled.rs`, `gpui_macros/styles.rs` 880+, `taffy.rs` 488+ |
| C | list / uniform_list / text / examples | subagent + constructors + example headers |

### `cargo check` (сессия + re-run post-judge, из `Source/`)

```text
PKG=path+file:///home/neo/projects/chronos-ecosystem/Source/gpui#0.2.2
cargo check --example scrollable    -p $PKG  → Finished ok
cargo check --example uniform_list  -p $PKG  → Finished ok
cargo check --example list_example  -p $PKG  → Finished ok
cargo check --example text_wrapper  -p $PKG  → Finished ok
cargo check --example grid_layout   -p $PKG  → Finished ok
```

(предупреждения `gpui_linux` nightly_coverage / future-incompat — pre-existing)

### Коммит (git ground truth)

```text
f4d2ebc skills : chronos-gpui — элементы, стили, лэйаут, скролл (разведка форка)
  skills/chronos-gpui/evals/elements-styling-layout.eval.md    |  67 +
  skills/chronos-gpui/references/elements-styling-layout.md    | 396 +
  2 files changed, 463 insertions(+)
```

`git show --name-only`: только эти два пути. Нет `SKILL.md`, Source/, examples-catalog.

### Source git (не наш мусор; snapshot update)

На момент обновления отчёта untracked в Source (чужое, **не** трогали):

```text
?? .mimocode/
?? REPORT.md
?? REVIEW.md
?? brief.md
?? findings/
?? plan.json
?? reflect.json
```

### Ключевые находки «думали X — оказалось Y»

1. **Скролл есть** — `overflow_y_scroll` на `StatefulInteractiveElement`; ключ `.id()` → `Stateful<Div>` (`div.rs:710`, `:1429`, `:3752`). Пример: `examples/scrollable.rs`.
2. **max height есть** — нет поля `max_height`; есть `max_size.height` + macro `.max_h` (`style.rs:234`, `styles.rs:899-903`, `taffy.rs:496`).
3. **`Styled` overflow ≠ scroll** — macro только `*_hidden` (`styles.rs:135-151`).
4. **`ScrollHandle::scroll_to_bottom`** — one-shot flag (`mem::take` `:2251`), не follow-tail; tail — `ListState::FollowMode::Tail` (`list.rs:113-119`, `set_follow_mode` ~617).
5. **Виртуализация готова** — `uniform_list` / `list` вместо «+N more»; `data_table` — 10k rows (`TOTAL_ITEMS = 10000`).
6. **gpui-layer-shell** всё ещё учит «NO max_height» / «overflow_y_scroll does not resolve» — **High stale** (Philip); канон зоны — `chronos-gpui/references/elements-styling-layout.md`.

### Post-submit: fable-judge

| | |
|---|---|
| **Вердикт** | **VERIFIED** |
| Scope | ровно 2 файла в `f4d2ebc` |
| Checks re-run | 5/5 examples OK |
| Citations spot-check | load-bearing lines совпали с Source |
| Frauds | none (no weakened tests, no push, no scope creep in commit) |
| UNVERIFIABLE | полный runtime examples; «сколько файлов прочитал» как число |

### Post-submit: fable-method audit

| Step | Статус |
|---|---|
| Triviality / fit / classify / evidence / act / verify / report | followed |
| Step 1 user-facing «done =» | **soft skip** (риск FM4 снят фактом коммита + judge) |
| FM14 verification theater | **не** сработал |
| Highest-value process fix | на старте следующей сессии одна фраза Step 1 |

### Post-submit: Philip (docs audit)

| Severity | Finding |
|---|---|
| Critical | **нет** в deliverable Grok №17 (phantom APIs / fake helpers не найдены) |
| High | `gpui-layer-shell` stale scroll + max_height claims + self-contradiction |
| High | `chronos-gpui/SKILL.md` ссылается на **MISSING** `state-async-executors.md` и `examples-index.md` (скелет волны; не зона Grok) |
| Medium | SKILL.md «60 examples» ≈ 40 gpui + 14 component, не 60 |
| Low | `~` line ranges могут дрейфовать; skill честен про check-only |

**Philip: Grok files — high quality; do not rewrite for accuracy.** Follow-up = layer-shell + SKILL nav (не №17).

## Новые риски / известные баги

- **High (вне зоны, follow-up Архитектору):** `gpui-layer-shell` переучит агентов старым «нет скролла / нет max_h» — патч: «нужен `.id()`»; «`.max_h` для element cap, `window.resize` для layer-shell surface»; ссылка на `elements-styling-layout.md`.
- **High (скелет волны):** битые nav-ссылки в `chronos-gpui/SKILL.md` пока Mimo/OpenCode не сдали файлы.
- **Low:** untracked шум в Source (brief/findings/REPORT…) — чужой.
- **Info:** plain-div autoscroll = повторный `scroll_to_bottom`; для terminal log предпочтительнее `list` + `FollowMode::Tail`.

## Статус ARCHITECTURE.md / DECISIONS.log

- Не обновлялись. Опровержение «скролла нет» уже в каноне (`e71f9aa`); №17 — детальная карта зоны elements/style/layout, не новый архитектурный decision ChronOS-кода.

## Коммит

- **ChronOS:** `f4d2ebc` — `skills : chronos-gpui — элементы, стили, лэйаут, скролл (разведка форка)`
- **Source:** без коммитов.
- Отчёт: `orchestration/reports/grok-report-17.md` (`orchestration/` в `.gitignore` — active report, не в git).
- Чужой untracked в ChronOS (`examples-catalog.md` и др.) **не** стейджил.

## Как разбил работу (требование пользователя)

1. Партия A — hierarchy + scroll  
2. Партия B — Style / macros / layout / units  
3. Партия C — lists / text / examples  

Параллельные subagents → синтез main-thread → `cargo check` examples → write skill+eval → commit → report.  
После сдачи: fable-judge **VERIFIED** → fable-method audit (loop ok, soft Step 1) → Philip audit (deliverable clean; layer-shell/SKILL nav — долг).

## Рекомендация приёмке

**Принять Grok №17** по `f4d2ebc`. Отдельные тикеты (не блокер №17): (1) правка `gpui-layer-shell`, (2) nav `chronos-gpui/SKILL.md` после сдачи остальных зон волны.
