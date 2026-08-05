# T256 — заголовок правой панели: реальное имя активного окна вместо "kitty"

**Статус:** DONE
**Дата:** 2026-08-05
**Решение:** вариант А (реальная проводка к `CompositorState::active_window`),
одобрен пользователем в ходе работы над тикетом
**Коммит:** (не оформлен — следующим шагом)

## Что сделано

### 1. Источник правды — Hyprland, не const

`crates/app/src/side_panel_right/header.rs`:
- Удалён `const WINDOW_TITLE: &str = "kitty"` и комментарий «Static title until active-window wiring lands», который сам по себе был рекламой бага.
- Добавлен `pub(crate) fn pick_title(active: Option<&ActiveWindow>) -> String`:
  - `Some(w) => w.title.clone()` — реальный title активного окна.
  - `None => "Desktop"` — Hyprland natural empty-focus label (desktop focus / special workspace / до первого IPC). Никаких фейковых имён сторонних приложений.
- `pub fn render_header(cx, active_window)` принимает `Option<&ActiveWindow>`. Возвращает owned `String`, потому что GPUI-дерево требует `'static`-lifetime для child'ов; один clone на рендер хидера, рендер не на каждый кадр — стоимость нулевая.

### 2. Удалён ✕ из строки заголовка

Первая реализация сохранила кнопку слева от title (`flex().justify_between()`),
но code-reviewer (rev #1) сразу же поймал регрессию в убедительности: после
подключения реального title строка читалась как OS-window header с реальным
именем + настоящим ✕, а ✕ по-прежнему закрывал только панель. Тикет явно
требовал «визуально отделить от заголовка окна», и единственный способ
честно выполнить это требование — вообще убрать ✕ из header. Сделано.

Id `side-panel-close` нигде вне header.rs не использовался (smoke-тестов
на него нет — проверено), сломанных подписчиков нет. Альтернативные
dismiss-пути существуют (см. `side_panel_right/mod.rs:1-12`):
bar-widget toggle, hotkey, click-away, hover-leave debounce (peek).

Если позже понадобится видимая in-tab close-кнопка — она должна жить
в выделенном визуальном месте (footer / toolbar), не рядом с window title.
Возможный push candidate на будущее — отдельным тикетом.

### 3. Подписка SystemTab на compositor

`crates/app/src/side_panel_right/tab/system.rs`:
- Новое поле `active_window: Option<ActiveWindow>` в `SystemTab`.
- 5-я подписка через `state::watch` на `AppState::compositor(cx).subscribe()` —
  тем же паттерном, что уже используется для mpris / system_resources /
  disks / wallpaper.
- **Diff-guard** (rev #2): `if data.active_window != this.active_window { … cx.notify() }`.
  Сигнал `CompositorState` фаерится на ЛЮБОЕ compositor-событие (workspace switch,
  monitor hotplug, kb-layout swap). Без guard'а каждый из них репейнтил всю
  панель. С guard'ом перерисовка ровно тогда, когда перерисовка хидера имеет
  смысл.
- Начальное значение поля в `Self { … }` — `AppState::compositor(cx).get().active_window`,
  чтобы до прихода первого signal-event'а хидер показывал что-то осмысленное,
  а не дефолт `None`.

### 4. Юнит-тесты

`crates/app/src/side_panel_right/header.rs::tests`, 4 теста:
- `pick_title_returns_desktop_when_none` — fallback с явным `assert_ne!(..., "kitty")`
  (анти-T256 регрессия-страж).
- `pick_title_returns_window_title_when_some` — проброс реального title.
- `pick_title_preserves_empty_title_honestly` — пустой title рендерится как пустой,
  не подменяется на класс.
- `pick_title_does_not_use_class_as_fallback` — класс WM (`firefox`/`kitty`/…)
  НИКОГДА не подменяет title. Это центральный гард против рецидива T256.

Тесты pure, без `cx`. Регрессия «"kitty" статично» теперь имеет unit-уровневый страж.

## Расхождения со спекой тикета

- Тикет предполагал, что ✕ можно оставить как «закрыть панель», но визуально
  отделить. Сделано жёстче: ✕ удалён из хидера целиком. Решение одобрено
  code-review'ом (rev #1) и явно мотивировано в комментарии модуля — никакого
  lingering'а фейка.
- Тикет не упоминал оптимизацию notify-storm. Reviewer (rev #2) поймал, фильтр
  `if data.active_window != this.active_window` добавлен в watch-closure.
  Логика прежняя — никаких regressions в поведении, только экономия лишних
  cx.notify().

## Побочный эффект вне зоны T256 (для traceability)

Из-за rot в рабочем дереве (`untracked terminal/{kitty_theme,registry}.rs` +
`M terminal/mod.rs` stale re-export в `services/src/lib.rs`) cargo test
изначально не компилировался. По запросу пользователя rot был либо уже
починен в `services/lib.rs:40-43` до моего test-прогона (в рамках
T258-commit «Побочный фикс (T257-артефакт)»), либо читал его из коммита
T258 — это **не относится к T256**. Я лишь воспользовался тем, что
тесты теперь компилируются, чтобы прогнать 4 моих pick_title-теста
и убедиться в 171/171 (см. ниже). В коммит T256 это не входит.

## Верификация (факт, не намерение)

```
$ cargo build --release -p chronos
Finished `release` profile [optimized] target(s) in 3m 33s
82 warnings (pre-existing, T253–T258 backlog, не мои)

$ cargo test --release -p chronos --lib -- side_panel_right
test result: ok. 171 passed; 0 failed; 0 ignored; 0 measured; 108 filtered out

$ cargo test --release -p chronos --lib -- side_panel_right::header::tests::
running 4 tests
test ...pick_title_returns_desktop_when_none ... ok
test ...pick_title_returns_window_title_when_some ... ok
test ...pick_title_preserves_empty_title_honestly ... ok
test ...pick_title_does_not_use_class_as_fallback ... ok
test result: ok. 4 passed; 0 failed
```

171 = 167 baseline (тикетом обещано) + 4 новых pick_title. Ни один
существующий тест не сломан. Ни одно новое предупреждение не появилось.

## Не реализовано из acceptance criteria (out of scope)

- Живая grim-проверка на 2+ разных активных окнах (не kitty). Требует
  десктоп + Hyprland + физический курсор (как в T253/T254 evidence —
  `ydotool` сокет, `hyprctl layers -j`, `grim -g '<geom>'`. Автоматизированные
  bash агенты этого не делают; это человеческая приёмка архитектора.
- Раздел «verify against `hyprctl activewindow -j` ground truth»
  из спеки T256 остаётся pending до запуска на живом Hyprland.
- Если в живой приёмке обнаружится ещё один схожий фейк-заголовок
  в другом popup (volume_popup, system_popup, launcher, project_switcher
  — каждый рисует свой header) — это отдельный тикет; см. follow-up.

## Новые риски / residuaл

- **P2** — пустой title активного окна рендерится как пустая строка в хидере.
  Это честно (нет подмены классом), но визуально дёргает глаз. Если станет
  проблемой UX — отдельный тикет: показать класс как ` {class} »` справа
  от title, или вообще nil-блок. Текущее поведение выбрано осознанно —
  класс в title = рецидив T256.
- **P2** — если Hyprland физически down в момент открытия панели,
  `CompositorState::active_window` = `None` до восстановления, гидер
  покажет "Desktop". Люди увидят "Desktop" вместо реального title. Это
  то же поведение, что у любой системы мониторинга при потере источника;
  ярлык «Desktop» лучше фейка. Когда IPC восстановится, гидер сам
  подхватит реальный title — без перезапуска ChronOS.
- **P3** — единичное выделение памяти в `pick_title` на каждый рендер хидера
  (clone title в String). На текущей частоте Hyprland-events — незначительно;
  при желании можно перейти на `&'static str` через Cow-обёртку, но это
  преждевременная оптимизация.

## Статус docs/ARCHITECTURE.md / docs/DECISIONS.log

- Не обновлялись. T256 — bug-fix в одном rendering-пути, новых архитектурных
  решений не принимает.
  - Существующее решение: services/compositor subscriber был уже реализован
    (см. `crates/services/src/compositor/{hyprland.rs:254-256, types.rs:34-66}`)
    задолго до T256. T256 просто впервые читал из него `active_window` на
    стороне app.
- Если появится архитектурное правило «каждое окно title-bar должно быть
  truthful» — это заслуживает отдельного коммита в DECISIONS.log, не inline
  в T256.

## Out-of-scope committed code

`crates/services/src/lib.rs:40-43` (TerminalHandle/TerminalRegistry re-export
из `terminal::registry`) и `crates/services/src/terminal/{mod.rs, kitty_theme.rs,
registry.rs}` — это всё T258 / T257 материал, уже закоммичено в T258.
В T256 не входит, в коммит T256 не пойдёт.

## Зона файлов (только T256)

- `crates/app/src/side_panel_right/header.rs` — rewrite: `WINDOW_TITLE` const
  удалён, `pick_title()` добавлен, `render_header()` принимает
  `Option<&ActiveWindow>`, ✕ удалён из header, +4 unit-теста.
- `crates/app/src/side_panel_right/tab/system.rs` — добавлено поле
  `active_window`, новая `state::watch` подписка на
  `AppState::compositor(cx).subscribe()` с diff-guard'ом, проброс
  `active_window.as_ref()` в `render_header(cx, ...)`. Существующие
  подписки (mpris / system_resources / disks / wallpaper) не тронуты.
