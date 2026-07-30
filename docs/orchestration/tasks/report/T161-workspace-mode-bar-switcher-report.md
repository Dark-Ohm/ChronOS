# T161 — workspace-mode: переключатель и плашка в баре

**Статус:** DONE. **Роль:** FRONTEND. **Ветка:** `feat/workspace-mode-bar` (worktree `../ChronOS-wt-workspace-bar`).

---

## Что сделано

### Виджет переключателя (Task 3 плана)

- `crates/app/src/bar/widgets/workspace_mode.rs` — новый файл, `WorkspaceModeWidget` реализует `BarWidget`.
  - Иконка (`svg().path(mode.icon_path())`) + подпись режима (`mode.label()`).
  - Клик по пилюле → `workspace_mode::toggle(cx)`.
  - Регистрация в `instantiate()` через `mod.rs` и `BUILTIN_NAMES`.
  - Дефолтный правый кластер: `project`, `workspace_mode`, `separator`, …, `clock`.

### Плашка предложения (Task 4 плана, шаги 5-7)

- Рендерится, когда `workspace_mode::pending(cx)` вернул `Some(PendingPrompt)`.
- Три действия:
  - «Да» → `accept_prompt(cx)` — единственный путь к смене режима.
  - «Нет» → `dismiss_prompt(cx, false)` — отказ, режим не меняется.
  - «Не спрашивать» → `dismiss_prompt(cx, true)` — отказ + `PromptPref::Never` в конфиг.
- Все токены темы через `Theme::global(cx)`, hex не хардкодится (спека §11).

---

## Чем доказано

| Команда | Результат |
|---|---|
| `cargo check -p chronos --bin chronos` | ✅ `Finished` (только пред-existing warnings) |
| `cargo test -p chronos --bins` | ✅ `194 passed; 0 failed` |
| `cargo build --release -p chronos` | ✅ `Finished [optimized]` |

**Живой кадр:** не проверял, за QA. Путь к несуществующему SVG молча рисует пустоту, а плашка может уехать за край кластера — оба случая видны только на grim.

---

## Используемые иконки

| Режим | Иконка | Источник |
|---|---|---|
| Developer | `icons/rail-editor.svg` | разведка T159 (кандидат №1) |
| Gamer | `icons/bolt.svg` | разведка T159 (кандидат №1) |

Оба файла подтверждены на диске: `ls crates/app/assets/icons/{rail-editor,bolt}.svg` — оба существуют. `code.svg`/`gamepad.svg` в дереве по-прежнему нет — замена на кастомные SVG, когда появятся.

---

## Пришлось ли править тест дефолтного лэйаута

Да. Тест `default_matches_historical_builtin_order` в `layout_config.rs` сверяет дефолтный правый кластер поимённо. Добавлен `"workspace_mode"` после `"project"` — обновлен ожидаемый вектор:

```rust
vec![
    "project", "workspace_mode", "separator", "volume", "network",
    "tray", "updates", "system", "notification_bell", "separator",
    "battery", "clock",
]
```

---

## Несколько независимых `on_click` (вопрос 4 разведки)

**Прецедента в дереве нет** — это первый виджет бара с тремя отдельными `div` + `on_click` в одном `render()`:
- `div#workspace-mode-prompt-yes` → `accept_prompt`
- `div#workspace-mode-prompt-no` → `dismiss_prompt(false)`
- `div#workspace-mode-prompt-never` → `dismiss_prompt(true)`

Каждый `div` получает уникальный `.id()` — без этого GPUI хеширует контент и может слить обработчики. Ближайший прецедент (`dock.rs`) — один элемент с двумя типами событий, не то же самое.

**Event bubbling:** в GPUI `on_click` на дочернем `div` не всплывает к родительскому — подтверждено поведением dock.rs. Пилюля и плашка — siblings в одном `row`, не parent-child, поэтому клик по «Да» не может триггерить `toggle`. `stop_propagation()` не потребовался.

**Это НЕ проверено живьём** — требуется grim-кадр с кликом по «Да» и проверкой, что режим не переключился дважды.

---

## Что НЕ сделано

- Живой прогон shell + grim-скриншот бара — за QA/архитектором.
- Проверка event bubbling на живом кадре — за QA.
- В edit-mode плашка предложения + стрелки ◀▶ могут быть тесными — визуально проверить на кадре.
- Замена `rail-editor.svg`/`bolt.svg` на кастомные `code.svg`/`gamepad.svg` — когда SVG появятся в дереве.

---

## Зона файлов — граница соблюдена

| Файл | Статус |
|---|---|
| `crates/app/src/bar/widgets/workspace_mode.rs` | создан |
| `crates/app/src/bar/widgets/mod.rs` | правка (mod + instantiate) |
| `crates/app/src/bar/layout_config.rs` | правка (BUILTIN_NAMES + default + test) |

Не тронуты: `workspace_mode.rs` (T160), `ipc/**`, `side_panel_left/**`, `side_panel_right/**`.
