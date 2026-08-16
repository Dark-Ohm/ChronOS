# T271 — проглоченные `Result` в IPC (`let _ =`)

**Статус:** OPEN, **не в поле**, пока T285 / T286 / T287-C в `active/`.
**Приоритет:** P2.
**Роль:** BACKEND. Зона **только** `crates/app/src/ipc/`.
**Правило-первоисточник:** `CLAUDE.md` — никогда не глушить ошибку через
`let _ = fallible_call()`.

## Зона (жёстко)

| Файл | Случаев (замер 2026-08-13) | Природа |
| --- | --- | --- |
| `crates/app/src/ipc/mod.rs` | 15 | все — `let _ = cx.update(...)` |
| `crates/app/src/ipc/service.rs` | 17 | cleanup сокета, timeout/flush/shutdown, `sender.send` |

**`crates/app/src/side_panel_left/**` — запрещён.** Там 14+ случаев, в том
числе `let _ = this.update` в спавне `ChatTab::new` (`tabs/chat.rs`) и в
`composer.rs` / `text_input.rs`. Это зона T285 (restore/`load_session`),
T286 (композер, `text_input.rs` уйдёт) и T287-C (chrome). Чистить `let _ =`
параллельно = ghost-window сага налево. Левую панель — отдельным тикетом
после их `done/`, по мере касания файлов.

Остальные ~90 случаев по дереву — не эта задача.

Карта «не пересекается с T263/T266/T269» устарела (2026-08-13). Актуальный
конфликт — левый фронт, не трей.

## Что делать с каждым случаем в `ipc/`

По смыслу, не шаблоном:

1. **`?`** — функция `Result`, ошибка едет выше.
2. **`.log_err()`** — результат осознанно игнорируется, нужен след.
   Дефолт для `cx.update(...)` в `ipc/mod.rs`.
3. **Явный `match` / `if let Err(...)`** — у ошибки своя логика
   (сокет уже снят — норма).

`ipc/service.rs`: teardown (`remove_file`, `flush`/`shutdown`) — `.log_err()`
на debug, не `?`. `let _ = sender.send(...)` (~248–284) — «получатель умер»:
обязано в лог, иначе IPC-команда исчезает.

**Запрещено:** `.unwrap()` / `.expect()`. Паника ≠ исправление.

## Верификация

- `rg -n 'let _ = ' crates/app/src/ipc/` → ноль вне `#[cfg(test)]`.
- Не гребть `side_panel_left`.
- `cargo test -p chronos --lib --bins` — без снижения числа тестов.
- `cargo check -p chronos` — без новых предупреждений.
- Живой смок: `chronos-ipc toggle-launcher`, `toggle-side-panel-right`,
  `select-tab:system`, `set-workspace-mode:gamer`, `toggle-theme`.

В отчёте: 2–3 строки ДО/ПОСЛЕ и сколько ушло в каждый исход.

## Нельзя

- `side_panel_left/**`, `composer.rs`, `text_input.rs`, `tabs/chat.rs`.
- `tray_menu/**`, `dock/context_menu.rs` — не эта уборка.
- Остальные ~90 мест. `clone()` «по пути» — нет.

## Коммит

`ipc : не глушить Result — log_err/? вместо let _ (T271)`
