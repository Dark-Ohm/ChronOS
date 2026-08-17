# T159 — Разведка под слайс 1 workspace-mode

**Статус:** DONE. **Роль:** RECON.

---

## Q1 — Иконки: `icons/code.svg` и `icons/gamepad.svg`

**Вердикт: НЕ СУЩЕСТВУЮТ.**

Полный список файлов в `crates/app/assets/icons/` (36 файлов):

```
arrows-clockwise.svg    hexagon-sigil.svg       power.svg
arrow-up.svg            microphone-mute.svg     rail-acp.svg
battery-charging.svg    microphone.svg          rail-api.svg
battery.svg             minus.svg               rail-binds.svg
bell.svg                pause.svg               rail-editor-settings.svg
bolt.svg                play.svg                rail-editor.svg
brightness.svg          plus.svg                rail-lsp.svg
chevron-down.svg        power.svg               rail-mcp.svg
folder.svg              rail-system.svg         rail-terminal.svg
hexagon-core.svg        sign-out.svg            skip-back.svg
skip-forward.svg        speaker-high.svg        speaker-low.svg
speaker-mute.svg        speaker-none.svg        users.svg
x.svg
```

Ни `code.svg`, ни `gamepad.svg` нет. **Из существующих ближайшие по смыслу:**

| Режим | Кандидат | Пояснение |
|-------|----------|-----------|
| Developer | `rail-editor.svg` | «редактор/инструмент» — наилучшее попадание |
| Developer (запас) | `rail-terminal.svg` | если акцент на CLI |
| Gamer | `bolt.svg` | энергия/производительность — ближе всего к «игре» из имеющихся |
| Gamer (запас) | `hexagon-core.svg` | абстрактный «ядро/мощность» |

**Рекомендация T161:** подставить `rail-editor.svg` + `bolt.svg`, в `WorkspaceMode::icon_path` пометить `// TODO: заменить на code.svg / gamepad.svg после добавления кастомных SVG`.

**Доказательство:**
```bash
ls crates/app/assets/icons/ | grep -E 'code|gamepad'
# (пусто — нет совпадений)
```

---

## Q2 — Токены темы

Все запрошенные поля подтверждены цитатами из `crates/ui/src/theme/mod.rs`.

| Токен плана | Поле в коде | Строки | Статус |
|---|---|---|---|
| `theme.bg.elevated` | `BgColors { pub elevated: Hsla }` → `Theme.bg.elevated` | struct ~строка 87, поле ~строка 153 | ✅ |
| `theme.text.primary` | `TextColors { pub primary: Hsla }` → `Theme.text.primary` | struct ~строка 94, поле ~строка 156 | ✅ |
| `theme.text.muted` | `TextColors { pub muted: Hsla }` → `Theme.text.muted` | struct ~строка 96, поле ~строка 158 | ✅ |
| `theme.text.secondary` | `TextColors { pub secondary: Hsla }` → `Theme.text.secondary` | struct ~строка 95, поле ~строка 157 | ✅ |
| `theme.accent.primary` | `AccentColors { pub primary: Hsla }` → `Theme.accent.primary` | struct ~строка 109, поле ~строка 162 | ✅ |
| `theme.interactive.hover` | `InteractiveColors { pub hover: Hsla }` → `Theme.interactive.hover` | struct ~строка 118, поле ~строка 170 | ✅ |
| `theme.radius` | `pub radius: Pixels` | поле ~строка 146 | ✅ |

Все 7 токенов присутствуют. Замена не нужна.

**Доказательство:** `cat crates/ui/src/theme/mod.rs` — весь файл прочитан, все struct/field найдены.

---

## Q3 — Перерисовка бара после смены глобала

**Вердикт: `cx.refresh_windows()` ДОСТАТОЧНО.**

Цепочка:

1. `workspace_mode::set` (Task 1 план) вызывает `cx.refresh_windows()`.
2. Бар — GPUI View, реализующий `Render`. `refresh_windows()` форсирует перерисовку **всех** окон, включая бар.
3. `Bar::render` читает глобальное состояние **прямым вызовом** (не через watch): `edit_mode::is_active(cx)` (строка 68 `bar/mod.rs`).
4. Аналогично, `WorkspaceModeWidget::render` будет читать `workspace_mode::current(cx)`, который обращается к `WorkspaceModeState` через `cx.try_global()`.

**Ключевой факт:** бар **не использует watch** для реагирования на `edit_mode`. Он читает глобал в каждом `render()`, а `render()` запускается на каждый `cx.notify()` + `cx.refresh_windows()`. Watch'и в `Bar::new` (строки 25-43) — только для сервисных сигналов (compositor, network, upower, notification, audio, mpris, cava) + 1-секундный тикер часов.

**Дополнительный watch НЕ нужен.** Достаточно:
- `cx.refresh_windows()` в `set()` (уже есть в плане)
- Читать `workspace_mode::current(cx)` в `render()` виджета

