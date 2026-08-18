# ChronOS kitchen

Кухня **репозитория ChronOS**. Не копия экосистемной. Под git — контрибьюторы
берут задания отсюда, история не теряется.

- Эта: `ChronOS/.chronos-ops/`
- Экосистема (кросс-репо): `/home/neo/projects/chronos-ecosystem/.chronos-ops/`
  (вне git — там корень вообще не репо, другая нумерация, не путать)

Правила — `RULES.md`. Снимок очереди:

```
bash .chronos-ops/bin/kitchen-status.sh
```

## Что здесь живёт (с 2026-08-17)

- `active/` `done/` `reports-fresh/` `reports-log/` `rework/` `reject/` —
  очередь тикетов, по ролям.
- `checkpoint/` — канон, реальное содержимое (не указатели на `docs/`):
  `HANDOFF.md`, `ARCHITECTURE.md`, `ARCHITECT.md`, `TBD.md`, `SOUL.md`,
  `MEMORY.md`, `REJECTED.md` (бывший `docs/DECISIONS.log`, переименован —
  файл только про отклонённое, имя было двусмысленным).
- `design/` — макеты (`.dc.html` и т.п.), бывший `docs/design/`, перенесён
  as-is.
- `superpowers/` — planning-скиллы (`plans/`, `specs/`), бывший
  `docs/superpowers/`, перенесён as-is.

`docs/` остаётся продуктовой документацией и **публичным сайтом**, разложена
по папкам (`product/`, `style/`, `guides/`) — то, что контрибьютору нужно
читать как обычный проектный докс, не как рабочее состояние архитектора.

**Периметр сайта — отдельный, в кухню не переезжает никогда:**
`docs/index.html`, `docs/.nojekyll`, `docs/landing/` — исходник
`dark-ohm.github.io/ChronOS/` (GitHub Pages сконфигурирован на serving из
`docs/` буквально). `docs/landing/index.html` — не копия сайта, голый
13-строчный редирект-стаб на `../` для старых ссылок на путь `/landing/`
(до переезда контента в `docs/index.html` коммитом `29d70142`). Всё, что
связано с сайтом, идёт туда; то, что в `docs/` за пределами сайта —
редиректит на кухню/канон, не хранит рабочее состояние само.

`docs/hyprland/` больше нет: T300 (2026-08-18) слил его в
`packaging/hyprland/40-windowrules-chronos.lua` — единственный источник
истины по Hyprland-правилам ChronOS.

## Cutover из `docs/orchestration/` — ЗАВЕРШЁН 2026-08-18

`docs/orchestration/` **закрыт и пуст**. Всё, что там жило, переехало сюда:

| Было | Стало |
|---|---|
| `tasks/active/TNNN.md` | `active/<role>/TNNN.md` |
| `tasks/active/pause/` | `active/hold/` |
| `tasks/report/` (инбокс) | `reports-fresh/` |
| `tasks/report-log/` | `reports-log/<role>/` |
| `tasks/done/` | `done/<role>/` |
| `tasks/rejected/` | `reject/<role>/` |
| `tasks/notes/` | `dump/notes/` (заметки, аудиты, кадры-улики) |
| `tasks/MIGRATION.md` | `MIGRATION.md` (в корне кухни) |
| `agents/<ROLE>.md` | `active/<role>/<ROLE>.md`, старые копии — `dump/legacy-agents/` |

Последними уехали живые тикеты до-cutover эпохи: T266 и T271 — приняты и
в `done/`, T285 — STOP (тупик публичного API ACP) в `reject/back/`,
T284 — открыт, в `active/front/`, его отчёт ждёт приёмки в
`reports-fresh/`. Там же в инбоксе — T281 (PARK, не архивировать до `+`
владельца) и T299 (разметка ролей архива).

167 архивных тикетов, которые эвристика не разложила по ролям, разбирает
**T299** (RECON) — отчёт лежит в `reports-fresh/`, приёмки ещё не было.

## Известный долг

Кросс-ссылки на старые пути поправлены в живых файлах (`CLAUDE.md`,
`README.md`, `CONTRIBUTING.md`, `AGENTS.md`, `checkpoint/ARCHITECT.md`,
`checkpoint/TBD.md`, `packaging/hyprland/README.md`). **Не** правились
ссылки внутри архивных тикетов `done/`, `reports-log/`, `reject/` и `skills/`
— исторический слепок, тот же принцип что с `checkpoint/SOUL.md` ("не
регламент, не переписывается задним числом").
