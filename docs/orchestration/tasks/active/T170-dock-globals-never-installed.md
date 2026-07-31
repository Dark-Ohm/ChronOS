# T170 — глобалы дока не ставятся никогда: `dock::register` не вызывается

**Статус:** active. **Роль:** FRONTEND. Общие правила —
`docs/orchestration/agents/RULES.md`.

Идёт **параллельно** слайсу 3 (T168/T169) — зона не пересекается ни с
`side_panel_right/**`, ни с `assets/icons/`.

**Зона (твоя):**
- `crates/app/src/bar/widgets/dock.rs`
- `crates/app/src/bar/widgets/mod.rs` — **только** механика регистрации
  (`instantiate`, `apply_layout`, `register_builtin`)
- `crates/app/src/dock/config.rs`, `crates/app/src/dock/signal.rs`,
  `crates/app/src/dock/context_menu.rs`

**НЕ трогать:** `side_panel_right/**` (там идут T168/T169), `scene.rs`,
`monitor.rs`, `workspace_mode.rs`, `Cargo.toml`.

**Отчёт:** `docs/orchestration/tasks/report/T170-dock-globals-never-installed-report.md`.

---

## Находка — проверена грепом, не гипотеза

`crates/app/src/bar/widgets/dock.rs:149`:

```rust
/// Register the dock widget with the global bar registry.
pub fn register(cx: &mut App) {
    // Init dock globals (context menu + config change signal).
    cx.set_global(crate::dock::context_menu::DockMenuState::default());
    cx.set_global(crate::dock::signal::DockConfigSignal::default());

    // Load config cache from disk.
    config::reload_cache();

    cx.global_mut::<chronos_luau::bar::BarWidgetRegistry>()
        .register(Box::new(DockWidget));
}
```

**У этой функции ноль вызовов.** Проверь сам, первым делом:

```
rg -n "dock::register|widgets::dock" crates/
```

Вывод пустой. Виджет дока попадает в бар совсем другим путём —
`bar/widgets/mod.rs:47`, внутри `instantiate`:

```rust
"dock" => Box::new(dock::DockWidget),
```

`apply_layout` зовёт `instantiate` по каждому слоту из `bar.toml` и
регистрирует результат напрямую. `register()` в этой цепочке нет.

**Следствия, все три реальные:**

1. **`DockMenuState` не установлен никогда.** Любое обращение
   `cx.global::<DockMenuState>()` панику даёт гарантированно, независимо
   от `bar.toml`. В `context_menu.rs` таких обращений девять: строки 55,
   86, 93, 161, 171, 177, 190, 202, 218. Именно эта паника лежит в логе
   живого прогона T166 (`/tmp/chronos-t166-evidence/chronos-1.log`):
   `no state of type chronos::dock::context_menu::DockMenuState exists`
   → каскад `The pointer should always be valid when dispatching in
   wayland` → `panic in a destructor during cleanup` → abort.
   **Контекстное меню дока мертво у всех и всегда**, а не при какой-то
   экзотической раскладке.
2. **`DockConfigSignal` не установлен.** `dock/signal.rs:26` делает
   `*cx.global::<DockConfigSignal>()...` — вторая гарантированная паника
   на том же основании.
3. **`config::reload_cache()` не вызывается на старте.**

Компилятор промолчал по той же причине, что и в T166: `pub fn` в модуле,
экспортированном из `lib.rs`, под dead-code-предупреждение не попадает.
Мёртвая публичная функция при зелёной сборке — это у нас уже второй
случай за неделю.

## Вторая половина задачи — пины дока

При приёмке T167 архитектор снял кадры дока с увеличением:

- `~/.config/chronos/dock.toml` содержит **пять** закреплённых:
  `kitty`, `thunar`, `firefox`, `code`, `vivaldi`
- в режиме Developer в доке рисуются **две** иконки: `kitty` и одна с
  буквенной подложкой «T»
- в режиме Gamer рисуются три: `Steam`, `Discord`, `kitty` — то есть
  дефолты режима отрабатывают нормально

