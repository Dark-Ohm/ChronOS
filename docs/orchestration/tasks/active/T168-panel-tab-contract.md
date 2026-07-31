# T168 — контракт вкладки правой панели и снос лесов T157

**Статус:** active. **Роль:** FRONTEND. Общие правила —
`docs/orchestration/agents/RULES.md`.

Первая задача слайса 3. План —
`docs/superpowers/plans/2026-07-31-right-panel-modularization-slice-3.md`,
читать целиком. Спека —
`docs/superpowers/specs/2026-07-30-adaptive-developer-gamer-shell-ide-design.md`,
§4.1 и §11.

Идёт **одна**, без параллельных соседей по этой зоне. T169 садится на
контракт, который ты отдашь.

**Зона (твоя):**
- `crates/app/src/side_panel_right/view.rs`
- `crates/app/src/side_panel_right/mod.rs`
- новые файлы `crates/app/src/side_panel_right/tab/**` (создаёшь ты)

**НЕ трогать:** `tabs.rs` (набор вкладок и композиция по режиму — принято в
T165, растёт в T169), `rail.rs`, `hover_strip.rs`, `header.rs`, карточки
(`disks.rs`, `power_row.rs`, `mpris_card.rs`, `spectrum_row.rs`,
`wallpaper_card.rs`, `surfaces.rs`, `permission.rs`), `dock/**`,
`monitor.rs`, `scene.rs`, `Cargo.toml` воркспейса.

**Отчёт:** `docs/orchestration/tasks/report/T168-panel-tab-contract-report.md`.

---

## Что сейчас в дереве — проверено архитектором, не пересказ

`view.rs` — **792 строки**. Внутри одного `Render::render` живёт всё:
резолв набора вкладок, весь контент System и заглушка для остальных.

**Девять вкладок из десяти рисуют заглушку** (`view.rs:582-596`):

```rust
.when(self.active_tab != PanelTab::System, |col| {
    col.child(
        div().size_full().flex().items_center().justify_center()
            .child(div().text_color(theme.text.muted).child(
                format!("{} — coming soon", self.active_tab.label())
            )),
    )
})
```

**Леса T157 сидят прямо во вкладке System.** Полный список, снят грепом
без обрезки:

```
view.rs:22   use gpui_component::input::{Input, InputState};
view.rs:23   use gpui_component::table::{Column, DataTable, TableDelegate, TableState};
view.rs:24   use gpui_component::v_virtual_list;
view.rs:82   measure_input: Option<Entity<InputState>>,
view.rs:83   /// T157/T158 temporary scaffolding — gpui-component `DataTable` footprint.
view.rs:87   measure_table: Option<Entity<TableState<DemoTableDelegate>>>,
view.rs:93   measure_vlist: Option<Entity<DemoVirtualList>>,
view.rs:447  InputState::new(window, cx)
view.rs:461  div().h(px(40.)).w_full().child(Input::new(state))
view.rs:463  // T157: real gpui-component DataTable consumer (measurement).
view.rs:470  TableState::new(DemoTableDelegate::new(), window, cx)
view.rs:478  DataTable::new(table_state)
view.rs:489  Some(cx.new(|_cx| DemoVirtualList::new()));
view.rs:686  // комментарий про временность measure_* / Demo*
view.rs:699  struct DemoTableDelegate
view.rs:704  impl DemoTableDelegate
view.rs:727  impl TableDelegate for DemoTableDelegate
view.rs:756  struct DemoVirtualList
view.rs:760  impl DemoVirtualList
view.rs:768  impl Render for DemoVirtualList
```

Замер закончен и принят: **+1.96 MiB за Input+Table+VirtualList**, из них
91 % — сам `Input`. Числа лежат в `docs/orchestration/tasks/MIGRATION.md`
(строка T157) и в `docs/DECISIONS.log` (запись 2026-07-28). Демо-таблица с
выдуманными строками при этом до сих пор рисуется в живой панели
пользователя. Это и есть «T157 scaffolding» из §14 пункт 3.

## Что делаем

### 1. Контракт вкладки

Каждая вкладка — **своя GPUI-сущность со своим `Render`**, живущая в своём
модуле под `side_panel_right/tab/`. Панель держит ленивый реестр и создаёт
вьюху **при первой активации вкладки**, не раньше:

```rust
// примерно так; конкретную форму выбираешь ты, но свойства обязательны
tab_views: HashMap<PanelTab, AnyView>,
```

Обязательные свойства, по которым буду принимать:

- **Лениво.** Не открыл вкладку — вьюха не создана. Проверяется тестом или
  логом, а не обещанием.
- **С кэшем.** Ушёл со вкладки и вернулся — та же вьюха, состояние на
  месте. Пересоздание на каждом переключении = потеря скролла и (в слайсе 4)
  перезапуск PTY. Это баг, а не экономия.
- **Без сброса при смене режима.** Вкладка, ушедшая из набора режима, просто
  не показывается. Кэш не чистим — если это окажется дорого, увидим замером.
- **Точка входа одна.** В `render()` — один вызов, отдающий контент активной
  вкладки. Ветвление `when(active_tab == ...)` по вкладкам должно исчезнуть.

