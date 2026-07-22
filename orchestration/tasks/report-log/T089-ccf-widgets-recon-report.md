# SESSION_REPORT — Разведка `ccf-gpui-widgets` против форка gpui

**Агент:** Hermes. **Дата:** 2026-07-21. **Режим:** read-only разведка компиляцией.
**Задание:** HERMES.md §«Разведка компиляцией `ccf-gpui-widgets`» — компилируется
ли текстовый ввод крейта против нашего форка gpui; брать зависимостью или вендорить.
**Коммит:** не требуется (ChronOS/Source не менялись). Правки — только в throwaway-клоне
`/home/neo/scratch/ccf-gpui-widgets-recon` (манифест).

---

## TL;DR — решение

**ВЕНДОР ОБЯЗАТЕЛЕН.** Крейт НЕ компилируется против нашего форка: **58 ошибок**
(`cargo check`), из них **43×E0061** — единый класс дельты (форк добавил `cx: &mut App`
в `FocusHandle::focus`). Это «animation-паттерн» (как Grok №18), НЕ «rsx-паттерн»
(чистый check у Cline №2). Взять как `[patch]`-зависимость нельзя — упадёт на нашем же
дереве.

Ввод при этом **хирургически извлекаем** (~4–5 файлов), НО весь крейт — единый пакет,
и дельты размазаны по всем виджетам, а не только по вводу: чинить придётся ровно те
файлы, что вендоришь.

Тема-маппинг `chronos_ui::Theme → ccf::Theme` — **не тривиален**: разные цветовые
модели (`Hsla`-группы vs плоские `u32`) + коллизия `gpui::Global` (оба регистрируют
свой `Theme`). Нужен адаптер-функция `Hsla→u32` на ~40 полей, не «копия структуры».

---

## Метод и доказательная база

- Клон: `git clone --depth 1 github.com/ComposerChrisF/ccf-gpui-widgets`
  → `/home/neo/scratch/ccf-gpui-widgets-recon`.
  **HEAD `2ae1725716ed22a0a2e2473007d97f22068956f5`, 2026-07-16 07:07 -1000**
  («Deep code review: file 80 verified bug reports»). Тег `v0.1.0` в репо
  отсутствует; `Cargo.toml` version = **0.1.2** (не 0.1.0, как в брифе — крейт
  ушёл вперёд; зафиксировано по факту).
- Правка ТОЛЬКО манифеста: `gpui = "0.2.2"` → `gpui = { path =
  "/home/neo/projects/chronos-ecosystem/Source/gpui" }`. `Cargo.lock` удалён для
  чистого резолва. Код крейта не тронут (первый заход).
- `cargo check` (default features) — лог `/tmp/ccf_check.log`.
- Единственность gpui в графе подтверждена (см. ниже).
- `git -C …/Source status --short` — ЧИСТО (кроме заранее существовавшего чужого
  `gpui-animation/`, не мой, не тронут).

---

## Структура крейта (важно для «ввод отдельно vs весь крейт»)

**МОНОКРЕЙТ, не воркспейс.** Один `cargo check` на всё (~18k строк, `src/widgets/*`).
Прямые зависимости: `gpui`, `smol 2.0`, `log 0.4` (+ optional `rfd`/`dirs`/`secrecy`/
`zeroize` под фичами `file-picker`/`secure-password`, по умолчанию выключены).

`cursor_blink`, `editing_core`, `focus_navigation` — **реальные модули-файлы**
(`src/widgets/{cursor_blink,editing_core,focus_navigation}.rs`), не inline. Спутники
ввода (грепом по `use super`):

| Файл | Тянет из соседей |
|---|---|
| `text_input.rs` | `cursor_blink::CursorBlink`, `editing_core::EditingCore`, `focus_navigation::{FocusNext,FocusPrev}` |
| `editing_core.rs` | ничего (только gpui) — лист |
| `cursor_blink.rs` | ничего (только gpui) — лист |
| `focus_navigation.rs` | ничего из ввода; **но его импортируют ВСЕ виджеты** крейта |
| `password_input.rs` | `text_input::*` + `cursor_blink` + `editing_core` + `sensitive_string` (feature `secure-password`) |
| `repeatable_text_input.rs` | `text_input::{TextInput,TextInputEvent}` + `focus_navigation` |

**Вывод по хирургии:** чистый текстовый ввод = `text_input + editing_core +
cursor_blink + focus_navigation` (4 файла; `focus_navigation` — лист, безопасно
вынести). `PasswordInput` добавляет `sensitive_string` (feature-gated). То есть
взять ТОЛЬКО ввод реально — 4–5 файлов, — но каждый из них несёт форк-дельты (ниже),
и чинить их придётся при вендоринге.

---

## Результат компиляции: 58 ошибок = дельты форк-vs-crates.io

Гистограмма (`grep -oE "error\[E[0-9]+\]"`):

