# T312 — два режима оболочки: `normal` / `wrapped`

**Роль:** FRONTEND. **Статус:** готово, не закоммичено (жду приёмки владельца).

## Что сделано

Переименовал режимы `Hide`/`Wrap` → `Normal`/`Wrapped` и убрал нижнюю планку
как самостоятельную сущность — она стала нижним краем плиты в `wrapped`, в
`normal` её нет. `normal` выражается как «ноль по всем краям» (той же функцией
инсета, без отдельной ветки отрисовки): в `normal` не создаётся ни одного слоя
`frame_wrap_*`.

### Полный список переименований

| было | стало |
|---|---|
| `FrameStyle::Hide` | `FrameStyle::Normal` (дефолт) |
| `FrameStyle::Wrap` | `FrameStyle::Wrapped` |
| `apply_hide` | `apply_normal` |
| `apply_wrap` | `apply_wrapped` |
| `STYLE_ID_HIDE` / `STYLE_ID_WRAP` | `STYLE_ID_NORMAL` / `STYLE_ID_WRAPPED` |
| `as_str()`: `"hide"`/`"wrap"` | `"normal"`/`"wrapped"` |

### Разбор конфига — алиасы и регистр (п.2 брифа)

`from_str_sanitized` теперь:

- `"normal"` | `"hide"` → `Normal`;
- `"wrapped"` | `"wrap"` → `Wrapped`;
- регистр не важен (`s.trim().to_ascii_lowercase()`);
- неизвестное → `Normal` + `tracing::warn!`, остальные поля конфига сохраняются
  (ручной `deserialize_style` не трогался — только добавил алиасы и регистр).

### Нижняя планка (п.3 брифа)

Удалён весь код отдельной планки: `BottomStripView`, `window_options` (планки),
`FRAME_WINDOW`/`frame_window()`, `open()`/`close()`, `hide_strip_wanted`,
`hide_strip_insets`. В `wrapped` нижний край рисует матте через `wrap.bottom`
(как и до тикета — планка и матте были взаимоисключающими, планка рисовалась
только в старом `Hide`).

**Решение по `bottom_strip.height` (требуемое брифа):** он становится
игнорируемым легаси (вариант «чище»). `[bottom_strip]` и `FrameJunction`
остаются в структуре конфига и парсятся (чтобы существующий `frame.toml` не
схлопнулся — T268), но не читаются при отрисовке. Чтобы «не молчать», добавлен
`AtomicBool`-гейт: один `warn!` на первый `apply`, **только если**
`bottom_strip != BottomStripConfig::default()`. Проверено живьём: с `height = 8`
лог содержит ровно один warn; с дефолтным `[bottom_strip]` (как у владельца)
warn не появляется.

### `normal` = нулевая толщина (п.4 брифа)

`apply_normal` только закрывает wrap-поверхности; толщина не создаётся. Все
пер-краевые хелперы (`wrap_inset_left/right/bottom`, `shell_top_gap`) в `Normal`
возвращают 0 / высоту бара — той же функцией, что и раньше для `Hide`, без
второй реализации рамки.

## Файлы (диффы)

- `crates/app/src/frame.rs` — 499 строк изменено (основное: переименование,
  удаление планки, тесты).
- `crates/app/src/bar/mod.rs` — 1 строка: `FrameStyle::Wrap` → `FrameStyle::Wrapped`.
- `crates/app/src/side_panel_right/tab/bar_settings.rs` — 24 строки:
  `FrameStyle::Hide/Wrap` → `Normal/Wrapped`, метки сегментов `"Hide"/"Wrap"` →
  `"Normal"/"Wrapped"`, id `frame-seg-*`.

**Границы зоны:** два последних файла — вне формальной зоны брифа
(`frame.rs`). Переименование энума не компилируется без них; метки сегментов в
настройках переименованы вместе с вариантами, чтобы UI не показывал старые
имена при новых ключах. Это чисто механическая правка (без изменения логики),
и её легко откатить, если владелец решит иначе. Плашка/поведение панелей и бара
не трогались.

