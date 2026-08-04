---
name: gpui-component-in-chronos
description: Use when wiring, measuring or cutting gpui-component (Longbridge toolkit) in ChronOS — feature gates, the second [patch] block, the Root wrapper Input requires, binary-size measurement methodology, and what the July/July-29 measurements actually cost. Read before adding any component widget or claiming a footprint number.
---

# gpui-component в ChronOS

## Статус решения

**Взят как инфраструктура IDE-панели** (`docs/DECISIONS.log`, 2026-07-29) —
реверс июльского «варианта C». Условие пересмотра было прописано в самом
июльском решении и сработало: строим полноценный Shell-IDE, а `table`,
`tree`, `virtual_list`, `dock`, `form`, `setting`, `select`, `sidebar`,
`tab` руками не пишем.

Живёт как наш крейт: worktree `../Source-wt-component`, ветка
`component/feature-gates`. Своя версия — **0.5.2**, свой `[workspace]`,
членом наших воркспейсов **не делать** (см. ловушки).

## Проводка (рецепт, доказан пилотом `20ee13a`)

Корневой `Cargo.toml` ChronOS:

```toml
[workspace.dependencies]
gpui-component = { path = "../Source-wt-component/gpui-component/crates/ui",
                   default-features = false }

# компонент тянет gpui с zed-URL — одного нашего patch мало, нужен второй
[patch."https://github.com/zed-industries/zed"]
gpui = { path = "../Source/gpui" }
gpui_macros = { path = "../Source/gpui_macros" }
gpui_platform = { path = "../Source/gpui_platform" }
gpui_web = { path = "../Source/gpui_web" }
```

`crates/app/Cargo.toml`: `gpui-component.workspace = true`.
`main.rs`: `gpui_component::init(cx)`.

## Два условия, без которых `Input` не работает

1. **Окно обязано быть обёрнуто в `gpui_component::Root`** — иначе паника
   на `window.root()`. Для layer-shell панели это значит
   `WindowHandle<Root>`.
2. **`KeyboardInteractivity::OnDemand`** — иначе панель не получает
   клавиатурные события вовсе.

Типы: виджет — `gpui_component::input::Input`
(`crates/ui/src/input/input.rs:37`), состояние — `InputState`
(`state.rs:342`), создаётся через `cx.new(...)`. **`TextInput` в этой
версии не существует** — писать по памяти не надо, открывать файл надо.

## Фичи (T156)

`markdown`, `html`, `time`, `chart`, `lsp` сделаны опциональными и
размечены `#[cfg]`. Кровные факты:

- **`lsp` обязан тянуть `markdown`** — LSP hover/diagnostic поповеры зовут
  `TextView::markdown`.
- **Ловушка инспектора:** `lib.rs:12` компилирует `inspector` в любой
  debug-сборке (`any(feature = "inspector", debug_assertions)`), а
  `inspector.rs` использует `lsp_types`. Гейт обязан быть
  `all(any(inspector, debug_assertions), lsp)`, иначе `--release` проходит,
  а `cargo check` падает.
- **`num-traits` приезжает НЕ от фичи `chart`**, а через
  `rust-i18n → serde-saphyr`. Выключение `chart` его из графа не уберёт.
- `chrono` линкуется независимо: он в `[workspace.dependencies]` ChronOS.
  Экономию от выключения `time` в дельту не записывать.

Матрица приёмки (все зелёные на `6118382`): `--all-features`,
`--no-default-features`, и по одной фиче `lsp`/`markdown`/`html`/`time`/
`chart`, плюс release без дефолтов — потому что `debug_assertions` меняет
состав кода.

## Как мерить размер (иначе цифра врёт)

1. **Голая зависимость даёт дельту ≈0.** При `lto = true`,
   `opt-level = "z"`, `strip = true` линкер выбрасывает всё, на что нет
   живых ссылок. Мерить только с настоящим потребителем в дереве рендера,
   у которого есть фокус/данные.
2. **Мерить базу и цель в ОДНОМ каталоге.** Путь сборки вшивается в
   бинарь: замер базы в соседнем worktree (`ChronOS-baseline`) дал
   расхождение 640 байт на ровном месте.
3. **Фиксировать HEAD `../Source`** перед каждой сборкой: шелл собирает
   gpui из общего дерева, и параллельная задача в форке (например RTL)
   сдвигает почву молча.
4. **From-scratch после `cargo clean`.** Инкрементал в июле занизил цену
   вчетверо (`+0.68` против честных `+2.66 MiB`).
5. `cargo tree -p chronos -i lsp-types|html5ever|markdown` — пусто
   («did not match any packages») = гейты дожили до реального графа.

### Измеренное

| Что | Цена |
|---|---|
| весь компонент без гейтов (июль) | +2.66 MiB |
| `Input` с гейтами T156 (29.07) | **+1 822 848 байт = +1.74 MiB** |
| база ChronOS на 29.07 | 22 520 192 байта |

## Фокус InputState через IPC (2026-08-04, T226)

`InputState` (из `gpui_component::input`) реализует `gpui::Focusable` —
`focus_handle(&self, cx: &App) -> FocusHandle`. Но:

1. **`InputState` создаётся лениво при первом рендере в Edit mode.**
   View mode (read-only) не создаёт editor вовсе — `active_tab_focus()`
   вернёт `None`.
2. **Чтобы файл открылся в Edit mode через IPC**, `preview_target()`
   должен ставить `PreviewIntent::Edit` (не `View`). Для нередактируемых
   файлов `resolve_view_mode` принудительно возвращает `View` — безопасно.
3. **Фокус надо деферить.** `select_tab()` вызывает `on_tab_select` →
   `cx.notify()` → GPUI планирует render → editor материализуется →
   через 50ms `active_tab_focus()` возвращает валидный хендл.
   Синхронный вызов сразу после `on_tab_select` — всегда `None`.

Паттерн в `PreviewTab`:
```rust
pub(crate) fn editor_focus_handle(&self, cx: &gpui::App) -> Option<gpui::FocusHandle> {
    self.editor.as_ref().map(|editor| editor.read(cx).focus_handle(cx))
}
```

## Ловушки, оплаченные кровью

1. **Не делать компонент членом `[workspace]`** — ни нашего, ни
   `Source/`. В T155 его добавили, не сняв его собственный `[workspace]`,
   и получили `multiple workspace roots`: `cargo` перестал работать во
   всём `Source/`. Откатывали весь заход.
2. **`LICENSE-APACHE`, `NOTICE`, `Copyright`-заголовки и поля `license`
   неприкосновенны** при любой обрезке. В T155 файл лицензии снесли
   вместе с `README`/`docs`/`themes` — восстанавливали. Крейт под
   Apache-2.0, §4 требует сохранять notice.
3. **Косметику (`cargo fmt`) выносить отдельным коммитом.** T156 в первом
   заходе прогнал fmt по всему крейту: 36 файлов из 54 не содержали ни
   строки `cfg`. Для вендоренного кода это лишние конфликты при каждом
   подтягивании апстрима.
4. **Мерить то, что просили.** Первый заход T157 померил `Button` вместо
   поля ввода и объявил +1.09 MiB ценой компонента. Кнопка тянет тему и
   ядро; масса — в `Input` (17 301 строка против 484 в нашем самописном).
