# T144 заход 4 — дропдаун под пальцем

## Что сделано

Единственная оставшаяся работа по Т144 — на UI-слое в `composer.rs`.
Сервисный слой не троган.

### Найденные баги в `composer.rs`

1. **Нет скролла в дропдауне модели.** 288 элементов уходят за экран — дропдаун
   (`div().absolute().bottom(px(26))`) рисуется без `overflow_y_scroll()` и
   без `max_h()`. Фикс: добавил `overflow_y_scroll()` + `max_h(px(300))` на
   `#composer-model-dropdown` (13-14 видимых элементов, остальное скролл).
2. **Та же проблема в дропдауне режимов.** Хотя режимов обычно 1-3, для
   единообразия добавил `overflow_y_scroll()` + `max_h(px(300))` и на
   `#composer-mode-dropdown`.

### Что уже работало (не трогал)

- **Подсветка текущей модели** — `.when(is_active, |el| el.bg(theme.border.default))`
- **Обработка клика** — `on_click` закрывает дропдаун, обновляет
  `composer_selected_model` и шлёт `client.set_model(&model_id)` (→ сервисный
  слой, где уже стоит `UntypedMessage::new("session/set_model")`)
- **Escape закрывает дропдаун** — `handle_composer_key`
- **Клик на textarea закрывает дропдаун** — `on_click` на `#composer-input-canvas`

### Потенциальная проблема: overflow_hidden на thread-column

`panel.rs:253` — `thread-column` имеет `.overflow_hidden()`. Дропдаун с
`max_h(300)` в типовом сценарии помещается (panel высотой ~1000px, composer
внизу, дропдаун 300px вверх — без пересечения границ). На очень мелких
панелях хвост дропдауна обрежется thread-column — приемлемо.

## Верификация

- `cargo build --release -p chronos` — 0 ошибок
- **Live smoke (клик) не запущен** — нет доступа к Wayland-сессии. За
  архитектором: открыть панель, раскрыть список (288 моделей со скроллом),
  кликнуть другую модель, проверить `Sending session/set_model` в логе и
  следующий ход на выбранной модели.

## Изменённые файлы

- `crates/app/src/side_panel_left/composer.rs` — +`overflow_y_scroll()` +
  `max_h()` на model и mode dropdown

## Коммит

```
acp : composer model dropdown scroll + max height
```

---

## Приёмка архитектора (2026-07-28) — ПРИНЯТО, T144 ЗАКРЫТА

Коммит `a44e9bd`, четыре строки в `composer.rs`, зона соблюдена,
`docs/HANDOFF.md` не тронут. Отдельно засчитано: «Live smoke (клик) не запущен —
нет доступа к Wayland-сессии, за архитектором» — ровно та форма, которая
принимается. Никаких выдуманных кадров.

### Живой прогон архитектора (release, ydotool + grim)

Шелл: `target/release/chronos`, слой `side_panel_left` 0,30 352x1410
(`hyprctl layers`). Клик синтезирован `ydotool` (демон поднят на время
прогона и остановлен после).

**1. Список раскрывается и не уезжает за экран.** Кадр
`docs/orchestration/tasks/notes/T144-dropdown-open.png` (снят `grim -g "0,30
352x1410"`): дропдаун открыт над композером, высота ограничена ~300 px,
видно ~9 строк из 288, элемент под курсором подсвечен. Нижняя граница
списка не пересекает композер, верхняя не уходит в тред.

**2. Выбор модели доходит до агента.** В логе прогона:

```
INFO  Sending session/set_model  model_id=nous:anthropic/claude-opus-4.7
DEBUG → {"jsonrpc":"2.0","method":"session/set_model",
         "params":{"sessionId":"bcb36d09-…","modelId":"nous:anthropic/claude-opus-4.7"}}
INFO  session/set_model OK
```

**3. Агент действительно переключается**, а не отвечает `Ok` в пустоту
(та самая проверка эффекта вместо кода возврата, см. скилл
`hermes-acp-tool-completed`):

```
$ grep -aoE "model switched to \S+ via provider \S+" <лог> | sort | uniq -c
      1 model switched to anthropic/claude-opus-4.7 via provider nous
      1 model switched to ~openai/gpt-mini-latest via provider nous
      1 model switched to z-ai/glm-5-turbo via provider nous
      1 model switched to tencent/hy3:free via provider nous
```

**4. Следующий ход идёт на выбранной модели:**

```
$ grep -a "turn START" <лог>
11:58:01  model=nous:anthropic/claude-opus-4.7  text_len=2
$ grep -aoE "provider=\S+ base_url=\S+ model=\S+" <лог> | sort | uniq -c
     12 … model=anthropic/claude-opus-4.7
     11 … model=tencent/hy3:free
```

Четыре разных модели за прогон, каждая доехала до агента и до турна.

**Оговорка о чистоте прогона:** часть кликов и один-два промпта в этом же
логе — от пользователя, работавшего в панели параллельно (промпты `"hi"` и
текст на 76 символов не мои). На вывод это не влияет: улики выше не зависят
от того, чей палец нажал — важно, что путь UI → `session/set_model` → смена
модели у агента отработал четыре раза подряд.

### Незакрытых хвостов по T144 нет

Ч1 (перехват), Ч2 (`SharedModels` + `#301`), D6 (`session/set_model`),
Ч3 (дропдаун под пальцем) — все закрыты и проверены живьём. Задача уходит
в `done/`.

### На будущее, не блокер

288 элементов рисуются обычным `.children(...)`, без виртуализации: каждый
кадр с открытым списком строит 288 `div` с листенерами. На глаз лагов нет,
но если список начнёт тормозить — смотреть в сторону `uniform_list`, а не
искать причину в другом месте.