## Верификация — юнит

- `cargo check -p chronos --bins` — чисто (только старые warnings).
- `cargo test -p chronos --lib` — **610 passed** (было 609: −2 `hide_strip_*`
  теста на удалённый код, +4 новых теста стиля).

Новые тесты: `style_normal_and_wrapped_parse`, `style_legacy_aliases_parse`,
`style_parse_is_case_insensitive`, `unknown_style_keeps_other_fields`
(banana → `Normal` + сохраняет `wrap.thickness`/`bottom_strip.height`).

**Тест реально ловит** (временное удаление алиасов):

```
test frame::tests::style_legacy_aliases_parse ... FAILED
  left: Normal
  right: Wrapped
```

— то есть `"wrap"` без алиаса падает в дефолт, что и ловит тест (главный
регресс T268).

## Верификация — живой смок (release-сборка)

Шелл перезапущен с новым бинарём, обои белые (`swaybg -c ffffff`), обе панели
открыты. Дословный вывод по пунктам приёмки:

**П.4 — алиас `style = "wrap"` + нестандартная толщина 24 живьём** (hot-reload,
pid не менялся):

```
frame_wrap_excl_left   0 21 24 1440   (было 16 → 24)
frame_wrap_excl_right 2536 21 24 1440  (было 16 → 24)
frame_wrap_excl_bottom 0 1424 2560 16  (bottom не тронут алиасом thickness → left+right)
```

**П.5 — `style = "banana"`:**

```
hyprctl layers | grep frame_wrap_  →  NO frame_wrap_* (Normal mode OK)
grep -c "unknown style"            →  1
  WARN frame: unknown style "banana", falling back to normal
grep thickness frame.toml          →  thickness = 24.0  (уцелел, конфиг не сброшен)
```

**П.3 — 5 переключений wrapped↔normal:**

```
frame_wrap_* layer count  →  4 (ровно matte + 3 excl, сирот нет)
panic / wrap open failed / Protocol error  →  нет
grep -c "Protocol error"  →  0
```

**П.2 — `normal`:**

```
frame_wrap_*        →  NO frame_wrap_* (normal OK)
bottom_strip layer  →  NO bottom strip layer
rail x=20 y=700     →  srgb(24,24,37)        (#181825, рельс на месте)
x=1280 y=1439       →  srgb(254,254,254)     (белые обои, планки нет)
```

**П.1 — `wrapped`:**

```
bottom x=1280 y=1439 → srgb(27,27,40)   (плита есть)
bottom-left угол: (44,1420) → srgb(27,27,40)  — хром заходит в угол,
                        при прямом угле здесь были бы обои (скругление есть)
```

**П.6 — обе темы** (light: плита/бар/рельс единые `#ECEEFA`, normal-низ белый):

```
LIGHT wrapped: bottom=bar=rail = srgb(236,238,250)
LIGHT normal:  bottom = srgb(255,255,255), rail = srgb(236,238,250), frame_wrap_* = NONE
```

**Легаси-warn `[bottom_strip]` (дословно, свежий процесс с `height = 8`):**

```
grep -c "legacy" → 1
  WARN frame: [bottom_strip] is legacy — height/junction/enabled are no longer
  read; the wrapped bottom edge is now wrap.bottom
```

С дефолтным `[bottom_strip]` warn = 0 (не шумим зря).

## Итог по окружению

Восстановлено: `theme.toml` → `Default`, `frame.toml` → исходный
(`style = "wrap"` + `[bottom_strip]` дефолты), обе панели открыты, `swaybg`
остановлен, шелл работает на новом бинаре, `Protocol error = 0`, `panics = 0`.

## Чего не делал

- Не трогал `crates/ui/**`, `crates/services/**`, панели/бар логически — только
  механический compile-fix переименования (см. выше).
- Не трогал `apply_frame_inset` панелей, форк, `wrap.bottom` семантику —
  вне зоны.
- Не коммитил (правки в рабочем дереве).
