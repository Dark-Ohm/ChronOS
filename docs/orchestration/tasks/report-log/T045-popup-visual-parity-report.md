<!-- T045 — migrated 2026-07-22 from docs/orchestration/report-log/hermes-report-13.md — see docs/orchestration/tasks/MIGRATION.md -->

# SESSION_REPORT — Hermes №12 + №13

**Дата:** 2026-07-19
**Задания:** №12 (жёсткий клип уведомлений) + №13 (бордер+hover+AUR-бейдж на 4 попапах)
**Зоны:** №12 — `crates/app/src/notifications/**`. №13 — `view.rs` четырёх попапов
(`updates_popup/`, `volume_popup/`, `notifications/`, `tray_menu/`), только стилевые вызовы.
**Статус:** ОБА КОДА ГОТОВЫ, КОММИТЫ ЗАБЛОКИРОВАНЫ чужим WIP (см. «Блокер»).

---

## ИСПРАВЛЕНИЕ ОШИБКИ (важно)

В предыдущем отчёте была ошибка: я написала «№13 — НЕ моё, владелец Grok».
**НЕВЕРНО.** У каждого агента своя нумерация; у Grok'а СВОЁ №13 (cava,
`c519e2e`+`eb043fd`, уже сделано и принято) — оно случайно совпало по
номеру с моим №13 (бордер/hover/badge на 4 попапах). Grok cava не трогал
ни один из этих `view.rs`. **Моё №13 — МОЁ, я его сделала (ниже).**

---

## №12 — жёсткий клип вместо pixel-угадайки (ГОТОВО)

### `crates/app/src/notifications/mod.rs` (переписан sizing-блок)
- Удалены `estimate_content_height`, `max_popup_height`,
  `max_popup_height_owned`, `BODY_CHARS_PER_LINE` и все pixel-константы
  карточек. Окно теперь **фиксированной высоты** `POPUP_HEIGHT =
  LIST_MAX_H = 360px` (как `updates_popup::MAX_POPUP_H`). `sync_window`
  больше не дёргает `window.resize()` при изменении снапшота, только
  `view_cx.notify()`.
- Константы-капы: `LIST_MAX_H` (клип стека карточек), `BODY_MAX_H = 90px`.

### `crates/app/src/notifications/view.rs` (два уровня клипа)
1. Body внутри карточки: `.max_h(px(BODY_MAX_H)).overflow_hidden()`.
2. Стек карточек: `.max_h(px(LIST_MAX_H)).overflow_hidden()`.

---

## №13 — бордер+hover+AUR-бейдж на 4 попапах (ГОТОВО)

Чисто визуально, ZERO логики (по зонам №13 — только стилевые вызовы в
`view.rs`, не трогала `mod.rs`/тему).

### Внешний бордер (`theme.border.subtle` = `#313244`, уже был в теме)
Добавлен `.border_1().border_color(border_subtle)` на внешний контейнер
каждого попапа:
- `updates_popup/view.rs:156` — `div().flex_col().rounded(radius_lg).bg(bg).border_1().border_color(border_subtle).overflow_hidden()`
- `volume_popup/view.rs:104` — аналогично (+ `w(px(300.))`)
- `tray_menu/view.rs:94` (и placeholder `:64`) — аналогично
- `notifications/view.rs:168` — на стек карточек (поверх №12-клипа)

### AUR-бейдж (`updates_popup/view.rs::render_row`)
Вместо голого `format!("{}{}", name, " (AUR)")` — отдельный div-бейдж
(пилюля): `rounded(radius)`, `px(6.)/py(1.)`, фон `accent.hover` на
пониженной альфе (`.opacity(0.18)`), текст `accent.hover`, `text_xs`,
буквы «AUR». Рендерится **ТОЛЬКО** если `source == UpdateSource::Aur`
(официальные — без бейджа, не пустая строка).

### Row-hover (`theme.interactive.hover`)
- `updates_popup/view.rs::render_row` — добавлен `.hover(|s| s.bg(hover))`
  (раньше hover был только на крестике/Upgrade all).
