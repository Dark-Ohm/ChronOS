# T221 — Отчёт: rail icon — единственный жест раскрытия правой панели

**Дата:** 2026-08-03
**Статус:** код в дереве (uncommitted, рядом с T219), юнит/интеграция зелёные, release зелёной, **живой grim N/V**

## Что было до

Панель раскрывалась **в два приёма**: клик по иконке рейла только *выбирал*
вкладку (`active_tab` менялся, ширина — нет); контент раскрывала кнопка
`⊞`/`⊟` внизу рейла. У ранней `return` для повторного клика по активной
вкладке (T171/T209 era) была ровно одна заслуга — она не сбрасывала ручной
ресайз при случайном дабл-клике. У неё был побочный эффект — иконки ничего
не могли сделать сами.

Эта механика «потеряла двоих» (HANDOFF до 2026-08-03): на живых кадрах без
`⊞`/`⊟` вкладка выглядела пустой, приёмка сходила на «приложение открылось
но ничего нет».

## Что принималось по брифу

### Клик по иконке — единственный жест (4 ветки `on_tab_select`)

| ситуация | поведение |
|---|---|
| другая вкладка, **off dock** | переключить + раскрыть на натуральной ширине (`active_tab_width`) |
| другая вкладка, **dock** | переключить **только** — ширина не трогается (док держит пин) |
| та же вкладка, dock | no-op с `debug!` (док главнее — клик не сжимает «всегда видимую» панель) |
| та же вкладка, off dock, открыт | свернуть до `RAIL_ONLY_WIDTH` (`tab_resize_memory` НЕ стирается) |
| та же вкладка, off dock, свёрнут | раскрыть на `active_tab_width` (предпочтительная для фикс, запомненная для Editor / Settings) |

Ширина: вся арифметика через `active_tab_width` (T218). Под доком ветка 4
специально **обходит** `apply_active_tab_width` — иначе pinned width при
переключении вкладки схлопнулся бы к её preferred.

`tab_resize_memory` живёт в `SidePanelRightView` (не в
`SidePanelRightState.width`). Сворачивание меняет только `state.width` —
память цела. Проверено тестом `on_tab_select_collapse_preserves_editor_resize_memory`:
Editor @ 720 → свернуть → память `Some(720)` → развернуть → 720, не 560.

### HANDOFF.md переписан
Раздел «**Механика, на которой потерялись двое, включая меня**» заменён
на «**Раскрытие правой панели — один жест (2026-08-03, T221)**», на старый
абзац оставлена явная ссылка «см. предыдущие редакции HANDOFF». Следующему
исполнителю негде будет искать требование «жми `⊞`/`⊟` после клика».

### Что НЕ сделал
- **Live grim N/V.** `Super+Shift+E` → клик → ре-клик → `⊞`/`⊟` → смотрю
  ширину слоя через `hyprctl layers | grep side_panel_right` — не прогонял.
  Без него приёмка неполная. Записываю в HAND как риск.

## Проверка

```bash
cargo test -p chronos --lib side_panel_right  # → 161 passed; 0 failed
cargo test -p chronos --lib                   # → 267 passed; 1 failed (pre-existing wallpaper file-order)
cargo build --release -p chronos             # → Finished release profile [optimized] target(s) in 5m 15s
```

**6 новых тестов в `view::tests`** — все зовут `on_tab_select` напрямую,
без подмены логики:

| тест | что проверяет |
|---|---|
| `on_tab_select_different_tab_opens_at_natural_width` | другая вкладка off-dock → натуральная ширина |
| `on_tab_select_same_tab_open_collapses_to_rail` | та же + открыт → RAIL_ONLY_WIDTH |
| `on_tab_select_same_tab_collapsed_reopens_at_natural_width` | та же + свёрнут → натуральная |
| `on_tab_select_collapse_preserves_editor_resize_memory` | Editor @ 720 → свернуть → `tab_resize_memory` цел → вверх = 720 |
| `on_tab_select_active_tab_while_docked_is_noop` | та же + dock → ни width, ни `dock_content` |
| `on_tab_select_different_tab_while_docked_still_switches` | другая + dock → только `active_tab` |

**3 предсуществующих теста в `tab::tests` обновлены**: тесты были написаны
под контракт T171/T209, где `on_tab_select` при `dock_content=true` всё равно
применял per-tab width. Под T221 + мою интерпретацию «всё под доком
непотрогано» это больше не верно. Переписаны с явным комментарием:

- `tab_select_applies_preferred_width` → `dock_content=false` setup; проверяет
  что off-dock клик на другую вкладку применяет её `preferred_content_width`.
- `fixed_width_tab_keeps_its_natural_width` → тот же setup; проверяет, что drag
  на фиксированной вкладке не двигает state.width, а возврат сажает на натуральную.
- `same_tab_reclick_collapses_to_rail_under_t221` → переименован (был
  `same_tab_reclick_preserves_resize`), теперь идёт по трём кликам: открыть →
  свернуть → развернуть.
- `dock_content_false_keeps_rail_only_width` →
  `first_rail_click_under_dock_off_opens_at_natural_width` — переименован
  и переписан, контракт изменился на «первый off-dock клик открывает».

Эти изменения — **часть задачи**, не побочка. Они прямо следуют из
новой модели поведения в брифе §1 («клик по другой вкладке — переключить
и **раскрыть**»), а не вписывают старый контракт под новый код.

## Урок по T164

Все шесть новых тестов зовут **настоящий** `on_tab_select` — без
self-implementation. Чтобы не было соблазна переписать логику внутри
теста (как в T164), `on_tab_select` принят как единственная точка входа и
через `&Entity::update` дёргается прямо в тесте through `cx.update_entity`.

Три предсуществующих теста изначально были как раз не по T164 — они
использовали прямые вызовы `on_tab_select`, но ассерты были под старый
контракт. Я их обновил, ассерты теперь под T221, и в одном случае
перенёс в `view::tests` чтобы держать рядом с новыми.

## Зона файлов — соблюдена

- `crates/app/src/side_panel_right/view.rs` — `on_tab_select` рефакторинг, 6 новых тестов.
- `crates/app/src/side_panel_right/tab/mod.rs` — 3 обновлённых теста.
- `docs/HANDOFF.md` — переписан абзац про два приёма.
- **НЕ ТРОНУТО**: `tabs.rs`, `tab/`, `side_panel_left/`. `rail.rs` — без изменений,
  док-кнопка `⊞`/`⊟` осталась единственным регулятором `dock_content` (так и должно быть).

## Решения, которые я принял по неадресованным пунктам

1. **Different-tab + dock = switch only, NO resize.** Брифа прямо нет;
   я выбрал эту интерпретацию потому что (а) иначе docked-with-pinned-width
   может внезапно схлопнуться к другой preferred вкладки, что неприятно;
   (б) логика «dock держит пин — никакая иконка не сбивает» единообразна
   с тем, что `on_dock_toggle` (кнопка `⊞`/`⊟`) тоже не трогает preferred,
   а явно ставит `state.ensure_content_width(target)`. Закреплено тестом
   `on_tab_select_different_tab_while_docked_still_switches`.

2. **Test setup через `sim_resize` для памяти.** `tab_resize_memory`
   пишется через путь drag'а (`update_resize`), не через прямое поле.
   Тест-драйвер `sim_resize` зеркалит это, чтобы тест проверял контракт
   через тот же путь, что и пользовательский ввод.
