# Bar widget layout config — design

_2026-07-24. Первая фаза "системы позиционирования панелей/виджетов".
Референс — Noctalia v5 (`docs.noctalia.dev/v5/bar/`): именованные бары,
три полосы `start/center/end`, TOML + живой GUI-редактор, per-monitor
оверрайды. Мы НЕ копируем Plasma (жёсткие applet-слоты, извращения ради
позиции) — но и не строим с нуля: у нас УЖЕ есть трёхполосная модель._

## Находка, которая определяет весь скоуп

`crates/luau/src/bar.rs` — `BarSection::{Left, Center, Right}` +
`BarWidgetRegistry` (`Vec<Box<dyn BarWidget>>`, `register`/
`replace_by_name`/`unregister_by_name`, добавлены 2026-07-09 для
LuaU-хот-релоада). Это ровно lane-модель Noctalia
(`start/center/end`), только:

1. Порядок внутри секции жёстко забит вызовами в
   `crates/app/src/bar/widgets/mod.rs:26` (`register_builtin`) — не
   конфиг, а код.
2. Нет операции "переставить существующий элемент" — только
   `register` (push в конец), `replace_by_name`, `unregister_by_name`.
3. Нет persistence — набор/порядок виджетов сбрасывается на каждый
   перезапуск шелла к тому, что написано в коде.

Значит эта спека — не "придумать модель позиционирования", а "вынести уже
существующую трёхполосную модель в конфиг + сделать её живой" — то же
самое движение, что уже проделали theme.toml (GLM №2, 2026-07-20) и
dock.toml (Mimo №8) для своих доменов.

## Цель этой фазы (Phase 1 — только бар, только порядок)

Пользователь может редактировать `~/.config/chronos/bar.toml` — какие
виджеты в каком порядке в какой секции — и увидеть изменение в живом баре
без рестарта процесса (тот же inotify+debounce паттерн, что у
`theme.toml`).

## Явно ВНЕ этой фазы (будущие спеки, не путать со scope creep)

- **GUI-редактор drag-and-drop** (тот "не-Plasma" режим, который
  обсуждали) — отдельная фаза, строится ПОВЕРХ конфига этой спеки, не
  раньше. Без него конфиг всё равно полезен (правишь TOML руками, как
  все остальные `*.toml` в проекте).
- **Создание новых панелей на произвольном крае экрана** — это
  единица работы на порядок больше (новое layer-shell окно за раз,
  каждое — свой lifecycle, exclusive zone, resize) и не имеет отношения
  к тому, что уже есть. Отдельная спека, когда до неё дойдёт очередь.
- **side_panel_left/side_panel_right** не переводятся на этот конфиг в
  этой фазе — у них нет `BarWidgetRegistry`/`BarSection`, это
  структурно другие модули (каждый — своя `view.rs` с ручной вёрсткой).
  Распространять конфиг-модель на них — решение для отдельной спеки
  после того, как эта фаза докажет себя на баре.
- **Per-monitor оверрайды** (Noctalia `[bar.<name>.monitor.*]`) — у нас
  и так весь chrome сидит на одном пультовом мониторе
  (`crate::monitor::pult_display`, `DECISIONS.log` 2026-07-19) — не
  актуально сейчас, не проектируем впустую.
- **Капсульная группировка виджетов** (Noctalia group drag) — визуальная
  фича поверх готового движка, не блокирует эту фазу.

## Архитектура

### Конфиг

Новый файл `crates/app/src/bar/layout_config.rs`:

```rust
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(default)]
pub struct BarLayoutConfig {
    pub left: Vec<String>,
    pub center: Vec<String>,
    pub right: Vec<String>,
}

impl Default for BarLayoutConfig {
    fn default() -> Self {
        // Byte-identical to the current hardcoded register_builtin order —
        // zero migration friction, no config file = today's behavior.
        Self {
            left: vec!["dock", "separator", "workspaces"],
            center: vec!["mpris", "cava"],
            right: vec![
                "project", "separator", "volume", "network", "tray",
                "updates", "system", "notification_bell", "separator",
                "battery", "clock",
            ],
        }
    }
}
```

Путь: `~/.config/chronos/bar.toml` (тот же каталог, что `theme.toml`/
`dock.toml`). Отсутствие файла = дефолт выше (текущее поведение,
проверяемо тестом на побайтовое совпадение порядка).

### Таблица имя→регистрация

Каждый `register()` виджета сегодня — свободная функция с разной
сигнатурой (некоторые без параметров, `battery` регистрируется прямым
`Box::new`). Нужна единая точка:

```rust
// crates/app/src/bar/widgets/mod.rs
type RegisterFn = fn(&mut App);

const BUILTIN_WIDGETS: &[(&str, RegisterFn)] = &[
    ("dock", dock::register),
    ("separator", |cx| separator::register(BarSection::Left, cx)), // см. ниже про секцию
    ("workspaces", |cx| workspaces::register(cx)),
    ("mpris", |cx| mpris::register(cx)),
    ("cava", |cx| cava::register(cx)),
    ("project", |cx| project::register(cx)),
    ("volume", |cx| volume::register(cx)),
    ("network", |cx| network::register(cx)),
    ("tray", |cx| tray::register(cx)),
    ("updates", |cx| updates::register(cx)),
    ("system", |cx| system::register(cx)),
    ("notification_bell", |cx| notification_bell::register(cx)),
    ("battery", |cx| {
        cx.global_mut::<chronos_luau::bar::BarWidgetRegistry>()
            .register(Box::new(battery::BatteryWidget));
    }),
    ("clock", |cx| clock::register(cx)),
];
```

**Открытый вопрос, который нужно решить при планировании (не при
брейншторме):** `separator::register` сегодня принимает `BarSection`
явно (нужен разный сепаратор в Left vs Right) — таблица имя→fn выше
теряет эту информацию, потому что вызывается из цикла по конкретной
секции конфига. Решение: не единая глобальная таблица на все три секции,
а тройка таблиц (или таблица, где `RegisterFn` принимает `(BarSection,
&mut App)` и секция передаётся из цикла — тогда `"separator"` в конфиге
`left` регистрируется как left-separator автоматически, без спец-кейса).
Второй вариант чище — зафиксировать в плане.

### Загрузка при старте

```rust
pub fn register_from_config(cx: &mut App, config: &BarLayoutConfig) {
    let registry = cx.global_mut::<chronos_luau::bar::BarWidgetRegistry>();
    registry.clear(); // новый метод — см. ниже

    for (section, names) in [
        (BarSection::Left, &config.left),
        (BarSection::Center, &config.center),
        (BarSection::Right, &config.right),
    ] {
        for name in names {
            match lookup_register_fn(name) {
                Some(f) => f(section, cx),
                None => tracing::warn!("bar.toml: unknown widget name '{name}', skipped"),
            }
        }
    }
}
```

`BarWidgetRegistry::clear()` — новый метод, тривиальный
(`self.widgets.clear()`), нужен и для старта, и для hot-reload (см.
ниже). Неизвестное имя в конфиге → `warn!` + пропуск, не паника — тот же
принцип graceful degradation, что у `theme.toml` (malformed config →
fallback, не краш).

### Hot-reload

Переиспользуем существующий inotify+debounce паттерн (`theme.toml`,
GLM №2, 2026-07-20: parent-dir watch, дебаунс 300мс). На изменение
`bar.toml`: распарсить заново → `register_from_config` (clear + заново
по новому порядку). Виджеты сами по себе не хранят долгоживущее
состояние (рендерят из `AppState`-сервисов через `cx`, см.
`bar/widgets/battery.rs`, `mpris.rs` и т.д.) — полная переустановка
Vec безопасна, живые данные (CPU/сеть/mpris) не теряются, потому что
живут в глобальных сервисах, не в самих `Box<dyn BarWidget>`.
**Это допущение нужно проверить на живом смоке** (Task N ниже) — если
какой-то виджет всё же хранит приватное состояние между рендерами
(нужно перепроверить `cava.rs`/`network.rs` перед реализацией — сетевой
виджет считает скорость через дельту между сэмплами), полная
переустановка может дать один "плохой" кадр сразу после релоада
(значения обнулятся на один тик) — приемлемо, не блокер, но зафиксировать
как известный компромисс, не тихо игнорировать.

## Что дальше с этим НЕ входит в реализацию сейчас

Живая GUI-правка конфига (drag-and-drop, "режим редактирования") —
следующая спека, строится поверх готового `BarLayoutConfig`. Эта фаза
даёт только: TOML-конфиг + hot-reload + сохранение обратной
совместимости (без файла = как сегодня).

## Тестируемость (для будущего плана, не финальные тесты)

- `BarLayoutConfig::default()` даёт тот же порядок, что сегодняшний
  `register_builtin` — сравнение списком имён, не рендерингом.
- Парсинг валидного/невалидного/отсутствующего `bar.toml`.
- Неизвестное имя виджета → `warn!`, не паника, остальные виджеты из
  конфига всё равно регистрируются.
- Живой смок: `bar.toml` с переставленными `network`/`volume` → живой
  бар показывает новый порядок без рестарта (grim до/после, по образцу
  hot-reload смока `theme.toml`).
