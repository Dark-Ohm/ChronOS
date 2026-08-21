НЕТ — левая панель пока выглядит и ведёт себя как прототип, а не как готовый продукт.

# T326 — отчёт QA по левой панели

Дата живой проверки: 2026-08-21. Проверялся существующий `./target/release/chronos` на живой Wayland-сессии Hyprland, монитор DP-1 2560×1440. Код не менялся.

Перед сдачей `cargo build --release -p chronos` завершился успешно за 1,99 с (`Finished release profile`); сборка выдала 81 существующее предупреждение. Live smoke выше выполнен тем же release artifact, процесс после проверки остаётся жив.

## Вердикт

Базовая оболочка панели уже рабочая: rail-only призыв соответствует канону, все восемь пунктов рельса нажимаются, Project переключает scope, `preview-target:` открывает файл, а `compose-and-send:` прошёл полный turn и показал ответ `OK`. До продуктового качества не хватает надёжного возврата в сессию, защиты активного проекта от повторного destructive select и содержимого пяти shell-tabs.

## Блокеры — 4

1. **P1, известный T281/T285: загрузка видимой сохранённой сессии зависает на пустом Chat.** Выбор единственной видимой строки отправил `session/load` для `b9a756be-4d2b-4fc1-a804-802ae03750a6`; через 0,55 с получен `session/load OK; consuming replay via stream_read_turn`. Однако transcript не появился, статус оставался `Thinking…`; через 43,11 с turn пришлось остановить, после чего лог дал только `turn END (reason=cancel)`. Кадры: `frames/19-session-reopen.png`, `frames/20-session-reopen-after-8s.png`, `frames/21-session-load-stop.png`. Это совпадает с уже поставленным upstream-блокером, новый дубль не создавал.
2. **P1, новый: повторный клик по уже активному проекту очищает текущую сессию/чат.** После успешного turn клик по уже подсвеченному `ChronOS` дал `project switched (session cleared)`, `now_project=ChronOS`, `now_session=None`. Причина подтверждается чтением кода: строка проекта безусловно emits `ProjectEvent::Select` и вызывает `set_active` (`tabs/project.rs:140-150`); dispatcher без проверки вызывает `switch_project` (`workspace_view.rs:169-184`), а reducer безусловно сбрасывает `active_session_id` и очищает Chat (`side_panel_left/mod.rs:779-800`). Черновик: `reports-fresh/DRAFT-active-project-reselect-clears-session.md`.
3. **P1 product completeness: пять из восьми назначений рельса — только заглушки.** `Plan` честно пишет `Coming in Slice B`, `Tools`, `Skills` и `Archive` — `Coming in Slice C`, `Context files` — `Coming in Slice B`. Каждая заглушка — одна надпись прямо над видимыми обоями, без plate, навигации или действий. Это честное состояние разработки, но для покупателя явно незавершённый продукт.
4. **P2 UX: строку сохранённой сессии невозможно опознать.** В Sessions видна строка только с `Sun 8:39 PM` и `⋯`; title отсутствует. После smoke-turn текст `T326 smoke: reply OK` в списке не появился. Рендер использует `item.short_title()` (`tabs/sessions.rs:480`), который в этом состоянии вернул пустую строку. Кадры: `frames/04b-sessions-click.png`, `frames/13-sessions-after-send.png`.

## Геометрия и призыв

- `toggle-side-panel-left` открыл только 40 px rail; это ожидаемое поведение, не баг (`frames/01-rail-only.png`).
- В rail-only состоянии `hyprctl layers -j` одновременно показал `side_panel_left_rail` в `x=0, w=40` и прозрачный content surface `side_panel_left_content` в `x=40, w=920`; exclusive edge был `x=40` (`log/layers-01-rail-only.json`).
- `expand-left` с ожиданием больше 0,5 с раскрыл Chat до 560 px видимого dock; оба фиксированных layer surface сохранили геометрию 40/920, exclusive edge стал `x=560` (`frames/02-chat-expanded.png`, `log/layers-02-chat-expanded.json`).
- Свежий Chat не является полностью пустой плитой: видны `Hermes`, `Connected`, модель, `Default`, `YOLO`, attachment, composer placeholder и disabled send; в центре — `No messages yet`.
- Финальный `toggle-side-panel-left` закрыл панель; namespaces rail/content исчезли, а `frame_wrap_excl_left` вернулся на `x=0` (`frames/22-final-closed.png`, `log/layers-22-final-closed.json`). Процесс оставлен работающим, PID 325155.

## Покрытие рельса

| Пункт | Результат | Улика |
|---|---|---|
| Project | PASS: список 3 проектов, branch, Files/Term/remove; переключение ChronOS → Chronos-AUR → ChronOS | `frames/03-project.png`, `frames/16-project-switch-aur-confirmed.png`, `frames/18-project-restored-chronos.png` |
| Sessions | PARTIAL: scope меняется; в Chronos-AUR честное `No sessions`; загрузка сохранённой сессии зависла | `frames/04b-sessions-click.png`, `frames/17-sessions-aur-empty.png`, `frames/20-session-reopen-after-8s.png` |
| Chat | PASS для fresh turn; PARTIAL для resume | `frames/02-chat-expanded.png`, `frames/12-compose-after-10s.png`, `frames/20-session-reopen-after-8s.png` |
| Plan | PLACEHOLDER: `Coming in Slice B` | `frames/05-plan.png` |
| Tools | PLACEHOLDER: `Coming in Slice C` | `frames/06-tools.png` |
| Skills | PLACEHOLDER: `Coming in Slice C` | `frames/07-skills.png` |
| Context files | PLACEHOLDER: `Coming in Slice B` | `frames/08-context-files.png` |
| Archive | PLACEHOLDER: `Coming in Slice C` | `frames/09-archive.png` |