Первый подозреваемый — `build_dock_icons` (`dock.rs:163`): он **молча
выбрасывает** всё, что не разрешилось. Это даже закреплено тестом
`build_dock_icons_skips_unresolved` (`dock.rs:352`), то есть поведение
задумано. Вопрос не в том, что оно есть, а в том, что три из пяти
приложений пользователя не разрешаются и **никто об этом не узнаёт**:
ни строки в логе, ни следа в UI.

Второй подозреваемый — не вызванный `reload_cache()` из пункта 3 выше.
Проверь, чем при этом оказывается `cached()` на старте.

**Задача — сначала выяснить причину, потом чинить.** Не начинай с правки
`resolve_icon`. Сначала ответь фактом: `thunar`/`firefox`/`code`/`vivaldi`
не находятся как `AppEntry`, или находятся, но без иконки, или иконка не
резолвится в файл? У тебя есть `resolve_icon_uncached` (`dock.rs:184`) и
`read_gtk_icon_theme` (`dock.rs:299`) — прогони по ним руками и приложи
вывод.

## Что чинить

1. **Глобалы дока обязаны ставиться.** Как именно — решаешь ты, но
   выбор обоснуй в отчёте одной строкой. Два очевидных пути: звать
   `register()` из `register_builtin`/`bar::init`, либо перенести
   установку глобалов туда, где она гарантированно отрабатывает вне
   зависимости от состава `bar.toml`.
   **Осторожно:** `apply_layout` зовётся не только на старте — она
   чистит реестр и перерегистрирует виджеты при смене раскладки (T134).
   Ставить глобал заново на каждый вызов — значит терять состояние меню.
   Глобалы ставятся **один раз**, реестр перестраивается сколько угодно.
2. **Молчаливое выбрасывание пина — заменить на видимое.** Минимум:
   `tracing::warn!` с именем приложения и причиной. Пользователь
   закрепил приложение, оно не показалось, и в логе тишина — так нельзя.
3. Причину непоказа трёх из пяти — починить, **если** она в нашем коде.
   Если окажется, что `firefox`/`code`/`vivaldi` просто не имеют
   `.desktop`-записей под этими именами на машине архитектора, то это не
   дефект: пиши в отчёт что нашёл, чини только логирование, и предложи,
   как пользователю об этом узнавать.

## Тесты

- глобалы установлены после инициализации бара; повторный `apply_layout`
  не сбрасывает `DockMenuState`
- нерезолвящийся пин даёт запись в лог (проверяемо через тестируемую
  чистую функцию, а не через захват логов, если так проще)
- существующие тесты `dock.rs` (`resolve_icon_returns_cached`,
  `build_dock_icons_skips_unresolved`) продолжают проходить — если
  меняешь поведение, меняй и тест **осознанно**, с объяснением в отчёте

## Верификация

```
cargo test -p chronos
cargo clippy -p chronos --all-targets
cargo build --release -p chronos
rg -n "dock::register|widgets::dock" crates/
```

Последний греп после починки обязан показывать живой вызов.

**Живой прогон обязателен** — это оконный код, «тесты зелёные» тут не
значат ничего.

1. Старт с `RUST_LOG=info`, лог в файл
2. **Открыть контекстное меню дока правым кликом по иконке.** Это главный
   пункт: сегодня он гарантированно роняет шелл. Кадр открытого меню.
3. Лог за весь прогон **без** `panicked at`
4. Кадр дока в Developer и в Gamer — сколько иконок рисуется, совпадает
   ли с `dock.toml`
5. Если после починки пинов стало больше двух — кадр до и после

`ydotool` на этой машине: **absolute-координаты = экран / 2** (подтверждено
четырежды: T157, T158, T162, T167). Сокет
`YDOTOOL_SOCKET=/run/user/1000/.ydotool_socket`.

**Кадры смотреть глазами, мелкое — увеличивать:**

```
magick кадр.png -crop 420x36+0+0 +repage -filter point -resize 400% dock.png
```

Именно так архитектор пересчитал иконки дока при приёмке T167 и увидел,
что в отчёте вывод был верный, а обоснование перевёрнутое. Не пиши
«визуально отличается» — назови, что именно за иконки и сколько их.

## Коммит

Ветка от актуального `master`. Сообщение: `dock : глобалы ставятся на
старте, нерезолвящийся пин больше не молчит (T170)`. Без AI-трейлеров,
`git diff --staged` глазами, поимённый `git add`. **Коммитишь ты.**
