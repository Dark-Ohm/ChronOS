<!-- T083 — migrated 2026-07-22 from orchestration/report-log/hermes-report-15.md — see orchestration/tasks/MIGRATION.md -->

# Hermes — Задание №15: токен-фундамент перед редизайном бара

**Дата:** 2026-07-20. **Статус:** готово к приёмке, build/test/clippy зелёные.
**Зона:** `crates/ui/src/theme/{mod,schemes}.rs`, `crates/luau/src/bar.rs`,
`crates/app/src/bar/{mod,widgets/dock,widgets/clock}.rs` (ровно 6 файлов из брифа).

## Что сделано

### 1. status.* → Catppuccin Mocha (№15.1)
Бриф велел править `DEFAULT_BASE16` в `schemes.rs`. Но вскрылось, что
рантайм-дефолт (`Theme::default()`, используемый `default_scheme()`)
читает `status.*` из `crates/ui/src/theme/mod.rs:172-178`, захардкоженно —
`DEFAULT_BASE16` влияет ТОЛЬКО на `Base16Colors::to_theme()` (путь
solarized, не используется по умолчанию). Если бы я тронул только
`DEFAULT_BASE16`, визуально ничего бы не изменилось.

Поэтому правил по факту в обоих местах (консистентность):
- `mod.rs:172-178`: error `f87171`→`f38ba8`, warning `fbbf24`→`f9e2af`,
  success `4ade80`→`a6e3a1`, info `60a5fa`→`89b4fa`
  (base0a тоже подтянут под teal `94e2d5`, base08 дублирует base0f — осознанно,
  как велел бриф).
- `schemes.rs DEFAULT_BASE16` синхронно теми же хексами (на будущее, если
  `to_theme()` станет дефолтом).

Тест-фикстуру `base16_roundtrip` (mod.rs:256-259) тоже перевёл на
Catppuccin-хексы — иначе греп верификации `#f87171` в `crates/ui/` не был бы
пустым (это были литералы теста, не применяемые цвета, но чистить надо).

### 2. Токен font_mono (№15.2)
Добавил поле `font_mono` в `Theme`. **ОТКЛОНЕНИЕ ОТ БРИФА:** тип —
`&'static str`, а не `SharedString`. Причина: `Theme` деривит `Copy`
(`#[derive(Clone, Copy, PartialEq)]`), а `SharedString` `Copy` не реализует →
компилятор упал с `E0204`. Убирать `Copy` с `Theme` — взрыв по всему дереву
(поле копируется везде). `&'static str` — `Copy`, решает задачу,
`.font_family(theme.font_mono)` компилируется (coerce в `&str`). Значение —
`"JetBrains Mono"` (шрифт установлен, `fc-list` подтверждён Архитектором
ранее).
Применено в `clock.rs`: `.font_family(Theme::global(cx).font_mono)`.

### 3. BAR_HEIGHT 32 → 30 (№15.3)
`crates/luau/src/bar.rs:16` → `30.0`. Грепом проверил: захардкоженной `32`
как высоты бара больше нигде нет (`crates/ui/src/theme/mod.rs:238` `XXL: 32.0`
— spacing-константа, не бар; `BAR_HEIGHT` единственный источник, уходит в
`exclusive_zone` в `bar/mod.rs`).

### 4. Фон бара → bg.tertiary (№15.4)
`crates/app/src/bar/mod.rs:77`: `bg.primary` → `bg.tertiary` (#181825).

### 5. De-hardcode dock (№15.5)
`crates/app/src/bar/widgets/dock.rs:75-80`: сырой `gpui::Hsla { h:0.56,.. }`
на Start-кнопке → `theme.accent.primary`. Грепом проверил весь `dock.rs` —
других сырых Hsla/hex нет.

## Верификация

```
cargo build --workspace        → Finished, 0 новых error/warning (только pre-existing:
                                   tray_menu/mod.rs:154 drop(&mut) и mpris unused imports — не мои)
cargo test --workspace         → 256 passed, 0 failed
cargo clippy --workspace       → в моих 6 файлах новых warning НЕТ
grep 'f87171|fbbf24|4ade80|60a5fa' crates/ui/   → EMPTY
grep 'bg.primary' crates/app/src/bar/mod.rs     → EMPTY
grep 'Hsla {' crates/app/src/bar/widgets/dock.rs → EMPTY
```

Живой прогон НЕ делал (нет дисплея в этой сессии) — как и велел бриф №15
«Живой прогон НЕ делай (нет дисплея у тебя) — это сделает Архитектор».

## Спорные места / решения
- **№15.1 место правки** — см. п.1, бриф ошибочно указывал только
  `DEFAULT_BASE16`; реальный источник `status.*` — `mod.rs`. Исправил оба.
- **font_mono тип** — `&'static str` вместо `SharedString` из-за `Copy` на
  `Theme` (см. п.2). Функционально эквивалентно, `Copy`-совместимо.
- Pre-existing rustfmt-дрейф в `bar.rs`/`dock.rs`/`battery.rs`/`mpris.rs` —
  не трогал (вне зоны, HANDOFF упоминает «rustfmt-дрейф 6 файлов»). Мой
  staged-diff — ровно 6 файлов зоны, чужого WIP нет.

## Коммит
`theme/bar : токен-фундамент — Catppuccin status, font_mono, высота 30, bg.tertiary, de-hardcode dock`
Поимённый `git add` 6 файлов зоны. Без AI-трейлеров.