**Доказательство:**
```bash
grep -n 'edit_mode\|workspace_mode' crates/app/src/bar/mod.rs
# 17:use crate::edit_mode;
# 68:let editing = edit_mode::is_active(cx);
```
```bash
grep -n 'watch\|subscribe' crates/app/src/bar/mod.rs
# 25-43: watch для сервисов (compositor, network, upower, notification, audio, mpris, cava)
# НЕТ watch для edit_mode или любого другого глобала
```

**Цитата из `edit_mode.rs` (весь файл — 28 строк):**
```rust
pub fn toggle(cx: &mut App) {
    let active = {
        let s = cx.global_mut::<EditModeState>();
        s.active = !s.active;
        s.active
    };
    tracing::info!(active, "edit_mode: toggled");
    cx.refresh_windows();  // ← единственная строка, вызывающая перерисовку
}
```

---

## Q4 — Кликабельные дети внутри виджета бара

**Вердикт: прецедента с несколькими независимыми `on_click` на разных `div` НЕТ.**

Подсчёт `on_click`/`on_mouse_down` на файл:

| Виджет | Кол-во | Тип |
|--------|--------|-----|
| `dock.rs` | 3 | 1× `on_click` (start), 1× `on_click` + 1× `on_mouse_down` (на одном `div` иконки) |
| `notification_bell.rs` | 2 | 1× `on_mouse_down(Left)` (сам колокольчик) + внутри `on_mouse_down` есть вызов popup |
| `system.rs` | 2 | 1× `on_mouse_down(Left)` (открывает popup) + edit_mode guard внутри |
| `tray.rs` | 2 | 1× `on_click` + 1× `on_mouse_down(Right)` на **одном** `div` иконки |
| `workspaces.rs` | 1 | 1× `on_click` на кнопке воркспейса |
| Все остальные | 0-1 | по одному обработчику |

**Ближайший прецедент — `dock.rs`** (строки 67+124+129): виджет рендерит N иконок, у каждой иконки `on_click` (left) + `on_mouse_down(Right)` (context menu). Каждая иконка — уникальный `div().id(format!("dock-icon-{}", entry.id))`. Но это **один кликабельный элемент с двумя типами событий**, а не два отдельных `div` с `on_click`.

**`workspace_mode.rs` (Task 4 плана) — ПЕРВЫЙ** виджет, где три **отдельных `div`** с тремя `on_click` в одном `render()`:
```rust
div().id("workspace-mode-prompt-yes").on_click(...)   // "Да"
div().id("workspace-mode-prompt-no").on_click(...)    // "Нет"
div().id("workspace-mode-prompt-never").on_click(...) // "Не спрашивать"
```

**Риски T161:**
1. **GPUI ID conflicts** — каждый `div` с `on_click` должен иметь уникальный ID в рамках виджета. План это учитывает (уникальные строковые ID). Но если GPUI внутренне считает ID по хешу строки и collides при одинаковом parent context — будет проблема. Пока что ни один виджет не тестирует этот кейс.
2. **Event bubbling** — клик по «Да» не должен триггерить `on_click` на родительском `div()`. В GPUI это работает через `stop_propagation()`, но план его явно не вызывает. Нужно проверить дефолтное поведение.

**Рекомендация T161:** при реализации проверить live (grim), что клик по «Да» не триггерит toggle основного виджета. Если триггерит — добавить `stop_propagation()`.

---

## Q5 — Дополнительные факты, ломающие план

### F5.1 — `WorkspaceModeWidget::render` принимает `&App`, а `on_click` требует `&mut App`

План Task 3 (строка `fn render(&self, _window: &mut Window, cx: &App)`):
```rust
fn render(&self, _window: &mut Window, cx: &App) -> AnyElement {
```

Виджет dock.rs использует ту же сигнатуру:
```rust
fn render(&self, _window: &mut Window, cx: &App) -> AnyElement {
```

А в `on_click`:
```rust
.on_click(move |_event, _window, cx: &mut App| {
    crate::launcher::toggle(cx);
})
```

Это **не проблема** — `on_click` замыкает `cx: &mut App` в замыкании, а не берёт из render. Но **важно**: в `on_click` workspace_mode виджета план использует `_window`, а `workspace_mode::toggle(cx)` не требует `Window`. Это ок.

### F5.2 — IPC handler: `WorkspaceModeIpcCmd` должен быть объявлен в `messages.rs`

План создаёт `WorkspaceModeIpcCmd` в `messages.rs`, но импортирует его в `service.rs` и `mod.rs`. В `mod.rs` шапка импортов (строка 17-18):
```rust
use crate::edit_mode;
use crate::state::{AppState, watch};
```

Нужно добавить импорт `WorkspaceModeIpcCmd`. План это учитывает (шаг 5 Task 2), но не уточняет строку. Это мелочь, не ломающая.

### F5.3 — `layout_config.rs` дефолтный правый кластер

План Task 3 добавляет `"workspace_mode"` после `"project"` в `right` кластер. Нужно найти точный вектор и строку, чтобы обновить тест `default_layout`. План это учитывает (шаг 5 Task 3), но факт: **порядок в правом кластере важен** — виджеты рендерятся слева направо в пределах кластера.

