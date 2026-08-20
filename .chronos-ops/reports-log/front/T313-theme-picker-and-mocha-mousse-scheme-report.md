# T313 — theme picker со свотчами + схема Mocha Mousse — отчёт

**Роль:** FRONTEND. **Статус:** готово к приёмке.
**Зоны:** `crates/ui/src/theme/schemes.rs`, `crates/app/src/side_panel_right/tab/bar_settings.rs`,
`crates/app/src/theme_config.rs`. Ничего вне зон не изменено (см. «Границы»).

## Решение «одна схема или две»: ОДНА, тёмная

**«Mocha Mousse» — только тёмная.** Обоснование:

1. Сам Pantone 17-1230 `#A47864` — тёмный тёплый коричневый; тёплая тёмная база —
   единственная честная светлотность для этой схемы.
2. Светлая палитра брифа (`#FCF5F3` тёпло-белый фон) — ровно та ошибка QA-миньона,
   которую бриф отменяет (T310 Эпик 2 принял этот фон за Mocha Mousse). Строить на ней
   вторую схему — воскрешать отменённую теорию.
3. Ворота контраста решают за нас. Тёмная база брифа проходит без правок, светлая — нет:

```
тёмная  text.muted #A0938A на bg.primary #241F23 = 5.43:1   (нужно ≥ 4.5)   ✓
светлая text.muted #A88B72 на bg.primary #FCF5F3 = 2.95:1   (нужно ≥ 4.5)   ✗
```

`is_light` — флаг схемы, не переключатель внутри неё; одна схема = одна светлота.

**Статусы брифа осветлены** (п.5 приёмки «статусы видно»): тёмные статусы брифа
(`error #B73E2A` = 2.88:1, `success #6B7F3C` = 3.65:1 на тёмной базе) нечитаемы как
текст. Осветлены в тёплый тон, как Default держит пастельные статусы:

```
status.error   #D4735F  4.95:1
status.warning #D9995C  6.69:1
status.success #9BAE63  6.66:1
status.info    #93ADCB  7.01:1
```

## Что сделано

### `schemes.rs`
- `mocha_mousse_scheme()`: тёплая тёмная база (`bg.primary #241F23`, `secondary #2D2830`,
  `tertiary #18141A`, `elevated #2D2830`), тёплый светлый текст (`primary #F2EBE5`,
  `secondary #D8CAC1`, `muted #A0938A`), акцент — сам Pantone `#A47864` + hover `#7E6244`,
  бордеры/интерактив/selection — тёплые аналоги ролей Default, статусы — осветлённые выше.
  `muted #A0938A` из брифа проходит ворота T317 без правки (5.43:1) — не тронут.
- Подключена в `builtin_schemes()` как `"Mocha Mousse"` (пробел, как `"Solarized Dark"`).
- Тесты: присутствие в `builtin_schemes()`; `select_scheme_by_name("mocha mousse")` без
  учёта регистра; контраст `text.muted` на `bg.primary` ≥ 4.5 с числами в сообщении.
  Общий тест T317 (`muted_passes_wcag_aa_on_primary_in_all_schemes` итерирует
  `builtin_schemes()`) автоматически гоняет новую схему через те же ворота.

### `theme_config.rs`
- Вынесена общая `pub fn select(name: &str, cx: &mut App)` из `toggle` (persist → resolve →
  overlay surface → set_global → sync_gpui_component_theme → refresh_windows). `toggle`
  зовёт её же. Одно тело apply-пути, копий нет.

### `bar_settings.rs` — секция Theme
- Вместо кнопки Toggle — сетка карточек-свотчей по `builtin_schemes()`: полоска живой
  палитры (`bg.primary/secondary/tertiary/elevated` + кружок `accent.primary`) + имя.
- Цвета свотча читаются **из `ThemeScheme` (собственная `Theme`)**, хардкода хексов в
  рендере нет. Активная карточка — рамка и подложка `accent.primary` (тот же язык
  состояний, что у `onoff_chip`/`seg_chip`).
- Раскладка по канону T231: `elevated_card` + `section_header`, респонсив через
  `is_wide(cx)` — узкий 1 колонка, широкий 2.
- Клик → `theme_config::select(name)`.

## Верификация

### Юнит
- `cargo test -p chronos-ui` → **27/27** (новые: присутствие, регистр, контраст).
- `cargo test -p chronos --lib` → **610/610**, включая `persist_scheme_preserves_surface_keys`.
- **Тест контраста реально кусается**: временно поставил `muted = #877B6F` (3.93:1) —
  покраснели и новый тест, и общий T317-тест с дословным `3.93:1`. Вернул `#A0938A` —
  зелено.

