# T275 — отчёт: Input, frecency, pin, честный футер

**Исполнитель:** FRONTEND (коммит `89dfd25`).
**Приёмка кода:** архитектор, 2026-08-15.
**Live:** каретка — PASS (владелец, 2026-08-15). Пустая выдача и Pin — ещё нет.

## Сверка дерева (не со слов)

`89dfd25` — 8 файлов, `Cargo.lock` нет.

| Часть | Факт |
|---|---|
| A | `launcher/mod.rs`: `WindowHandle<Root>`, `InputState::new`. `view.rs`: `Input::new(&self.input).appearance(false).cleanable(true)`, подписка на `InputEvent::Change`. Своего `pattern.push` нет. |
| A клавиши | `handle_key`: escape/enter/up/down/tab. Enter пишет frecency и `launch`. В ките single-line `enter` делает `cx.propagate()` (`input/state.rs:1668`). `escape` пропагирует, если не IME и не `clean_on_escape` (дефолт `false`; `.cleanable` — крестик, не Esc). |
| B | Комментарий в `render_footer`: tune убран. Греп `tune` в footer — только этот NOTE. |
| C | `crates/services/src/applications/frecency.rs`: `~/.config/chronos/frecency.toml`, half-life 7d, `flush` на `close`/`close_this`. `rank`: пустой query → frecency; иначе nucleo primary. |
| D | `pin_menu.rs`: `PopupMenu` Pin/Unpin, `grab: false`, click-catcher, autoclose 5s, `Root`. |

Twin `pub(crate) mod dock` / `popup_click_catcher` в `lib.rs` — тот же паттерн, что `desktop_terminal`. Не красиво, но в каноне дерева.

Nucleo score через позицию — честно описано; тесты это фиксируют.

## Тесты (этот заход)

```
cargo test -p chronos-services frecency → 5 passed
  empty_query_sorts_by_frecency
  recent_beats_frequent          ← 10 месяц назад vs 2 вчера
  nonempty_query_keeps_relevance_primary
  frecency_breaks_tie_on_equal_relevance
  score_is_zero_for_unknown_app
```

`--lib` / `--bins` целиком не гонял повторно; коммит на HEAD.

## Live (этот заход)

Пересобрал release (mtime 15:44), `chronos-stop && chronos-start` (PID 1820881).
`chronos-ipc toggle-launcher` → клиент `class: chronos-launcher` 720×560 @ 920,455.

Кадр (смотрел глазами, полный DP-1 + кроп):

- Шапка Launcher / APPS, поле поиска, **каретка видна**. Tune в футере нет
  (`luau plugin · hot-reload` + live). Часть A (открылся + Input) и B — ок.
- Список: **«No matches»** при только что открытом лаунчере. Пустой query
  должен отдать приложения (тест `empty_pattern_returns_all` зелёный).
  Живьём — пусто. Часть C на сессии **не доказана**.
- Каретка **рабочая** — владелец, этот заход. Часть A live закрыта.
- Pin в док / пустая выдача — ещё нет.

## Вердикт

Код T275 — принять (frecency 5/5, Input/Root/PopupMenu в дереве).
Pin (D) живьём не работал: якорь меню брал `event.position` в координатах
окна лаунчера, а Overlay click-catcher считает экран. Дырка ложилась не
туда, клик по «Pin» попадал в catcher и только закрывал меню.
`dock.toml` не менялся — подтверждено. Фикс: `window.bounds().origin +
position` (`fix(launcher): pin menu anchor…`).

Тикет **не закрывать**, пока владелец не прогонит Pin на новом release.
Пустой query «No matches» тоже открыт.
