# Worktree automation — design

**Дата:** 2026-08-16
**Статус:** согласован в диалоге, ждёт твоей вычитки файла перед планом реализации.
**Продолжает:** T272 (гигиена воркетри и сборки) — этот дизайн реализует
механику поверх правил, зафиксированных там; не переопределяет их.

## Проблема

Активно ведутся 5 репозиториев (ChronOS, Chronos-Engine, Chronos-FM,
Chronos-lm, Source), в каждом — свои воркетри под тикеты, у каждого своя
раскладка (сосед vs `.worktrees/` внутри репо), свой default-branch
(`master`/`main`/`chronos-main`). Создание воркетри, сбор статуса по всем
сразу и детект дрифта делаются руками. T272 уже написал правила
дисциплины (база доказывается, свой `target/`, дерево умирает с тикетом) —
но их применение по-прежнему ручное, каждый раз заново.

**Явно out of scope:** Chronos-IDE, Chronos-Editor, Chronos-AUR — не
трогаем ни конфигом, ни скриптами. Не путать с флагом `hermes -w`
(workdir Гермеса) — это другая ось, не имеет отношения к этим скриптам.

## Границы автоматизации (сознательно не трогаем)

- **Какой тикет создавать / кому раздавать** — решение архитектора, не
  автоматизируется.
- **Мерж и приёмка** — только руками, через существующий процесс
  «приёмки».
- **Удаление воркетри** — только руками (`wt-rm`), автодетект лишь
  подсказывает кандидатов, никогда не удаляет сам.
- **LLM-слои (drift, digest)** — read-only отчёты. Ни один из них не
  имеет доступа к git-командам на запись, terminal или file-write внутри
  воркетри. Технически это гарантируется тем, что оба LLM-шага реализованы
  как `--no-agent` bash-скрипты с одним HTTP-запросом к инференсу — там
  физически нет agent-loop и нет тулов, а значит нечем действовать.

## Расположение файлов

Корень `chronos-ecosystem` — не git. Ничего туда не кладём как git-артефакт.

- **Скрипты + конфиг** → `ChronOS/tools/wt/` (версионируется вместе с
  ChronOS; T272 — это правило в `docs/ARCHITECT.md`, каталога `tools/wt/`
  под него ещё не существует, создаём с нуля этим планом).
- **Отчёты** (`STATUS.md`, `DRIFT.md`, `DIGEST.md`) →
  `/home/neo/projects/chronos-ecosystem/.wt-status/` — вне всех
  репозиториев. Причина: генерируются каждые 15 мин / 2ч, если положить их
  внутрь ChronOS/FM/Engine — это каждые 15 минут пачкает `git status`
  живых исполнителей в тех же деревьях.
- **Этот spec** → `ChronOS/docs/superpowers/specs/`.

## Конфиг репозиториев

`ChronOS/tools/wt/repos.yaml`, одна запись на репозиторий, поля:

```yaml
repos:
  ChronOS:
    root: /home/neo/projects/chronos-ecosystem/ChronOS
    default_branch: master
    worktree_parent: /home/neo/projects/chronos-ecosystem   # сосед
    name_pattern: "ChronOS-wt-t{ticket}"
    branch_pattern: "feat/t{ticket}-{slug}"
    task_glob: docs/orchestration/tasks/active/*.md
    alias_bare: true          # читать старые wt-t### рядом с экосистемой
    alias_legacy: none
    exceptions: [ChronOS-wt-measure]   # долгоживущие, не тикетные — T272

  Chronos-FM:
    root: /home/neo/projects/chronos-ecosystem/Chronos-FM
    default_branch: main
    worktree_parent: /home/neo/projects/chronos-ecosystem/Chronos-FM/.worktrees
    name_pattern: "FM-wt-t{ticket}"
    branch_pattern: "feat/t{ticket}-{slug}"
    task_glob: docs/orchestration/tasks/active/*.md
    alias_bare: false
    alias_legacy: t-slug      # t051-dnd
    exceptions: []

  Chronos-Engine:
    root: /home/neo/projects/chronos-ecosystem/Chronos-Engine
    default_branch: chronos-main
    worktree_parent: /home/neo/projects/chronos-ecosystem
    name_pattern: "Engine-wt-t{ticket}"
    branch_pattern: "t{ticket}-{slug}"
    task_glob: null
    alias_bare: true
    alias_legacy: none
    exceptions: [Chronos-Engine-upstream-test]

  Chronos-lm:
    root: /home/neo/projects/chronos-ecosystem/Chronos-lm
    default_branch: master
    worktree_parent: /home/neo/projects/chronos-ecosystem
    name_pattern: "lm-wt-t{ticket}"
    branch_pattern: "feat/t{ticket}-{slug}"
    task_glob: null
    alias_bare: true
    alias_legacy: none
    exceptions: []

  Source:
    root: /home/neo/projects/chronos-ecosystem/Source
    default_branch: main
    worktree_parent: /home/neo/projects/chronos-ecosystem
    name_pattern: "Source-wt-t{ticket}"
    branch_pattern: "feat/t{ticket}-{slug}"
    task_glob: null
    alias_bare: true
    alias_legacy: none
    exceptions: []
```

