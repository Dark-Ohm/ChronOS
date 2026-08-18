# T301 — composer `Select` popup: text ellipsis — report

**Статус:** корень найден и починен по механизму, НО живой grim НЕ снят —
среда не доставляет синтетические клики (см. «Верификация»). Это тот же
класс риска, что брифа прямо запрещает: «тикет уже дважды спотыкался
именно на пропущенной живой проверке». Ниже — что реально проверено
командой, а что нет, без подмены одного другим.

## Диагноз (прочитан исходник, не память)

`.truncate()` в форке = `overflow_hidden().whitespace_nowrap().text_ellipsis()`
(`Source/gpui/src/styled.rs:139`; `text_ellipsis` →
`text_style().text_overflow = Truncate(ELLIPSIS)`, `styled.rs:89`).

Сам эллипсис рисует не стиль, а лэйаут текста — `Source/gpui/src/elements/text.rs:670-690`:
ширина обрезки берётся из `known_dimensions.width.or(available_space.width)`,
и только когда оно `AvailableSpace::Definite`. Иначе `truncate_width == None`
→ текст не обрезается строкой, а просто режется родительским
`overflow_hidden` — ровно тот «голый обрез» из T298-v5.

Почему старый `render()` (`w_full().min_w(px(0.)).truncate()`) не давал
Definite: item рендерится внутри `div().whitespace_nowrap()` из
`SearchableListAdapter::render_item`
(`Source/gpui-component/crates/ui/src/searchable_list/adapter.rs:126-129`),
а тот — флекс-ребёнок колонки `.w_full()` внутри `SearchableListItemElement`
(`item.rs:126-140`). `w_full()` = `width: 100%` (процент,
`gpui_macros/src/styles.rs:845-848`), а процент не резолвится в
MinContent/MaxContent-проходе измерения флекса (нет definite родителя) —
поэтому до текстового элемента доходит неопределённая ширина, и
`truncate_width` остаётся `None`.

Фикс — **пиксельный `max_w`** вместо `w_full`. `max_w(px(x))` ставит
`max_size.width = Definite(x)`, а Taffy клэмпит доступную ширину для
детей по `max_size` (`taffy-0.12.1/src/compute/block.rs:1243`:
`AvailableSpace::Definite(area_width.maybe_clamp(min, max))`). Текстовый
элемент получает `Definite(max_w)` → `truncate_width = Some(..)` →
эллипсис реально вычисляется. Это подтверждено чтением кода лэйаут-движка,
а не «должно работать».

## Что сделано

`crates/app/src/side_panel_left/composer.rs` — только `ModelSelectItem::render`
и `ModeSelectItem::render` (плюс две константы и док-комментарий):

```rust
const MODEL_SELECT_TEXT_MAX_W: f32 = 280. - 48.; // menu_width − list pad(2×4) − row pad(2×12) − check-icon(12) − gap(4)
const MODE_SELECT_TEXT_MAX_W: f32 = 200. - 48.;
// ...
div().max_w(px(MODEL_SELECT_TEXT_MAX_W)).whitespace_nowrap().truncate().child(self.title())
```

`w_full().min_w(px(0.))` убрано, `.truncate()` остался. Бюджет 48px
сложен из констант кита, прочитанных в исходнике: `list_size(XSmall)` →
`px_3()` (12px×2, `styled.rs:491`), `gap_x_1` (4px), check-иконка
`xsmall()` = `size_3()` (12px, `icon.rs:161`), List `.paddings(px(4.))`
(4px×2, `select.rs:575`).

## Верификация

- `cargo check -p chronos` — чисто, 0 warnings в `composer.rs`.
- `cargo test -p chronos --lib` — **598 passed, 0 failed**.
- `cargo test -p chronos --bins` — **790 passed, 0 failed**.
- `cargo build --release -p chronos` — чисто.

### Живой смок — НЕ ДОВЕДЁН (среда, не код)

Запустил release-шелл, открыл левую панель (`toggle-side-panel-left` +
`expand-left`), ACP-сессия поднялась (`ACP client connected`, session
`daf68f79`), модели в композере есть (grim показывает текст в триггере,
не заглушку «Model»). Но **открыть попап кликом не смог**: `ydotool`
клики не доставляются ни одной layer-поверхности.

Доказательство, что это не мой диф — воспроизвёл на баре: клик по часам
бара (x1274,y14, точно интерактивный виджет) не открывает calendar-popup.
`mousemove` при этом работает (`hyprctl cursorpos` подтверждает ровно
экран/2), а `click 0x00 0x80` уходит в никуда. То же самое было в T303
(«ввод к layer-поверхностям не доставляется») и совпадает с
задокументированной деградацией среды в HANDOFF: «Ядро CachyOS после
обновления живёт без модулей до ребута — ломает podman-сеть и ydotool».
Подтверждение — podman-гейтвей `:20128` вниз, `/dev/uinput` пересоздан в
19:22 после старта `ydotoold` (17:40), т.е. fd демона протух. Ребута
сделать не могу (владелец), sudo нет.

`wtype` (виртуальная клавиатура, не uinput) тоже не помог: контентное
окно не в фокусе, а сфокусировать его без клика нельзя.

## Честные оговорки

- **Эллипсис на живом кадре НЕ подтверждён.** Механизм подтверждён чтением
  `taffy` + `text.rs`, компиляцией и тестами — но не пикселями. Это ровно
  то, за что бриф штрафует; фиксирую как есть, а не как «проверено».
- Точное значение `max_w` (232/152) — вывод из констант кита; граница
  эллипсиса может потребовать ±2-4px подстройки по живому гриму, когда
  среда оживёт. Если живьём окажется, что имя впритык к границе режется
  без `…`, уменьшить бюджет на 4-8px.
- Коммит не делал (в брифе раздела «Коммит» нет). В дереве только
  `composer.rs`.
- `Source/` (gpui-component) не трогал — корень внутри `composer.rs`, как
  требует зона брифа.

## Рекомендация приёмке

Принять механизм (переход `w_full`→`max_w` обоснован чтением лэйаут-движка),
но живой грим считать открытым пунктом: прогнать после ребута среды
(`pkill ydotoold` → `sudo ydotoold` + `chmod 666 /tmp/.ydotool_socket`),
открыть model-picker кликом, снять grim и проверить, что длинное имя
кончается `…` до края строки. Если граница на 2-4px не сошлась —
поправить константы.
