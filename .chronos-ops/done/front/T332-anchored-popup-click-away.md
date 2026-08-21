# T332 — Sound/Calendar: click-away и singleton

**Роль:** FRONTEND. **P1.** Живая находка T325 B1.
**Не стартовать, пока T329 в `active/front/`** — оба правят
`calendar_popup/mod.rs`. T331 параллелен T329, этот — нет.
**Зона:** `crates/app/src/volume_popup/mod.rs`,
`crates/app/src/calendar_popup/mod.rs` (open/close/state, не вёрстка
сетки), `crates/app/src/start_menu/mod.rs` (`open` — dismiss чужих
попапов), `crates/app/src/edit_mode.rs` (`toggle` — то же).
**Не трогать:** `popup_click_catcher.rs` API (уже есть), `grab: true`
(T264), заливка календаря (T329), `volume_popup/view.rs` карточка,
tray/dock меню (у них catcher уже есть).

## Зачем

Sound и Calendar — anchored popup с `grab: false` и без
`popup_click_catcher`. Клик мимо их не закрывает; они живут друг над
другом, над Start и над edit-mode. T264 запретил compositor grab
(Hyprland 0.56 держит seat после destroy и убивает мышь до relogin);
dismiss обязан быть наш. Tray/dock/start уже так сделаны — эти двое
забыты.

Кадры T325: `dump/qa-ux/T325/frames/14-volume-then-calendar.png`,
`15-volume-calendar-outside-click.png`, `16-volume-then-start.png`,
`18-volume-then-edit-mode.png`, `31-calendar-outside-click-stays.png`.
`layers-14-volume-then-calendar.txt` — namespace
`chronos-popup-click-catcher` нет, пока открыты оба.

## Корень (сверено)

- `volume_popup/mod.rs:131` `grab: false`; `VolumePopupState` — только
  `handle`/`watcher`, поля catcher нет. Коммент модуля врёт: «no
  close-on-focus-loss (only explicit toggle / ✕)» — это дыра T264,
  не фича.
- `calendar_popup/mod.rs:101` то же.
- Образец: `tray_menu/mod.rs` `TrayMenuState.click_catcher` +
  `open_click_catcher` → `popup_click_catcher::open_for_popup`
  (`tray_menu/mod.rs:289-300`); dock `context_menu.rs:192-201`.
- `start_menu` ловит клик мимо своим catcher (`mod.rs:123-134`);
  volume при этом не закрывает, потому что никто его не зовёт.

## Что сделать

1. У volume и calendar: поле `click_catcher: Option<AnyWindowHandle>`,
   открывать `open_for_popup` вместе с окном, закрывать catcher на
   каждом close-пути (toggle, ✕, `close()`, destroy). Handler —
   как у трея: `close_from_click_catcher`.
2. Singleton бара: `volume_popup::open` закрывает calendar;
   `calendar_popup::open` закрывает volume. Два catcher'а сразу —
   хуже, чем сейчас.
3. `start_menu::open` и `edit_mode::toggle` (вход в edit) закрывают
   volume и calendar. Не наоборот: Start сам с catcher, не ломай его.
4. `grab` остаётся `false`. Тест на отсутствие grab у volume уже
   есть у трея (`anchored_popup_does_not_request_a_compositor_grab`) —
   зеркало для volume/calendar, если его нет.
5. Коммент `volume_popup/mod.rs` про «only explicit toggle / ✕»
   поправить: dismiss = click-away / Escape / toggle / ✕.

`KeyboardInteractivity::Exclusive` запрещён. `let _ =` на fallible
в новом коде — нет (в `popup_click_catcher.rs:69` уже есть — не
размножать).

## Готово когда

Живой release-прогон, grim в отчёт (`dump/` не `/tmp`):

- Sound открыт → клик `(1400,100)` (ydotool `/2`) закрывает Sound.
  `hyprctl layers -j` на время открытия содержит
  `chronos-popup-click-catcher`.
- Calendar то же.
- Sound → клик по часам: Sound закрыт, Calendar один.
- Sound → Start: Sound закрыт, Start с catcher как сейчас.
- Sound → `toggle-edit-mode`: Sound закрыт.
- Клик **внутри** карточки Sound не закрывает её (дыра catcher'а).
- `grab` в `window_options` обеих — `false`.

`cargo test -p chronos --lib` не краснеет.

**Отчёт:** `.chronos-ops/reports-fresh/T332-anchored-popup-click-away-report.md`