Новый репо в скоуп = новая запись, без изменения кода скриптов.
`task_glob: null` — Engine/lm/Source сейчас без `docs/orchestration`; когда
появится, добавляется тем же способом, что у FM.
`alias_bare: true` включает чтение `wt-t###` (старая безрепная схема).
Не хардкодить путь экосистемы в `lib.sh` — иначе scratch-фикстуры и
переезд корня ломают парсер. `alias_legacy: t-slug` — только FM
(`t051-dnd`).

**Столкновение имён (почему `name_pattern` несёт префикс репо).** Все пять
репозиториев делят `worktree_parent` = корень экосистемы (сосед), а номера
тикетов независимы между репозиториями — `T060` у FM и `T060` у ChronOS не
одна и та же задача. Плоский `wt-t{ticket}` для двух репо в одном каталоге
даёт коллизию имён на диске. `name_pattern` обязан нести репо
(`ChronOS-wt-t###`, `Source-wt-t###`, ...) — так уже называется реально
живой `ChronOS-wt-t266`.

Живые деревья по старой безрепной схеме (`wt-t285`, `wt-t290`, `wt-t291`,
`wt-t296`) — это префактный факт, не цель для ренейма прямо сейчас. `wt-status`
принимает их как алиас (см. ниже), `wt-new` создаёт только по новой схеме
с префиксом репо.

## `tools/wt/wt-new.sh <repo> <ticket> [slug] [--base <sha>]`

**Что wt-new НЕ делает:** не пишет и не копирует brief. Brief
(`docs/orchestration/tasks/active/T###-slug.md`) живёт в основном дереве
репозитория и пишет его архитектор, до или независимо от создания воркетри.
Воркетри — это чекаут того же git, поэтому brief, закоммиченный в основную
ветку, и так виден из воркетри через `default_branch..HEAD`/файловую систему
основного дерева. Копировать `docs/orchestration` и генерировать
`T{ticket}-*.md` заново внутри воркетри — либо задваивает то, что уже есть в
коммите, либо прячет реальный brief на feature-ветке, где архитектор его не
ищет. **Требование к brief** (обязанность архитектора, не скрипта) —
машинный блок:
```
## Scope (machine)
allow:
  - <путь-паттерн>
deny:
  - <путь-паттерн>
base: <sha>
```
`wt-drift` читает этот блок из `task_glob` **основного дерева репо**, не из
воркетри.

Что делает `wt-new`:

1. **База.** `--base <sha>` — использовать явно; если флаг не передан —
   HEAD текущего `default_branch`. Проверка: `git -C <root> merge-base
   --is-ancestor <sha> <default_branch>` (sha обязан быть предком
   default-ветки, иначе отказ с понятной ошибкой). Никакого чтения
   зависимости «из конфига/задания» — единственный источник sha это флаг.
2. `git worktree add <worktree_parent>/<name_pattern> -b <branch_pattern>`
   от найденной базы.
