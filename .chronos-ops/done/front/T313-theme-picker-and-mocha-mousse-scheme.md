# T313 — theme picker со свотчами + схема Mocha Mousse

**Роль:** FRONTEND. **Приоритет:** P2.
**Зоны:** `crates/ui/src/theme/schemes.rs`,
`crates/app/src/side_panel_right/tab/bar_settings.rs`,
`crates/app/src/theme_config.rs`.
**Не трогать:** `crates/app/src/frame.rs` (там T311/T312, конфликт),
`crates/app/src/ipc/**`, бар, левую панель.

Независим от T311/T312 — можно вести параллельно, зоны не пересекаются.

## Что делаем и зачем

Сейчас схему можно выбрать только двумя способами: переменной окружения
`CHRONOS_THEME` или руками в `~/.config/chronos/theme.toml`
(`scheme = "..."`). В UI есть ровно одна кнопка «Toggle», которая
переключает Default ↔ Light и больше ничего.

Нужен нормальный выбор схемы из UI, свотчами — чтобы видеть, что
выбираешь, до применения. Плюс добавить четвёртую встроенную схему.

**Про Mocha Mousse отдельно.** Это Pantone Color of the Year 2025,
17-1230, тёплый коричневый ≈ `#A47864`. Просто ещё один пункт списка
наравне с Solarized. Никакой «визуальной идентичности продукта» — если
встретишь такую формулировку в старых бумагах (T310 Эпик 2), она
отменена: там QA-миньон принял тёпло-белый фон `#FCF5F3` за
Mocha Mousse и построил на этом теорию. Палитра ChronOS остаётся своя
(`DEFAULT_BASE16` + Light C), дефолт `theme.toml` не меняем.

## Часть A — схема Mocha Mousse

`crates/ui/src/theme/schemes.rs`. Рядом с `light_scheme()` (`:55`) и
`solarized_dark_scheme()` завести `mocha_mousse_scheme()`, подключить в
`builtin_schemes()` (`:151-158`).

Имя схемы: `"Mocha Mousse"` (с пробелом — `select_scheme_by_name`
триммит и сравнивает без учёта регистра, пробел не мешает; посмотри,
как сделано у `"Solarized Dark"`).

Отправная палитра — из разбора мокапа, доводить на глаз в живом
прогоне разрешено и ожидается:

```
light: bg.primary #FCF5F3  bg.secondary #F0E7DF
       bg.tertiary #E5D9CE bg.elevated #D8C9B9
       text.primary #3D2C1F text.secondary #7A5F4B text.muted #A88B72
dark:  bg.primary #241F23  bg.secondary #2D2830
       bg.tertiary #18141A bg.elevated #2D2830
       text.primary #F2EBE5 text.secondary #D8CAC1 text.muted #A0938A
accent.primary #A47864   accent.hover #7E6244
status: warning #C46A2B  error #B73E2A  success #6B7F3C  info #718BA8
```

Решить и записать в отчёт: схема тёмная, светлая, или их две
(`"Mocha Mousse"` / `"Mocha Mousse Light"`). Смотри, как это решено у
существующих — `is_light` — флаг схемы, не переключатель внутри неё.
Одна схема = одна светлота.

**Обязательный тест на контраст.** Зеркалить существующий
`light_scheme_status_is_latte_not_mocha`: `text.muted` на
`bg.primary` должен давать **≥ 4.5:1**. Пастельные цвета, рассчитанные
на противоположный фон, — это тот же грабель, на который наступали в
T239 и в светлой теме (жёлтый `#f9e2af` на светлом баре был не виден).
Если предложенные хексы не проходят — правь хексы, а не тест.

Ничего, кроме `schemes.rs`, часть A не трогает.

## Часть B — picker

### Куда

`crates/app/src/side_panel_right/tab/bar_settings.rs`, секция
**«Theme»** — она уже существует, `:665-714`, заголовок
`section_header(theme, "Theme", "theme.toml — hot-reload")`, внутри
строка с текущей схемой и кнопкой `Toggle` (`on_click(toggle_theme)`).

Таб — `PanelTab::EditorSettings`, лейбл «System settings», рендерится
`BarSettingsTab`. **Таб не переименовывать и новых табов не заводить** —
секция под тему там уже есть, растим её.

### Что должно получиться

Вместо одной кнопки — сетка карточек-свотчей, по одной на каждую схему
из `builtin_schemes()`. Карточка:

- полоска живой палитры этой схемы: `bg.primary`, `bg.secondary`,
  `bg.tertiary`, `bg.elevated` подряд + кружок `accent.primary`;
