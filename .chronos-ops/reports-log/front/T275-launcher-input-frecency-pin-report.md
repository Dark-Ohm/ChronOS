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

## Резолюция (заход 3, 2026-08-16)

**Пустой query — корень найден и починен.**

`LauncherView::new` строит вьюху с `results: Vec::new()`, затем синхронно
зовёт `refresh_results()`, который заполняет `self.results`. Но без
`cx.notify()` после этого первая отрисованная рамка показывала пустой сид
(«No matches») — вьюха была построена с пустым `results`, а мутация в `new`
не инвалидировала первичный рендер. Юнит `empty_pattern_returns_all` был
зелёный потому, что `search.rs` (nucleo) тут ни при чём — дыра была в
bind/notify, не в ранжировании. Диагностический лог захода 2 в живом логе
так и не появился (владелец не прогонял тот бинарь) — заменён одно-строчным
фиксом.

Фикс: `cx.notify()` после `refresh_results()` в `new` — тот же канон, что
`LibraryTab::new → set_games → cx.notify()`. Собран release, живьём
(grim): пустой query отдаёт список приложений, не «No matches».

Юниты: `cargo test -p chronos --lib launcher` — 9/9.

**Pin — код в дереве (`180fe884`), живой прогон за владельцем.**

Правый клик по строке → меню → Pin пишет `dock.toml` → иконка в доке →
повтор = Unpin. Синтетически не прогнать: `ydotool` требует sudo (пароль),
а живой стол занят владельцем. Проверить вручную и дописать сюда
`pin PASS`/детали — тогда тикет в `done/`.

## Резолюция (заход 4, 2026-08-16) — pin PASS

`180fe884` (window-local vs output-local anchor split) оказался
недостаточным: `catcher_anchor = window.bounds().origin + event.position`
математически равнялся `event.position` — GPUI `WaylandWindow::bounds()`
никогда не переприсваивается после создания окна (Wayland `xdg_toplevel`
не сообщает клиенту реальную позицию), а лаунчер открывается с
`origin: (0,0)` и центрируется исключительно Hyprland-windowrule
(`center = true`), клиенту невидимо. Дырка catcher'а стояла в левом
верхнем углу вывода независимо от того, что попап реально рисовался в
центре экрана — курсор над видимым меню не превращался в руку, клик по
«Pin» не долетал.

Два фикса поверх `180fe884`:
- `3eeaac18` — дырка catcher'а стала симметричной по Y (`FLIP_Y`, не
  только `FLIP_X`) на случай переворота меню у нижних строк списка.
- `162798b4` — настоящий фикс: `chronos_services::compositor::hyprland::
  window_position("chronos-launcher")` (живой `Clients::get()` по
  Hyprland-сокету) вместо `window.bounds().origin`. Дырка теперь строится
  от реальной экранной позиции окна.

Живой прогон владельцем (release, свежая сборка, PID стартовал ровно в
mtime бинаря): правый клик по строке → меню → курсор над «Pin to dock»
теперь рука → клик пинит иконку в док немедленно (бар подписан на
`DockConfigSignal`, `a4b22e9`) → повтор = честный Unpin. **Pin PASS.**

Юниты: `cargo test -p chronos --bins` 686/686.

Тикет закрыт — pin и empty-query оба живьём подтверждены. → `done/`.
