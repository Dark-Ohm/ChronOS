# T293 — вкладка Notifications на правой рельсе

**Статус:** DONE 2026-08-16 (`2ffe9e5`). Live grim ещё открыт.
**Приоритет:** P2 IA.
**Роль:** FRONTEND.
**Не параллелить** с T289 (`view.rs` / `on_tab_select`) и T291 (`SystemTab`).

## Сейчас

Бар `notification_bell` открывает `notifications/history_popup` (AnchoredPopup):
лента `history` newest-first, dismiss, Clear all, empty «No notifications».
Тосты (`notifications/view.rs`) — отдельные всплывашки, **не** эта вкладка.

Правой вкладки уведомлений нет.

## Задача

Новая **вкладка** правой панели = то, что сейчас в history popup.

`PanelTab::Notifications`:

- `id` = `"notifications"`
- label `"Notifications"`
- иконка `icons/bell.svg` (уже в дереве)
- в `ALL`, `parse_id`, `label`, `icon_path`, `preferred_content_width`
  (как System / ~420)
- `for_mode` **обоих** режимов: после `System` (частый вход)
- `default_dev_top` / `default_gamer_top` — тот же слот
- `TabContent::Notifications` — живая вьюха, не `EmptyTab`
- образец: `tab/acp_settings.rs` / `SystemTab`

Контент: вынести список+карточки+Clear all из
`history_popup/view.rs` в общий рендер (например
`notifications/history_list.rs`). Вкладка и (пока жив) попап не
дублируют разметку. Пустой стейт — тот же хелпер, что T269, иконка
`bell`.

Поведение 1:1: newest first, urgency strip, monogram, actions, dismiss,
Clear all при `len > 1`, `NotificationCommand::ClearHistory`. Скролл
на всю высоту канваса, не `MAX_LIST_H` попапа.

## Бар

Колокольчик **остаётся** (бейдж unread). Клик больше не
`history_popup::toggle`:

`side_panel_right::select_tab(PanelTab::Notifications, cx)`
(+ открыть панель, если закрыта — `select_tab` уже это делает).

`history_popup/` снести, когда греп пуст (`init` в `main.rs` тоже).
Пустой попап не оставлять (T246).

Тосты не трогать.

## Нельзя

- Тащить тосты во вкладку.
- Вторая копия карточек «на всякий».
- Left Display / T290, Perf Gaming / T291, Shell Gamer / T292.
- `Source/gpui/`, `Cargo.lock`.

## Тесты

- `parse_id("notifications")`, `for_mode` оба режима содержат вкладку
  ровно раз.
- Inventory `ALL` + rail defaults.
- Список: empty / один / два (Clear all только при >1) — если хелперы
  чистые.

## Верификация

```
cargo test -p chronos --lib side_panel_right
cargo test -p chronos --lib notifications
cargo test -p chronos --lib bar
```

Live: клик колокольчика → правая Notifications, лента как в попапе.
Попапа в `hyprctl` нет. Тост при новом уведомлении жив. Unread на баре
тот же. Grim вкладки empty + с карточками.

## Коммит

`feat(right-panel): Notifications tab replaces history popup (T293)`
