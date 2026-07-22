# Session: philip maintain — капстоун правой панели (docs) — 2026-07-21

## Сделано (факт, не намерение)

Режим Philip **maintain** (не audit/write с нуля). Цель: сверить docs с
кодом после Tasks 1/2/6/7, вырезать stale/overclaim.

### Коммиты (только docs)

| Hash | Сообщение |
|---|---|
| `e485989` | (уже был в дереве) philip maintain — net_stats boundary + plan progress (Tasks 1/2/7) |
| `18057b2` | philip maintain — Task 7 registration errata + uncommitted audio not shipped |
| `0f1c830` | philip maintain — Task 6 stream-mute WIP status (not shipped) + DECISIONS |

### Файлы, изменённые в `18057b2` + `0f1c830`

- `ARCHITECTURE.md` — audio stream mute **не** как shipped API; в §4.1
  назван `side_panel_right` skeleton (`da744a2`).
- `docs/superpowers/plans/2026-07-20-right-side-panel.md` — Task 7:
  errata registration (`main.rs`, не `lib.rs`), package `chronos`, smoke
  evidence, strikethrough stale `chronos-app` / `lib.rs` steps.
- `HANDOFF.md` — Task 6: код готов, **коммита нет**; Task 7 accepted;
  working-tree note про named add.
- `docs/superpowers/specs/2026-07-20-right-side-panel-design.md` — §3.1/§4/§8
  Task 6 symbols + uncommitted disclaimer.
- `DECISIONS.log` — 2026-07-21: stream mute остаётся на `wpctl`+`pw-dump`.

**Код не трогал.** `crates/services/src/audio/**` остаётся uncommitted WIP.

## Расхождения со спекой/планом

- План Task 7 изначально требовал `lib.rs` / `pub mod` — **ложь для
  этого дерева**: app = binary, модули в `main.rs`. Errata в плане;
  landed `da744a2` без `lib.rs`.
- ARCHITECTURE после `e485989` описывал stream mute как факт 2026-07-21 —
  **overclaim**: symbols есть только в working tree. Откатил формулировку
  до «WIP until accept+commit».
- Design open Q4: «формат pw-dump закрыт» + «git commit ещё нет» —
  оставлено: закрыт research/schema, не merge.

## Не реализовано из acceptance criteria

- Полный rewrite исторических шагов Tasks 3–5/8–12 в плане (открыты —
  это не stale).
- Не гонял `philip diff` JSON artifact store (diff-scoped manual maintain).
- Не писал user-facing README (нет user surface для панели без Task 9).

## Проверено фактом, не на словах

```
# Landed Task 7
git show da744a2 --name-only
# → main.rs, side_panel_right/{mod,view}.rs only

# Registration truth
rg -n 'mod side_panel|side_panel_right::init' crates/app/src/main.rs
# → mod + init present
ls crates/app/src/lib.rs && rg 'side_panel' crates/app/src/lib.rs || true
# → lib.rs has only monitor/notifications/state; no side_panel

# Task 6 not on master
git status --short crates/services/src/audio/
# → M mod.rs pw_dump.rs types.rs (still dirty after docs commits)

rg -n 'toggle_stream_mute_for_player|ToggleStreamMute|parse_pw_dump_streams' \
  crates/services/src/audio/
# → symbols exist in WIP (mod.rs / pw_dump.rs / types.rs)

rg -n 'AudioStream' crates/services/src/lib.rs || echo 'no crate-root re-export'
# → no AudioStream in lib.rs

# Doc commits
git log -3 --oneline
# → 0f1c830, 18057b2, e485989 (docs only)
```

## Новые риски / известные баги

- **Medium:** DECISIONS.log фиксирует backend-решение Task 6; читатель
  может принять «решено» за «в master». Смягчено HANDOFF + ARCHITECTURE
  («not on master until committed»).
- **Low:** в plan body остались старые package names внутри closed Task 1
  steps; сверху errata/progress table — достаточно для агентов.
- **Process:** `orchestration/` gitignored — этот отчёт на диске, не в git.

## Статус ARCHITECTURE.md / DECISIONS.log

- **ARCHITECTURE.md** — обновлён (`18057b2`): audio honesty + side_panel
  in §4.1 list.
- **DECISIONS.log** — добавлена запись 2026-07-21 stream mute (`0f1c830`).
- **HANDOFF.md** / design / plan — синхронизированы с evidence.

## Итог одной строкой

Philip maintain: docs больше не врут, что Task 6 в master и что панель
регистрируется в `lib.rs`; код audio по-прежнему только WIP.
