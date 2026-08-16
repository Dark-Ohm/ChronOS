# Spec: Live smoke — residual tails (customization + Follow + settings)

**Дата:** 2026-08-03. **Статус:** approved for execution (architect).  
**Задача:** `docs/orchestration/tasks/active/T209-live-smoke-residuals.md`.  
**Бинарник:** `target/release/chronos` @ `9435cc0`+ (rebuild if HEAD moved).  
**Канон:** unit green ≠ UX done (`docs/HANDOFF.md`); grim + log + eyes.

---

## 1. Зачем

Волна **T198–T208** + **T195** + **T196** принята по дереву/unit.  
Все `LIVE NOT VERIFIED`. Этот документ — **единый чеклист живого смока**  
хвостов, без фабрикаций и без «должно работать».

**Выход:** отчёт `docs/orchestration/tasks/report/T209-live-smoke-residuals-report.md`  
с PASS/FAIL/SKIP по каждой строке матрицы + пути к grim + цитаты лога.

---

## 2. Non-goals

- Не переоткрывать T181 edge-states (парк в TBD).
- Не смокать scenes/gamer slice (T191 park) в этом прогоне.
- Не fork `set_anchor` / inline ACP CRUD — это residual product, не блокеры смока.
- Не ydotool-оркестр «всё само» если клики по layer-shell лгут —  
  **ручной клик пользователя + grim/log** предпочтительнее ложного PASS.

---

## 3. Preflight (обязательно до S1)

| # | действие | PASS |
|---|---|---|
| P0 | `git log -1 --oneline` ≥ `9435cc0` (или новее с T195/T196) | commit id в отчёте |
| P1 | `cargo build --release -p chronos` exit 0 | путь + mtime бинаря |
| P2 | Убить старый шелл: `pkill -x chronos` (осторожно) | нет `hyprctl layers` chronos-bar/panel от старого |
| P3 | Старт: `RUST_LOG=info target/release/chronos` (или workspace log path) | log file path зафиксирован |
| P4 | `hyprctl layers` — bar + side panels на pult display | namespace `bar` / panels видны |
| P5 | grim/slurp/hyprctl в PATH | `which grim hyprctl` |
| P6 | **ydotool:** absolute = **screen/2** на этой машине; `uinput`/ydotoold up | иначе manual-only |

**Артефакты:** `/tmp/t209-smoke/YYYYMMDD-HHMM/`  
`mkdir -p` в начале; все grim туда.

**Лог:** `~/.local/state/chronos/chronos.log` (или stdout tee).  
Перед прогоном: `truncate -s0` или `tail -n0 -f` с timestamp маркером `T209-START`.

---

## 4. Правила доказательства

1. **PASS** = кадр `grim` **и/или** строка лога **и** текст «что видел» (1 предложение).  
2. **FAIL** = симптом + grim + (если есть) log snippet. Не чинить в смоке —  
   завести residual / reopen T-ID.  
3. **SKIP** = blocked env (нет Hermes, нет hypr modules dir) + почему.  
4. **Запрещено:** vision-only PASS на 12px тексте; «компилируется = ок»;  
   кадры без привязки к шагу.  
5. **Панель справа открывается в два приёма:** клик иконки рейла = select tab;  
   контент = кнопка dock `⊞`/`⊟` **внизу рейла**.  
   Признак rail-only: `apply per-tab width … content_open=false` / ширина ~40px.  
6. `CHRONOS_SMOKE_SIDE_PANEL=1` / `_LEFT=1` — pin-open без hover, если env ещё жив  
   (проверить `side_panel_*::init`; не полагаться вслепую).

---

## 5. Матрица смока

Приоритет: **P0** блочит daily driver; **P1** product claim; **P2** polish residual.

### 5.1 Bar live customization (T200 / T202 / T207)

