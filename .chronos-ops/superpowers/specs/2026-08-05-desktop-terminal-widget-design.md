# Desktop-terminal widget — от спайка к продукту

**Дата:** 2026-08-05. **Роль:** дизайн, брейншторм с пользователем.
**Источник:** `docs/HANDOFF.md` (2026-07-20/21) — пользователь: «виджет
терминала которым не могу пользоваться»; сегодняшняя жалоба «терминал на
рабочем столе так и не стал тем виджетом что я хотел». T250 (2026-08-05)
закрыл только побочный эффект (zsh-wizard), сам виджет остался спайком
(`desktop_terminal/mod.rs:1-4`: «Not a product widget — fixed
size/position, no Luau API, no skins, no resize/drag/copy»).

## Итог брейншторма (согласовано с пользователем)

1. **Множественность.** Несколько независимых экземпляров терминального
   виджета на столе одновременно (не один статичный).
2. **Полноценный ввод.** Option 1 из вопроса — рабочий терминал, не
   read-only rice-декорация.
3. **Edit Mode интеграция** (Super+Shift+E, существующий глобальный тумблер
   `crate::edit_mode`) — в режиме редактирования: рамка + drag (двигать) +
   resize-хэндл + крестик (удалить) на каждом открытом виджете, кнопка
   "+ Add terminal" в System-табе правой панели.
4. **Настоящий шелл.** Отказ от песочницы T250 (`/tmp/chronos-terminal-
   empty-zdot`) — реальный `$HOME`/`.zshrc`, p10k/oh-my-zsh хуки грузятся
   как в обычном терминале. Применяется **везде**, где используется общий
   `Terminal::launch()` — и десктоп-виджеты, и вкладка Terminal в правой
   панели. Шум промпта — сознательно принятый трейд-офф, не баг.
5. **Kitty-тема.** Косметика (`font_family`, `font_size`,
   `background_opacity`, `foreground`/`background`/`color0-15`) читается
   из `~/.config/kitty/kitty.conf` и применяется к VT100-рендеру.
6. **Слой окна.** Остаётся `Layer::Background` (за иконками стола, как
   сейчас) — rice-эстетика сохранена. Drag НЕ живой (fork не даёт runtime
   API на repositioning layer-shell surface — только `window.resize()` на
   размер, не на позицию): отпустил мышь → окно закрывается и
   переоткрывается на новой позиции. Не "тащишь — окно едет за
   курсором", а "тащишь — на mouse-up окно телепортируется".
7. **PTY переживает drag.** Терминальная сессия (`Terminal`, PTY+VT100)
   выносится из `DesktopTerminalView` (владеет окном) в сервисный реестр
   (владеет процессом) — закрытие+переоткрытие окна НЕ убивает шелл,
   новый `View` подключается к той же живой сессии по `WidgetId`.

## Архитектура

### 1. Реестр виджетов + persistence

`~/.config/chronos/desktop_terminal.toml` (тот же паттерн, что
`monitor.toml`):

```toml
[[widget]]
id = "a1b2c3d4"          # random on creation, stable across restarts
anchor_x = 48.0            # px from left (Anchor::LEFT margin)
anchor_y = 80.0             # px from top (Anchor::TOP margin)
width = 600.0
height = 400.0
```

Нет файла / пустой список = 0 виджетов на старте (в отличие от текущего
спайка, который всегда открывает один). Первый виджет пользователь
создаёт явно через "+ Add terminal" в Edit Mode.

`desktop_terminal::init(cx)` читает конфиг, открывает по одному
`WindowKind::LayerShell` окну на спек (тот же `window_options()`, что
сейчас, но `margin`/`size` берутся из спека, не из констант
`TERM_WIDTH`/`MARGIN_TOP`/`MARGIN_LEFT`).

### 2. PTY реестр (сервисный слой)

Новый модуль `crates/services/src/terminal/registry.rs`:

```rust
pub struct TerminalRegistry {
    sessions: HashMap<WidgetId, Terminal>,
}

impl TerminalRegistry {
    /// Idempotent: returns the existing session if already spawned,
    /// otherwise launches a new one.
    pub fn get_or_spawn(&mut self, id: WidgetId, size: TermSize) -> anyhow::Result<&mut Terminal>;
    pub fn kill(&mut self, id: WidgetId);
}
```

Живёт как `Global`/`AppState`-подобный сервис (сверься с существующим
паттерном `Service` trait в `chronos_services` — не изобретать новый
lifecycle). `DesktopTerminalView::new(cx)` берёт `Terminal` из реестра по
`WidgetId` вместо `spawn_terminal()` напрямую (текущий
`view.rs:297-309`). При закрытии окна (drag/удаление) — `View::drop`
**не** убивает PTY (только явный крестик в Edit Mode зовёт
`registry.kill(id)`).

Тот же реестр используем для side-panel вкладки Terminal? **Нет** — эта
вкладка держит одну сессию на весь lifetime приложения уже сейчас (не
переоткрывается), ей реестр не нужен, трогаем только сам `launch()` (см.
§3).

### 3. Реальный shell config

`crates/services/src/terminal/mod.rs::launch()`:
- Убрать `ZDOTDIR` sandbox константу и `cmd.env("ZDOTDIR", ZDOTDIR)`.
- `ensure_empty_zdotdir` — удалить вызов (функция может остаться мёртвым
  кодом с `#[allow(dead_code)]` и комментарием "T250 sandbox, superseded"
  ИЛИ удалить целиком, юнит-тест `ensure_zdotdir_creates_zshrc_idempotently`
  тоже удаляется — реши по месту, что чище).
