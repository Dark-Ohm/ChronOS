# T169 — набор рейла до четырнадцати вкладок

**Статус:** ЗАБЛОКИРОВАНА до приёмки T168. **Роль:** FRONTEND.
Общие правила — `docs/orchestration/agents/RULES.md`.

Вторая задача слайса 3. План —
`docs/superpowers/plans/2026-07-31-right-panel-modularization-slice-3.md`.
Спека — `docs/superpowers/specs/2026-07-30-adaptive-developer-gamer-shell-ide-design.md`,
**§4.1 «Right rail tab set»** — читать буквально, это твой источник истины.

**Не начинай, пока T168 не принята.** Ты садишься на контракт вкладки,
который она отдаёт; до этого его формы не существует, и любая работа
наперёд будет переписана.

**Зона (твоя):**
- `crates/app/src/side_panel_right/tabs.rs`
- `crates/app/src/side_panel_right/tab/**` — только новые файлы вкладок
- `crates/app/assets/icons/` — только четыре новых `rail-*.svg`

**НЕ трогать:** `view.rs` и `mod.rs` (T168 их только что переписала —
подключение новых вкладок делается через контракт, а не правкой точки
входа; если контракт этого не позволяет, это дефект T168, пиши в отчёт),
`rail.rs`, карточки, `dock/**`, `scene.rs`, `monitor.rs`.

**Отчёт:** `docs/orchestration/tasks/report/T169-rail-fourteen-tabs-report.md`.

---

## Что говорит спека, дословно

§4.1, существующие десять:

> System, Files, Editor, Terminal
> AcpSettings, McpSettings, LspSettings, ApiProviders, EditorSettings, HyprlandBinds

§4.1, добавляемые этой задачей:

> Preview, Inspector, Build, SourceControl

> Fourteen tabs total. Ordering groups work tools first, settings second,
> with a visual separator between the groups; settings tabs keep the same
> icon language rather than becoming a different control class.

И отдельно, строка 149 спеки — та, из-за которой в T165 исполнитель
поправил архитектора и был прав:

> Gamer mode replaces the work-tool group with its own tools and **keeps
> the settings group intact**.

**Спека выше этого задания.** Если увидишь расхождение — идёшь по спеке и
цитируешь строку в отчёте. Это плюс, а не отступление.

## Что делаем

### 1. Четыре новых варианта `PanelTab`

`Preview`, `Inspector`, `Build`, `SourceControl` — в группу **рабочих
инструментов**, то есть до настроечных, рядом с Files/Editor/Terminal.
Конкретный порядок внутри группы выбираешь ты и обосновываешь одной
строкой в отчёте.

Каждому варианту нужны `id()`, `parse_id()`, `label()`, `icon_path()` —
всё по образцу существующих в `tabs.rs`.

**`PanelTab::ALL` становится длиной 14.** В `tabs.rs` есть тесты, которые
ходят по `ALL` (`ALL.len() == 10`, `ALL[9] == HyprlandBinds`,
уникальность иконок, покрытие подписей) — их надо **обновить, а не
ослабить**. Тест, из которого убрали утверждение, чтобы он проходил, — это
не починенный тест, это выключенный.

### 2. Композиция по режиму

`for_mode` уже написана в T165 и работает живьём (подтверждено кадрами в
T167: Developer 10 иконок, Gamer 7). Твоя правка — только добавить новые
вкладки в **Developer**.

**Gamer:** четыре новых — рабочие инструменты разработчика, в Gamer их
быть не должно. Группа настроек в Gamer остаётся целой (строка 149).
То есть после задачи: Developer — 14, Gamer — те же 7.

Есть тест на **стабильность относительного порядка общих вкладок** между
режимами (§5 спеки в машинной форме) — он обязан продолжать проходить.
Если упал — значит порядок поехал, и чинить надо порядок, а не тест.

### 3. Иконки — их придётся нарисовать

Проверено архитектором: в `crates/app/assets/icons/` **нет** иконок под
`Preview`, `Inspector`, `Build`, `SourceControl`. Текущий набор рейла:

```
rail-acp.svg  rail-api.svg  rail-binds.svg  rail-editor-settings.svg
rail-editor.svg  rail-lsp.svg  rail-mcp.svg  rail-system.svg  rail-terminal.svg
```

Ссылка на несуществующий файл = вкладка без иконки. Это ловилось в T159:
`code.svg` и `gamepad.svg` в задании были, в дереве их не было.

Формат — ровно как у существующих, один в один:

```svg
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 256 256" fill="currentColor">…</svg>
```

`viewBox="0 0 256 256"`, `fill="currentColor"`, без обводок и без цветов —
цвет даёт тема. Существующие весят 240–675 байт: это геометрия из
нескольких примитивов, а не трассированный рисунок. Держись того же языка
форм — «settings tabs keep the same icon language» из §4.1 относится ко
всему рейлу.

Именование: `rail-preview.svg`, `rail-inspector.svg`, `rail-build.svg`,
`rail-source-control.svg`.

### 4. Содержимого не делаем

Четыре новых вкладки получают **честное пустое состояние** — тот самый
общий компонент, который T168 сделала взамен `coming soon`. Реальные
Files / Terminal / Build / Preview — это **слайс 4** по §14, не твоя
задача. Не начинай их, даже если кажется, что «там на полчаса».

## Тесты

- `ALL.len() == 14`, первая — System, последняя — по факту нового порядка
- уникальность `icon_path()` по всем четырнадцати (тест уже есть, должен
  продолжать ловить)
- `id()` ⇄ `parse_id()` роундтрип для всех четырнадцати, включая
  регистронезависимость и мусор → `None`
- `for_mode(Developer).len() == 14`, `for_mode(Gamer)` — без четырёх новых
  и **с целой группой настроек**
- относительный порядок общих вкладок между режимами не изменился

## Верификация

```
cargo test -p chronos
cargo clippy -p chronos --all-targets
cargo build --release -p chronos
ls crates/app/assets/icons/rail-*.svg
```

**Живой прогон обязателен.** Релиз, `RUST_LOG=info`, режимы — по IPC без
рестартов (`$XDG_RUNTIME_DIR/chronos.sock`, `set-workspace-mode:<mode>`,
`toggle-side-panel-right`), рецепт есть в задании T168 и в отчёте T167.

Кадры:

1. Рейл в Developer — **четырнадцать** иконок, все нарисованы, ни одной
   пустой клетки, разделитель между группами на месте
2. Рейл в Gamer — **семь**, четырёх новых нет, группа настроек цела
3. Каждая из четырёх новых вкладок открыта — честное пустое состояние с
   читаемым текстом

**Кадры смотреть глазами, с увеличением.** Иконки в рейле мелкие, на глаз
по vision их не сосчитать:

```
magick кадр.png -crop 60x900+2500+30 +repage -filter point -resize 300% rail.png
```

Так их считал архитектор при приёмке T167 — 10 против 7 видно однозначно.
Не пиши «вероятно 14»: посчитал — назови число; не посчитал — «не
проверено».

## Коммит

Ветка от `master` **после** приёмки T168. Сообщение: `side_panel_right :
рейл вырос до четырнадцати вкладок по §4.1 (T169)`. Без AI-трейлеров,
`git diff --staged` глазами, поимённый `git add`. **Коммитишь ты.**