Лог содержит переходы rail-tab для всех восьми назначений (`log/key-events.log`). Для кликов использовался реальный pointer input через `ydotool`, а не только IPC.

## IPC и сценарии

### `preview-target:` — PASS

Команда с абсолютным путём к `README.md` открыла правый editor, загрузила Markdown (6802 bytes), показала `Edit` / `Saved`; `README.md` после проверки не изменён (`git diff -- README.md` пуст). Правый panel детально не оценивался — он вне T326. Улика: `frames/10-preview-target.png`, crop `crops/10-preview-target-right-crop.png`.

### `compose-and-send:` — PASS с backend-noise

Отправлено один раз: `T326 smoke: reply OK`. Лог показывает ровно один `composer: send`, один `turn START`, затем через 5,17 с `ACP streaming reply complete`, `chars=2`, и `turn END (reason=ok)`. На экране появился ответ `OK`, status вернулся в `Connected` (`frames/11-compose-send.png`, `frames/12-compose-after-10s.png`).

Во время turn Hermes stderr вывел `ModuleNotFoundError: No module named 'nemo_relay'`, однако основной turn восстановился и завершился успешно; это не классифицировано как blocker данного smoke.

### Переключение проектов — PASS с отдельным reselect-багом

- ChronOS → Chronos-AUR: active highlight сменился, Sessions показал `No sessions`.
- Chronos-AUR → ChronOS: active highlight и scope вернулись.
- Чужая сессия между проектами визуально не протекла.
- Повторный select уже активного ChronOS destructive — blocker №2.

## Panic / protocol

- Rust panic: **0** (`panicked at|thread .* panicked`).
- Строк с literal `protocol error|Protocol error`: **0**.
- Hermes `Failed to parse JSONRPC message from server`: **1**, на старте; рядом в stderr есть traceback. Приложение продолжило работу.
- `ModuleNotFoundError: No module named 'nemo_relay'`: **1**, во время smoke-turn; turn всё равно завершился `reason=ok`.
- Полный лог валидного запуска: `log/chronos.log`; отфильтрованные события: `log/key-events.log`.
- Первый bootstrap без унаследованных `HYPRLAND_INSTANCE_SIGNATURE` / `XDG_RUNTIME_DIR` не смог подключиться к display; этот инфраструктурный запуск сохранён отдельно в `log/bootstrap-invalid-env.log` и не использовался для продуктовых выводов. Валидный запуск выполнен с параметрами текущего Hyprland instance.

## Конфиги

До запуска сделана точная копия 11 файлов `~/.config/chronos/*.toml` в `config-backup/`. После smoke SHA-256 каждого текущего файла совпал с backup — **11/11 без изменений**. Полные списки: `log/config-before.sha256`, `log/config-after.sha256`.

| Файл | SHA-256 до и после |
|---|---|
| bar.toml | `26af9a89b1b7b95d3e0e83ac7aaf92a6355a76e7ec73f718946d96738b9e415b` |
| dock.toml | `9a86dfcc2178dd8dced716d2538720909350faea75e5a3a34c042d3a43fb991f` |
| frame.toml | `7617c40630d6f6ac1e179c34f80b6352159e65032fbee5712d67ea1b53f94e42` |
| frecency.toml | `a666d769373ead5740e41a122f3fa3b22321b4fede70645090bc11df22808462` |
| launcher.toml | `00f0a04f68da132849c587767dfb1bd5e9a5a3374556a0c1308d509b743e8f9d` |
| monitor.toml | `2b114e95148dbfd777954b5a4e58005a7a678316e5636cedb6d7804b208c8ac6` |
| panels.toml | `bba9070546180194f418cef712483d6cbb18767c9ad6f9edb612ece60fd6d433` |
| projects.toml | `8501e28514db4705caa7747ace78112c434088314ebefa2adcd353de1dd4fb18` |
| scenes.toml | `7c4429e028876f6763eef5e0da2c42b44c904704f837fe320c77fc5b637d9836` |
| theme.toml | `3841c70c58d9bf1faa48617a0a88e3c431c339c2e345decbe9769d2bf2be524f` |
| workspace.toml | `d05510d5d6393c39a1ca6047b1e8428ad42b9d82cc22adbd62ae0188599351ae` |

## `ls frames`

```text
00-closed.png
01-rail-only.png
02-chat-expanded.png
03-project.png
04-sessions.png
04b-sessions-click.png
05-plan.png
06-tools.png
07-skills.png
08-context-files.png
09-archive.png
10-preview-target.png
11-compose-send.png
12-compose-after-10s.png
13-sessions-after-send.png
14-project-switch-aur.png
15-sessions-aur.png
16-project-switch-aur-confirmed.png
17-sessions-aur-empty.png
18-project-restored-chronos.png
19-session-reopen.png
20-session-reopen-after-8s.png
21-session-load-stop.png
22-final-closed.png
```

Всего: 24 полных кадра, 14 crops, 10 log/evidence files и 11 config backups в `.chronos-ops/dump/qa-ux/T326/`. Ранние `04-sessions.png`, `14-project-switch-aur.png`, `15-sessions-aur.png` сохранены как честные промежуточные кадры; подтверждающие кадры имеют суффиксы `04b`, `16`, `17`.

## Что не делал

- Не менял Rust-код, тему, обои, конфиги или `README.md`.
- Не исправлял T281/T285 и не создавал его дубликат.
- Не оценивал правую панель за пределами интеграционного результата `preview-target:`.
- Не делал commit и не редактировал `.chronos-ops/checkpoint/`, `.rules` или `CLAUDE.md`.