- `tray_menu/view.rs::render_node` — добавлен `.hover(|s| s.bg(hover))` на
  обе ветки строки (clickable и non-clickable). `hover` протащен
  параметром в `render_node` (отдельная функция, не замыкание `render`).
- `volume_popup` — hover УЖЕ был на `device_row`/`title_row`
  (`theme.interactive.hover`), совпадает с требуемым цветом — не трогала.

---

## Верификация

### Выполнено
- **`cargo check -p chronos` (основное дерево):** `Finished` без ошибок.
  Мой №12+№13 код компилируется. Только pre-existing warnings (`unused`
  imports из чужого WIP в `tray_menu`/`mpris` — не мои).
- **Изолированный worktree (HEAD `67f7d10` + мои файлы):** `cargo check`
  зелёный, `cargo test --workspace --lib --bins` → **0 failed** (worktree
  НЕ содержит чужого сетевого/dock-WIP, поэтому тесты зелёные там).

### ЗАБЛОКИРОВАНО — чужой WIP вне зоны (БЛОКЕР на коммит)
Полный `cargo test --workspace --lib --bins` в основном дереве КРАСНЫЙ
ИСКЛЮЧИТЕЛЬНО из-за чужого некомпилящегося WIP, который я по зонам не
имею права трогать:
1. `crates/services/src/network/mod.rs:265,269` — `error[E0728]` await вне
   async + `error[E0308]` (чужой network-WIP, регрессия теста).
2. `crates/app/src/dock/config.rs:114` — `error[E0433]` cannot find crate
   `tempfile` (чужой dock-WIP, dep не добавлен в `crates/app/Cargo.toml`).

Оба — НЕ мой код, НЕ моя зона. По HANDOFF «чужой некомпилящийся WIP =
СТОП» я НЕ правлю чужие файлы (утечка в чужую зону + «НИКОГДА не git
checkout чужих файлов»). Краснота дерева — не моя ответственность.

### Живой release-смок — НЕ прогнан
Требует графическую сессию (headless-агент). Критерии №12/№13: 2-3
длинных `notify-send` → grim-скрин (карточки не съезжают за попап,
бордер `#313244` виден вокруг всех 4 попапов, AUR-бейдж — пилюля,
hover на строках updates/tray меняет фон), `hyprctl layers -j` `h` ≤ 360px
для notifications, лог без error/panic. Смок снимает Архитектор.

---

## Коммиты (НЕ сделаны)

Критерий «`cargo test --workspace` зелёные» недостижим из-за чужого WIP.
Коммитить в красное дерево без верификации — нарушение правил приёмки.

Планируемые коммиты (когда дерево станет зелёным):
- №12: `notifications : жёсткий клип вместо pixel-оценки высоты (тот же паттерн, что updates_popup 67f7d10)` → `mod.rs` + `view.rs`.
- №13: `ui : бордер+hover+AUR-бейдж на попапах (visual parity с design/*.dc.html)` → 4 `view.rs`.

ВНИМАНИЕ ПО АТОМАРНОСТИ: №12 и №13 оба правят
`notifications/view.rs` (№12: строки body/list-clip; №13: бордер на тот
же контейнер list-clip). При разблокировке разделю через `git add -p`
(№12-ханки отдельно, №13-ханки отдельно). Если hunk'и слипнутся в одном
блоке строк — сделаю один честный коммит
`notifications : жёсткий клип + бордер (№12+№13)` с явной ссылкой на оба
задания. `git diff --staged` — глазами перед каждым коммитом.

---

## Эррата
- `overflow_hidden` резолвится в нашем форке gpui (используется в
  `updates_popup`/`volume_popup`/`tray_menu`/`osd`); `overflow_y_scroll` —
  НЕТ (другой метод, не путать).
- `border_1()` + `border_color()` резолвятся (в `notifications` уже был
  `border_l_3().border_color(accent)` — тот же API).
- №11 (точнее посчитать `estimate_content_height`) формально отменён №12:
  проблема была не в точности формулы, а в подходе pixel-угадайки.
- Чужой upower-WIP (Cline, `0918ec1`) больше не ломает сборку.
- Архивная копия предыдущего (ошибочного) отчёта:
  `docs/orchestration/report-log/hermes-report-12b-confusion.md`.
