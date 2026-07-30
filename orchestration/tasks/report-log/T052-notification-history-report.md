<!-- T052 — migrated 2026-07-22 from orchestration/report-log/hermes-report-14.md — see orchestration/tasks/MIGRATION.md -->

# Hermes Report — №14: История уведомлений (notification history)

- **Дата:** 2026-07-19
- **Агент:** Hermes (Lead Architect Agent)
- **Задача:** HERMES.md §14 — история уведомлений: функция «collected log» всех уведомлений за сессию + bell-виджет в баре с непрочитанным бейджем.
- **Статус:** ✅ Готово к приёмке. `cargo check -p chronos` → Finished (0 errors). Юнит-тесты сервиса → 12 passed.

---

## Что сделано

### 1. Сервис-слой (история + счётчик непрочитанных)
**Файл:** `crates/services/src/notification/types.rs`

- `NotificationState` расширен полями:
  - `history: Vec<Notification>` — полный лог уведомлений сессии (ring buffer).
  - `unread: usize` — счётчик непрочитанных (для бейджа в баре).

**Файл:** `crates/services/src/notification/mod.rs`

- `MAX_HISTORY = 100` — кап истории. `push_history()` дропает самые старые за пределами лимита.
- Новый `NotificationCommand::MarkAllRead` — гасит `unread`, историю **не** трогает.
- `notify()` теперь: пушит в `history` + `unread += 1` вне зависимости от судьбы эфемерного попапа.
- `dispatch(MarkAllRead)` → `mark_all_read()`: `unread = 0`.
- **Ключевое решение:** `DismissAll` (существующий) стирает эфемерные уведомления — его НЕ трогал. `unread` сбрасывается только через `MarkAllRead` (при открытии попапа истории), чтобы точка в баре гасла именно когда юзер посмотрел лог, а не при автозакрытии всплывашки.

### 2. UI — общая карточка
**Файл:** `crates/app/src/notifications/view.rs`

- Вынес `render_notification_card(n: &Notification, theme: &Theme, close_button: Option<AnyElement>) -> AnyElement`.
  - Эфемерный попап вызывает с `Some(close_btn)`, история — с `None`.
  - Urgency-полоса `border_l_3` внутри карточки (critical=error, normal=accent, low=subtle).
- Удалён мёртвый closure `accent_for`.
- `render()` упрощён: `map` по `notifications`, каждый — через `render_notification_card`.

### 3. Попап истории (новый модуль)
**Файлы:** `crates/app/src/notifications/history_popup/mod.rs`, `.../view.rs`

- Жизненный цикл 1:1 с эталоном `updates_popup`: `WindowOptions{focus:false, show:true}`, `open()` / `close()` / `close_this()` (guard по `cx.window().kind == "history_popup"`, без реентрантного бага `remove_window`), `toggle()` (проверяет наличие окна), `init()` (запускает фоновый вотчер `notify()`-изменений → `cx.notify()`).
- **Не закрывается по фокусу** (поведение как у updates_popup, не как у эфемерного notifications).
- При `open()` → диспатч `MarkAllRead` (точка в баре гаснет сразу при клике на колокол).
- `view.rs`: заголовок («Notification history» + кнопка ✕ закрытия) + scrollable стек карточек `render_notification_card(.., None)`, **newest-first**, жёсткий клип `max_h`/`overflow_hidden`, плейсхолдер «No notifications yet» если пусто.
- Зарегистрирован в `main.rs`: `notifications::history_popup::init(cx)` (после `notifications::init`).

### 4. Bell-виджет в баре (новый)
**Файл:** `crates/app/src/bar/widgets/notification_bell.rs` + `bar/widgets/mod.rs`