| ID | Pri | Шаги | PASS | FAIL symptoms |
|---|---|---|---|---|
| **B1** height live | P0 | System settings → Bar → height slider ±4px | бар/exclusive gap меняются без рестарта; log `appearance applied` / panel gap ok | gap stale; crash |
| **B2** exclusive + floating | P0 | floating **on** → exclusive forced off (sanitize); floating **off** + exclusive on | windows не лезут под бар при exclusive on; floating без exclusive | exclusive stuck on while floating |
| **B3** presets | P1 | chip preset (minimal / floating / …) | bar.toml + UI + visual match; hot-reload | silent no-op; invalid toml |
| **B4** edge bottom | P0 | edge → Bottom (UI or `bar.toml`) | бар **снизу**; recreate ≤1s; no ghost layer (`hyprctl layers`) | stuck top; ghost bar; blank |
| **B5** fraction width | P0 | width 70% or 50%, align center | pill не full-width; margins center; input region hits only pill | compositor stretch full width |
| **B6** cold start double-open | P2 | restart shell, watch log | at most one recreate on boot after open (residual T207 OK if one flicker) | loop open/close; missing bar |
| **B7** agent tools (T201) | P1 | agent: «set bar height 36» **or** IPC/tool if exposed | bar.toml + live height | tool error / no write |
| **B8** dogfood skill (T203) | P2 | optional NL via skill path | optional SKIP if no agent session | — |

**Config safety:** before B4/B5 backup `~/.config/chronos/bar.toml`.  
Restore after FAIL or end of section.

**Evidence:** grim full pult + `hyprctl layers -j | jq` namespaces bar;  
`rg "bar: (recreated|appearance)" log`.

---

### 5.2 Right panel chrome + resize (T204 / T206)

| ID | Pri | Шаги | PASS | FAIL |
|---|---|---|---|---|
| **R1** rail-only no gray lip | P0 | content closed (width ~40) | нет серой/белой полосы у правого края; desktop виден за transparent handle | gray lip, white hole |
| **R2** expand via handle drag | P0 | rail-only → grab 4px inner handle → drag left | content opens to tab width; **no snap-back to ~36–40** mid-drag | snaps to rail; dead handle |
| **R3** shrink | P0 | open content → drag right | shrinks smoothly; can return near rail-only | jump; stuck wide |
| **R4** one-frame jank | P2 | observe expand first frame | residual OK if ≤1 frame jump then stable | continuous thrash |
| **R5** left panel untouched | P1 | left rail resize still works | no regression | left broken |
| **R6** hairline only when open | P1 | rail-only vs open | border/chrome only when content_open | hairline in rail-only |

**Evidence:** grim rail-only + mid-drag + open; optional `RUST_LOG`  
`side_panel_right: handle grab expanded`.

---

### 5.3 Editor surface (T194c / T194b / T205 / T208)

| ID | Pri | Шаги | PASS | FAIL |
|---|---|---|---|---|
| **E1** open md View | P0 | Files → `README.md` (or any .md) → Editor/Preview | View = rendered md; default View not Edit | auto Edit; network badge fetch (shields) |
| **E2** Preview\|Edit only md | P0 | md: both buttons; plain text / image: no dual or forced View | T194c guard | Edit on image |
| **E3** themed buffer | P0 | Edit mode dark theme | buffer **not** pure white; matches shell | white glare |
| **E4** gutter lines | P0 | multi-line file Edit | 1-based line numbers visible left | no gutter |
| **E5** Ln/Col | P0 | arrows / click in buffer | status `Ln X, Col Y` updates **live** (observe errata) | frozen until type |
| **E6** soft wrap | P0 | long line; toggle Wrap | wrap on = soft wrap; off = h-scroll | desync button vs input |
| **E7** Save/dirty | P1 | edit + Save | dirty indicator; file on disk changes | silent fail |
| **E8** terminal drawer | P1 | Terminal ▾ under editor | drawer opens; PTY input works (user confirm) | missing; no focus |
| **E9** light theme buffer | P1 | theme Light + Edit | readable, not inverted mess | unreadable |

**Evidence:** grim Edit with gutter+status; Wrap on/off pair; log no panic.

---

### 5.4 Agent Follow (T195)

| ID | Pri | Шаги | PASS | FAIL |
|---|---|---|---|---|
| **F1** toggle UI | P0 | left thread header 👁 | muted → accent when on | missing control |
| **F2** Follow ON open path | P0 | Follow on; agent `edit_file`/`write_file` real path **or** simulate if test tool | right opens Editor/PreviewTarget path | no open; wrong path |
| **F3** Follow OFF quiet | P0 | Follow off; agent touches file | right **does not** jump | still jumps |
| **F4** clear last_tool on off | P1 | on → tool → off | global last_tool cleared (log/debug or no stale strip) | — |
| **F5** activity strip | P2 | — | **expected SKIP / residual** — strip UI deferred | claim PASS without UI |

**Env:** needs live Hermes ACP session. If no agent: **SKIP F2–F4** with note.  
Optional unit-only already covers `extract_file_path` — does **not** replace F2.

---