| Код | Кол-во | Класс дельты |
|---|---|---|
| E0061 | 43 | `FocusHandle::focus` получил доп. аргумент `cx` |
| E0599 | 10 | `update_entity`→`R` (не Result); `Bounds::from_corner_and_size` удалён |
| E0433 | 2 | `Corner` не найден (тип удалён) |
| E0308 | 2 | `ScrollHandle::max_offset()` теперь `Point`, не `Size` |
| E0432 | 1 | `use gpui::Corner` — нет в корне |

### Дельта 1 — `FocusHandle::focus(window)` → `focus(window, cx)` (43×E0061)

- **Их вызов:** `self.focus_handle.focus(window)` (1 арг).
- **Наш форк:** `pub fn focus(&self, window: &mut Window, cx: &mut App)`
  — `Source/gpui/src/window.rs:457` (2 арга помимо `&self`).
- **Дока компилятора:** `argument #2 of type &mut gpui::App is missing`, `note:
  method defined here → …/Source/gpui/src/window.rs:457`.
- **Минимальная правка:** добавить `cx` в каждый вызов (в render/handler `cx` в
  скоупе). Механическая, но в 43 местах по всем виджетам.
- Примеры сайтов: `checkbox.rs:195`, `checkbox_group.rs:233`, `collapsible.rs:278`,
  `dropdown.rs:{312,343}`, `number_stepper.rs:{447,558,634,712,749}`,
  `password_input.rs:{899,902,922}`, `text_input.rs:{1058,1064,1083,1085,1105}`, …

### Дельта 2 — `update_entity(...)` возвращает `R`, не `Result<R>` (E0599 ×3 + связанные)

- **Их код:** `async_cx.update_entity(&e, |this,cx| {...bool...}).unwrap_or(false)`.
- **Наш форк:** `fn update_entity<T,R>(...) -> R` — `Source/gpui/src/app.rs:2591-2605`
  (возвращает `R` напрямую, не `Result`). Отсюда `no method named unwrap_or found
  for type bool`.
- **Сайты:** `text_input.rs:{665,855}`, `password_input.rs:{481,698}`.
- **Минимальная правка:** убрать `.unwrap_or(false)` (значение уже `bool`). Требует
  сверки контекста async (upstream, вероятно, `AsyncApp::update_entity -> Result`;
  наш форк — инфаллибельный вариант или другой ресивер).

### Дельта 3 — тип `Corner` удалён (E0432 + E0433 ×2)

- **Их код:** `use gpui::{… Corner …}` (`scrollbar.rs:9`); `anchored().anchor(
  Corner::TopLeft)` (`dropdown.rs:403`, `color_swatch.rs:693`).
- **Наш форк:** `Corner` в корне gpui НЕТ (греп `enum/struct Corner` → 0). Есть
  `Corners<T>` (`geometry.rs:2258`, это padding-набор углов, не enum). `anchored()`
  теперь принимает `Anchor`: `pub fn anchor(mut self, anchor: Anchor)` —
  `Source/gpui/src/elements/anchored.rs:40`.
- **Минимальная правка:** заменить `Corner::TopLeft` → соответствующий `Anchor`-API
  форка; переписать `use`. Не механическая — редизайн API (Corner→Anchor).

### Дельта 4 — `Bounds::from_corner_and_size` удалён (E0599 ×6)

- **Их код:** `Bounds::from_corner_and_size(...)` — `scrollbar.rs:{441,447,453,461,
  467,473}`.
- **Наш форк:** такой ассоциированной функции нет (есть `Bounds::from_corners`,
  `geometry.rs:818` — другая сигнатура). Нужна ручная замена на существующий
  конструктор.

### Дельта 5 — `ScrollHandle::max_offset()` тип сменился (E0308 ×2)

- **Их код:** `…max_offset() + …bounds().size` (`scrollbar.rs:356`) — складывают с
  `Size<Pixels>`.
- **Наш форк:** `max_offset()` возвращает `Point<Pixels>` (ошибка: `expected
  Size<Pixels>, found Point<Pixels>`). Правка типов на месте.

---

## Доказательство уровня typecheck (не «должно работать»)

Синтетический «намеренно-неверный вызов» из брифа здесь **неприменим и избыточен**:
крейт целиком НЕ компилируется, поверх него example собрать нельзя. Но требуемое
доказательство — что раскрытие кода типизируется против НАШЕГО gpui — уже дано
**самим билдом, и сильнее**: КАЖДАЯ из 58 ошибок несёт `note: … defined here →
/home/neo/projects/chronos-ecosystem/Source/gpui/src/…` (`window.rs:457`,
`app.rs:2591`, `geometry.rs:818`, `anchored.rs:40`). То есть компилятор резолвит
вызовы ccf против нашего форка и падает именно на дельтах форка — это и есть
typecheck-привязка к форку, а не к crates.io. Честно фиксирую: отдельный
throwaway-негатив не писал, потому что он не добавил бы доказательности к 58
форк-резолвнутым ошибкам (и физически не собрался бы на незелёном крейте).

---

## Единственный gpui в графе

```
$ cargo tree -i gpui
gpui v0.2.2 (/home/neo/projects/chronos-ecosystem/Source/gpui)
└── ccf-gpui-widgets v0.1.2 (/home/neo/scratch/ccf-gpui-widgets-recon)

$ cargo tree | grep -c "gpui v"   →  1
```