- `BellWidget` (BarWidget): секция `BarSection::Right`, имя `"bell"`.
- Рендер: glyph 🔔 (яркий, если `unread>0`, иначе `text.muted`) + опциональный красный бейдж (`status.error`) с числом `unread` (кап «99+»).
- `on_click` → `crate::notifications::history_popup::toggle(window, cx)`.
- `describe(cx)` читает `AppState::notification(cx)` (поля `unread`/`icon`), возвращает метрики для регистрации/тестов.
- `register(cx)` + 2 теста `describe()` (базовый + badge-логика через `Render`).
- Подключён в `bar/widgets/mod.rs`: `mod notification_bell;` + `notification_bell::register(cx)`.

### 5. Авто-обновление бейджа
Бар уже подписан на смену `notification` (см. `bar/mod.rs:32-33` — `cx.observe(...)` на `AppState::notification`). При изменении `unread` бар перерисовывается сам → бейдж живой без доп. вотчеров. Проверено по коду, не требует правки `bar/mod.rs`.

---

## Вне зоны №14 (флагую первым, per AGENTS.md)

При сборке обнаружил предсуществующий сломанный стейт (не мой WIP, не моя зона):
- `crates/app/src/bar/widgets/dock.rs` лежал в дереве и вызывался как `dock::register(cx)` в `widgets/mod.rs:19`, но **`mod dock;` не был объявлен** → crate не собирался целиком (`E0433: cannot find module dock`).
- Добавил ровно одну строку `mod dock;` в `bar/widgets/mod.rs`, чтобы проект компилировался. Больше dock не трогал.

Это предсуществующий баг (кто-то закоммитил `register` без `mod`). Если хочешь — выношу в отдельный коммит/задачу, но без этой строки `cargo check` падал до моих правок.

---

## Верификация (реальный прогон)

```
$ cargo test -p chronos-services notification
running 12 tests
test notification::tests::close_keeps_history_mark_all_read_clears_unread ... ok
test notification::tests::history_is_bounded ... ok
test notification::tests::notify_builds_history_and_unread ... ok
... (остальные 9 pre-existing) ...
test result: ok. 12 passed; 0 failed

$ cargo check -p chronos
Finished `dev` profile [unoptimized + debuginfo] — 0 errors
```

- Юнит-тесты сервиса №14 (history/unread/bounded) — зелёные.
- `cargo check -p chronos` — Finished, ошибок нет (только pre-existing warnings: unused imports/vars в чужих файлах — не мои).

## Что НЕ делал (границы)

- **Не коммитил и не пушил** — per HANDOFF только Архитектор коммитит. Код висит в рабочем дереве, зелёный.
- **Живой UI-смок** (grim-скрин белла + бейджа + попапа истории) — headless недоступен, снимает Архитектор на релизе. Логика проверена компиляцией + юнит-тестами сервиса.
- `bar/mod.rs` не трогал (per бриф — зона только `widgets/mod.rs` 2 строки + новый виджет).

## Файлы (изменения №14)

| Файл | Статус | Суть |
|------|--------|------|
| `crates/services/src/notification/types.rs` | changed | `history` + `unread` в `NotificationState` |
| `crates/services/src/notification/mod.rs` | changed | `MAX_HISTORY`, `MarkAllRead`, `notify`→history/unread, `mark_all_read`, 3 теста |
| `crates/app/src/notifications/view.rs` | changed | `render_notification_card(n, theme, close_button)` |
| `crates/app/src/notifications/mod.rs` | changed | `pub mod history_popup;` |
| `crates/app/src/notifications/history_popup/mod.rs` | new | open/close/close_this/toggle/init + вотчер |
| `crates/app/src/notifications/history_popup/view.rs` | new | заголовок + scroll карточек |
| `crates/app/src/bar/widgets/notification_bell.rs` | new | bell + бейдж + on_click |
| `crates/app/src/bar/widgets/mod.rs` | changed | `mod notification_bell;` + `register` (+ `mod dock;` вне зоны) |
| `crates/app/src/main.rs` | changed | `notifications::history_popup::init(cx)` |

## Рекомендация

Коммить отдельным коммитом: `notifications: история + bell-виджет (№14)`. Строку `mod dock;` — либо в этот же коммит (как «fix build»), либо отдельным, на твой выбор.