- `zsh`/`$SHELL` наследует нормальный `$HOME`, никакого override.
- Живой прогон должен показать: реальный prompt (p10k/oh-my-zsh, если
  настроены), НЕ zsh-newuser-install wizard (обычный `.zshrc` уже
  существует в реальном `$HOME`, T250-класс бага физически не
  воспроизводим на настоящем конфиге).

### 4. Kitty theme parser

Новый модуль `crates/services/src/terminal/kitty_theme.rs`:

```rust
pub struct KittyTheme {
    pub font_family: Option<String>,
    pub font_size: Option<f32>,
    pub background_opacity: Option<f32>,
    pub foreground: Option<Rgba>,
    pub background: Option<Rgba>,
    pub palette: [Option<Rgba>; 16], // color0..color15
}

pub fn load(path: &Path) -> KittyTheme;
```

Парсер: построчно `key value` (kitty.conf — не TOML/YAML, свой формат,
`#`-комментарии, пустые строки игнорировать), поддержать `include
<path>` (относительно директории текущего файла, один уровень
рекурсии — глубже не нужно на практике). Неизвестные ключи — игнорировать
молча (kitty.conf содержит сотни опций, нам нужны только косметические).
Отсутствующий файл → `KittyTheme::default()` (текущие хардкод-константы
`view.rs` FONT_SIZE=…, CELL_W/H default палитра).

`DesktopTerminalView`/панель-таб применяют `KittyTheme` вместо констант
`COLS`/`ROWS`/`CELL_H`/`FONT_SIZE`/`CELL_W` (`view.rs:18-26`) — палитра
идёт в рендер VT100-грида (сверить, как алиасятся ANSI-цвета 0-15 в
текущем парсере `alacritty_terminal`, чтобы не задваивать источник
истины).

### 5. Edit Mode UI

- `desktop_terminal` окна проверяют `edit_mode::is_active(cx)` каждый
  render (паттерн один в один с `bar/widgets/*.rs`).
- В edit mode: рамка (`border` + акцентный цвет), drag-хэндл (весь
  заголовок/верхняя полоса виджета — `cursor_grab`/`cursor_grabbing`),
  resize-хэндл в правом нижнем углу (`on_drag_move`, как
  `side_panel_right/view.rs:670`/`panel.rs:485`, конечное действие —
  не `window.resize` текущего окна, а запись нового `width/height` в
  конфиг + закрыть/переоткрыть).
- Drag mouse-up: посчитать новый `anchor_x/anchor_y` из позиции курсора
  относительно экрана, записать в `desktop_terminal.toml`, закрыть
  текущее layer-shell окно, открыть новое с новым spec (тот же
  `WidgetId` → тот же PTY из реестра, сессия не рвётся).
- Крестик (только в edit mode, отдельно от drag-хэндла — не путать с
  T256 находкой про фейковый крестик в header.rs): `registry.kill(id)` +
  удалить спек из конфига + закрыть окно.
- "+ Add terminal" — кнопка в System-табе правой панели (рядом с
  существующими карточками CPU/RAM/GPU/Wallpapers), видна только в edit
  mode (тот же принцип, что кнопки reorder у bar-виджетов). Создаёт спек
  с дефолтной позицией (например смещённой от последнего добавленного,
  чтобы новые окна не спавнились друг на друге), пишет в конфиг, вызывает
  `desktop_terminal::open_one(spec, cx)`.

## Зона файлов (для будущего тикета/тикетов)

- `crates/app/src/desktop_terminal/{mod.rs,view.rs}` — реестр окон, Edit
  Mode рамка/drag/resize.
- `crates/services/src/terminal/{mod.rs,registry.rs,kitty_theme.rs}` —
  PTY реестр, real-config launch, kitty parser.
- `crates/app/src/side_panel_right/tab/system.rs` (или отдельный файл
  `desktop_terminal_card.rs`) — кнопка "+ Add terminal".
- Новый конфиг `~/.config/chronos/desktop_terminal.toml` + загрузчик
  (аналог `monitor.rs` чтения `monitor.toml`).

Даёт минимум 3 непересекающиеся зоны для параллельных миньонов, если
решишь дробить на подтикеты (registry+persistence / real-config+kitty-
theme / edit-mode UI) — но это не обязательно, можно одним тикетом на
исполнителя, который умеет держать контекст.

## Верификация (для итогового приёмки)

- `cargo build --release -p chronos` + `-p chronos-services` — чисто.
- Юнит-тесты: `TerminalRegistry::get_or_spawn` идемпотентность,
  `kitty_theme::load` парсинг фикстуры (валидный + отсутствующий файл +
  `include`), сериализация/десериализация `desktop_terminal.toml`.
- Живой прогон, обе темы: 0 виджетов на чистом конфиге → Edit Mode → Add
  → виджет появился с реальным prompt (не wizard) → напечатать команду,
  убедиться что реально выполняется → drag → отпустить → окно на новой
  позиции, **та же command history / тот же процесс** (доказательство:
  `echo $$`  до и после drag — тот же PID) → resize → окно новых
  размеров → крестик → окно и процесс исчезли (`ps` не находит PID).
- Kitty-тема: временно поставить тестовый `kitty.conf` с нестандартным
  `font_size`/`background_opacity`/цветом — живой grim показывает разницу
  против дефолта, вернуть родной конфиг после.

## Не входит в scope

- Скроллбэк-поиск, копипаст мышью, множественные PTY-вкладки внутри
  ОДНОГО виджета (splits) — это уже "стать kitty", не "родной десктоп-
  виджет". Если понадобится — отдельная веха.
- Живой drag "едет за курсором" — заблокировано отсутствием
  runtime-repositioning API в форке (см. `layer-shell-windows` skill).
  Если это станет критичным — отдельная веха "add margin-update API to
  gpui-ce fork" (полноценная фичи форка, не consumer-side правка).
