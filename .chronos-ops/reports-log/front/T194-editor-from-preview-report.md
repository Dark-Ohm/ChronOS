# T194 report

> **ПРИНЯТА 2026-08-02 с residual.** `7d0be09` edit/save; terminal drawer → T194b; live N/V.

**Зона:** `tab/preview.rs` (evolve, файл/модуль **не** переименован — id
остался `preview`/`"preview"`, см. §2), `tab/mod.rs` (не тронут — не
понадобилось), `preview_target.rs` (не тронут, поведение расширено извне
через `view.rs`, не сам файл), `tab/files.rs` (только комментарий), `view.rs`
(новая подписка на `PreviewTarget` + поле). Ничего вне зоны.

## 1 Решение по имени/id

Оставил модуль `preview.rs` и `PanelTab::Preview`/id `"preview"` как есть —
задание прямо предлагало этот вариант («prefer stable id `preview`→document
as editor to avoid scene.toml churn»). Label уже был переименован в
«Editor» в T192; я это не трогал повторно. Итог: файл называется
`preview.rs`, но содержимое — полноценный view+edit; это задокументировано
в doc-комментарии `is_editable` и в структуре `PreviewTab`, чтобы следующий
читатель не терялся.

## 2 Что сделано

### Editable buffer (T194 §"Scope edit")

- Новая чистая функция `is_editable(kind, truncated) -> bool`: `Text` и
  `Markdown` редактируемы **только когда не truncated**. Обоснование в
  коде и в отчёте: truncated-буфер держит только первые `TEXT_CAP_BYTES`
  (128 KiB, существующий cap T179) — Save при этом состоянии молча
  отрезал бы хвост файла. Я сознательно **не** завёл отдельный
  256–512 KiB edit-cap, который предлагало задание как «e.g.» — переиспользовал
  уже протестированный read-cap и существующий флаг `truncated`, вместо
  двух параллельных порогов, которые могли бы разойтись.
- **Markdown редактируется как raw source, не как отрендеренный вид** —
  задание прямо просило «Text / markdown source (plain buffer)». Значит,
  когда файл редактируется, `gpui_component::text::markdown` (рендер) не
  вызывается вообще — это сознательное упрощение: один буфер, одна кнопка
  Save, без сплита preview/source, соответствует `docs/PRODUCT.md`
  «не второй Zed/VS Code». Не-редактируемые случаи (truncated
  Markdown/Text, Image, WebPreview, Unsupported) идут через прежний
  `render_loaded`/`render_markdown`/`render_text` без изменений.
- **Images остаются view-only** — не тронуто, `render_image` не менялся.
- **Binary/unavailable как в T179** — `render_unsupported`/
  `render_web_unavailable`/`render_error` не менялись.

### Editor state (новые поля `PreviewTab`)

`editor: Option<Entity<InputState>>`, `_editor_subscription:
Option<Subscription>`, `editor_generation: Option<u64>`, `dirty: bool`,
`saving: bool`, `save_result: Option<(bool, String)>`.

- `InputState` (`gpui_component::input`) создаётся **лениво в `render()`**,
  не в `PreviewTab::new` — конструктор `InputState::new` требует
  `&mut Window`, а `new()` его не получает (и не должен: существующие
  тесты создают `PreviewTab` без окна вовсе, см. §5). `render()` уже
  получал `window: &mut Window` (был `_window`, не используемый) —
  переиспользовал этот параметр вместо протаскивания `window` через
  `TabContent::create`/`ensure_tab_view`/`on_tab_select` по всему дереву
  вызовов (это был бы кратно больший блэк-радиус, затронувший
  `tab/mod.rs`, `view.rs` в куче мест — не входило в зону и не требовалось).
- Один `InputState` **переиспользуется между файлами** (не создаётся
  заново на каждый клик в Files) — при смене `generation` (новый файл или
  повторный клик после внешнего изменения) вызывается `set_value` поверх
  того же entity.
- `set_value` **не** эмитит `InputEvent::Change` (проверено чтением
  `state.rs:800-817` в форке gpui-component — `emit_events = false` вокруг
  вызова) — значит программная загрузка контента не путается с
  пользовательским вводом при выставлении `dirty`.
- Подписка на `InputEvent::Change` через `cx.subscribe` — единственный
  реальный триггер `dirty = true`.

### Save