### Живой прогон (release, grim, все скриншоты в `/tmp/t313/`)

**П.1 — все схемы видны, каждая своей палитрой.** `01-picker-default.png`: четыре карточки
со свотчами — Default (тёмная полоска), Light (светлая `#E6E9FA`), Solarized
(`#073642`), Mocha (тёплая тёмная + кружок акцента `#A47864`). Полоски — чужие палитры,
не активная тема.

**П.2 — клик применяет мгновенно, без рестарта.** Клик по карточке Mocha → в логе
`theme: selected scheme="Mocha Mousse"`, `theme.toml` обновился, панель перерисовалась
в тёплой тёмной палитре (фон `#241F23`, rail `#18141A`) — `s3.png`/`s4.png` после,
`01-picker-default.png` до. Бар/рельсы/панель сменили цвет синхронно (скриншоты после —
`after-solarized.png`, `p5-system.png`).

**П.3 — в файле меняется только ключ `scheme`.** До клика записал
`surface_alpha = 0.85` и `blur_enabled = true`, кликнул Solarized:

```
до клика:   blur_enabled = true / scheme = "Mocha Mousse" / surface_alpha = 0.85
после:      blur_enabled = true / scheme = "Solarized Dark" / surface_alpha = 0.85
```

Оба ключа на месте байт-в-байт; в логе селект применил `surface_alpha=0.85` живьём.
RMW-писатель `write_config_key` не тронут (тест `persist_scheme_preserves_surface_keys`
зелёный).

**П.4 — отметка переезжает.** Mocha активна: её карточка подложена тоном акцента
`#403538`, остальные `#2D2830` (`s5.png`). После клика Solarized: его карточка `#5B7D90`,
остальные `#365963` (`after-solarized.png`). Активная рамка — `accent.primary`.

**П.5 — Mocha читаема.** Таб System (`p5-system.png`): muted `#A0938A` — 1552 px
(5.43:1 на `bg.primary`), status.success `#9BAE63` — 2262 px, текст `#F2EBE5`,
акцент `#A47864` — 6292 px. Таб Updates (`p5-updates.png`): muted 224 px, primary 587 px.
Всё видно на тёмной тёплой базе.

**П.6 — узкая и широкая раскладки.** Узкая (410 px, 1 колонка) — все скриншоты пикера.
Широкая (2×2 сетка) — `wide.png` (см. «Границы»).

**П.7 — рестарт поднимает схему из файла.** Холодный старт с `theme.toml` = Mocha:
в логе `theme: env=None, file=/home/neo/.config/chronos/theme.toml, bg.primary l=0.13`
(L* ≈ 0.13 = `#241F23`). Финальный перезапуск с Default — та же строка, Default резолвится
из файла.

## Границы

**Временная правка для п.6, откачена.** На стоковом дереве широкая раскладка для секции
Theme структурно недостижима: `PanelTab::EditorSettings` фиксирован на 410 px
(`tabs.rs::preferred_content_width`, не-resizable — только Preview тянется), а брейкпоинт
`GRID_BREAKPOINT = 720` (`tab/ui.rs:23`). Чтобы снять широкую раскладку живьём, временно
поднял `EditorSettings => 760.` в `tabs.rs`, пересобрал, снял `wide.png` (2×2 сетка,
карточки Default|Light / Solarized|Mocha, активная Mocha с accent-рамкой), затем вернул
410 и пересобрал. В финальном диффе `tabs.rs` отсутствует. Оставляю как факт на
рассмотрение владельца: респонсив-код сетки на месте, но достижим только при ширине
таба ≥ 720 — сейчас ни один таб туда не дотягивается.

## Обнаруженное поведение (раздел «что НЕ делаем» брифа)

`theme_config::toggle` (и IPC `toggle-theme`) жёстко переключает Default ↔ Light по флагу
`is_light` — после появления пикера это странно: выбрал Solarized, нажал горячую
клавишу — уехал в Default. Не чинил по брифингу; вынесено как обнаруженное поведение,
владелец решит, заводить ли тикет. `CHRONOS_THEME` по-прежнему выигрывает на холодном
старте — не трогал.

## Окружение

Вернул дословно: `theme.toml` = `Default`/`surface_alpha 1.0`/`blur_enabled true`
(исходное состояние на момент старта), `frame.toml`/`bar.toml` не трогал, шелл на
финальной release-сборке, обе панели открыты, `0` паник/ошибок/protocol-ошибок в логе.
Коммитов нет; в дереве — только три файла зоны (`git status`).
