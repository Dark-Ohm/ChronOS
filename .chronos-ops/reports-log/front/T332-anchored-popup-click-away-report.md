# T332 — отчёт: Sound/Calendar click-away и singleton

**Роль:** FRONTEND. **Вердикт:** код + живой release-прогон. Все гейты брифа
пройдены.

## Что сделал

Зона по брифу: `volume_popup/mod.rs`, `calendar_popup/mod.rs`,
`start_menu/mod.rs`, `edit_mode.rs`. Плюс одна строка в `lib.rs`
(обоснование ниже). Вёрстку карточек, `popup_click_catcher.rs` API,
`grab`, tray/dock меню не трогал.

1. **`volume_popup/mod.rs`** — click-away + singleton:
   - Поля `click_catcher: Option<AnyWindowHandle>` и
     `window_closed_subscription: Option<Subscription>` в `VolumePopupState`.
   - `open_click_catcher` через `popup_click_catcher::open_for_popup`,
     handler `close_from_click_catcher` (как у трея).
   - Catcher закрывается на всех путях: `toggle`/`close()`, `close_this`
     (✕), внешний `destroy` (`on_window_closed`) и сам catcher-callback.
   - Singleton: `open()` первым делом закрывает `calendar_popup`.
   - Дыра под расширенным списком устройств: catcher-дырка резервируется
     на `MAX_POPUP_HEIGHT` (= `BASE_HEIGHT + 8*DEVICE_ROW_H`), чтобы
     выросший пикер не вылез из pass-through-дырки.
   - Поправлен коммент модуля (см. «Замечания» про Escape).
   - Новый тест `anchored_popup_does_not_request_a_compositor_grab`.

2. **`calendar_popup/mod.rs`** — зеркально: catcher + `on_window_closed` +
   singleton (`open()` закрывает `volume_popup`) + grab-тест.

3. **`start_menu/mod.rs`** — `open()` закрывает volume и calendar перед
   своим catcher (Start сам с catcher, не трогал).

4. **`edit_mode.rs`** — `toggle()` при входе в edit (`active == true`)
   закрывает volume и calendar.

5. **`lib.rs`** — добавил `pub(crate) mod volume_popup;` (Lib-side twin).
   Без этого `--lib` не собирался: `calendar_popup`/`start_menu`/
   `edit_mode` — модули lib, а `volume_popup` был объявлен только в
   `main.rs` (bin). Тот же twin-паттерн, что у `calendar_popup`/`start_menu`.

`let _ =` на fallible в новом коде не добавлял — все новые
`remove_window` обёрнуты в `if let Err(e) = … { tracing::warn!(…) }`.
`open_click_catcher(...).ok()` — та же best-effort идиома, что у
`tray_menu`/`dock` (catcher — улучшение, при его отсутствии попап просто
живёт без click-away).

## Как проверил (компиляция + тесты)

- `cargo check -p chronos --tests` → **0 ошибок** (только предсуществующие
  warnings: `close_this` неиспользуемый в `calendar_popup/view.rs`,
  `CalendarEvent` и т.п. — не мои, не трогал).
- `cargo test -p chronos --lib` → **617 passed, 0 failed** (было 611;
  +6 = 2 новых grab-теста + 4 теста `volume_popup::view` теперь идут под
  `--lib`, т.к. модуль вошёл в lib-крейт).
- Точечно:
  ```
  cargo test -p chronos --lib anchored_popup_does_not_request_a_compositor_grab
  running 3 tests
  test calendar_popup::tests::anchored_popup_does_not_request_a_compositor_grab ... ok
  test dock::context_menu::tests::anchored_popup_does_not_request_a_compositor_grab ... ok
  test volume_popup::tests::anchored_popup_does_not_request_a_compositor_grab ... ok
  ```
- `cargo build --release -p chronos` → `Finished release profile … 3m 29s`,
  exit 0.

## Живой release-прогон (гиперланд, DP-1 2560×1440)

Свежий `./target/release/chronos` (перезапущен `setsid -f`, PID 2692760).
Координаты виджетов сняты по пиксельному анализу грима бара и подтверждены
кликом: volume-icon ≈ (2227,14), часы ≈ (2435,14), Start ≈ (52,14).
Клик — `ydotool click 0xC0`; warp только `hl.dsp.cursor.move` с ретрай-лупом
до `hyprctl cursorpos` (анимированный move проскакивает цель, если не
дожать — в прогоне ловилось (2059,170)/(2292,4)/(2317,8) вместо цели).

