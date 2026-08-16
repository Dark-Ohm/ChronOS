# T205 report — Editor: themed buffer + line gutter

**Отчёт:** 2026-08-02. **Зона:** preview.rs / surfaces.rs / theme_config.rs / main.rs.
**Коммит:** `editor : themed buffer + line gutter (T205)`.

## Theme / Input approach

Корень «белого прожектора»: gpui-component `Input` по умолчанию красит себя из СВОЕГО
`theme::Theme` (Light default от `gpui_component::init`, main.rs:79). chronos_ui::Theme
глобал на это не влиял.

1. **`surfaces::editor(theme)`** (surfaces.rs) — явный surface-токен буфера:
   dark → `bg.primary` (тот же, что тело панели → буфер читается единым листом,
   не «листом A4»); light → `bg.secondary` (мягкая бумага, не glare pageBg).
   + 2 assert'а в тестах поверхностей (dark/light).
2. **Themed `Input`** (preview.rs `render_editor_input_body`): на элемент Input
   добавлены `.bg(editor(theme))`, `.text_color(theme.text.primary)`,
   `.font_family(theme.font_mono)`, `.text_size(px(13.))`. Работает, потому что
   `Styled()` применяется ПОСЛЕ встроенного `appearance` bg (Input::render:506
   `.when(appearance, bg)`, затем :513 `.refine_style(&style)`) — мои перекрывают.
3. **Мост gpui-component темы** (theme_config.rs `sync_gpui_component_theme`):
   вызывает `gpui_component::theme::Theme::change(mode, None, cx)` под активный
   chronos_ui::Theme. Без него gutter-числа (`muted_foreground`) и внутренние
   заполнения Input остались бы светлыми даже в тёмном шелле. Вызывается из
   `apply()` (hot-reload), `toggle()`, и в **main.rs после `gpui_component::init`**
   — тот перезаписывает mode на Light default, поэтому синк обязан идти ПОСЛЕ,
   иначе тёмный шелл получит светлый gutter до первого hot-reload.

## Gutter + scroll

`InputState::new(window, cx).code_editor("plaintext")` — встроенный CodeEditor-режим
gpui-component: линия-нумерация `1..N` (paint через `cx.theme().muted_foreground`),
**синк скролла из коробки** (gutter и буфер — один элемент, общий scroll), mono-семья.
`highlighter` оставлен None → синтакс-подсветки нет (non-goal спеки).

## Verification

- `cargo test -p chronos --lib` — **219/219 зелёных** (включая preview 34, surfaces 2
  с новыми `editor()` assert'ами).
- `cargo check -p chronos --bin chronos` — единственная ошибка **вне моей зоны**:
  `E0432 unresolved import crate::bar_settings` в `side_panel_right/tab/bar_settings.rs:9`
  (параллельная T202, untracked WIP, две копии `bar_settings.rs`). Мои theme_config.rs /
  main.rs в выводе ошибок отсутствуют.
- **Заблокировано чужим WIP:** `cargo build --release -p chronos` и bin-тесты
  (включая `theme_config::tests`) не прогоняются, пока T202 не закоммитит бар.
  Прогоню сразу после стабилизации. Код моей зоны к сборке готов — тип-чек прошёл.

## Live smoke — НЕ ПРОВОДИЛСЯ (нужны руки)

Тёмная тема → открыть `.md` → Edit → буфер не белый; номера строк видны; Save;
назад в Preview render. grim опционален. NOT VERIFIED — честно, по правилам задачи.

## Что НЕ сделано

- Syntax highlight — явный non-goal.
- Zed Editor port, LSP, multi-file tabs — non-goal.
- Перенос mapping'а токенов gpui-component 1:1 — осознанно НЕ делал: синк только
  mode (Dark/Light), нейтрали визуально ложатся. Полный mapping — отдельная задача.
- Bar / Files buttons — не касался (T202 / чужая зона).

---

## Приёмка (Lead Architect / Grok, 2026-08-02)

**Вердикт: ACCEPTED WITH RESIDUAL**

**Коммит:** `8b36055` (оформил архитектор — отчёт без SHA; uncommitted T205-only
add: surfaces, preview, theme_config, main).

| claim | check |
|---|---|
| `surfaces::editor` dark primary / light secondary | ✅ + tests |
| Input bg/text/mono override | ✅ render_editor_input_body |
| `code_editor("plaintext")` gutter + sync scroll | ✅ InputState |
| `sync_gpui_component_theme` after init/apply/toggle | ✅ |
| lib tests 219 | ✅ re-ran |
| release build | blocked by T202 dirt (honest); zone compiles |
| live grim | **NOT VERIFIED** |

**Residual:** live smoke dark Edit; T208 Ln/Col after this; full gpui-component
token map out of scope.

**T208 разблокирован.**

