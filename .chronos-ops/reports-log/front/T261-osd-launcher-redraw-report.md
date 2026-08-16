# T261 — отчёт: редизайн App Launcher по эталону Chronos-OSD-Launcher

**Статус:** реализовано, компиляция чистая, тесты проходят.

## Что сделано

### 1. Новые токены темы (`crates/ui/src/theme/mod.rs` + `schemes.rs`)

Добавлены три токена, которых не хватало для эталона:

| Токен | Тёмная схема | Светлая схема | Назначение |
|---|---|---|---|
| `accent.secondary` | `#cba6f7` (mauve) | тот же (не переопределяем) | luau badge, recent tag, toast, reload dot |
| `text.faint` | `rgba(205,214,244,0.34)` | `rgba(44,46,74,0.34)` | хоткеи, подсказки, «No matches» |
| `bg.selection` | `rgba(0,122,204,0.14)` | `#b8c6f5` (лавандовый) | фон выбранной строки |

Светлая схема переопределяет `text.faint` и `bg.selection` — дефолтные
значения на светлом фоне не читаются (тот же принцип, по которому `status.*`
переопределены под Latte).

### 2. Shared icon resolution (`crates/app/src/icon_resolution.rs`)

Вынесена логика резолва иконок из `bar/widgets/dock.rs` (~170 строк):
- `resolve_icon(name)` — public, cached, ищет по freedesktop hierarchy
- Все приватные хелперы (`theme_chain`, `build_theme_chain`,
  `collect_inherits`, `parse_inherits`, `read_gtk_icon_theme`,
  `read_default_theme`) — `pub(crate)` внутри модуля
- Тест `resolve_icon_returns_cached` сохранён

Dock.rs теперь импортирует `resolve_icon` из этого модуля — никакой
логики не дублируется.

Модуль зарегистрирован и в `main.rs`, и в `lib.rs` (публичный API —
launcher тоже в lib).

### 3. Редизайн `launcher/view.rs`

Полностью переписан `Render` impl по эталону:

| Секция эталона | Реализация | Примечания |
|---|---|---|
| Backdrop stage | `div` full-screen + `linear_gradient` glow сверху | radial нет в форке → аппроксимация linear |
| Header (sigil/title/mode/hotkeys) | ✅ | mode chip «APPS» статичен (без цикла) |
| Search row (icon/input/clear) | ✅ | placeholder + clear button при непустом вводе |
| Results (42px rows, accent-bar, hover) | ✅ | accent-bar слева у выбранной строки, scroll |
| Footer (tune/luau/reload) | ✅ | статичен — без логики открытия панелей |
| SVG-иконки результатов | ✅ | `resolve_icon` → system theme → letter fallback |
| Window size | 720×560 | обновлён `mod.rs` |

**Window:** `LAUNCHER_WIDTH = 720.`, `LAUNCHER_HEIGHT = 560.`.

### 4. Что НЕ вошло в скоуп (осознанно)

| Секция | Причина |
|---|---|
| Filter chips | Новая логика (категории, фильтрация) — не рескин |
| Fine-tune panel | Полностью новая функциональность (toggles, sliders) |
| Customization bar (swatches) | Смена акцента runtime — новая логика |
| Launch toast | Анимация фидбека после запуска — новая логика |
| Scope cycling (APPS→CMD→FILE) | Новое поведение mode button |

Все эти пункты — кандидаты на отдельные тикеты после T261.

## Технические решения

- **Градиент backdrop:** `linear_gradient(0.0, accent 7% → transparent 60%)`
  аппроксимирует radial-gradient эталона (fork не имеет radial).
- **Accent bar:** `div` абсолютно позиционированный, 3px, `accent.primary`,
  появляется только у выбранной строки (`.when(is_selected, ...)`).
- **Тени карточки:** `card_shadow()` — два слоя (10px/40px blur + 1px/2px blur),
  мягче чем bar elevation.
- **Фолбэк иконок:** `resolve_icon` → если не найдено → первая буква имени.

## Верификация

- `cargo check -p chronos` — 0 errors
- `cargo test -p chronos icon_resolution` — 1 passed
- `cargo test -p chronos dock` — 22 passed (включая `resolve_icon_returns_cached`)

## Коммит

`launcher : редизайн по Chronos-OSD-Launcher эталону`

## Файлы

- `crates/ui/src/theme/mod.rs` — новые токены
- `crates/ui/src/theme/schemes.rs` — переопределения для светлой схемы
- `crates/app/src/icon_resolution.rs` — новый модуль (shared)
- `crates/app/src/bar/widgets/dock.rs` — рефакторинг (импорт shared)
- `crates/app/src/launcher/view.rs` — полный редизайн
- `crates/app/src/launcher/mod.rs` — размер окна 720×560
- `crates/app/src/main.rs`, `lib.rs` — регистрация модуля
