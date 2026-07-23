<!-- T095 — migrated 2026-07-22 from orchestration/report-log/hermes-report-20.md — see orchestration/tasks/MIGRATION.md -->

# Session: Hermes №20 — Tasks 8+9 hover-peek + MPRIS card — 2026-07-21

**Агент:** Hermes (исполнение Lead Architect / Grok).  
**Решение стека:** C — div + `gpui-animation` (state-driven), **без** `gpui-component`.  
**Коммит:** **не делался** (бриф: приёмка/коммит Архитектор).  
**Cargo.toml:** **не трогался** (депы `gpui-rsx`/`gpui-animation` уже `962adec`).

---

## Исход (первая строка)

**T8+T9 в дереве, unit 5/5, release green, live smoke частичный.**  
Hover-strip на **pult DP-1** (2556×4), pin-open + MPRIS-карточка на grim  
(`vivaldi.instance…`, cyan swatch, transport, mute). Peek open/close по log  
раньше подтверждён; ydotool multi-head по-прежнему врёт — hover round-trip  
не автокликнут. **Один `on_hover` на узел** соблюдён; анимация —  
`transition_when` на **inner** body, debounce `on_hover` на **outer** root.

---

## Сделано (факт)

| Файл | Что |
|---|---|
| `hover_strip.rs` | 4px layer-shell strip, один `on_hover` → open_peek / schedule_release; deferred init + `display_id` log |
| `mod.rs` | `pinned` + `peek_generation`; `open_peek`/`open_pinned`/`hold_peek`/`schedule_release_peek`/`close_peek_if_not_pinned`; deferred strip (50ms) чтобы pult = как у bar; smoke env `CHRONOS_SMOKE_SIDE_PANEL` → pin |
| `mpris_card.rs` | карточка: swatch `0x5fd3e8`, title/artist, prev/play/next + mute → `toggle_stream_mute_for_player`; `display_title` pure + 3 tests |
| `view.rs` | MPRIS `state::watch`; **outer** root = debounce `on_hover` only; **inner** body = `.with_transition` + `.transition_when(revealed)` fade-in (**не** `transition_on_hover`) |

### Кровный факт `on_hover` (форк)

```
outer #side-panel-right-root  →  on_hover(hold / schedule_release)   // единственный
inner  #side-panel-body       →  with_transition + transition_when   // AnimatedWrapper
                                                                   // сам ставит on_hover
                                                                   // на inner — ок, другой узел
strip  #side-panel-hover-strip → on_hover(open_peek / schedule)     // другое окно
```

`transition_on_hover` на root **не** использовался — иначе assert с debounce.

### rsx / animation

| | |
|---|---|
| `gpui-rsx` | **не** использовал — div-чейн из плана читаемее для card |
| `gpui-animation` | **да**: `TransitionExt` + `Linear` + `transition_when` fade 180ms на inner body |

---

## Проверено фактом

```
cargo test -p chronos --bin chronos side_panel
# 5 passed (2 peek/pin + 3 display_title)

cargo build --release -p chronos
# Finished release in ~2m 31s, EXIT 0

# Live (master binary, not pilot):
readlink /proc/$(pgrep -x chronos)/exe
# …/ChronOS/target/release/chronos

# Strip + panel same pult (after deferred init fix):
# side_panel_hover_strip  xywh: 2556 15/30 4 1440/1410  namespace: side_panel_hover_strip
# side_panel_right        xywh: 2260 15    300 1440      namespace: side_panel_right
# display_id=Some(DisplayId(5))  — same as bar log

# CHRONOS_SMOKE_SIDE_PANEL=1 → opened (pinned)
# grim: orchestration/reports/hermes-20-smoke/panel-pinned.png
#   cyan 64px swatch + "vivaldi.instance…" + <  >  ||  M

# Earlier session log (before strip-display fix): open peek → close on leave debounce
# side_panel_right: opened (peek)
# side_panel_right: closed   (~300ms later, cursor not held on panel)

# no panic / no debug_assert on_hover in log
rg "todo!|let _ =" crates/app/src/side_panel_right/   # only doc comment about let _
```

**Не дожато автоматом (честно):**

| Пункт | Статус |
|---|---|
| Hover strip → peek open (log) | ✅ было; strip теперь на DP-1 |
| Leave → close (log) | ✅ было (~280ms debounce) |
| grim peek open (hover path) | ⚠️ ydotool coords broken dual-head; grim **pinned** smoke env |
| play/pause live pause player | ⚠️ не кликнуто (ydotool); dispatch в коде |
| mute → wpctl flip | ⚠️ не кликнуто; log path `mute toggle for player_id=` при клике |
| `gpui-component` | не в графе (C) |

---

## Расхождения с планом

1. **Дебаунс generation-based** (strip leave + panel leave), не «голый leave → close» — иначе strip leave закрывал бы peek до входа на панель; pin-upgrade peek→pin в `open_window`.
2. **Deferred strip init 50ms** — без этого strip садился на HDMI-A-1 (h=1200), panel/bar на DP-1; после defer strip `2556` на pult.
3. **Smoke env** `CHRONOS_SMOKE_SIDE_PANEL` — pin-open для grim (как Task 7 temp hook). Архитектор может вырезать при коммите.
4. **Animation** — fade opacity via `transition_when`, не slide translate (API margin/opacity; translate не в таблице свойств README).
5. **Package name** в плане `chronos-app` — реально `chronos` / bin tests.

---

## Не реализовано из acceptance

- Полный live click play/pause + mute + wpctl (нужен ручной клик или стабильный input).
- Hover grim round-trip screenshots (есть pin grim + log peek).
- Bar/IPC toggle wiring (не в scope T8/9 plan — hotkey later).
- `gpui-rsx` markup (сознательно div).

---

## Новые риски

| Sev | |
|---|---|
| Med | ydotool unusable for dual-head acceptance — plan on human click / ARCH smoke |
| Low | Smoke env left in `init` — strip before product commit if Architect wants pure |
| Low | Fade-only reveal; if layer-shell + AnimatedWrapper misbehaves, drop `with_transition` and keep static (API ready) |
| Info | Empty title shows `player_id` (vivaldi.instance…) — correct fallback when no track metadata |

---

## Статус ARCHITECTURE / DECISIONS

Не обновлялись. Кандидат после приёмки: «panel body div+animation C; one on_hover/node; strip deferred for pult display».

---

## Артефакты

- Код: `crates/app/src/side_panel_right/{mod,view,hover_strip,mpris_card}.rs`
- Grim: `orchestration/reports/hermes-20-smoke/panel-pinned.png`
- Logs: `/tmp/t20-test2.log`, `/tmp/t20-rel3.log`, `/tmp/t20-smoke2.log`
- Binary: `target/release/chronos` (master tree, not pilot worktree)
