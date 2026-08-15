# T286 — композер Chat на `gpui-component::Input`

**Родитель:** эпик `T287-left-chat-onto-gpui-component.md` (волна 1).
**Приоритет:** P1 — пользователь не видит, что печатает.
**Роль:** FRONTEND (`side_panel_left/composer.rs`, `text_input.rs`, `tabs/chat.rs`).
**Канон:** `docs/DECISIONS.log` 2026-08-15 — новые контролы из нашего кита,
не руками. T275 уже посадил `Input` в лаунчер; композер остался на T154.

## Симптом (живой, владелец, 2026-08-15)

Текст в поле композера уезжает **вправо за край вкладки** в пустоту,
строка не переносится. Каретка и хвост набора не видны.

## Почему

`text_input.rs` — однострочный `Element`:

- `request_layout`: высота ровно `line_height`, ширина `relative(1)`.
- `prepaint`: `shape_line` без wrap-ширины. Одна линия, `x_for_index`.
- `composer.rs` ~L116–133 считает `visible_lines` по числу символов и
  растит `h()`, плюс `overflow_y_scroll`. Высота растёт, **краска
  остаётся одной горизонтальной линией**. Скролл по Y пустой текст не
  спасает.

Чинить wrap в этом Element — отказ. Это тот же тупик, что T154 vs кит
(484 строки против `input/` на 17k).

## Задача

Заменить поле композера на `gpui_component::input::Input` + `InputState`.

- Режим **multi-line**, wrap по ширине колонки (не single-line как в T275).
- Enter = отправка (сейчас так). Shift+Enter = новая строка. В ките:
  `submit_on_enter` + `enter` делает `cx.propagate()` / `PressEnter` —
  посадить send на это, не на самописный `handle_key`.
- Левая панель уже `Root` + `OnDemand` — второе окно не создавать.
- `appearance(false)` или токены темы, как у лаунчера / preview editor.
- Высота поля растёт с числом **визуальных** строк, потолок как сейчас
  (`min(45% панели, …)`), дальше скролл внутри Input.

## Нельзя

- Дописывать wrap в `TextInputElement` / `shape_line`.
- Трогать ACP connect / `create_session` / `load_session` (это T285,
  дыра гейта 8). Если файл `chat.rs` общий — только поле композера и
  `send_composer` / `compose-and-send`.
- Ломать: send/cancel, YOLO, model/mode picker, `@` / `/` если уже есть,
  IPC `compose-and-send:<text>` (сейчас пишет в
  `composer_input.content` — перевести на `InputState`).
- `Cargo.lock`, `Source/gpui/`.

## Что выкинуть

После замены композера `TextInputElement` больше никому не нужен, если
греп по `side_panel_left` пуст. Тогда удалить `text_input.rs` и импорты.
Не оставлять мёртвый модуль «на всякий».

## Верификация

- `cargo test -p chronos --lib side_panel_left`
- Live, release: набрать длинную строку без пробелов и с пробелами —
  перенос внутри карточки, хвост виден. Shift+Enter — новая строка.
  Enter — один send. `compose-and-send:hello` по-прежнему один turn.
- Кадр `grim` композера с длинным текстом (не fullscreen).

## Конфликт зон

`chat.rs` / `composer.rs` — не параллелить с T285. T287-A не начинать,
пока T286 не в git. Пикеры model/mode здесь не трогать.

## Коммит

`fix(left-panel): composer uses gpui-component Input and wraps (T286)`
