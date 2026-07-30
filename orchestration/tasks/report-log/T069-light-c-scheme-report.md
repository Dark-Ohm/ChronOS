<!-- T069 — migrated 2026-07-22 from orchestration/report-log/glm-report-1.md — see orchestration/tasks/MIGRATION.md -->

# GLM Report №1 — светлая тема «Light C» + переключатель схем

**Дата:** 2026-07-20. **Агент:** GLM (Lead Architect ↔ GLM).
**Задание:** `orchestration/agents/GLM.md` (Задание №1).
**Зоны:** `crates/ui/src/theme/schemes.rs`, `crates/ui/src/theme/mod.rs`
(только `init` + хелпер `select_scheme`), `crates/ui/Cargo.toml` (+`tracing`).

## Что сделано

1. **`light_scheme()` переписана под Light C** (`schemes.rs:55`). Старая
   Latte-инверсия удалена. Новая схема — холодная сине-лавандовая база,
   индиго-текст, акцент `#007acc` НЕ переопределён (правило design.md /
   DECISIONS). Имя схемы `"Light"`, описание «Светлая схема ChronOS (Light C)».
2. **Механизм выбора схемы (MVP — env).** `Theme::init` теперь читает
   `std::env::var("CHRONOS_THEME")`; валидное имя (case-insensitive) →
   соответствующая схема из `builtin_schemes()`; мусор/пусто/отсутствие →
   `Theme::default()` + `tracing::warn!` со списком доступных имён при
   мусоре. Поведение без переменной не изменилось (как до ввода механизма).
   Хелпер `Theme::select_scheme(Option<String>)` вынесен отдельно и покрыт
   юнит-тестами (не требует gpui `App`).
3. **`tracing` добавлен в `crates/ui/Cargo.toml`** (workspace-dep, нужен
   для `warn!` в `select_scheme`).
4. **`Debug` добавлен в derive `Theme` и `FontSizes`** — нужен для
   `assert_eq!` на `Theme` в юнит-тестах. `Clone, Copy, PartialEq` сохранены.

## Таблица: hex Light C → токен → роль

| hex (мокап) | токен | роль | источник |
|---|---|---|---|
| `#dde0f2` | `bg.primary` | pageBg — базовый фон страницы/окна | мокап `lightBase.pageBg` |
| `#e6e9fa` | `bg.secondary` | cardBg (accepted) — поверхность карточки/попапа | мокап override «Light C, accepted» |
| `#eceefa` | `bg.tertiary` | cardBase — фон пилюли/свёрнутого | мокап `lightBase.cardBg` |
| `#e0e3f4` | `bg.elevated` | hoverBg — приподнятый слой/hover-фон | мокап `lightBase.hoverBg` |
| `#2c2e4a` | `text.primary` | textPrimary — основной индиго-текст | мокап `lightBase.textPrimary` |
| `#5a5d80` | `text.secondary` | textMuted — приглушённый (вторичный) | мокап `lightBase.textMuted` |
| `#7d80a6` | `text.muted` | chevron — ещё приглушённый (третичный) | мокап `lightBase.chevron` |
| `#9a9dc0` | `text.disabled` | disabled — индиго-лавандовый | **додумано** (разбеливание muted) |
| `#9a9dc0` | `text.placeholder` | placeholder = disabled | **додумано** |
| `#c4c8e6` | `border.default` | cardBorder — разделитель карточки | мокап `lightBase.cardBorder` |
| `#d4d7ee` | `border.subtle` | subtle-разделитель (тоньше default) | **додумано** (осветление default) |
| `#007acc` | `border.focused` | accent — glow-ребро/фокус-контур | мокап `lightBase.accent` (неон в деталях) |
| `#007acc` | `accent.primary` | акцент НЕ переопределён | `Theme::default` (правило design.md) |
| `#007acc` | `accent.selection` | selection — дефолтный | `Theme::default` (MVP) |
| `#1f9bdc` | `accent.hover` | hover — дефолтный | `Theme::default` (MVP) |
| `#c4c8e6` | `interactive.default` | cardBorder — контур контрола | мокап `lightBase.cardBorder` |
| `#e0e3f4` | `interactive.hover` | hoverBg — hover-состояние | мокап `lightBase.hoverBg` |
| `#d4d7ee` | `interactive.active` | active — чуть глубже hover | **додумано** (затемнение hover) |
| `#007acc` | `interactive.toggle_on` | accent — включённый тоггл | мокап `lightBase.accent` (неон в деталях) |
| `#1f9bdc` | `interactive.toggle_on_hover` | hover — дефолтный | `Theme::default` (MVP) |
| Catppuccin Mocha | `status.*` (success/warning/error/info) | статусы — из `Theme::default` | `Theme::default` (MVP, Light C не диктует) |

## Додумано (не из мокапа)