- имя схемы под полоской;
- выбранная карточка помечена — рамкой `accent.primary`, как размечены
  активные состояния в этом же файле (посмотри `on_blur_toggle` и
  кнопку `sys-theme-btn`, `:696-712`, чтобы не изобретать свой стиль);
- клик применяет схему.

Данные для свотча брать **из самой схемы**, а не из активной темы:
`ThemeScheme` несёт полный `Theme` (он `Copy`), то есть
`scheme.theme.bg.primary` и т.д. Хардкод хексов в рендере запрещён —
иначе свотчи разъедутся с палитрой при любой правке схемы.

Раскладка — по канону T231, который живёт в этом же файле:
`elevated_card`, `section_header`, `setting_row` из
`side_panel_right/tab/ui.rs`, респонсив через `is_wide(cx)`
(`ui.rs:34`). На узком — одна колонка, на широком — две. Свою сетку не
писать.

### Как применять

Путь уже есть, новый не строить:

- `theme_config::persist_scheme(name)` (`theme_config.rs:241`) пишет
  ключ `scheme` через RMW-writer `write_config_key` (`:266`), который
  сохраняет все остальные ключи байт-в-байт. `surface_alpha` и
  `blur_enabled` обязаны пережить смену схемы — на это уже есть тест
  `persist_scheme_preserves_surface_keys` (`:461`), не сломать;
- дальше как в `theme_config::toggle` (`:218-238`):
  `Theme::select_scheme(Some(name))` → `apply_surface_config(scheme,
  &cfg)` → `cx.set_global(theme)` → `sync_gpui_component_theme(cx)` →
  `cx.refresh_windows()`.

Правильно — вынести из `toggle` общую `pub fn select(name: &str, cx:
&mut App)` и заставить `toggle` звать её же. Копипастить тело `toggle`
в новую функцию нельзя: разъедутся при следующей правке.

### Что НЕ делаем в этом тикете

`theme_config::toggle` (и IPC `toggle-theme`) жёстко переключает
Default ↔ Light по флагу `is_light` (`:219-224`). После появления
пикера это становится странным: выбрал Solarized, нажал горячую
клавишу — уехал в Default. **Не чинить здесь.** Написать в отчёте
отдельным абзацем как обнаруженное поведение, владелец решит, заводить
ли тикет.

Не трогать `CHRONOS_THEME` — переменная окружения по-прежнему
выигрывает на холодном старте, это задокументировано в
`theme_config.rs:96-101`.

## Верификация

Юнит:

- `cargo test -p chronos-ui` — новый тест на контраст `text.muted`,
  тест на присутствие схемы в `builtin_schemes()`, тест на
  `select_scheme_by_name("mocha mousse")` в разном регистре;
- `cargo test -p chronos --lib` — существующие тесты
  `theme_config` зелёные, включая `persist_scheme_preserves_surface_keys`.

Живой прогон (обязателен, это UI):

```bash
cargo build --release --bin chronos
RUST_LOG=info ./target/release/chronos &
chronos-ipc toggle-side-panel-right
# открыть таб «System settings», секция Theme
```

Приёмка:

1. Видны все схемы из `builtin_schemes()`, каждая своей палитрой.
   Свотч Light выглядит светлым, свотч Default тёмным — то есть
   рисуется чужая палитра, а не активная. Скриншот `grim`.
2. Клик по схеме применяет её мгновенно, без рестарта, во всех
   поверхностях: бар, оба рельса, панель, попапы. Скриншот до/после.
3. В `~/.config/chronos/theme.toml` изменился только ключ `scheme`.
   Проверить дословно: до клика положить в файл
   `surface_alpha = 0.85` и `blur_enabled = true`, после клика
   `cat` — оба ключа на месте с теми же значениями.
4. Отметка выбранной схемы переезжает на кликнутую карточку.
5. Mocha Mousse читаема: открыть таб с текстом (System, Updates),
   убедиться, что muted-текст и статусы видно. Скриншот.
6. Узкая и широкая раскладка панели — сетка не ломается.
7. Рестарт шелла — выбранная схема поднялась из файла.

## Отчёт

`.chronos-ops/reports-fresh/T313-theme-picker-and-mocha-mousse-scheme-report.md`.

Обязательно: скриншоты пунктов 1, 2, 5; дословный `cat theme.toml` до
и после (пункт 3); результат теста на контраст с числами, а не «прошёл»;
решение по «одна схема или две» с обоснованием; абзац про поведение
`toggle-theme` из раздела «что не делаем».

## Коммиты

```
theme : схема Mocha Mousse в builtin_schemes
theme : picker схем со свотчами в секции Theme
```

Поимённый `git add`, `git diff --staged` глазами. `schemes.rs` и
`bar_settings.rs` — разные коммиты, не смешивать. Без AI-трейлеров.
