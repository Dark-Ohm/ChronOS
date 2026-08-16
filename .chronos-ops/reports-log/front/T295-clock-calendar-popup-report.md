# T295 — отчёт приёмки

**Исполнитель:** параллельная сессия. Упёрлась в дневной лимит модели
прежде чем смогла закоммитить — заявила "Коммит: `feat(bar): clock opens
kit Calendar popup (T295)`", но в git его не было вообще, всё висело
незакоммиченным в рабочем дереве.

**Дозакоммичено архитектором**, `87eb0992`, по именам файлов (не через
`git commit -am`, чтобы не задеть чужой одновременный WIP — тот же урок,
что и на T294).

## Сверка с деревом

`WindowHandle<Root>` + `Root::new(view, window, cx).bordered(false)
.bg(transparent_black())` — тот же паттерн, что уже работает в
`tray_menu`/`pin_menu`/`dock/context_menu` (архитектурно не эксперимент).
`calendar_popup/view.rs` — `Calendar` из `gpui-component::time`,
`set_date(Date::Single(Some(today)))` при открытии. Клик по часам —
`canvas` + `Rc<Cell<Bounds<Pixels>>>` для захвата экранных координат,
тот же рецепт, что у volume/updates виджетов. `Cargo.toml` — фича `time`
добавлена на `gpui-component`.

## Мой прогон (не со слов)

```
cargo build -p chronos --bins               → чисто
cargo test -p chronos --lib calendar_popup  → 2/2
cargo test --workspace --lib                → 524 passed, 0 failed (chronos-lib)
cargo build --release -p chronos            → чисто
```

Совпадает с заявленным (`2 passed`, `524 passed, 0 failed`).

## Не сделано

Live grim (клик по часам открывает попап в нужном месте, календарь
кликабелен, закрытие по клику вовне) — не гонялось, за владельцем.

## Вердикт

**Код принят.**
