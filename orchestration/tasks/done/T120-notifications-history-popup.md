# T120 — ПРИНЯТ WITH CAVEATS (2026-07-24)

**Статус: ACCEPTED WITH CAVEATS.** Anchored history popup + Clear/dismiss.
Review: `report-log/T120-notifications-history-popup-review.md`.
Отчёт: `report-log/T120-notifications-history-popup-report.md`.
Commits: `0ebe6de` `7415fcb` `a90a71a` + errata `253f25b` (detach/ROW_H/✕).

---

<!-- T120 — Notifications history popup: anchored + mockup UI + history clear/dismiss.
     Агент не назначается в брифе — только T-ID. План с нарезанными Task 1–5. -->

# T120 — Notifications history popup (anchored redesign)

**Статус: OPEN, не назначен.**  
**План (канон шагов):**  
`docs/superpowers/plans/2026-07-24-notifications-history-popup.md`  
**Мокап:** `design/Notifications Popup.dc.html`  
**Эталон окна/якоря:** `crates/app/src/updates_popup/` + `bar/widgets/updates.rs` (post-T117)

## Как выполнять (обязательно)

Это **план из 5 task'ов** под workflow:

- **superpowers:subagent-driven-development** — один свежий implementer на
  Task N, затем task-review (spec + quality), потом следующий task; **или**
- **superpowers:executing-plans** — один проход по чеклистам plan-файла.

**В ChronOS «subagent» = локальный миньон по T-ID, не spawn из сессии
Архитектора.** Бриф **не** привязан к имени агента. Указатель инструмента
(`orchestration/agents/<TOOL>.md`) — тонкий линк на этот T120, без
переноса требований в личный файл.

После **каждого** Task 1–4: коммит (или явный «squash-with-next» в
отчёте). После Task 5: отчёт. Приёмку делает **Архитектор**, не миньон.

Отчёт:  
`orchestration/tasks/report/T120-notifications-history-popup-report.md`

Опциональный ledger:  
`.superpowers/sdd/progress.md` — `Task N: complete (commits …)`.

---

## Цель

Попап истории уведомлений (клик по bell в баре) — тот же класс, что
updates после T117:

1. Анкор к иконке bell (`AnchoredPopup` + fallback LayerShell).
2. Реальный скролл списка, не hard-clip без прокрутки.
3. Визуал 1:1 dark-мокапу (360px, urgency-strip, monogram, body clamp,
   outlined actions, **Clear all**, empty «No notifications»).
4. Per-item dismiss + Clear all в **history** (сейчас backend этого **нет**).

## Текущее состояние (не выдумывай)

| Место | Факт |
|---|---|
| `history_popup/mod.rs` | `open(cx)` / `toggle(window,cx)` — **без** anchor; fixed TOP\|RIGHT; height cap 416; `MarkAllRead` on open |
| `history_popup/view.rs` | Header «Notifications» + ✕; cards via `render_notification_card(..., None)` — **нет** row dismiss; `overflow_hidden` max 380; **нет** Clear all |
| `notification_bell.rs` | `on_click` → `toggle`; **нет** canvas bounds |
| `NotificationCommand` | `Close`, `InvokeAction`, `DismissAll` (эфемерный стек), `MarkAllRead` — **нет** history remove/clear |
| `NotificationState::history` | ring `MAX_HISTORY=100`; close/dismiss live **не** чистят history |

Эфемерный toast (`notifications/view.rs`) — **вне скоупа**, кроме
вынужденного shared helper.

## Задачи (кратко; детали и чеклисты — в plan-файле)

### Task 1 — Service: history mutations

- `NotificationCommand::RemoveFromHistory(u32)`
- `NotificationCommand::ClearHistory`
- pure helpers + unit tests; regression: `DismissAll` ≠ clear history
- default: history-only remove **не** обязан закрывать live toast

```bash
cargo test -p chronos-services --lib notification -- --nocapture
```

### Task 2 — Bell: bounds + mouse-down

- Зеркало `bar/widgets/updates.rs`: `canvas` + **`.relative()`** +  
  `on_mouse_down(Left)` → `toggle(anchor_rect, parent, window, cx)`
- Урок T117: без `.relative()` якорь врёт

### Task 3 — Window: AnchoredPopup

- Сигнатуры как у updates:  
  `open(cx, anchor_rect, parent)`,  
  `toggle(anchor_rect, parent, window, cx)`
- `BottomRight` + `BottomLeft`, grab, SLIDE_X|FLIP_X, offset y=4
- Fallback `PopupNotSupportedError` → LayerShell TOP\|RIGHT
- `close` / `close_this` reentrancy — **как updates** (ghost-window HANDOFF)
- `MarkAllRead` on open сохраняется
- Width **360**

### Task 4 — View: mockup

- **Убрать** header title + panel ✕
- List: scroll + id; urgency 3px; monogram; summary/body; actions; row ✕
- Footer **Clear all** если `history.len() > 1` (как мокап)
- Empty: `No notifications`
- Dispatch: `RemoveFromHistory` / `ClearHistory` / `InvokeAction`
- Новый код: без `let _ = fallible` без лога

```bash
cargo build --release -p chronos
```

### Task 5 — Live smoke + report

```bash
# убить stale chronos, поднять release
notify-send "Zed" "Build finished" -u normal
notify-send "Mail" "Re: design" -u low
notify-send "System" "Battery critical" -u critical
# клик bell → grim: list / dismiss one / clear all / empty
```

Unit green **без** live → **не** done.

---

## Зона файлов

**Писать:**
- `crates/services/src/notification/{types,mod}.rs`
- `crates/app/src/bar/widgets/notification_bell.rs`
- `crates/app/src/notifications/history_popup/{mod,view}.rs`
- опционально точечно `notifications/view.rs` только для shared card

**Не трогать:**
- `updates_popup`, `volume_popup`, `system_popup`, `tray_menu`
- side_panel_*, T113–T115, AUR upgrade path
- notify-after-upgrade (отложено отдельно)

## Что НЕ делать

- Не именовать бриф/отчёт «для Hermes/Cline/…» — только **T120**
- Не `-Syu` / AUR / updates UI
- Не «исправлять» ephemeral toast layout «заодно»
- Не закрывать history через `DismissAll`
- Не `on_click` для grab-popup open
- Не фабриковать тесты; копировать реальные `cargo test` выводы в отчёт
- Не AI-trailers в коммитах

## Верификация (суммарно)

1. `cargo test -p chronos-services --lib notification` — зелёный + новые
2. `cargo build --release -p chronos` — зелёный
3. Live: anchor у bell, scroll, ✕, Clear all, empty, badge unread off
4. Отчёт с коммитами, командами, grim-путями, честным PENDING

## Accept criteria (Архитектор)

- [ ] Plan Task 1–4 код в дереве, дифф сверяется с брифом
- [ ] History commands реальные, не алиас на DismissAll
- [ ] Anchored + fallback оба существуют
- [ ] Мокап-скелет: нет header, есть strip/scroll/footer/empty
- [ ] Live evidence (лог + grim) или честный блокер в отчёте
- [ ] Нет регрессии ghost-window (`close_this` discipline)

**Reject:** «компилируется» без live; canvas без `.relative()`; история
чистится через ephemeral Close only; отчёт без команд/выводов.