3. **Sidecar target, не tracked-конфиг.** Пишет `CARGO_TARGET_DIR` НЕ в
   `.cargo/config.toml` внутри воркетри (это либо уедет в коммит, либо
   испачкает git status untracked-файлом в tracked-дереве) — а как
   untracked sidecar-каталог рядом с воркетри, по уже живой конвенции
   (`wt-t285-target/`, `wt-t291-target/`): `<worktree_parent>/<name>-target/`.
   Плюс маленькая untracked-обёртка (`.envrc`/shell-hook) внутри воркетри,
   которая экспортирует `CARGO_TARGET_DIR` на этот sidecar — сам файл-хук
   untracked, в `.gitignore` дерева не обязателен (он вне поддерева, если
   репо это позволяет, либо явно в локальном `.git/info/exclude`).
4. Если репо не в конфиге — ошибка с подсказкой добавить запись, никакой
   магии по умолчанию.

**Известное ограничение v1: стек на незамёрженный тикет не поддержан.**
`--is-ancestor` требует, чтобы `<sha>` был предком `default_branch` —
буквально «уже в мастере». Тикет, стоящий на другом ещё не принятом тикете
(пример: `T267` от `T266`, пока `T266` не слит) `wt-new` отклонит: sha
ветки `T266` предком `default_branch` не является. Для v1 это осознанно
приемлемо — типичный случай это «база = коммит уже в default_branch», а
стек — редкое и явно видимое архитектору решение. Обход на сейчас: создавать
такое дерево вручную (`git worktree add` от ветки `T266` напрямую), не
через `wt-new`. Расширять `wt-new` под явный стек (`--base` = ветка, не
обязательно потомок default) — отдельная задача, не в этом заходе.

## `tools/wt/wt-status.sh`

Read-only, чистый bash, без LLM. По каждому репо из конфига:

- `git worktree list` по каждому репо, за вычетом самого primary checkout
  (путь == `root` репо) — это не тикетное дерево, не участвует в
  сопоставлении с `task_glob`/`exceptions`.
- Для каждого оставшегося воркетри: путь, ветка/`detached`,
  `git status --short` (dirty да/нет), `git log --oneline
  <default_branch>..HEAD` (коммиты вне базовой ветки).
- **Извлечение номера тикета из имени — два паттерна, не один.** Основной:
  `name_pattern` репо (`ChronOS-wt-t###`). Алиас: старая безрепная схема
  `wt-t###`, которая реально живёт сейчас (`wt-t285`, `wt-t290`, `wt-t291`,
  `wt-t296`) — регэксп на номер тикета должен матчить оба варианта для
  каждого репо, где такие деревья ещё не переименованы. `wt-new` создаёт
  только по новой схеме; алиас — только для чтения существующих.
- Если `task_glob` задан — сопоставляет номер тикета с файлом в
  `tasks/{active,check,pause,done}/T###-*.md`, определяет состояние
  (active|check|pause|done|none).
- Помечает `exceptions` из конфига отдельным полем — не путает их с
  забытыми тикетными деревьями.
- Детект слитых веток: `git branch --merged <default_branch>` **для той
  же ветки**, не хардкод `master`. `detached HEAD` никогда не
  классифицируется как «смержено» — отдельная категория в отчёте.
- Пишет `.wt-status/STATUS.md` (перезапись, атомарно — пишет во временный
  файл и делает `mv`), плюс печатает то же в stdout.

Cron: `hermes cron create "*/15 * * * *" --no-agent --script /home/neo/projects/chronos-ecosystem/ChronOS/tools/wt/wt-status.sh --name chronos-worktree-status`
— абсолютный путь сразу, без symlink в `~/.hermes/scripts/` (уже проверено,
что `--script` принимает абсолютный путь напрямую). `--no-agent` = 0
токенов, тикер живёт в gateway Гермеса — это внешняя зависимость, не
деталь: если gateway не поднят, статус не обновляется, `wt-status.sh`
можно запустить и вручную.

## `tools/wt/wt-rm.sh <repo> <ticket>`

Только вручную, никогда по расписанию/автотриггеру.

- Отказывает, если `git status --short` не пуст или есть коммиты вне
  `default_branch`, без `--force`.
- `--force` печатает, что именно теряется (файлы + коммиты), требует
  повторного подтверждения.
- После удаления: `git worktree prune`.

## LLM-слой — единый паттерн: no-agent + прямой HTTP к OmniRoute