Мокап Light C даёт хексы только для поверхностей/текста/бордера/акцента.
Недостающие токены выведены по духу палитры (холодная сине-лавандовая,
индиго, неон в деталях):

- `text.disabled` / `text.placeholder` = `#9a9dc0` — разбеливание `muted`
  (`#7d80a6`) к лавандовому. Контраст с поверхностью сохраняется, но
  читается как «приглушённый».
- `border.subtle` = `#d4d7ee` — осветление `border.default` (`#c4c8e6`).
  Тонкий разделитель, менее заметный чем default.
- `interactive.active` = `#d4d7ee` — затемнение `hover` (`#e0e3f4`).
  Активное состояние чуть глубже hover, в духе «приподнятый слой».

Все додуманные хексы помечены в коде комментарием `// додумано`.

## Не тронуто (по брифу)

- `DEFAULT_BASE16` / `default_scheme()` — тёмная схема эталон, 0 изменений.
- `accent.primary` = `#007acc` — НЕ переопределён (правило design.md).
- `status.*` — Catppuccin Mocha из `Theme::default`, Light C не диктует.
- crates/app, виджеты, попапы, design/* — чужая зона.

## Верификация

### Сборка + тесты

- `cargo build --release -p chronos` — **зелёный** (19 warnings, все
  pre-existing: `drop(state)` references, unused `Task` в
  `notifications/view.rs` — чужая зона, не мои).
- `cargo test --workspace --lib --bins` — **зелёный**:
  - `chronos-ui`: 9 passed (6 новых: `light_scheme_uses_light_c_palette`,
    `light_scheme_status_kept_from_default`, `select_scheme_default_when_unset`,
    `select_scheme_by_name_case_insensitive`,
    `select_scheme_garbage_falls_back_to_default`,
    `builtin_schemes_contains_default_and_light` + 3 pre-existing).
  - workspace: 131 + 98 + 25 + 4 passed, 0 failed.

### Живой смок (release + grim)

1. **Светлый бар:** `pkill -x chronos` → `CHRONOS_THEME=light
   RUST_LOG=info ./target/release/chronos` → grim. Гистограмма кропа
   бара (1920×30):
   - `#ECEEFA` (54197 px) — основной фон бара (`bg.tertiary`, пилюля).
   - `#E0E3F4` (2157 px) — hover/приподнятый (`bg.elevated`).
   - `#E6E9FA` — карточка (`bg.secondary`).
   - `#9A9DC0` (145 px) — disabled/placeholder.
   - `#2C2E4A` (17 px) — индиго-текст (`text.primary`).
   - `#007ACC` (539 px) — акцент (остался `#007acc`, не перекрашен).
   - Коричневые/тёмные пиксели — иконки (Nerd Font глифы, SVG-маски) и
     обои по краям бара, не тема.
   - В логе нет `tracing::warn!` по теме — схема выбрана чисто.
2. **Тёмный бар (сравнение):** `pkill` → `./target/release/chronos` (без
   переменной) → grim. Гистограмма:
   - `#181825` (54581 px) — `bg.tertiary` (фон бара, как было).
   - `#313244` (2157 px) — `bg.elevated`.
   - `#007ACC` (155 px) — акцент.
   - Поведение по умолчанию не изменилось.
3. **Возврат:** `pkill` → перезапущен тёмный шелл (pid 90147 alive).
   Пользователь остался на тёмном баре, как и было до смока.

### Зоны (поимённый add)

`git diff` — только `crates/ui/Cargo.toml`, `crates/ui/src/theme/mod.rs`,
`crates/ui/src/theme/schemes.rs`, `Cargo.lock` (lock-дрейф от +tracing).
crates/app, виджеты, попапы, design/* — НЕ тронуты.

## Хвосты / эскалация

- **Хардкоды в виджетах не выявлены** на уровне бара: гистограмма светлого
  бара показывает Light C-палитру как ожидаемо, тёмных пятен сверх иконок
  нет. Попапы/launcher/system_popup в этом смоке НЕ открывались — рендер
  светлой темы на попапах отдельно не верифицирован (чужая зона, бриф
  просил только бар). Если при открытии попапов всплывут тёмные хардкоды —
  отдельная задача (свип палитры, как Grok №15 для STYLE.md).
- `Cargo.lock` дрейфнул от добавления `tracing` в `chronos-ui` —
  workspace-dep уже был в дереве, lock просто зафиксировал. Не блокер.
- rustfmt-дрейф в `mod.rs` (parse_hex перенос строки, base16_roundtrip
  массив в одну строку) — авто-форматирование при редактировании, не
  смысловая правка. HANDOFF упоминает rustfmt-дрейф как обычное явление.

## Коммит

`theme : светлая схема Light C + выбор схемы через CHRONOS_THEME`
(по факту — не коммитал, жду добра; staged-дифф чистый, только мои зоны).
