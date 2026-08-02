# T183 — фича `markdown` в gpui-component не отключается: unconditional use

**Статус:** active, не назначено. **Роль:** Developer (Frontend). Общие
правила — `docs/orchestration/agents/RULES.md`.

## Откуда это

Найдено в T182 (QA-хвост слайса 4) при попытке замерить дельту размера
бинаря без фичи `markdown` — попытка провалилась не таймаутом, а реальным
компиляционным провалом. Архитектор перепроверил находку независимо:
подтверждена.

## Баг

`gpui-component` честно гейтит символ:

```
Source/gpui-component/crates/ui/src/text/mod.rs:36
#[cfg(feature = "markdown")]
pub fn markdown(source: impl Into<SharedString>) -> TextView
```

Но потребитель использует его безусловно, без встречного гейта:

```
crates/app/src/side_panel_right/tab/preview.rs:720
body.child(gpui_component::text::markdown(safe.as_str()))
```

Итог: `cargo build --release -p chronos` с `features = []` на
`gpui-component` в воркспейс-`Cargo.toml` падает —

```
error[E0425]: cannot find function `markdown` in module `gpui_component::text`
 --> crates/app/src/side_panel_right/tab/preview.rs:720:29
```

— то есть фича `markdown` фактически не отключаема, хотя по контракту
(T157/T179) должна быть опциональной и её стоимость (+? MiB к бинарю)
измеримой. Замер T157 для сравнения: `Input` +1.84 MiB, `Table` +199 KB —
для `markdown` числа нет вообще, потому что без неё дерево не собирается.

## Что сделать

1. В `crates/app/src/side_panel_right/tab/preview.rs` (функция вокруг
   строки 720, `render_markdown` судя по соседнему коду) — обернуть вызов
   `gpui_component::text::markdown(...)` в `#[cfg(feature = "markdown")]`
   с альтернативной веткой под `#[cfg(not(feature = "markdown"))]`
   (например, рендер как обычный текст через уже существующий
   `render_text`, который в этом же файле — см. соседнюю функцию
   `render_text` сразу за `render_markdown`).
2. Проверить, зависит ли `crates/app` от фичи `markdown` в своём
   `Cargo.toml` (скорее всего наследуется от воркспейс-дефолта
   `gpui-component`) — если фича должна быть отключаемой независимо от
   `chronos`, потребуется собственный feature-флаг в `crates/app`,
   прокинутый в `gpui-component/markdown`.
3. Собрать **дважды**: `cargo build --release -p chronos` с фичей
   (дефолт) и без (`features = []` на `gpui-component` в воркспейс-
   Cargo.toml, `cargo clean -p gpui-component` перед сборкой без фичи —
   иначе инкрементальный кэш даёт ложный таймаут вместо билда, это уже
   дважды путало QA). Оба билда должны успешно завершиться. Замерить
   `ls -la target/release/chronos` для обоих, дельта в отчёт (сравнение с
   T157: `Input` +1.84 MiB, `Table` +199 KB).
4. Проверить живьём (или хотя бы `cargo test`), что Preview на `.md`-файле
   всё ещё рендерит markdown при фиче включённой (дефолт) — это
   регрессионный риск №1 при добавлении cfg-гейта.

## Зона файлов

`crates/app/src/side_panel_right/tab/preview.rs`. Если фича требует
собственного флага в `crates/app/Cargo.toml` — тоже в зоне. Не трогать
`Source/gpui-component/**` — это чужой воркспейс, у него фича уже
корректно гейтит символ, чинить нечего.

## Отчёт

`docs/orchestration/tasks/report/T183-markdown-feature-gate-report.md`.
Обязательно: два лога сборки (с фичей / без), два размера бинаря, дельта,
подтверждение что markdown в Preview не сломан при дефолтной сборке.

## Коммит

`preview : markdown feature отключаема без падения сборки` (или похожее),
без AI-трейлеров, `git add` поимённо.
