# T304 — `TabContent::create` обобщить до `&mut App`

**Роль: FRONTEND.** Предварительный тикет для T305 (control-center
popup). **T305 не стартует, пока T304 не принят** — общий файл
`tab/mod.rs`, параллелить нельзя.

## Контекст

`TabContent::create` (`crates/app/src/side_panel_right/tab/mod.rs:78`)
сейчас:

```rust
pub(crate) fn create(
    tab: PanelTab,
    cx: &mut Context<crate::side_panel_right::view::SidePanelRightView>,
) -> Self
```

Жёстко типизирован на `Context<SidePanelRightView>` — вызвать его из
будущего popup-хоста (не являющегося `SidePanelRightView`) невозможно.
`cx.new(...)` внутри тела работает на любом `&mut App`/`&mut Context<T>`
через `AppContext`, так что обобщение чисто механическое.

## Задача

1. Сменить сигнатуру на `cx: &mut App`. Пройтись по телу — все
   `cx.new(|cx| ...)` вызовы переживут смену без изменений (метод
   доступен на `&mut App` так же, как на `&mut Context<T>`).
2. Обновить оба call site (`side_panel_right/view.rs` — докнутый
   рейл; новых пока нет, T305 добавит свой) под новую сигнатуру —
   докнутый путь передаёт `cx` из своего `Context<SidePanelRightView>`
   (авто-deref/reborrow до `&mut App`, либо явный `cx.as_mut()` —
   смотреть по факту, что требует компилятор).
3. `tracing::info!(tab = tab.label(), "side_panel_right: lazy-create
   tab view")` (tab/mod.rs:82) — строка врёт после обобщения (не
   всегда `side_panel_right` вызывающий). Переформулировать нейтрально
   (например `"tab: lazy-create tab view"` без модуля-владельца, или
   передавать источник параметром) — мелкая правка, но в рамках
   тикета, не потом.
4. **Инвариант, который T305 обязан унаследовать без пересмотра**:
   `TabContent` остаётся **одним общим enum-реестром** — этот тикет
   НЕ режет его на «рейловые» и «popup» варианты. Все текущие
   варианты (System/Files/Terminal/.../AcpSettings/HyprBinds/...)
   остаются как есть; меняется только *кто* их создаёт. `PanelTab::ALL`
   (что показывает рейл) и множество вариантов `TabContent` (что
   умеет создаваться) — два разных множества, это разъединение живёт
   в T305, не здесь.
5. `cargo check`/`cargo test --lib` — существующие тесты `tab/mod.rs`
   и `side_panel_right` не должны измениться по смыслу (сигнатура
   внутренняя, `pub(crate)`, наружу не течёт).

## Зона файлов

`crates/app/src/side_panel_right/tab/mod.rs` — только сигнатура
`create` и её тело/call site в `view.rs` (сам вызов, не остальная
логика `view.rs`). Не трогать `PanelTab`/`tabs.rs`/`power_row.rs` —
это T305.

## Отчёт

`.chronos-ops/reports-fresh/T304-tabcontent-create-generalize-to-app-report.md`
