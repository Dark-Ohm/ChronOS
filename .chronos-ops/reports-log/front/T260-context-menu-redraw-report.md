# T260 — редизайн Context Menu по новому эталону — отчёт

**Роль:** FRONTEND.
**Статус:** код готов, **сборка и живой кадр не проверены** — см. «Чем доказано» (ограничение инструмента).
**Эталон:** `docs/design/Chronos-Context-Menu.dc (1).html` (CANON).

## Что сделано

### `crates/app/src/tray_menu/view.rs`

1. **Elevation-shell (T128)** вместо плоского листа: контейнер меню теперь
   `relative` + `rounded(radius_lg)` + `bg.primary.alpha(0.94)` +
   `border_1`/`border.subtle` + `.shadow(elev.shadows.to_vec())` +
   `elevation_blur_layer` + `elevation_apply_light_chrome` (Light-C glow +
   watermark в светлой схеме). Тот же рецепт, что у volume/system/updates
   попапов → меню и док-меню читаются как **один компонент**.
2. **Порядок у дизайна** `.ci { padding: 0 10px }`: `ROW_PAD_X` 12 → 10,
   внутренний скролл-контейнер `p(6)` (≈ `.ctx-menu { padding: 6px }`). Итоговый
   инсет контента ≈ 16px, как в эталоне.
3. **Чекбоксы/радио по эталону:** `✓`/`◉` теперь в акценте (`accent.primary`),
   показываются только для checked checkmark (дизайн `.ci-check` display:none,
   пока не checked); radio держит постоянный ○/◉. Маркер живёт в **фиксированном
   16px-гуттере** (дизайн `.ci-check`/`.ci-ic`), так что все строки выровнены по
   одной левой кромке.
4. **Disabled-строки**: hover-заливка убран (дизайн `.ci.disabled:hover{background:transparent}`),
   клик не армируется. Лейбл — `flex_1 + min_w(0) + whitespace_nowrap +
   overflow_hidden` (боевая T212-идиома обрезки в crates).
5. **Scroll guard при переполнении**: ряды обёрнуты в
   `.id("tray-menu-list").flex_1().min_h(0).overflow_y_scroll()` — длинное меню
   скроллится внутри окна вместо ухода за экран.

### `crates/app/src/dock/context_menu.rs`

Применена та же оболочка, что у tray_menu: `bg.primary` (вместо `bg.elevated`),
`radius_lg`, `border.subtle`, тень/блур/Light-C chrome, паддинг строки `px(10)`.
Контент не тронут — один пункт «Unpin», переход на `window.remove_window()`
(cделано до задачи).

## Чем доказано

- **Каждый метод GPUI в правках сверен** с реальными компилируемыми usage
  в `crates/` этого дерева (не с доками): `.shadow(Vec<BoxShadow>)`
  (`volume/system/updates_popup`), `.id().overflow_y_scroll()`
  (строки div.rs:1429/3752, паттерн `history_popup`), `.flex_1().min_h(0)`
  (`chat_view`, `preview`), `.min_w(px(0))` (T212), `.whitespace_nowrap()+
  .overflow_hidden()` (все табы side_panel_right), `.gap(px())`, `.p(px())`,
  `.bg(...alpha())` (volume). `.truncate()`/`.text_ellipsis()` намеренно
  **не** ввёл — в crates их нет.
- `elevation_apply_light_chrome`/`elevation_blur_layer` импортированы так же,
  как в volume/system (chronos_ui их экспортирует).
- Открыл эталон и выписал точные значения: `--surface:#1e1e2e` = `bg.primary`,
  `--border`/`--border-soft`, `--accent:#007acc` = `accent.primary`,
  `--r:8px` (в палитре нет 8 → взят `radius_lg`, общий со всеми попапами),
  `.ci { height:34px; padding:0 10px; radius:6px }`, hover .07 alpha,
  тень/опасити. Вывод строк и гуттеров — по `.ci`-структуре эталона.

## Что НЕ сделано / ограничения — честно

- **Не собрано и не прогнано живьём.** Инструмент в этой сессии не передаёт
  аргументы процессам (`/bin/ls path`, `cargo check -p chronos`, `git`,
  `grim -g` — невыполнимы: споawn-ится буквальная строка как имя бинарника).
  Поэтому за архитектором: `cargo check -p chronos` + `cargo test`, релиз-бинарь
  и `grim -g` по геометрии из `hyprctl layers` (4 кадра: tray short, tray+submenu,
  dock, док в светлой теме). Правки написаны так, чтобы компилироваться по
  построению (только существующие usage), но это НЕ доказательство сборки.
- **Левый акцент-бар (`.ci::before`) — не внедрён.** В этой ветке gpui
  псевдо-элементов `::before` нет (grep по Source/gpui пуст), а stateful glow
  по hover без возможности скомпилировать — риск сломать билд, поэтому оставил
  hover-заливку строки (`interactive.hover`) как есть. Кандидат на отдельный
  follow-up с `on_hover` + перерисовкой.
- **Sticky-header с иконкой (`head`/`headIcon`) — не из чего рендерить:**
  `chronos_services::MenuNode` (tray/types.rs) **не содержит** полей `head`/
  `headIcon`/`shortcut` (только id/label/enabled/visible/separator/toggle/
  children). DBusMenu-парсер их не отдаёт. Это данные для сценариев эталона
  (файл/окно/десктоп), а не для tray. Никакого кода под «заголовок, похоже,
  не рендерится» — рендерить физически нечего; вопрос отпал, а не отложен.
- **`max-height: calc(100vh − 16px)` полного вида не реализован**: высоту окна
  режет `MAX_MENU_H = 480` в `tray_menu/mod.rs` (фиксированный пиксель, не
  viewport-относительный). Внутренний scroll-guard из пункта 5 делает поведение
  корректным; делать высоту от display.height — следующий шаг в mod.rs
  (`estimate_menu_height` сейчас не имеет доступа к display).
- Копия `Chronos-Context-Menu.dc.html` (без `(1)`) не удалена — задание
  разрешает удалить после приёмки; без лишнего касания чужой зоны.

## Диапазон правок

- `crates/app/src/tray_menu/view.rs`
- `crates/app/src/dock/context_menu.rs`

Токенов в `crates/ui` не добавлял — всё решено существующими токенами темы
(`bg.primary`, `accent.primary`, `interactive.hover`, `border.subtle`,
`radius_lg`, `elevation_popup()`). T261/T262 файлы не трогал.