`PreviewTab::save(cx)`: читает `editor.read(cx).value()`, пишет в
`cx.background_spawn` (не блокирует GPUI-поток), по успеху —
`dirty = false` + `tracing::info!`, по ошибке — `save_result =
Some((false, message))` + `tracing::warn!`. Кнопка Save — обычный
`div()+on_click(cx.listener(...))` в стиле остального приложения (`build.rs`
паттерн), **не** `gpui_component::button::Button` — не добавлял новую
поверхность gpui-component сверх уже проводного `input` модуля, footprint
дисциплина из скилла `gpui-component-in-chronos`. Кнопка активна только
когда `dirty && !saving`; лейбл переключается «Save» → «Saving…» → «Saved»;
при ошибке — отдельная строка `"Save failed: {message}"` в шапке.
Успех **не** дублируется отдельным баннером — переключение кнопки в
«Saved» уже честно сообщает результат (§13 — не плодить лишний UI).

### Wire (Files click → Editor)

Не трогал `preview_target.rs`/`FilesTab::open_entry` по логике (только
поправил устаревший комментарий в `files.rs`, который прямо противоречил
новому поведению). Вместо точечной связки Files→View добавил в
`SidePanelRightView::new` подписку на тот же глобал `PreviewTarget`, что
уже читает `PreviewTab` — при непустом `path` вызывает
`this.on_tab_select(PanelTab::Preview, cx)`. Мотив: `files.rs` остаётся
**tab-agnostic** — не знает, что где-то есть Editor-таб, только пишет
общий глобал; будущий T195 (agent follow) получит переключение вкладки
**бесплатно**, выставив тот же `PreviewTarget`, без правки `files.rs`
вообще. Это меньший блэк-радиус, чем протаскивать прямой вызов
`on_tab_select` из `FilesTab` в `SidePanelRightView` (которых у `FilesTab`
и нет — не было ссылки на родителя).

## 3 Что НЕ сделано

- **Ctrl+S** — пропущен. В дереве нет ни одной существующей
  `actions!`/`KeyBinding::new`/`on_action` инфраструктуры для
  `side_panel_right` вообще (проверил grep-ом) — добавление хоткея
  потребовало бы завести новый Action-тип + регистрацию биндинга +
  обработчик с нуля, это не «cheap», как формулировало задание условие
  для опциональности («if key handling exists cheaply»). Save-кнопка
  кликом покрывает цель задачи.
- **Дискард-подтверждение** при переключении на другой файл с
  несохранёнными правками — не реализовано. Задание не просило, а в духе
  «не второй IDE» (`docs/PRODUCT.md` анти-цели) посчитал лишним
  усложнением без явного запроса; при смене файла редактор молча
  перезаписывается новым содержимым (dirty сбрасывается вместе с
  контентом). Явно называю это упрощением, не забытым требованием.
- **Живой прогон** (открыть README, поправить строку, Save, переоткрыть) —
  **не выполнен**. Причина: требует запуска шелла + `ydotool`-клики по
  точным координатам (Files → клик файла → Editor → правка → Save) и
  визуальную проверку через `grim`; в рамках этой сессии не поднимал живой
  инстанс. Честно фиксирую это как несделанное, не как «предположительно
  работает» — механика проверена статически (grep на реальные вызовы
  `InputState::new`/`set_value`/`Input::new` в вендоренном форке,
  совпадающие сигнатуры, компиляция без ошибок), но глазами не смотрел.

## 4 Верификация

```
$ cargo test -p chronos side_panel_right::
test result: ok. 114 passed; 0 failed

$ cargo test -p chronos
test result: ok. 338 passed; 0 failed; 0 ignored

$ cargo build --release -p chronos
Finished `release` profile [optimized] target(s) in 3m 16s   (exit 0)
```

Новые тесты (`preview.rs`): `text_and_markdown_are_editable_when_not_truncated`,
`truncated_text_and_markdown_are_not_editable`,
`non_text_kinds_are_never_editable` — покрывают `is_editable` целиком.
Существующие `PreviewTab`-тесты (T179/T180, `#[gpui::test]` без окна) не
трогал и не сломал — они никогда не вызывают `render()`, поэтому не
задевают новый оконный путь ни разу; убедился в этом чтением их кода
перед тем как полагаться на «зелёные тесты» как доказательство
безопасности рефакторинга.

## Коммит

`7d0be09` — `editor : preview + text edit (T194)`. `git add` поимённо:
`tab/files.rs`, `tab/preview.rs`, `view.rs`. `git status --short` перед
коммитом подтвердил, что в staged ничего лишнего (посторонние удаления
`docs/orchestration/tasks/active/T188…/T190…/T192…/T193…` — не мои, оставил
нетронутыми/нестейдженными, это чужая архитекторская уборка).
