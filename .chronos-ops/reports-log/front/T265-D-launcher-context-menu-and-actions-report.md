# T265-D — Launcher app context menu and Desktop Actions — Report

**Date:** 2026-08-16
**Role:** FRONTEND. Zone: `crates/app/src/launcher/**` only.
**Commit:** `0405711` `feat(launcher): app context menu and desktop actions (T265-D)`.

**Приёмка (архитектор, 2026-08-16):** код + юниты **приняты**. Сверил дерево
и прогнал сам: launcher 47/47, `--lib` 522/522. Свежий `--release` блокирован
T295 (`calendar_popup` не в crate). Live grim — долг; бинарь
`target/release/chronos` 18:29 уже несёт строки меню.

## Status

**Done (code + unit tests green).** Live release verification deferred — see
"Не сделано / Live". Same isolation caveat as C: worked on master, staged only
the launcher files (verified `git diff --staged`, no foreign lines).

## Что сделано

### 1. `pin_menu.rs` → `app_menu.rs` (rename, тот же коммит)

Правый клик по клетке/строке открывает **одно** меню — то, что было pin-меню,
расширено до полного app-меню. Движок прежний: `gpui-component::PopupMenu` в
AnchoredPopup, root = `gpui_component::Root`, `grab: false` (T264), Overlay
click-catcher с дыркой, `Root`. Якорь — **живой композитор-запрос**
`catcher_anchor_for` (урок `162798b4`/`180fe88`): спека пишет «`window.bounds().
origin + position`» как сокращение, но реальный фикс был именно в живой
`window_position("chronos-launcher")` — центрированное Hyprland-окно навсегда
докладывает `(0,0)`. Не тронул, он корректен.

Пункты (честное состояние, по спеке):

| Пункт | Бэкенд |
|---|---|
| Launch | `launch.rs` как Enter: `record_launch` + `launch` + `launcher::close` |
| Desktop Actions | `entry.actions` (T265-A); **submenu** `PopupMenu::submenu`, секция опущена когда `actions` пусто |
| Add/Remove favorite | `launcher.toml` `favorites.order` (T265-C) — toggle |
| Pin / Unpin dock | `dock.toml` — ровно прежний pin-код |
| Hide from list | `launcher.toml` `hidden` — toggle id, **не** правка `.desktop` |
| Show in file manager | `xdg-open` каталога `.desktop` (fallback — каталог exec) |
| Properties | **disabled** `"Properties — no dialog in kit yet"` (T246) |
| Launch as other user | **disabled** `"Launch as other user — no pkexec backend"` (T246) |

Ни одного `.unwrap()` на launch/hide — все пути `if let Err` + `tracing::error!`.

### 2. `launcher_config.rs`

- Поле `hidden: Vec<String>` (top-level ключ `hidden = [...]`; RMW-запись теперь
  заменяет 4 ключа — `favorites/recents/folders/hidden`, неизвестные секции
  сохраняются).
- **Сигнал изменений** `subscribe()` (`futures_signals::Mutable<()>`, bump в
  `update()`). Лаунчер подписан: Hide/favorite/folder-операции мгновенно
  перерисовывают сетку/секции без рестарта.

### 3. `favorites.rs`

- `desktop_file_path(id, dirs)` (для Show-in-FM); `desktop_mtime` переиспользует его.

### 4. `view.rs`

- Правый клик зовёт `app_menu::open(..., entry.clone())` (клетка сетки и
  section-cell — оба).
- `raw_entries` + `apply_hidden_filter()`: hidden-id выкидываются из `all`/поиска/
  секций; `[hidden]` → нет в сетке, остаются в сервисе (T265-G).
- Подписка на `launcher_config::subscribe()` → re-filter + пересчёт секций.
- Убран неиспользуемый `DragFrom` (target решает действие, payload только id).

## Verification

| Command | Result |
|---|---|
| `cargo test -p chronos --lib launcher` | **47 passed; 0 failed** (было 39 → +8: hidden-roundtrip, desktop_file_path, 6 menu unit-тестов) |
| `cargo test -p chronos --lib` | **522 passed; 0 failed** |
| `cargo build --release -p chronos` | **bin BLOCKED чужим WIP (T295 calendar)** — см. ниже |

Юниты спеки на месте:
- **Pin vs Unpin** — `pin_label(false/true)` = "Pin to dock"/"Unpin".
- **Hide пишет id в hidden** — `toggle_hidden_in` добавляет/убирает id.
- **action id мапится на exec** — фикстура `entry_with_actions()`, `NewWorkspace` → `/usr/bin/zed --new`.

**Про release.** lib собирается и 522 теста зелёные. bin сейчас не компилится из-за
**параллельной T295** (calendar popup): 7 ошибок в `calendar_popup/` +
`bar/widgets/{clock,mod}.rs` (`gpui_component::Calendar` не резолвится, `Entity`
не в scope, `match`-arms). Это не моя зона, файлы не трогал — мои launcher-файлы
в выводе сборки фигурируют только в warnings до чистки, после чистки — ни одной
строки. Релизную сборку перегнать после того, как T295 доведёт calendar/clock
(или принимать в ворктри).

## Что НЕ сделано (Архитектор / дальше)

1. **Live grim** (спека): rest/hover меню; Launch; Desktop Action живого
   приложения (если есть); favorite toggle; hide → исчез из сетки; pin →
   `dock.toml`; якорь не мимо catcher. Требует живого шелла. Отдельно глянуть
   живьём: **Launch из меню закрывает и лаунчер, и меню** (меню — AnchoredPopup с
   parent=лаунчер; порядок `launcher::close` → `DismissEvent` → `close_this`
   должен уложиться без `window not found` — юнитом не ловится).
2. **Перенос в `done/` + статус родителя T265** — самоприём не делаю.
3. **Свойства** — disabled честно, диалог/панель не делал (в ките нет; спека
   разрешает disabled + причина).

## Отчёт одной строкой (выборы из спеки)

- Меню — **одно**, `app_menu.rs` (pin_menu переименован), не второй стек.
- Hidden — **top-level `hidden = [...]`** в `launcher.toml`, не `[hidden]`-таблица
  и не `.desktop` на диске.
- Desktop Actions — **submenu**, опускается при пустом `actions`.
- Disabled-пункты — причина в тексте лейбла (T246).

## Коммит

```
feat(launcher): app context menu and desktop actions (T265-D)
```

(6 files: `app_menu.rs` новый / `pin_menu.rs` удалён, `favorites.rs`,
`launcher_config.rs`, `mod.rs`, `view.rs`. `Cargo.lock`, `Source/gpui/`,
`tray_menu/**` — не тронуты.)
