# T257 — desktop-terminal: PTY-реестр + multi-instance persistence

**Роль:** FRONTEND+BACKEND (Rust, GPUI + services).
**Приоритет:** P1 — фундамент, T258/T259 частично зависят от этого
(WidgetId/spec структуры).
**Источник:** `docs/superpowers/specs/2026-08-05-desktop-terminal-widget-design.md`
§«Архитектура» пп.1-2. Читать целиком перед началом — здесь только вырезка
под эту зону файлов.
**Зависимости:** нет (стартует первым в серии T257/T258/T259).

## Контекст

Сейчас `crates/app/src/desktop_terminal/{mod.rs,view.rs}` — спайк:
`init()` открывает РОВНО ОДНО фиксированное `WindowKind::LayerShell` окно
(`mod.rs:60-76`, константы `TERM_WIDTH=600`/`TERM_HEIGHT=400`/
`MARGIN_TOP=80`/`MARGIN_LEFT=48`), `DesktopTerminalView` (`view.rs:29-39`)
владеет PTY-сессией напрямую (`spawn_terminal()` → `Terminal::launch()` в
`view.rs:297-309`) — сессия умирает вместе с окном.

Нужно: N независимых виджетов, каждый — своё layer-shell окно, но PTY
живёт отдельно от окна (переживает закрытие/переоткрытие — понадобится
T259 для drag).

## Что сделать

### 1. Конфиг + persistence

Новый модуль (например `crates/app/src/desktop_terminal/config.rs`):

```rust
pub struct TerminalWidgetSpec {
    pub id: String,       // random on creation (используй uuid или nanoid-подобное — сверься, что уже есть в Cargo.lock, не тяни новую зависимость без нужды)
    pub anchor_x: f32,
    pub anchor_y: f32,
    pub width: f32,
    pub height: f32,
}

pub fn load() -> Vec<TerminalWidgetSpec>;   // ~/.config/chronos/desktop_terminal.toml, нет файла = пустой Vec
pub fn save(specs: &[TerminalWidgetSpec]) -> std::io::Result<()>;
```

Формат TOML — `[[widget]]` массив таблиц (пример в спеке §1). Паттерн
чтения конфига — как `crates/app/src/monitor.rs` читает `monitor.toml`
(`dirs::config_dir().join("chronos/desktop_terminal.toml")`), не
изобретай новый способ резолвить путь.

**Важно:** нет файла или пустой список = 0 виджетов при старте (не как
сейчас — спайк всегда открывает один). Первый виджет создаётся только
явно через T259 (Add-кнопка), которая появится позже — на этом этапе
(T257) можно временно тестировать через ручную правку TOML-файла или
тестовый хелпер, live-виджет добавлять некому до T259, это ок.

### 2. `desktop_terminal::init()` → N окон

`mod.rs`:
- `init(cx)` читает `config::load()`, для каждого spec зовёт
  `open_one(spec, cx)` (новая функция, оборачивает текущую логику
  `open()` — параметризуй `margin`/`size` из spec вместо констант).
- Публичный `pub fn open_one(spec: TerminalWidgetSpec, cx: &mut App)` и
  `pub fn close_one(id: &str, cx: &mut App)` — понадобятся T259 для
  add/remove/drag, держи сигнатуры простыми и стабильными (T259 будет их
  вызывать, но реализовывать не будет).
- `window_options()` берёт `margin`/`size` параметром вместо констант
  `TERM_WIDTH`/`MARGIN_TOP`/`MARGIN_LEFT` (можно оставить как fallback
  defaults для нового spec, см. T259, но не как единственный источник).

### 3. PTY-реестр (сервисный слой)

Новый модуль `crates/services/src/terminal/registry.rs`:

```rust
pub struct TerminalRegistry {
    sessions: HashMap<String, Terminal>,  // key = widget id
}

impl TerminalRegistry {
    pub fn new() -> Self;
    /// Idempotent — returns the existing session if already spawned.
    pub fn get_or_spawn(&mut self, id: &str, size: TermSize, cell_w: f32, cell_h: f32) -> anyhow::Result<&mut Terminal>;
    pub fn kill(&mut self, id: &str);
    pub fn contains(&self, id: &str) -> bool;
}
```

Живёт как `Global` в GPUI `App` (сверься, как остальные state-держатели
регистрируются — `AppState`/`Service` trait в `chronos_services`, не
изобретай параллельный lifecycle). Создать один раз в `main.rs`/app init,
доступен через `cx.global::<TerminalRegistry>()` / `global_mut`.

`DesktopTerminalView::new(cx)` берёт `Terminal` через
`registry.get_or_spawn(widget_id, …)` вместо `spawn_terminal()` напрямую.
`View::drop`/окно закрывается — PTY **не убивается** (только явный
`registry.kill(id)`, вызывает T259 на крестик).

### 4. Юнит-тесты

- `config::load`/`save` roundtrip (temp dir, не трогать реальный
  `~/.config`).
- `TerminalRegistry::get_or_spawn` идемпотентность: два вызова с тем же
  id возвращают ту же сессию (сверь PID или внутренний указатель, не
  просто "не паникует").
- `TerminalRegistry::kill` — после kill `contains(id)` → false, повторный
  `get_or_spawn` с тем же id создаёт НОВУЮ сессию (не путать с "той же").

## Зона файлов

- `crates/app/src/desktop_terminal/{mod.rs,config.rs}` (новый файл
  config.rs).
- `crates/services/src/terminal/{mod.rs,registry.rs}` (новый файл
  registry.rs; правки в mod.rs — только экспорт `pub mod registry;`, НЕ
  трогай `launch()`/`ZDOTDIR` — это T258, отдельная зона).
- `crates/app/src/desktop_terminal/view.rs` — минимальная правка: брать
  `Terminal` из реестра вместо `spawn_terminal()`. НЕ трогай рендер/тему
  (константы FONT_SIZE/CELL_W/CELL_H — это T258).
- `crates/app/src/main.rs` (или где сейчас регистрируются глобальные
  сервисы) — регистрация `TerminalRegistry`.

**НЕ трогать:** `terminal/mod.rs::launch()`/`ZDOTDIR`/`kitty_theme.rs`
(T258), Edit Mode/drag/resize/add-кнопку (T259 — зависит от этого
тикета, начнётся после).

## Верификация

- `cargo build --release -p chronos -p chronos-services` — чисто.
- `cargo test --release -p chronos-services --lib -- terminal` — все
  зелёные + новые тесты реестра.
- `cargo test --release -p chronos --lib -- desktop_terminal` — новые
  тесты config roundtrip.
- Живой прогон: вручную создать `desktop_terminal.toml` с 2 spec-ами
  (разные anchor_x/y) → `chronos-start` → **два** окна `namespace:
  desktop-terminal` в `hyprctl layers -j` на разных позициях, оба
  печатают/принимают ввод независимо.

## Коммит

`services+ui : desktop-terminal multi-instance registry + persistence (T257)`.

## Отчёт

`docs/orchestration/tasks/report/T257-desktop-terminal-registry-and-persistence-report.md`.