| # | Сценарий | Результат | Кадр |
|---|---|---|---|
| 1 | Sound открыт → `hyprctl layers -j` содержит `chronos-popup-click-catcher` | **да**, `DP-1 lvl3 xywh 0 6 2560 1440` | `01-volume-open.png` |
| 2 | Sound открыт → клик (1400,100) закрывает | **да**, catcher=0 | `02-…-closed-after-click-away.png` |
| 3 | Calendar открыт → клик (1400,100) закрывает | **да**, catcher=0 | `03`, `04` |
| 4 | Sound → клик по часам: Sound закрыт, Calendar один | **да**, catcher=1 (не 2); sound-область 2706→638 ярких px, calendar 701→3796 | `05-sound-open.png`, `06-sound-then-clock-calendar.png` |
| 5 | Sound → `toggle-start-menu` (IPC): Sound закрыт, Start с catcher | **да**, catcher=1 + `chronos-start-menu`=1; sound-область=0, start-область=8434 | `10-sound-closed-start-open.png` |
| 6 | Sound → `toggle-edit-mode` (IPC): Sound закрыт | **да**, catcher=0 | `09-…-by-edit-mode.png` |
| 7 | Клик **внутри** карточки Sound не закрывает | **да**, клик (1950,45) → catcher остался 1 | `07`, `08` |
| 8 | `grab: false` у обоих | **да** (unit-тесты + код) | — |

После прогона: шелл оставлен на новом бинаре, попапы/меню закрыты, edit-mode
выключен, catcher=0.

**Замечание по логу:** `RUST_LOG=info`-лог живого прогона в файл ушёл
пустым — tracing-субскрайбер буферизует вывод в не-TTY (файл 0 байт, флаш
только на exit). Открытие/закрытие volume/calendar не пишет info-строк (в
коде есть только warn на ошибки close-путей), поэтому живого лога-следов
этих переходов и не было бы; доказательство — `hyprctl layers -j` + grim
(выше), а не лог.

## Замечания

- **Escape в комменте.** Бриф (п.5) просил написать в комменте
  `dismiss = click-away / Escape / toggle / ✕`. Я написал
  `click-away / re-toggle / ✕` и **не** вписал Escape: у этих попапов
  `grab: false` и в `PopupOptions` форка нет поля keyboard-интерактивности
  (док в `Source/gpui/src/platform/popup.rs`: grabbing popup «take keyboard
  focus», неграблящий — нет), фолбэк LayerShell — `KeyboardInteractivity::None`,
  а в `volume_popup/view.rs`/`calendar_popup/view.rs` нет ни одного
  keystroke-хендлера. Escape реально не приходит — писать его в коммент
  значило бы снова завести лгущий коммент (ровно то, за что тикет ругал
  старый «only explicit toggle / ✕»). Предсуществующая строка
  «click-away / Escape / re-toggle» в `volume_popup::window_options`
  (коммент T264 A2) оставлена как была — вне пункта брифа.
- `calendar_popup::close_this` в дереве уже был мёртвым (импортируется в
  `view.rs`, не вызывается — календарь без ✕). Обновил его тело под
  закрытие catcher'а, но не выпиливал и не чинил предсуществующий
  unused-import — это вне зоны.
- `popup_click_catcher.rs:69` `let _ =` — предсуществующий, не трогал
  (бриф прямо это разрешает не размножать, а не чинить).
- **Клик по бару при открытом попапе.** Пока Sound/Calendar открыты, их
  full-output catcher (Overlay lvl3) лежит над баром (Top lvl2) вне
  pass-through-дырки, поэтому живой клик по кнопке Start (52,14) закрывает
  попап, но сам до кнопки не доходит (Start открывается вторым кликом).
  Это ровно то же поведение, что уже у tray/dock catchers, не регрессия
  T332; сценарий «Sound → Start» из брифа прогнан детерминированно через
  IPC `toggle-start-menu` (он же реальный путь keybind'а `bindr`/Super).
