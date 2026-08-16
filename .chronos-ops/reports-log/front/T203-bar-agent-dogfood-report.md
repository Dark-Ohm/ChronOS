# T203 report

**Зона:** `skills/chronos-bar-config/SKILL.md` (extend), `crates/app/src/bar/agent_api.rs`
(thin glue hook), одна строка видимости в `crates/app/src/side_panel_right/mod.rs`
(`mod preview_target` → `pub(crate) mod preview_target` — вынужденная,
`agent_api.rs` теперь ссылается на `PreviewTarget`/`PreviewIntent` из другого
модуля дерева, они были module-private). `bar/` дальше не трогал —
`T204` (панели/рейлы) параллельно правил тот же `side_panel_right/mod.rs`
своим куском (ширины рейлов 44→36 и т.п.) — застейджил через `git add -p`
только свой один хунк (video проверил `git diff --cached` до коммита и
`git status` после — T204-шный дифф остался нетронутым/unstaged).

## Skill path + NL→key table

`skills/chronos-bar-config/SKILL.md` (уже существовал с T201, расширил, не
переписывал заново). Добавлены разделы:

- **«Natural-language phrase → schema key»** — таблица из ~14 строк,
  покрывает все поля из epic-фразы задания (edge/floating/width/align/
  radius/elevation/margin) + типичные widget-операции (remove/move/add).
  Явно предупреждает: незнакомая фраза → не гадать имя поля, сказать
  пользователю прямо (несуществующий top-level ключ молча игнорируется на
  загрузке — неправильная догадка выглядит как «ничего не произошло», это
  хуже честного «не уверен»).
- **«Full worked example — the epic demo phrase»** — дословно фраза из
  задания («бар снизу, 80% ширины по центру, скругление 12, тень, без cava,
  clock справа»), разобрана в полный TOML-патч со всеми затронутыми полями,
  плюс явное «читай `right`/`center` до записи, `clock` уже справа —
  no-op для этого поля, не выдумывай лишнее действие».
- **«Never restart/pkill»** — отдельный абзац перед таблицей: «hot-reload
  ~300ms, restart/logout/pkill никогда не правильный ответ» — прямое
  требование acceptance-чеклиста задания («No instruction to restart shell
  as happy path»).
- **«Always read before write»** — read-modify-write подчёркнут явно (был
  implicit в исходной версии скилла из T201, сделал explicit отдельным
  абзацем с обоснованием «почему», не только «что»).
- Sanitize-таблица (clamp height/radius/fraction, floating⇒!exclusive) —
  уже была из T201, не переписывал, только сослался на неё из нового
  раздела «Never restart» («если что-то не применилось — проверь тип TOML,
  не советуй рестарт»).
- Новый раздел «After you write: the user sees which file changed» —
  документирует hook ниже.

## Hook / Follow integration (or residual)

**T195 (Follow UI) не построен** — задание явно разрешало residual-путь
(«minimal last-config-path toast **or** open bar.toml in Editor via
PreviewTarget»). Выбрал второй вариант — он был буквально бесплатным,
потому что T194 (`SidePanelRightView` наблюдает `PreviewTarget` и
переключает активную вкладку на Editor) уже существует и не требовал ни
одной новой строки UI-кода.

Добавил в `agent_api.rs`:

1. `set_bar_config`'s success-ветка: `tracing::info!(path, "bar: agent
   applied")` — ровно как просило задание («optionally»-часть я сделал
   безусловной для успеха, не opt-in флагом, потому что залогировать факт
   применения — no-op по стоимости и полезен для любого будущего дебага
   «агент правда писал в файл?»).
2. Новая `point_editor_at_bar_config(cx)`, вызывается из
   `set_bar_config_applied` **только** при `result.ok` (после
   `layout_config::apply(cx)`) — переиспользует существующий глобал
   `PreviewTarget` тем же способом, каким `FilesTab::open_entry` его уже
   выставляет: `path = bar.toml`, `intent = View`, `generation` бампается.
   `SidePanelRightView`'s уже существующая подписка на этот глобал (T194)
   подхватывает изменение сама, переключает правую панель на Editor —
   **ноль новых строк в `view.rs`**, только переиспользование готового пути.

**Честно называю компромисс, не прячу**: это переключает активную вкладку
правой панели на Editor **каждый раз**, когда агент успешно применяет
патч — даже если пользователь был занят чем-то другим в панели. Это
буквально то, чего просит epic-демо («4. User sees which file changed»), но
это не «мягкий toast», это hard tab-switch — тот же UX-компромисс, что уже
принят для Files→Editor (T194). Если это окажется навязчивым в реальном
дневном использовании — тонкая настройка (например, не переключать, если
пользователь уже что-то печатает в другой вкладке) осталась бы за T195 или
отдельной эрратой, не делал её превентивно без сигнала, что она нужна.

## Live dogfood evidence / NOT VERIFIED