Оба LLM-шага (drift, digest) — **не** `hermes cron`-агенты с промпт-инъекцией
и `--deliver`. Штатные механизмы `hermes cron` не гарантируют файл:
`--deliver local` глушит доставку (`_resolve_single_delivery_target`
возвращает `None`), `hermes cron runs`/`executions.db` — статус-ledger без
текста ответа, а `~/.hermes/cron/output/<job_id>/*.md` — внутренний
формат Гермеса (полный prompt + ответ вперемешку), которым не стоит
завязываться как контрактом.

Вместо этого: `--no-agent --script`, скрипт сам делает один HTTP-запрос к
уже работающему на машине OmniRoute-гейтвею (`127.0.0.1:20128`,
`/v1/chat/completions`). URL и модель зашиты явно — не «текущий дефолт
Гермеса». Модель — отдельное комбо **`cron`** (не `hindsight-combo`: тот
для retain Hindsight). В теле запроса обязательно `"stream": false`
(шлюз по умолчанию отдаёт SSE). Никакого agent-loop — значит физически
нет terminal/file-тулов, гарантия read-only сильнее, чем отсутствие
`--workdir`.

### `tools/wt/wt-drift.sh`

- Cron: `hermes cron create "0 */2 * * *" --no-agent --script /home/neo/projects/chronos-ecosystem/ChronOS/tools/wt/wt-drift.sh --name chronos-worktree-drift`.
- Для каждого активного (не exception) воркетри по очереди:
  - собирает бандл: `git diff --name-only <base>..HEAD` + `## Scope (machine)` блок из task-файла (в основном дереве репо, см. `wt-new`);
  - **один** HTTP-запрос на воркетри (не все деревья одним промптом —
    держит вход маленьким и предсказуемым по времени);
  - ответ — секция в `.wt-status/DRIFT.md`, помеченная тикетом.
- Контракт скрипта:
  - пишет только `.wt-status/DRIFT.md`, никогда внутрь воркетри/репо;
  - таймаут на каждый HTTP-запрос; ненулевой `exit`, если итоговый файл
    не появился или пуст;
  - ни одной git-команды на запись, ни одного вызова, меняющего
    воркетри;
  - если у воркетри нет task-файла со Scope-блоком — воркетри пропускается
    с пометкой «no scope declared» в DRIFT.md, не галлюцинирует правила
    из прозы.

**Холодный старт.** На день внедрения ни один существующий live-brief
(`T265`, `T266`, `T271`, ...) не содержит `## Scope (machine)` — первый
прогон `wt-drift` пометит их все «no scope declared», содержательного
дрифта не будет. Это одноразовая миграция, не баг скрипта: блок добавляется
в brief для новых тикетов сразу при заведении, и вручную дописывается в
живые `tasks/active/*.md` перед тем, как ждать от drift первой полезной
находки.

### `tools/wt/wt-digest.sh`

- Cron: `hermes cron create "0 9 * * *" --no-agent --script /home/neo/projects/chronos-ecosystem/ChronOS/tools/wt/wt-digest.sh --name chronos-worktree-digest`.
- Вход: текущие `STATUS.md` + `DRIFT.md`.
- Один HTTP-запрос к OmniRoute, тот же контракт (только пишет
  `.wt-status/DIGEST.md`, атомарно, ненулевой exit при пустом ответе).

## Верификация

- `wt-new` на тестовом тикете: воркетри создан в правильном месте с
  префиксом репо в имени (`<Repo>-wt-t###` — по конфигу репо), ветка от
  доказанной/явно переданной базы, sidecar `CARGO_TARGET_DIR` рядом с
  деревом (не внутри tracked-конфига). Brief и Scope-блок при этом не
  трогает — это отдельно проверяется на стороне brief в основном дереве.
- `wt-status`: прогон вручную и через `hermes cron run <job_id>` — файл
  обновился, детект merged/detached корректен на текущих живых
  деревьях (`ChronOS-wt-t266` — detached, не «смержено»).
- `wt-drift`: тест на воркетри с заведомо нарушенным scope (правка файла
  из `deny:`) — нарушение попадает в `DRIFT.md`.
- `wt-rm`: попытка удалить грязное дерево без `--force` — отказ,
  `--force` показывает, что теряется.
- Ни один скрипт не пишет внутрь `crates/`/кода репозиториев — только в
  `.wt-status/` или в свой воркетри при создании.