РОВНО ОДИН `gpui v0.2.2 (…/Source/gpui)`. `[patch."https://github.com/zed-industries/
zed"]` НЕ понадобился: у ccf нет транзитива, тянущего gpui (deps = smol/log/rfd/dirs/
secrecy/zeroize — ни один не зависит от gpui). Прямой path-override достаточен, двойного
gpui в графе нет.

---

## Тема-маппинг `chronos_ui::Theme` → `ccf::Theme`

**Не «одна функция на десяток полей». Две несовместимости:**

1. **Разные цветовые модели.**
   - `ccf::Theme` (`src/theme.rs:169-292`): **52 плоских поля `u32`** (0xRRGGBB),
     `#[derive(Copy)]`, конвертит через `rgb(theme.x)`. Плюс `Palette` (7 seed-цветов
     → генерит 52). Поля: `bg_primary/secondary/input/input_hover/hover/section_header
     /…`, `text_primary/label/section_header/value/muted/placeholder/dimmed/icon/…`,
     `border_default/checkbox/input/menu/focus/focus_on_color/error`, `primary/
     primary_hover/primary_active/accent`, `success/error/warning/error_text`,
     `tooltip_bg/border/text`, `selection`, `disabled_bg/text`, `secondary_bg/…/
     border`, `bg_tab_hover/border_tab_active`, `delete_bg/hover`, `bg_path_hover`.
   - `chronos_ui::Theme` (`crates/ui/src/theme/mod.rs:154-166`): **семантические
     группы `Hsla`** — `BgColors{primary,secondary,tertiary,elevated}`, `TextColors
     {primary,secondary,muted,disabled,placeholder}`, `BorderColors{default,subtle,
     focused}`, `AccentColors{primary,selection,hover}`, `StatusColors{success,
     warning,error,info}`, `InteractiveColors{default,hover,active,toggle_on,
     toggle_on_hover}` + `radius/radius_lg/font_sizes/font_mono`.
   - Маппинг требует **адаптер `Hsla → u32`** (упаковать RGB, отбросить alpha) на
     каждое из ~40 нужных ccf-полей — не копия структуры.

2. **Коллизия `gpui::Global`.** Оба типа делают `impl gpui::Global for Theme {}`
   (`ccf src/theme.rs:301`; `chronos_ui mod.rs:219`). gpui хранит по одному global
   на тип — конфликта типов нет (разные типы), но ccf-виджет читает СВОЮ тему через
   `get_theme_or` (`text_input.rs:40`); наш `Theme::global(cx)` его не покроет.
   Придётся либо `cx.set_global(ccf_theme)` отдельно, либо передавать тему пер-виджет
   (`TextInput::new(cx).theme(ccf_theme)` — ccf это поддерживает, `custom_theme`).

**Поля-сироты (ccf → нет прямого источника в chronos):**
`bg_white`, `bg_light_hover`, `text_black`, `text_dark`, `border_checkbox`,
`border_menu`, `border_focus_on_color`, `error_text`, `secondary_bg_active`,
`delete_bg[_hover]`, `bg_path_hover`, `bg_section_header[_hover]`, `bg_tab_hover`,
`border_tab_active`. Их можно закрыть либо из ближайших chronos-ролей (напр.
`ccf.secondary_* ← chronos.interactive.*`), либо оставить дефолты `ccf::Theme::dark()`.
В обратную сторону (chronos → ccf) потеря невелика (chronos беднее полями); критично
только, что chronos различает `radius`/`radius_lg`/шрифты, которых у ccf нет — но они
темой ccf и не управляются.

---

## Что НЕ проверял / границы

- **Runtime не гонял** (headless, и крейт не собирается). Всё выше — typecheck-уровень.
- Фичи `file-picker`/`secure-password` — check шёл БЕЗ них (default). Их код
  (`directory_picker`/`file_picker`/`sensitive_string`) в 58 ошибок не входит, но с
  большой вероятностью несёт те же дельты (`focus`, `Corner`). Не проверял отдельно.
- `smol`-executor-паттерн ccf (тот же класс вопроса, что был у animation) — не
  доходили руки: крейт падает на API-дельтах раньше, чем до executor-семантики.
- Не читал построчно все 24 виджета — только сайты ошибок + ввод + тему.

## Заметка (одной строкой, не рекомендация к архитектуре)

Если брать ввод — вендорить `text_input + editing_core + cursor_blink +
focus_navigation` (~4 файла) с адаптером темы `Hsla→u32`, и пройтись по 5 классам
дельт выше; `[patch]`-зависимостью весь крейт не подключить.

---

## Файлы

- Клон-разведка: `/home/neo/scratch/ccf-gpui-widgets-recon` (throwaway; манифест
  пропатчен на наш форк, код не тронут).
- Полный лог: `/tmp/ccf_check.log` (58 ошибок).
- `Source` — read-only, чисто (`gpui-animation/` чужой, не тронут).
