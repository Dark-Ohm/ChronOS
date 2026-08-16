# tools/wt

Ручные ворктри-скрипты для экосистемы Chronos.
Спека: `docs/superpowers/specs/2026-08-16-worktree-automation-design.md`.

## Команды

    tools/wt/wt-new.sh <repo> <ticket> [slug] [--base <sha>]
    tools/wt/wt-rm.sh <repo> <ticket> [--force]

`wt-new` создаёт воркетри с префиксом репо в имени (`<Repo>-wt-t###`),
sidecar `CARGO_TARGET_DIR`-каталог рядом (`<name>-target/`),
untracked `.envrc` внутри воркетри и строку в `root/.git/info/exclude`.
Не пишет brief — это обязанность архитектора в основном дереве.

`--base` обязан быть предком `default_branch`.
Стек на незамёрженный тикет (v1) не поддержан —
создавайте такое дерево вручную через `git worktree add`.

`wt-rm` — только вручную. Отказывает на грязном дереве / незакоммиченных
коммитах без `--force`. С `--force` печатает потерю и требует `YES`.

## Конфиг

`tools/wt/repos.yaml` — одна запись на репозиторий.
Новый репо в скоуп = новая запись, без правки кода скриптов.

## Тесты

    bash tools/wt/tests/run.sh

Фикстуры — временные git-репо в `mktemp`, не трогают живые деревья ChronOS.

## Зависимости

- bash 5, git, python3 (только в `lib.sh` для regex-escape)
- `wt-new` и `wt-rm` читают `repos.yaml` через `tools/wt/lib.sh`
