# T287 — левый Chat на gpui-component (эпик)

**Статус: ЭПИК ЗАКРЫТ (2026-08-17).** Все волны приняты, коммиты на
master (проверено `git merge-base --is-ancestor`):

- **T286** — композер → kit `Input` multi-line — `c5151fb9`, принят live
  смок Ctrl+Enter+wrap (`d5e93c0f`).
- **T287-A** — model/mode пикеры → kit `Select` — `25ac46a8`;
  попап-клиппинг вынесен в T298 (принят частично, остаток — T301/T302).
- **T287-B** — вкладка Sessions → kit `List` — `8e84d3fb`, принят live
  grim pin/archive/rename/delete.
- **T287-C** — Zed-шапка + внутренний Sessions rail срезаны, Follow в
  `composer-pickers-row` — `220a05e2`.

Брифы волн: `.chronos-ops/done/front/T286-…`, `T287-A-…`, `T287-B-…`,
`T287-C-…`. Открытые хвосты — отдельные тикеты в
`.chronos-ops/active/front/`: **T302** (P1, контентная зона левой панели
пустая) и **T301** (P3, эллипсис в Select-попапе). Очередь по
FRONTEND.md — T302 → T301.

---

**Приоритет:** P1 — самописный композер/пикеры ломают ввод.
**Роль:** FRONTEND. Волны — отдельные тикеты, не один заход.
**Канон:** DECISIONS 2026-08-15; кит `../Source/gpui-component/crates/ui`.

## Уже на ките (не трогать)

| Что | Где |
|---|---|
| `Root` | обе панели |
| `Input` | лаунчер T275, Preview editor |
| `PopupMenu` | dock / tray / launcher pin |
| `text::markdown` | Preview |

## Chat-вкладка сейчас

| Кусок | Сейчас | Кит | Волна |
|---|---|---|---|
| Поле ввода | `text_input.rs` `shape_line`, одна линия | `Input` multi-line | **T286** (уже в active) |
| Model picker | самописный dropdown + `String` поиск | `Select` / `ComboBox` | T287-A |
| Mode picker | то же | `Select` | T287-A |
| Поиск сессий | `thread_search: String` | `Input` | T287-B |
| Список сессий | свои `div` | `List` / `VirtualList` | T287-B |
| Внутренний Sessions rail + `＋☰👁⋯` | Zed-хром, `☰`/`⋯` без click | выкинуть | **T287-C** |
| Лента / reasoning / tool cards | свои | нет аналога | **не переносить** |
| Рабочая рельса вкладок (40 px) | своя | нет | не трогать |

Не тащить `dock`/`table`/`tree` в Chat. Не включать `lsp`/`chart`.

## Порядок

1. **T285** — ACP `load_session`. Не с T286 (`chat.rs`).
2. **T286** — композер. Боль пользователя, `text_input.rs` уходит.
3. **T287-C** — срезать Zed-шапку, внутренний Sessions rail, Follow в
   `composer-pickers-row`. **После T286 в git.** Не параллелить: Follow
   садится в тот же `composer.rs`, который T286 переписывает.
4. **T287-A** — пикеры model/mode на `Select`. После T286.
5. **T287-B** — список на вкладке Sessions (не внутри Chat). После A.

Каждая волна — свой бриф, свой коммит, live grim. Эпик целиком не отдавать.

## Модель

300B на волну. Не 30B (снова нарисует dropdown). Не 1T на весь эпик.