### 5.5 System settings + ACP (T196)

| ID | Pri | Шаги | PASS | FAIL |
|---|---|---|---|---|
| **S1** System settings tab | P0 | rail «System settings» (EditorSettings) | Bar page + Theme/Hypr/About sections | empty placeholder |
| **S2** theme toggle | P0 | Toggle in Theme section | dark↔light live; `theme.toml` updated | no visual; no persist |
| **S3** hypr modules | P1 | list `~/.config/hypr/modules/*.lua`; click Open | opens in Editor View | empty when files exist; wrong path |
| **S4** About version | P2 | About row | version == `CARGO_PKG_VERSION` of binary | garbage |
| **S5** ACP agents list | P0 | rail «ACP agents» | Hermes + built-in badge; command line | empty/error falsely |
| **S6** Open agents.toml | P1 | Edit button | Editor opens `~/.config/chronos/agents.toml` | wrong path |
| **S7** Reload after edit | P1 | add `[[agents]]` stub → Save → Reload | new row appears (or honest parse error) | stale list |
| **S8** inline CRUD | P2 | — | **expected SKIP** — deferred; edit-toml path only | — |

---

### 5.6 Regression quick (optional P1 if time)

| ID | Pri | Шаги | PASS |
|---|---|---|---|
| **X1** left agent chat send | P1 | short message | stream tokens; no 100% CPU pin |
| **X2** notifications | P2 | `notify-send t209 test` | toast appears |
| **X3** no panic | P0 | full session | `rg -i 'panicked at' log` empty for run window |

---

## 6. Known residual — allowed PASS with note

Не проваливать смок **только** из-за:

| residual | source |
|---|---|
| One-frame jank rail→expand | T206 |
| Cold-start single bar recreate | T207 |
| No activity strip UI | T195 |
| No inline ACP add/remove | T196 |
| No fork live set_anchor (recreate flash) | T207 |
| Highlight current line deferred | T205 |
| PreviewTarget generation hardcode 1 (same path re-open) | T195/T196 |

Если residual **хуже** documented (e.g. permanent snap-to-40) → **FAIL**.

---

## 7. Порядок прогона (рекомендуемый ~45–90 мин)

```
Preflight P0–P6
  → R1–R3, R6          (panel feel first — user pain)
  → E1–E6, E8          (editor daily)
  → B1–B5              (bar; backup toml)
  → S1–S2, S5–S7       (settings)
  → F1–F3              (if Hermes up)
  → B6, E9, S3, X3     (cleanup / secondary)
  → restore bar.toml if changed
```

---

## 8. Отчёт (формат)

```markdown
# T209 report — live smoke residuals

**Binary:** path · mtime · `git rev-parse --short HEAD`
**Env:** display res · hypr version · Hermes y/n · ydotool y/n
**Artifacts:** /tmp/t209-smoke/...

## Matrix

| ID | result | evidence |
|---|---|---|
| B1 | PASS/FAIL/SKIP | grim:… log:… note |

## Failures (root cause guess — not fix)
## Residuals confirmed still true
## Verdict: PASS | PASS WITH RESIDUAL | FAIL
```

**Приёмка архитектора:** сверяет grim на диске, не narrative.  
Unit tests **не** перезапускать как замену live.

---

## 9. Автоматизация (optional, later)

`scripts/dev/t209-smoke.sh` — **сбор артефактов**, не авто-PASS:

- start marker in log
- grim full screen named by step id when called
- dump `hyprctl layers -j`
- **не** assert UI (architect eyes)

Полный ydotool script для bar.toml edge/fraction **можно** добавить  
после ручного PASS B4/B5 (калибровка координат).

---

## 10. Definition of done (wave live)

| уровень | критерий |
|---|---|
| **PASS** | все P0 = PASS; P1 ≤1 SKIP env; 0 FAIL |
| **PASS WITH RESIDUAL** | все P0 PASS; documented residual only |
| **FAIL** | any P0 FAIL → reopen thin task, not re-accept old T-ID as done live |

После PASS/WITH RESIDUAL: HANDOFF «live N/V» → «live smoke T209 …».

---

## 11. Traceability

| area | tasks |
|---|---|
| Bar apply / recreate | T200, T202, T207, T201 |
| Panel resize | T204, T206 |
| Editor | T194c, T194b, T205, T208 |
| Follow | T195 |
| Settings / ACP | T196 |
| Plan | `docs/superpowers/plans/2026-08-02-live-customization.md` |