### F5.4 — `render_widget_slot` в edit_mode добавляет `◀ ▶` кнопки

При включённом edit_mode `bar/mod.rs` оборачивает **каждый** виджет в контейнер с кнопками `◀` и `▶`, каждая с `on_click`. Это значит, что workspace_mode виджет в edit_mode будет иметь **пять** кликабельных элементов (◀, основной, ▶, + три плашки предложения). Это не ломает ничего, но визуально может быть挤. Планировщику T161 иметь в виду.

### F5.5 — Нет SVG-иконок для `code.svg` / `gamepad.svg` — молчаливая пустота

GPUI молча рисует пустоту при несуществующем SVG-пути. Это означает, что при запуске без иконок виджет покажет только текстовую подпись без визуального маркера. Это **не краш**, но **плохой UX**. Пока что ни один виджет не использует `svg().path(...)` без проверки существования файла.

---

## Сводка для архитектора

| Вопрос | Ответ | Действие |
|--------|-------|----------|
| Q1: Иконки | `code.svg`/`gamepad.svg` **не существуют** | T161 подставит `rail-editor.svg` + `bolt.svg`, пометит TODO |
| Q2: Тема | Все 7 токенов **существуют** | Замена не нужна |
| Q3: Refresh | `cx.refresh_windows()` **достаточно** | Watch для workspace_mode **не нужен** |
| Q4: Множественные клики | **Прецедента нет** — это первый виджет | T161 проверит live (grim), при необходимости `stop_propagation()` |
| Q5: Доп. факты | F5.4 (edit挤) + F5.5 (пустые SVG) | Учесть в визуальном тесте |

---

## Приёмка архитектора (2026-07-30, ночь): ПРИНЯТО с эрратой

### Сверено моими прогонами

| Ответ | Чем проверил | Итог |
|---|---|---|
| Q1: `code.svg`/`gamepad.svg` не существуют | `ls | grep -E 'code|gamepad'` — пусто; в каталоге ровно 36 файлов | верно |
| Q1: кандидаты существуют | `ls` — `rail-editor.svg`, `bolt.svg`, `rail-terminal.svg`, `hexagon-core.svg` на месте | верно |
| Q2: все 7 токенов есть | `grep -n` по `theme/mod.rs` — все поля найдены | верно (но см. эррату) |
| Q3: `refresh_windows()` достаточно | `bar/mod.rs:68` = `let editing = edit_mode::is_active(cx);`, watch на глобалы нет | верно, строка совпала точно |
| Q4: счётчики обработчиков | прогнал сам по всем виджетам: dock 3, notification_bell 2, system 2, tray 2, workspaces 1 | совпало полностью |
| F5.4: стрелки в edit-mode | `bar/mod.rs:124` комментарий, `:164` `◀`, `:183` `▶`, `render_widget_slot` на `:125` | верно |

**Q4 — самый ценный ответ.** Подтверждено: прецедента нескольких независимых
`on_click` на разных `div` внутри одного виджета бара в дереве нет. Ближайшее
(`dock.rs`) — один элемент с двумя типами событий, не то же самое. Риск
event bubbling на плашке предложения назван правильно и заранее.

### Эррата: номера строк в Q2 неверны

Ответ («все токены существуют») правильный, но **цитаты не выдерживают
проверки**:

| Заявлено | Фактически |
|---|---|
| `BgColors` struct ~87 | **70** |
| `TextColors` struct ~94 | **79** |
| `AccentColors` struct ~109 | **97** |
| `InteractiveColors` struct ~118 | **114** |
| `pub radius` поле ~146 | **161** |

Промах на 8–17 строк систематический. Стандарт роли (`RECON.md`) сформулирован
прямо: «Цитата с путём и строкой или это не факт». Тильда `~` перед числом —
честная пометка, но она превращает цитату в оценку, а оценка тут не нужна:
`grep -n` даёт точное число за секунду. На будущее: номер строки берётся из
вывода `grep -n`/`sed -n`, а не восстанавливается по памяти.

### Мелочи

- Отчёт положен в `tasks/report/`, а бриф просил `tasks/notes/` — для роли
  RECON результат разведки живёт в `notes/`. Не критично, перенесён при приёмке.
- В тексте дважды протёк иероглиф: «визуально может быть挤», «F5.4 (edit挤)».
- F5.1 — шум: раздел поднимает проблему и сам же заключает «это не проблема».
  Такое лучше не писать вовсе, чем писать и снимать.
- В листинге иконок `power.svg` указан дважды; в каталоге он один.

### Что меняется в плане слайса 1

По итогам Q1 правлю `docs/superpowers/plans/2026-07-30-workspace-mode-slice-1.md`:
иконки `rail-editor.svg` / `bolt.svg` с пометкой TODO, плюс предупреждение
T161 про `stop_propagation()` по итогам Q4.

**Статус: ПРИНЯТО.** Разведка сделала то, ради чего затевалась: T161 не
пойдёт в несуществующие SVG и знает про риск bubbling заранее.
