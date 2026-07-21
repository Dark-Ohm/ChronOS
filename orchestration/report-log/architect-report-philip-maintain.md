# Session: Philip maintain — net_stats / правая панель docs — 2026-07-21

**Исход: DONE.** Living-доки приведены к коду после Task 1 (`dbce8ac`) +
смежных DONE/WIP капстоуна. Коммиты docs: `e485989` (+ follow-ups
`18057b2` / `0f1c830` если уже в дереве).

**Контекст сессии Архитектора (не миньон):** user → «выполни Task 1 за
Cline» → fable-judge → `/philip`. Код net_stats: `dbce8ac`. Отчёт Cline
по extract: `orchestration/report-log/cline-report-1.md` (уже архив).
**Этот файл** — отчёт по doc-maintain (раньше был только в чате).

## Сделано (факт, не намерение)

### Код (раньше в сессии, не этот проход Philip)

- `dbce8ac` — `crates/services/src/net_stats.rs` + `lib.rs` (`pub mod
  net_stats`) + rewire `bar/widgets/network.rs`.
- `8cc2fa5` — приёмка Task 1: HANDOFF, CLINE.md, `report-log/cline-report-1.md`.

### Docs (Philip maintain, `e485989`)

| Файл | Изменение |
|---|---|
| `docs/superpowers/plans/2026-07-20-right-side-panel.md` | Progress table; Task 1/7 `[x]` + DONE; Task 6 checkboxes **reopened** + NOT ACCEPTED; constraint path → `net_stats`; cargo errata (`chronos` bin, не `chronos-app`) |
| `ARCHITECTURE.md` §7 | pure non-`Service` modules; `net_stats` + pointer that design §3.3 is stale on data path |
| `skills/chronos-shell/SKILL.md` | table `net_stats`; render-immunity → `net_stats::update_speed`; `side_panel_right/` in app map; audio per-app mute marked WIP |

### fable-judge (чат, не файл)

Вердикт по `dbce8ac` / cline-report-1: **VERIFIED WITH CAVEATS** (live grim
не заявлен; plan rewrite vs byte-port тестов). Frauds: нет.

## Расхождения со спекой/планом

1. Design §3.3 всё ещё «данные из `bar/widgets/network.rs`» — **frozen**,
   не правили; истина в ARCHITECTURE §7.
2. Plan Progress Task 6: «code ready, uncommitted» vs checkboxes — сначала
   были ложные `[x]`; **сняты** (Philip).
3. Package name в plan Step 1/7 был `chronos-app` — помечен stale, команды
   заменены на `chronos --bin chronos`.

## Не реализовано из acceptance criteria

- User-facing README/CHANGELOG — не трогали (extract internal).
- Design rewrite — out of scope (frozen).
- Task 6 audio commit/приёмка — **не** docs-задача; WIP остаётся dirty.
- Live grim бара после extract — не в scope Philip.

## Проверено фактом, не на словах

```
# code still on HEAD
test -f crates/services/src/net_stats.rs
rg -n 'pub mod net_stats' crates/services/src/lib.rs   # :14

# plan checkboxes after maintain
# Task 1: 8× [x], 0 open
# Task 2: 7× [x] (already landed 18c88f0)
# Task 6: 0× [x], 10 open  ← false completion removed
# Task 7: 7× [x], DONE da744a2

# tests (re-run earlier in session / judge)
cargo test -p chronos-services --lib net_stats
# → 5 passed

cargo test -p chronos --bin chronos bar::widgets::network
# → 14 passed

# audio NOT on HEAD
git show HEAD:crates/services/src/audio/types.rs | rg AudioStream
# → empty (WIP only in working tree)

# docs commit
git show e485989 --stat
# ARCHITECTURE.md | plan | chronos-shell SKILL.md
```

## Новые риски / известные баги

- **Medium:** агент, читающий только design §3.3, снова скопирует sampling
  в панель. Mitigation: ARCHITECTURE + plan Progress + skill.
- **Medium:** Task 6 WIP + shared `lib.rs` — риск чужого hunk в чужой коммит
  (уже ловили AudioStream re-export при Task 1).
- **Low:** Progress table в plan снова устареет, если чекбоксы двигать без
  коммита.

## Статус ARCHITECTURE.md / DECISIONS.log

- **ARCHITECTURE.md** — обновлён §7 (`e485989`).
- **DECISIONS.log** — в этом проходе Philip не трогал; follow-up коммиты
  `18057b2`/`0f1c830` могут нести доп. docs (сверить `git log` при чтении).

## Ссылки

| Артефакт | Путь / SHA |
|---|---|
| net_stats code | `dbce8ac` |
| Cline extract report (архив) | `orchestration/report-log/cline-report-1.md` |
| Philip docs | `e485989` |
| Этот отчёт (active) | `orchestration/reports/architect-report-philip-maintain.md` |
