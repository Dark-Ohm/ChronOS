# T257 — desktop-terminal multi-instance registry and persistence

**Дата:** 2026-08-05
**Статус:** код реализован и проверен юнит-тестами + release-сборкой; живой
grim-прогон НЕ выполнен (агент без GUI — см. «Что не сделано»)
**Приоритет:** P1 (фундамент под T258 real-shell и T259 edit/drag)

## Что закрыто

- Реестр PTY-сессий, независимый от окна виджета — закрытие окна не убивает
  оболочку (требование тикета «закрыл виджет → шелл жив», нужно T259 для
  перетаскивания/переоткрытия).
- Конфиг-персистентность `[[widget]]` в
  `~/.config/chronos/desktop_terminal.toml`; пустой/отсутствующий файл → 0
  виджетов (старый spike-автостарт убран).
- N виджетов на разных позициях (anchor_x/anchor_y/size) — параметризуется
  из spec, не хардкод.

## Реализация

**services (GPUI-agnostic, как и требовалось):**

- `crates/services/src/terminal/registry.rs` (новый) — `TerminalRegistry`:
  - `get_or_spawn(id, size, cell_w, cell_h)` — идемпотентный: повторный вызов
    с тем же `id` отдаёт тот же `Arc<Mutex<Terminal>>`.
  - `kill(id)` — удаляет сессию; следующий `get_or_spawn` поднимает свежую.
  - `contains(id)`, `keys()`.
  - Сессии хранятся как `Arc<Mutex<Terminal>>`, потому что `Terminal`
    не `Sync` (внутри `std::sync::mpsc::Receiver`); реестр сам остаётся
    `Send + Sync`.
- `crates/services/src/terminal/mod.rs` — `pub mod registry;`
- `crates/services/src/lib.rs` — `pub use terminal::registry::{TerminalHandle, TerminalRegistry};`

**app (GPUI-глобал, orphan-rule соблюдён):**

- `crates/app/src/desktop_terminal/config.rs` (новый) — `TerminalWidgetSpec`
  (id/anchor_x/anchor_y/width/height), `load`/`save` (toml, по пути из
  `dirs`), `new_id()` через `uuid` (уже в deps), `make_spec()`,
  `config_path()`. Паттерн пути/ошибок — как в `monitor.rs`.
- `crates/app/src/desktop_terminal/mod.rs`:
  - `TerminalRegistryGlobal` (newtype, `impl gpui::Global`) — `Arc<Mutex<TerminalRegistry>>`
    + `Mutex<HashMap<id, WindowHandle<DesktopTerminalView>>>`.
  - `open_one(spec)` / `close_one(id)` — close_one берёт tracked
    `WindowHandle` и дёргает `remove_window()` (как в других попапах, без
    ре-энтрантного `handle.update` — см. HANDOFF «СИСТЕМНЫЙ БАГ»). PTY в
    реестре НЕ трогается.
  - `window_options(spec)` — параметризует margin/size из spec.
  - `init` — `config::load()` → открывает по списку (0 при пустом).
- `crates/app/src/desktop_terminal/view.rs` — `widget_id`, поле
  `terminal: Option<TerminalHandle>`; `spawn_terminal` берёт сессию из
  реестра. `cfg(test)` возвращает ошибку, чтобы юнит-тесты не поднимают
  реальный шелл.
- `crates/app/src/main.rs` — регистрация глобала до `desktop_terminal::init`.
- `crates/app/Cargo.toml` — `tempfile` (dev-dep, для config-тестов).

## Верификация (реальный прогон, не «поверил на слово»)

```
cargo build --release -p chronos -p chronos-services
  → Finished `release` [optimized], 0 errors (только pre-existing warnings)
cargo test --release -p chronos-services --lib -- terminal
  → 20 passed (в т.ч. 5 registry: idempotency / kill / distinct / noop / mutex-drive)
cargo test --release -p chronos --bin chronos -- desktop_terminal
  → 7 passed (config roundtrip / missing file / empty file / parse 2-widget /
    new_id unique / make_spec / path)
```

Тесты idempotency действительно проверяют указательное равенство `Arc`
(один и тот же `Mutex` при повторном вызове) и что `kill`+ресурспавн даёт
уже *другой* указатель.

## Что НЕ сделано (и почему)

- **Живой grim-прогон**: `hyprctl layers` показывает 2 окна
  `desktop-terminal` на разных позициях + `grim -g` кадры в 320px для
  проверки реального рендера. Не выполнено — у агента нет GUI/Wayland-сессии
  (это зафиксировано в памяти: LIVE SMOKE агент не может). Код к этому
  готов: `init` откроет N окон из TOML, `open_one` параметризует позицию из
  spec. Проверка за живым запуском — на тебе (или отдельный T-заход с
  человеком за экраном).
- **Коммит**: не делал без слова. Готов одним коммитом
  `services+ui : desktop-terminal multi-instance registry + persistence (T257)`
  по тикету — скажи, и запушу/закрою.

## Файлы

- новые: `crates/services/src/terminal/registry.rs`,
  `crates/app/src/desktop_terminal/config.rs`
- изменены: `crates/services/src/{lib.rs,terminal/mod.rs}`,
  `crates/app/src/desktop_terminal/{mod.rs,view.rs}`,
  `crates/app/src/{main.rs,Cargo.toml}`