**LIVE NOT VERIFIED.** На этой сессии нет живого Hermes-сеанса, через
который можно было бы реально прогнать фразу пользователя и увидеть
изменение бара. Задание прямо разрешает этот исход («If no agent session:
mark LIVE NOT VERIFIED, skill file must still land») — скилл-файл landed,
код скомпилирован и протестирован, живого прогона через реального агента
не было.

**Не писал в реальный `~/.config/chronos/bar.toml` вручную**, чтобы
симулировать «демо» — это живой конфиг пользователя (проверил: файл
реально существует, порядок виджетов отличается от кодового default —
значит пользователь его уже настраивал руками/через edit-mode). Молча
переписать чужой рабочий конфиг ради фейкового смока — ровно то поведение,
которое проект явно осуждает (фабрикация живых прогонов). Вместо этого —
чек-лист ниже, который architect/следующий исполнитель может прогнать
буквально, с реальными командами.

### Smoke checklist (ручной, шаги + ожидаемый фрагмент)

```bash
# 1. Снять текущее состояние (для отката после теста).
cp ~/.config/chronos/bar.toml ~/.config/chronos/bar.toml.smoke-backup

# 2. В agent-панели ChronOS дать Hermes фразу:
#    "бар снизу, 80% ширины по центру, скругление 12, тень, без cava, clock справа"

# 3. Сразу после ответа агента — проверить, что файл реально изменился:
cat ~/.config/chronos/bar.toml
# ожидаемый фрагмент:
#   version = 2
#   [appearance]
#   edge = "bottom"
#   width = "fraction:0.8"
#   align = "center"
#   radius = 12
#   elevation = "soft"
#   ...
#   center = [...]   # без "cava"

# 4. Живым взглядом (не логом) — бар реально переехал вниз, стал уже,
#    появилась тень, cava исчезла из center. grim до/после для протокола.

# 5. Проверить, что редактор справа сам открылся на bar.toml (Follow-hook,
#    §Hook выше) — без ручного клика по Files.

# 6. Проверить лог на строку:
#    grep "bar: agent applied" <лог сессии>

# 7. Откатить бэкап, если правки были только ради смока:
mv ~/.config/chronos/bar.toml.smoke-backup ~/.config/chronos/bar.toml
```

## Верификация

```
$ cargo test -p chronos bar::
test result: ok. 109 passed; 0 failed   (без изменений в числе — hook не добавил новых тестов, только glue)

$ cargo test -p chronos
test result: ok. 208 (lib) + 389 (bin); 0 failed

$ cargo build --release -p chronos
Finished `release` profile [optimized] target(s) in 3m 19s   (exit 0)
```

Новых юнит-тестов не писал — `point_editor_at_bar_config`/лог-строка это
чистая GPUI-глобал-мутация без ветвящейся логики, тестировать пришлось бы
через `#[gpui::test]` с моком `App`; счёл избыточным для одной строчки
переиспользования уже протестированного (T194) паттерна — если architect
не согласен, тривиально добавить.

## Что НЕ сделано

- **Живой прогон через реального агента** — см. выше, LIVE NOT VERIFIED,
  причина честно названа.
- **Полноценный Follow UI (T195)** — сознательно не строил, residual путь
  через `PreviewTarget` выбран заданием как допустимая замена.
- **`.bak`-файл перед агентской записью** — не строил (не входило в
  задание; упомянул в скилле как «пока не существует», честно, не выдумал).
- **System-prompt / character injection для агента** — не нашёл
  существующего механизма инъекции session-промптов в ChronOS (grep по
  `system_prompt`/`character` в `crates/app` ничего profile-специфичного не
  дал за пределами ACP-протокола самого Hermes) — задание разрешало
  пропустить этот пункт, если такого стека нет («find existing character/
  prompt inject; don't invent second stack»); не изобретал новый.
- **Guaranteeing Hermes model compliance** — явно вне scope по заданию,
  задача — качество скилла, не гарантия поведения модели.

---

## Приёмка (Lead Architect / Grok, 2026-08-02)

**Вердикт: ACCEPTED WITH NOTE**

Коммит: `b0d3ff3`.

| claim | check |
|---|---|
| NL → key table in skill | ✅ |
| epic phrase worked example | ✅ |
| no restart/pkill as happy path | ✅ skill text |
| read-modify-write explicit | ✅ |
| set_bar_config log + PreviewTarget hook | ✅ agent_api |
| preview_target pub(crate) | ✅ 1 line justified |
| bar:: 109 green | ✅ re-ran |
| LIVE dogfood | **NOT VERIFIED** (honest) |
| T195 not built | residual ok |

**NOTE (skill accuracy):** T200 v1 — `edge`/`width!=full` **не** live (restart warn).
Skill говорит «всё hot-reload ~300ms» — **частично неверно** для bottom/fraction.
Height/radius/elevation/widgets — live. Follow-up: one-line skill errata
«edge/fraction may need shell restart until fork set_anchor».

**PreviewTarget hard switch on every agent apply** — intentional, same as
Files→Editor; ok for epic, may annoy later → T195.

