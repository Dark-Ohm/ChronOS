<!-- T044 — migrated 2026-07-22 from docs/orchestration/report-log/hermes-report-12.md — see docs/orchestration/tasks/MIGRATION.md -->

# SESSION_REPORT — Hermes №12

**Дата:** 2026-07-19
**Задание:** №12 — попап уведомлений: структурный клип вместо pixel-угадайки
**Зона:** `crates/app/src/notifications/**` ТОЛЬКО
**Статус:** КОД ГОТОВ, ВЕРИФИКАЦИЯ ЧАСТИЧНО ЗАБЛОКИРОВАНА чужим WIP (см. «Блокер»)

---

## Что сделано

Задание №12 переформулирует подход №11: вместо уточнения pixel-формулы
`estimate_content_height` — структурный жёсткий клип по образцу
`updates_popup` (коммит `67f7d10`). №11 официально ЗАМЕНЁН этим подходом.

### `crates/app/src/notifications/mod.rs` (переписан sizing-блок)
- Удалены `estimate_content_height`, `max_popup_height`,
  `max_popup_height_owned`, `BODY_CHARS_PER_LINE` и все pixel-константы
  карточек (CARD_PAD_Y/HEADER_H/TITLE_H/BODY_LINE_H/ACTION_H/STACK_GAP…).
  Это была сама болезнь — pixel-математика на непромеренных GPUI
  text-метриках, раз за разом недосчитывающая высоту.
- Окно теперь **фиксированной высоты** `POPUP_HEIGHT = LIST_MAX_H = 360px`
  (как `updates_popup::MAX_POPUP_H`). Окно НЕ резинится под контент —
  `sync_window` больше не дёргает `window.resize()` при изменении
  снапшота, только `view_cx.notify()`.
- Добавлены константы-капы: `LIST_MAX_H` (клип стека карточек) и
  `BODY_MAX_H = 90px` (~5 строк body).

### `crates/app/src/notifications/view.rs` (два уровня клипа)
1. **Внутри карточки:** body обёрнут в `.max_h(px(BODY_MAX_H)).overflow_hidden()`
   — длинный body обрубается клипом, не вылезает за карточку на соседнюю.
2. **Список карточек целиком:** контейнер карточек обёрнут в
   `.max_h(px(LIST_MAX_H)).overflow_hidden()` — любое число уведомлений
   не может вырастить окно выше капа; старые карточки молча обрезаются
   снизу (допустимо — они и так истекают по таймеру, в отличие от
   привилегированной кнопки, которую надо держать видимой; здесь её нет).

### Комментарии — честные
- В `view.rs` явно написано, что `.overflow_hidden()` — это clip БЕЗ
  скролла (в этой сборке gpui `overflow_y_scroll` не резолвится — факт из
  №9/№12 брифа). Никакой лжи про «internal scroll», которая была в
  замечании к приёмке №9.
- В `mod.rs` модульная дока фиксирует: sizing = фиксированный кап,
  контент клипится, не резайзит surface.

---

## Верификация

### Выполнено
- **`cargo check -p chronos` (основное дерево, после разблокировки
  upower-WIP коммитом `0918ec1`):** `Finished` без ошибок (только
  pre-existing warnings — `tray_menu` drop_references и пр., не мои).
- **Изолированный `git worktree` на HEAD `67f7d10` + мои 2 файла
  (рецепт HANDOFF, без чужого WIP):**
  - `cargo check -p chronos` → зелёный.
  - `cargo test --workspace --lib --bins` → **0 failed** (92 + 3 +
    остальные; в т.ч. `notification::tests::expiry_closes_after_timeout`
    зелёный). Мои правки не сломали ни одного имеющегося теста.

### ЗАБЛОКИРОВАНО — чужой WIP вне зоны (БЛОКЕР)
Полный `cargo test --workspace --lib --bins` в основном дереве КРАСНЫЙ
ИСКЛЮЧИТЕЛЬНО из-за чужого некомпилящегося WIP, который я по зонам №12
не имею права трогать (`services/**` и всё кроме `notifications/**`):

1. `crates/services/src/network/mod.rs:265,269` — `error[E0728] await
   only in async` + `error[E0308] mismatched types`. Чужой network-WIP:
   `.await` внутри НЕ-async блока в тесте (регрессия). Файл изменён
   параллельно (в `git status` на старте сессии его не было).
2. `crates/app/src/dock/config.rs:114` — `error[E0433] cannot find crate
   tempfile`. Чужой dock-WIP добавил `#[cfg(test)]` с `tempfile`, но НЕ
   добавил dep в `crates/app/Cargo.toml` (grep подтвердил: tempfile там
   НЕТ).

Оба — НЕ мой код, НЕ моя зона. По правилу HANDOFF «Чужой
некомпилящийся WIP = СТОП и вопрос Архитектору» я НЕ правлю чужие файлы
(это была бы утечка в чужую зону + нарушение «НИКОГДА не git checkout
чужих файлов»). Краснота дерева — не моя ответственность.

### Живой release-смок — НЕ прогнан
Требуется графическая сессия (headless-агент). Критерий №12: 2-3
уведомления с длинным summary+body → grim-скрин, карточки не съезжают за
попап, `hyprctl layers -j` `h` ≤ новый кап (360px), лог без error/panic.
Смок снимает Архитектор (у меня headless, как и в №8/№9).

---

## Коммит

**НЕ сделан.** Причина: критерий верификации «`cargo test --workspace`
зелёные» недостижим из-за чужого WIP (блокер выше), а коммитить в красное
дерево без полной верификации — нарушение правил приёмки. Код готов и
изолированно проверен (worktree: check+test зелёные).

Планируемый коммит (когда дерево станет зелёным):
`notifications : жёсткий клип вместо pixel-оценки высоты (тот же паттерн, что updates_popup 67f7d10)`
Поимённый add: `crates/app/src/notifications/mod.rs`,
`crates/app/src/notifications/view.rs`. `git diff --staged` — глазами
перед коммитом.

---

## Эррата / наблюдения
- Во время сессии `crates/services/src/upower/mod.rs` появился и исчез из
  `git status` (в начале сессии его не было, потом был красным
  `.map(map_profile)` — `&str` vs `String`, ломал `cargo check`), затем
  ушёл в коммит `0918ec1 WIP cline-10` и перестал ломать сборку. Это
  параллельный агент (Cline, судя по сообщению) — мимо моей зоны, просто
  зафиксировано для контекста.
- `overflow_hidden` подтверждённо резолвится в нашем форке gpui
  (используется в `updates_popup`/`volume_popup`/`tray_menu`/`osd`);
  `overflow_y_scroll` — НЕТ (это другой метод, не путать).
- №11 (точнее посчитать `estimate_content_height`) формально отменён
  этим заданием: проблема была не в точности формулы, а в самом подходе
  pixel-угадайки на непромеренных метриках. Клип решает класс багов
  сразу, без арифметики.
