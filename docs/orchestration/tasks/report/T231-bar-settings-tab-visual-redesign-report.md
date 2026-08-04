# T231 — System settings → Bar tab: визуальный редизайн на full-width — отчёт

**Дата:** 2026-08-04. **Роль:** FRONTEND. **Статус:** выполнено.
Кадры «до» не снимались — по решению пользователя (см. §Ограничения).

## Контекст: что было в рабочем дереве при старте

`bar_settings.rs` уже содержал **несобранный partial-rewrite** (кто-то начал
T231 и бросил в незакоммиченном виде):

- `impl DragMoveEvent for HeightSliderDrag` — `DragMoveEvent` это не трейт, не
  компилировалось;
- `BarSettingsTab::new` возвращал `Entity<Self>` из `-> Self`;
- сломанная drag-математика (`ev.resolve(...)`, несуществующие методы);
- `persist` переписан с `apply_patch` на прямой `fs::write` — терялся live-apply
  через `bar_settings::apply_patch` и error-баннер.

T231-правка сделана **начисто поверх HEAD-логики** (`git show HEAD`). Поведение
не менялось: `persist` по-прежнему пишет через `bar_settings::apply_patch`
(виджеты/неизвестные ключи выживают, `version` форсится в 2), preset-ids
остались `&'static str`, drag-математика (rel_x/width → frac → clamp) та же,
`PreviewTarget`/`PreviewIntent` (Open bar.toml / Hypr module) на месте,
error-баннер сохранён. Клик-хендлеры не тронуты — задача чисто визуальная.

## Изменения (пункты тикета §1-§5)

1. **Резиновая раскладка (§1).** Appearance-блок (Edge / Height / Width /
   Floating / Radius / Elevation / Exclusive) — полноценный CSS-подобный grid:
   **2 колонки от `GRID_BREAKPOINT = 720px`**, 1 колонка на дефолтной ширине
   (`DEFAULT_CONTENT_WIDTH = 560` — тест `breakpoint_keeps_default_width_single_column`
   это фиксирует). Hypr modules — grid **3 колонки** на широкой панели при
   ≥3 модулях, 2 при двух, 1 на узкой.
   Grid в форке подтверждён: `Source/gpui/src/styled.rs` (`.grid()/.grid_cols()/…`),
   живые примеры `examples/grid_layout.rs`, `examples/anchor.rs`. Ложный негатив
   «в форке нет grid» был из-за поиска по корню ChronOS — форк лежит sibling-ом
   в `../Source/gpui`.
2. **Визуальная иерархия (§2).** Новый `section_header()`: акцентный тик
   (3×12px, `accent.opacity(0.85)`) + SEMIBOLD 12.5px заголовок + mono-подпись.
   `setting_label()`: MEDIUM 11px лейбл + mono-путь (`appearance.*`). Секции
   разнесены `gap(16px)` на карточке — секция от секции визуально отделена,
   в отличие от старого плоского `gap(14px)` на все дети. Пути-подписи
   оставлены на каждой строке (System settings — это техническая поверхность,
   mono-путь там полезен, а не debug-артефакт).
3. **Контролы (§3).** `-`/`+` — bordered-кнопки 24×24 с hover-фоном (было:
   голый текст). Слайдер: трек 6px (было 4px) + thumb 16px с border и
   drop-shadow — читается как перетаскиваемый. Сегменты/onoff — сохранён
   accent-язык active-состояния, единый вес со step-кнопками.
4. **Hypr modules (§4).** `module_card()` — компактная карточка: имя (mono,
   truncate), путь (muted, truncate), «Open ▸» (акцент). Grid вместо стены строк.
5. **Elevation (§5).** Всё скроллируемое содержимое — на `theme.bg.elevated`
   карточке с `elevation_popup()`-тенями через `elevation_apply_light_chrome`
   (тот же язык глубины, что в `side_panel_left/panel.rs`). Панель больше не
   сливается с обоями.

## Кадры «после» (4/4)

Default width (320px) и full-width (960px), обе темы
(лежат в `docs/orchestration/tasks/notes/`, прецедент — T144-скрины):

- `T231-bar-settings-dark-320.png` — дефолтная ширина, Default (тёмная)
- `T231-bar-settings-dark-960.png` — full-width, Default (тёмная)
- `T231-bar-settings-light-320.png` — дефолтная ширина, Light
- `T231-bar-settings-light-960.png` — full-width, Light

960px получен **живым drag-ресайзом** ручки панели (C-тул через `/dev/uinput`),
геометрия подтверждена `hyprctl layers` (`w=960`). На 960-кадре видно 2-колоночный
grid Appearance (два кластера контролов в ряд: слева сегменты Edge, справа
слайдер с thumb), компактные модульные карточки, карточку-подложку с тенью.

## Верификация

- `cargo build --release -p chronos` — чисто, без warning-ов в bar_settings.rs.
- `cargo test --release -p chronos --lib -- side_panel_right` — зелёные, включая
  новые тесты `breakpoint_keeps_default_width_single_column` и
  `slider_frac_clamps_and_handles_zero_width`.
- **Live** (рабочий путь ввода — собственный uinput-тул; `ydotool` на этой
  системе нерабочий — kernel-module mismatch, см. HANDOFF):
  - Drag ресайза 320 → 960: успех.
  - Клик `+` на Height-слайдере: **успех** — `bar.toml` `height` изменился
    (35.6 → 38.4, шаг 2.8). Значит клик → listener → `persist` → `apply_patch`
    → файл работает на новом коде.
  - Полный click-through всех ~10 контролов не добит: синтетический ввод
    капризен (ABS-маппинг uinput растягивается на весь десктоп 4480px,
    координаты плывут), плюс параллельная активность пользователя (панель
    переключалась/закрывалась в процессе). Сама логика контролов при этом не
    менялась — та же, что принималась в T202/T230.

## Ограничения

- «До»-кадры не сняты (решение пользователя): для них нужна пересборка прежней
  версии `git stash` + 2 рестарта живого шелла. Описание «до» — дословный
  вердикт архитектора в тикете: debug-меню, одна колонка на 960px, голые
  `-`/`+`, тонкий трек 4px, стена строк modules, ноль elevation.
- Полный клик-прогон — отдельной задачей, когда появится надёжный синтетический
  ввод (или вручную).

**Коммит:** `ui : bar settings tab responsive grid + visual hierarchy (T231)`.
