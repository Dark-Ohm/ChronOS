# T166 — единственный резолвер вывода и поведение при hotplug

**Статус:** active. **Роль:** BACKEND. Общие правила —
`docs/orchestration/agents/RULES.md`.

Идёт **параллельно** T164 и T165 — зона не пересекается ни с одной из них.

План слайса — `docs/superpowers/plans/2026-07-31-shell-composition-slice-2.md`.
Спека — `docs/superpowers/specs/2026-07-30-adaptive-developer-gamer-shell-ide-design.md`,
твой раздел §3.6 «Outputs and the pult monitor» + §11 «Refactor», последний
пункт.

**Зона (твоя):**
- `crates/app/src/monitor.rs`
- `crates/app/src/bar/mod.rs` — **только** строка 223
- `crates/app/src/side_panel_left/mod.rs` — **только** строка 55
- `crates/app/src/side_panel_left/hover_strip.rs` — **только** строка 42
- `crates/app/src/dock/context_menu.rs` — **только** строка 129

**НЕ трогать:** `scene.rs` (T164), `side_panel_right/**` и `dock/config.rs`
(T165). В `bar/mod.rs` и `dock/context_menu.rs` — ровно указанные строки, эти
файлы живут рядом с чужими зонами.

**Отчёт:** `docs/orchestration/tasks/report/T166-pult-display-consolidation-report.md`.

---

## Что делаем

§3.6 спеки: «`crates/app/src/monitor.rs::pult_display()` is the only
resolver. Surfaces must not call `cx.primary_display()` directly.»

Половина работы уже сделана в дереве: `pick_display()` в `notifications`,
`system_popup`, `volume_popup`, `notifications/history_popup` и
`dock/context_menu` — тонкие обёртки над `monitor::pult_display`. Осталось
четыре **прямых** вызова `cx.primary_display()` в поверхностях:

```
crates/app/src/bar/mod.rs:223                    .or_else(|| cx.primary_display())
crates/app/src/side_panel_left/mod.rs:55         .or_else(|| cx.primary_display())
crates/app/src/side_panel_left/hover_strip.rs:42 .or_else(|| cx.primary_display())
crates/app/src/dock/context_menu.rs:129          .or_else(|| cx.primary_display())
```

Каждый — размазанный фолбэк. Фолбэк обязан жить **внутри** `monitor.rs`, а
не повторяться в четырёх файлах: сегодня они случайно согласованы, завтра
разъедутся, и «панель уехала не на тот монитор» будут искать в четырёх
местах вместо одного.

**Сначала перечитай эти четыре места сам.** Номера строк даны по
`master` на 2026-07-31 — если дерево уехало, ищи грепом
`rg -n "primary_display" --type rust crates/`, а не по номеру вслепую.
Возможно, часть из них — осмысленный фолбэк на случай, когда
`pult_display()` вернул `None` (нет ни одного дисплея). Если так — фолбэк
всё равно переезжает в `monitor.rs`, а вызов становится `pult_display(cx)`.

## Ловушка, которую надо знать до правок

`pult_display()` **пишет на диск как побочный эффект**: при фолбэке на
крупнейший дисплей он авто-назначает победителя и вызывает `save_config`
(`monitor.rs:97-107`). Это осознанное поведение «auto-designates on first
run», не баг. Но значит: функция не бесплатна и не чиста, и звать её в
цикле рендера — плохая идея. Если по ходу задачи выяснится, что кто-то так
делает, **не чини молча** — напиши в отчёт отдельным пунктом, это тянет на
свою задачу.

## Hotplug (§3.6)

> On hotplug, if the configured pult output disappears, the shell re-resolves
> via the fallback chain and surfaces a visible notice; scene state is
> preserved and re-applied if the output returns.

Что требуется:

1. исчезновение сконфигурированного вывода → пере-резолв по цепочке
   фолбэков, шелл **не падает и не остаётся без хрома**
2. **видимое уведомление** пользователю, не только строка в логе. В дереве
   есть свой сервис уведомлений (`crates/app/src/notifications/`) — используй
   его, а не изобретай второй канал
3. состояние сцены не теряется; возврат вывода → применяется снова

**Про пункт 3 честно:** сцена — зона T164, которая идёт параллельно. Тебе
**не нужно** её читать или писать. Твоя часть — не потерять и не затереть:
пере-резолв вывода не должен ничего сбрасывать. Если для честного «re-applied
if the output returns» нужен API, которого ещё нет, — напиши это в отчёт
как выявленную зависимость, а не выдумывай интеграцию с чужим модулем.

Сегодняшний `warn!` «configured uuid not found among N displays, using
fallback» (`monitor.rs:78`) — это уже половина пункта 1. Проверь, что путь
действительно отрабатывает, а не только логируется.

## Тесты

Резолвинг завязан на `cx.displays()`, поэтому чистыми функциями покрывается
не всё. Что покрыть обязательно:

- выбор крупнейшего по площади из списка (вынеси в чистую функцию, если ещё
  не вынесено — сейчас логика сидит прямо в `pult_display`, `monitor.rs:85-96`)
- сконфигурированный UUID найден → он и выбран
- сконфигурированный UUID отсутствует в списке → фолбэк, конфиг
  перезаписывается на нового победителя
- пустой список дисплеев → `None`, без паники

## Верификация

```
cargo test -p chronos
cargo clippy -p chronos --all-targets
cargo build --release -p chronos
rg -n "primary_display" --type rust crates/
```

Последний греп должен давать попадания **только внутри `monitor.rs`**.
Приложи его вывод целиком — это главное доказательство задачи.

**Живой прогон обязателен:**

1. старт с `RUST_LOG=info`, `hyprctl monitors -j` в отчёт (у нас два вывода,
   HDMI с offset 2560)
2. `hyprctl layers` — бар, обе боковые панели на пультовом выводе
3. кадры `grim` подтверждают, что хром на одном выводе, а не размножен
4. **hotplug живьём:** отключить пультовый вывод (`hyprctl keyword monitor
   <имя>,disable`, вернуть — `,enable` или повторив исходную строку из
   `hyprctl monitors`). Шелл жив, хром переехал, уведомление видно, лог без
   паник. Кадры до/после
5. `~/.config/chronos/monitor.toml` — показать `cat` до и после

Если отключение вывода валит сессию или ломает раскладку — **остановись и
опиши в отчёте**, не воюй с компоновщиком. Это железо архитектора, второй
монитор ему нужен рабочим.

## Коммит

Ветка от актуального `master`. Сообщение: `monitor : единственный резолвер
пультового вывода и пере-резолв при hotplug (T166)`. Без AI-трейлеров,
`git diff --staged` глазами, поимённый `git add`. **Коммитишь ты.**
