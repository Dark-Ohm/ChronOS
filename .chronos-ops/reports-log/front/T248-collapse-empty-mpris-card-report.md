# T248 — свернуть пустую «No player» mpris-карточку: отчёт

**Дата:** 2026-08-05
**Статус:** закрыт.
**Роль:** FRONTEND (Rust, GPUI).

## Решение

`render_mpris_card()` разветвлён по `state.has_player`:
- **нет плеера** → `render_no_player_card()` — компактная карточка высотой
  ~56px: приглушённая mpris-иконка (14px) + строка «No player»
  (`display_title(state)` — единый источник истины, по ревью), без
  art-фрейма/progress/transport. `rounded(9)` + `surfaces::card` + border
  как у полной карточки.
- **есть плеер** → полная карточка без изменений (условный рендер, без
  анимации — по тикету приемлемо).

Ранний return потребовал конкретного типа `gpui::Div` (opaque
`impl IntoElement` в двух ветках — E0308), текст — owned String
(заимствованный `&str` в возвращаемом Div — E0521).

## Верификация

- **Без плеера, обе темы (живой grim):** чёрный art-фрейм отсутствует
  (0 чёрных строк в полосе y135..335, где раньше был 198px чёрный блок).
  Компактная карточка ~56px вместо ~330px.
- **С реальным плеером:** MPRIS-мок (dbus-python, имя
  `org.mpris.MediaPlayer2.chronosmock`, Playing + Metadata) — 154 чёрные
  строки в той же полосе = полный art-фрейм на месте, регрессии нет.
  (На машине нет настоящего MPRIS-плеера: mpv без mpris.lua.)
- Юнит-тест на вариант рендера невозможен без живого окна
  (`Theme::global(cx)`) — честно помечено, доказательство — живой grim.
  `no_player_shows_placeholder_title` (display_title) зелёный.
- `cargo test --release -p chronos --lib -- side_panel_right`: **167/167**.
- `cargo build --release -p chronos`: чисто.

## Коммит

`ui : collapse empty mpris card when no player (T248)`
+ `orchestration : T248 closed — report + ticket moved to done/`.
