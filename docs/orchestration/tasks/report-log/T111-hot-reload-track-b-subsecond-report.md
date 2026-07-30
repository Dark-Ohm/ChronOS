# T111 Report — Track B (subsecond) hot-reload bake-off

**Date:** 2026-07-24  
**Agent:** GLM  
**Worktree:** `/home/neo/projects/chronos-ecosystem/ChronOS-wt-hotreload-b` (branch `spike/hot-reload-track-b`)

---

## 1. Разведка (spec step 1)

### Актуальное состояние `subsecond` (crates.io / GitHub DioxusLabs)

| Пункт | Значение |
|---|---|
| Версия на crates.io | `0.8.0-alpha.0` (опубликовано 19 May 2026) |
| Репозиторий | `github.com/DioxusLabs/dioxus/tree/main/packages/subsecond` |
| Основной API | `subsecond::call(|| ...)` — безопасная обёртка для горячего вызова функции |
| Патчинг | Требует внешнего раннера (Dioxus CLI `dx serve --hotpatch` или `cargo-hot`) |
| ThinLink (линкер) | Встроен в Dioxus CLI, **недоступен как standalone** |
| Workspace support | Только «tip crate» (крейт с `main.rs`); зависимости workspace игнорируются |

### Два кандидата раннера

| Раннер | Статус | Пригодность для Track B |
|---|---|---|
| **Dioxus CLI (`dx serve --hotpatch`)** | Работает, но тянет весь Dioxus toolchain, требует `dx` binary, feature-splitting под Dioxus conventions | **Запрещён спекой** (п.2: «Не затягивай `dx`/Dioxus CLI целиком») |
| **`cargo-hot` (hecrj/cargo-hot)** | 86 ⭐, v0.1.1, помечен «Very broken! Will eat your laundry!» | **Слишком нестабильный**, не поддерживает workspace tip-crate паттерн |

**Вывод разведки:** Ни один существующий раннер не удовлетворяет критерию «без Dioxus-тулинга, минимальные средства». `subsecond` сам по себе — только jump-table runtime; для загрузки патчей нужен отдельный процесс, реализующий протокол (WebSocket + ThinLink). ThinLink недоступен отдельно.

---

## 2. Попытка интеграции в `crates/app` (spec step 2)

### Изменения
1. `crates/app/Cargo.toml` — добавлен `subsecond = { version = "0.8.0-alpha.0", optional = true }` + feature `hot-reload`
2. `network.rs` — render-логика вынесена в `render_network_widget`, обёрнута в `subsecond::call(|| ...)` под `#[cfg(feature = "hot-reload")]`
3. `main.rs` — добавлен `subsecond::register_handler` для приёма патчей

### Результат сборки с `--features hot-reload`

```
error: usage of an `unsafe` block
   --> crates/app/src/main.rs:72:47
    |
72  |                     if let Err(e) = subsecond::apply_patch(jump_table) {
    |                                               ^^^^^^^^^^^^^^^^^^^^^^^^^^
    |
    = note: requested on the command line with `-D unsafe-code`

error: usage of an `unsafe` block
   --> crates/app/src/main.rs:73:41
    |
73  |                         tracing::error!("Failed to apply hot-reload patch: {e}");
    |                                         ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
    |
    = note: requested on the command line with `-D unsafe-code`
```

### Причина
Workspace `Cargo.toml:27` — `unsafe_code = "deny"`.  
Функции `subsecond::get_jump_table` и `subsecond::apply_patch` — **pub unsafe**. Их вызов из `crates/app` требует `unsafe {}` блоков в коде приложения.

---

## 3. Вердикт по треку B (spec step 3)

> **Track B не завёлся минимальными средствами.**  
> **Причина:** `subsecond` экспортирует публичный unsafe API для применения патчей (`get_jump_table`, `apply_patch`). Вызов их из `crates/app` требует `unsafe` блоков в коде шелла, что нарушает политику workspace (`unsafe_code = "deny"`) и условие брифа: *«unsafe, если есть, инкапсулирован внутри самого subsecond»*.

Это не техническая деталь и не обходится добавлением `#[allow(unsafe_code)]` — это архитектурное ограничение текущего API `subsecond` 0.8.0-alpha.0. ThinLink (линкер, генерирующий jump-table) доступен только внутри Dioxus CLI; standalone-интеграция без Dioxus-тулчейна невозможна.

**Track A побеждает автоматически по спеке.**

---

## 4. Артефакты в ворктри

Ветка `spike/hot-reload-track-b` содержит:
- `crates/app/Cargo.toml` — optional dep + feature flag
- `crates/app/src/bar/widgets/network.rs` — render function wrapped in `subsecond::call`
- `crates/app/src/main.rs` — `register_handler` stub (не компилируется из-за unsafe)

Коммиты **не пушены** в origin (по правилам — архивирует архитектор).

---

## 5. Оценка стабильности (честно)

| Аспект | Оценка |
|---|---|
| `subsecond::call` runtime | Работает, безопасен, jump-table механика прозрачна |
| Доставка патчей (runner) | **Блокер** — нет production-ready standalone runner; `dx` тянет Dioxus, `cargo-hot` сломан |
| ThinLink / линкер | Только внутри Dioxus CLI, не отделяем |
| Workspace support | Только tip-crate (крейт с `main.rs`); патчи в `chronos-services`, `chronos-ui` и др. не подхватываются |
| Unsafe в app-коде | Неизбежный при применении патча — **нарушает workspace policy** |
| Краши/зависания при правках | Не проверено (не дошло до живого прогона) |

**Итог:** Track B в текущем виде (subsecond 0.8) **непригоден** для встраивания в ChronOS без отмены `unsafe_code = "deny"` или форка subsecond с safe-обёрткой над `apply_patch`. Рекомендуется принять Track A (`hot-lib-reloader` + отдельный dylib-крейт `crates/hotview`) как победителя.

---

## 6. PENDING / Не проверено живьём

- Протокол 10 правок (spec §Protocol) — **PENDING** (билд не проходит)
- Время «сохранил → увидел» — **PENDING**
- Краши/зависания/некорректная отрисовка при патчинге — **PENDING**

> Фрод-таблица `rules.md` §31–37: не выдаю синтетические PASS за непроверенное.