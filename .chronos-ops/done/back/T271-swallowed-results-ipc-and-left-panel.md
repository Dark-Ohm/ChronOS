# T271 — проглоченные `Result` в IPC (`let _ =`)

**Статус:** OPEN. Блокеры сняты (T285 STOP, T286 / T287-A/B/C в `done/`).
Можно параллелить с T298: зона T298 — `composer.rs` / `workspace_view.rs`,
сюда лезть нельзя.
**Нужен:** да. В `ipc/` по-прежнему глотается каждый `cx.update` и
почти каждый `sender.send`. Правило из `CLAUDE.md` живое; это тот же
класс, что прятал ghost-window. Не срочно, не фича — гигиена, пока не
сгниёт в следующий раз.
**Приоритет:** P2.
**Роль:** BACKEND. Зона **только** `crates/app/src/ipc/`.
**Правило:** никогда не глушить ошибку через `let _ = fallible_call()`.

## Замер 2026-08-16 (не 2026-08-13)

| Файл | Живых `let _ =` | Природа |
| --- | --- | --- |
| `crates/app/src/ipc/mod.rs` | 16 | все — `cx.update(...)` (плюс arm `toggle-start-menu` после T265-H) |
| `crates/app/src/ipc/service.rs` | 18 прод + 2 в `#[cfg(test)]` | Drop/acquire teardown; `sender.send` в `accept_loop` |

Тестовые два `remove_file` в `second_acquire_on_same_path_becomes_secondary`
не чистить.

**`side_panel_left/**` — запрещён.** Там свои `let _ =` (chat/composer).
Не этот тикет. После левого фронта — отдельно, по касанию файла.

Остальные ~90 мест по дереву — не эта задача.

## Что делать с каждым случаем

По смыслу, не шаблоном:

1. **`?`** — функция `Result`, ошибка едет выше.
2. **`if let Err(e) = … { tracing::warn!(…); }`** — результат осознанно
   игнорируется, нужен след. **Дефолт для `cx.update` и `sender.send`.**
3. **Явный `match`** — у ошибки своя логика (сокет уже снят — норма).

`.log_err()` из `gpui_ce_util` в крейт `chronos` **не подключён**
(в дереве есть только комментарии в `osd/mod.rs`). **Не тащить
`gpui_ce_util` ради этого тикета.** Не выдумывать хелпер на весь крейт.

`ipc/service.rs`: teardown (`remove_file`, `flush`/`shutdown`) — warn
на debug/info, не `?`. `sender.send` — «получатель умер»: в лог,
иначе IPC-команда исчезает молча.

**Запрещено:** `.unwrap()` / `.expect()`. Паника ≠ исправление.

## Верификация

- `rg -n 'let _ = ' crates/app/src/ipc/` → ноль вне `#[cfg(test)]`.
- Не гребть `side_panel_left`.
- `cargo test -p chronos --lib --bins` — без снижения числа тестов.
- `cargo check -p chronos` — без новых предупреждений.
- Живой смок: `chronos-ipc toggle-launcher`, `toggle-start-menu`,
  `toggle-side-panel-right`, `select-tab:system`, `toggle-theme`.

В отчёте: сколько ушло в `?` / `warn` / `match`.

## Нельзя

- `side_panel_left/**`, `composer.rs`, `workspace_view.rs`, `tabs/chat.rs`.
- `tray_menu/**`, `dock/context_menu.rs`.
- Остальные ~90 мест. `clone()` «по пути» — нет.
- Dependency на `gpui_ce_util` / `Source/`.

## Коммит

`ipc : не глушить Result — warn вместо let _ (T271)`