Если по типам форк не даёт того, что ты задумал, — **напиши в отчёт, что
именно не сходится, и как обошёл**. Не подгоняй под красивую картинку.
Полезное чтение по форку: скиллы `chronos-gpui`, `gpui`.

### 2. System переезжает первой и служит образцом

Весь контент вкладки System (сеть, диски, питание, MPRIS, спектр, обои,
футер) уезжает в `tab/system.rs`. Карточки как файлы **не трогаем** — они
уже вынесены и здоровы; переезжает только сборка их в экран.

`view.rs` после этого обязан **существенно похудеть**. Если в нём осталось
~800 строк и ветвление по вкладкам — контракт не сделан, а обёрнут; это
прямой признак провала из плана §8.

### 3. Леса T157 — снести

Убираешь `measure_input`, `measure_table`, `measure_vlist`,
`DemoTableDelegate`, `DemoVirtualList` и их рендер из System.

**Что при этом НЕ делаешь:**

- **`Root` не трогаешь** (`mod.rs:34,189`). Обёртка обязательна — без неё
  `Input` паникует на `window.root()` (находка T158). `Input` вернётся в
  слайсе 4, снимать обёртку сейчас значит закладывать панику на будущее.
- **`gpui_component::init(cx)` в `main.rs:78` не трогаешь** — `main.rs` вне
  твоей зоны, и зависимость остаётся по решению `DECISIONS.log` 2026-07-28.
- Зависимость `gpui-component` из `Cargo.toml` **не выпиливаешь**. Если
  после сноса лесов компилятор скажет, что импорты не нужны, — убери
  импорты, а не зависимость.

### 4. Заглушка `coming soon` — заменить

`"{label} — coming soon"` уходит. §13 спеки: «missing integrations are
represented honestly and fail locally». «Скоро» — это обещание срока,
которого никто не давал.

Общий компонент пустого состояния: иконка вкладки, название инструмента,
одна строка «что здесь будет». Текст — **данные вкладки** (метод рядом с
`label()`/`icon_path()` по духу, но в твоей зоне, не в `tabs.rs`), не
копипаста по девяти местам и не строка в `format!`.

Никаких сроков, никаких «в разработке», никаких прогресс-баров.

## Тесты

Чистыми функциями тут покрывается немного, но обязательно:

- **ленивость**: реестр пуст, пока вкладка не активирована; после
  активации в нём ровно одна запись
- **кэш**: активация A → B → A даёт ту же вьюху, а не новую (сравнение по
  идентичности сущности)
- **пустое состояние**: у каждой вкладки без содержимого текст непустой и
  различается между вкладками (защита от копипасты одной строки)

Тест, который воспроизводит логику продукта внутри себя, тестом не
считается — этим уже отличилась T164, не повторяй.

## Верификация

```
cargo test -p chronos
cargo clippy -p chronos --all-targets
cargo build --release -p chronos
rg -n "coming soon" crates/           # должно быть пусто
rg -n "Demo(TableDelegate|VirtualList)|measure_(input|table|vlist)" crates/   # пусто
wc -l crates/app/src/side_panel_right/view.rs
```

Последние три — главные доказательства задачи, приложи вывод целиком.

**Живой прогон обязателен.** Релизный бинарь, `RUST_LOG=info`, лог в файл.
Режим и панель переключаются **по IPC, без рестартов** — сокет
`$XDG_RUNTIME_DIR/chronos.sock`, диспетч `ipc/mod.rs:143-150`, дебаунс
200 мс:

```python
import socket
s = socket.socket(socket.AF_UNIX); s.connect("/run/user/1000/chronos.sock")
s.sendall(b"toggle-side-panel-right")      # открыть панель
s.sendall(b"set-workspace-mode:developer") # сменить режим
s.close()
```

Что снять кадрами:

1. Панель открыта, вкладка System — содержимое **на месте и не поехало**
   (сеть, диски, питание, MPRIS, спектр, обои, футер). Это регрессионный
   кадр, он важнее остальных.
2. Демо-таблицы и поля ввода в System **нет**.
3. Любая пустая вкладка — новое честное состояние, текст читается.
4. Рейл в Developer — 10 иконок, в Gamer — 7 (регрессия T165 не допущена).

**Кадры смотреть глазами.** Мелкий текст — вырезать и увеличить, а не
гадать по vision:

```
magick кадр.png -crop 600x400+1900+200 +repage -filter point -resize 300% out.png
```

В T167 исполнитель написал «не могу утверждать, vision врёт» о кадре,
который лежал у него на диске, и занизил собственный результат. Не
повторяй: посмотрел — пиши что видел; не посмотрел — пиши «не проверено».
«За архитектором» на непроверенном пункте не принимается.

## Коммит

Ветка от актуального `master`. Сообщение: `side_panel_right : контракт
вкладки, System в свой модуль, леса T157 снесены (T168)`. Без AI-трейлеров,
`git diff --staged` глазами, поимённый `git add`. **Коммитишь ты** — это
уже четвёртый раз, когда работа приезжает незакоммиченной, и каждый раз
оформляю я.
