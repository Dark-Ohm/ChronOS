# Worktree Automation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Поставить в `ChronOS/tools/wt/` ручные скрипты ворктри плюс два no-agent LLM-отчёта (drift/digest через OmniRoute), не трогая приёмку, мерж и раздачу тикетов.

**Architecture:** Общий `lib.sh` читает `repos.yaml` и резолвит имена/тикеты. `wt-new` / `wt-rm` меняют только worktree + untracked sidecar target. `wt-status` пишет `.wt-status/STATUS.md`. `wt-drift` и `wt-digest` — bash с одним `curl` на `127.0.0.1:20128` (`stream:false`) и атомарной записью `DRIFT.md` / `DIGEST.md`. Hermes cron только будит абсолютным `--script --no-agent`.

**Tech Stack:** bash 5, `git worktree`, `curl`, `python3` (JSON encode/decode, без PyYAML), OmniRoute `:20128` / модель `cron` (отдельное комбо, не `hindsight-combo`). Тесты — временные git-репо в `mktemp`, без живых ChronOS-деревьев.

## Global Constraints

- Канон: `docs/superpowers/specs/2026-08-16-worktree-automation-design.md` (после выреза про стек).
- Пять репо: ChronOS, Chronos-Engine, Chronos-FM, Chronos-lm, Source. Out of scope: Chronos-IDE, Chronos-Editor, Chronos-AUR. Не путать с `hermes -w`.
- Скрипты + конфиг только в `ChronOS/tools/wt/`. Отчёты только в `/home/neo/projects/chronos-ecosystem/.wt-status/` (override: `WT_STATUS_DIR`).
- Тикет / мерж / приёмка / удаление — не автоматизировать. `wt-rm` только вручную.
- `wt-new` не пишет brief. Scope-блок — обязанность архитектора в основном дереве. Drift читает `task_glob` от `root`, не от ворктри.
- `--base <sha>` обязан быть предком `default_branch`. Стек на незамёрженный тикет (T267 от незамёрженного T266) — известный вырез v1; обход: ручной `git worktree add`, не расширять `wt-new` в этом заходе.
- `name_pattern` с префиксом репо. `wt-new` только новая схема. Чтение: канон + алиас `wt-t###` + FM-наследие `t{ticket}-*` (пример `t051-dnd`).
- `task_glob` в yaml указывает на `active/*.md`; поиск состояния — ещё `check|pause|done` рядом. Побеждает проза spec.
- `CARGO_TARGET_DIR` = untracked sidecar `<worktree_parent>/<name>-target/`. Не tracked `.cargo/config.toml`.
- Primary checkout (`path == root`) не тикетное дерево. `exceptions` не «забытые». `detached` ≠ merged.
- OmniRoute: `http://127.0.0.1:20128/v1/chat/completions`, модель **`cron`**, **`"stream": false`** (иначе SSE и пустой parse). URL/модель зашиты, не дефолт Hermes и не `hindsight-combo`.
- `--script` — абсолютный путь. Cron не создавать в CI-тестах; регистрация — отдельный шаг с живым gateway.
- Коммиты без AI-трейлеров. Формат: `docs :` / `chore :` / `test :`. Не стейджить чужой WIP лаунчера/T265.
- Не писать в `crates/`. Не `git stash`. Не `/tmp` как worktree ChronOS (тестовые репо в `mktemp` — ок, это не ChronOS).

## File map

| Файл | Роль |
|---|---|
| `tools/wt/repos.yaml` | пять репо, паттерны, exceptions |
| `tools/wt/lib.sh` | yaml-поля, expand, ticket parse, task state, atomic write |
| `tools/wt/wt-new.sh` | worktree + sidecar + exclude `.envrc` |
| `tools/wt/wt-status.sh` | STATUS.md |
| `tools/wt/wt-rm.sh` | ручное удаление |
| `tools/wt/wt-omni.sh` | один POST, `stream:false` |
| `tools/wt/wt-drift.sh` | бандл + секции DRIFT.md |
| `tools/wt/wt-digest.sh` | STATUS+DRIFT → DIGEST.md |
| `tools/wt/prompts/drift.txt` | системный текст drift |
| `tools/wt/prompts/digest.txt` | системный текст digest |
| `tools/wt/README.md` | вызов + cron-команды |
| `tools/wt/tests/helpers.sh` | фикстура git-репо |
| `tools/wt/tests/run.sh` | раннер |
| `tools/wt/tests/test_*.sh` | по задаче |

Переменные окружения (все опциональны, для тестов):

```
WT_ROOT          # каталог tools/wt (авто из BASH_SOURCE)
WT_REPOS_YAML    # путь к yaml
WT_STATUS_DIR    # куда писать отчёты
WT_OMNI_URL      # default http://127.0.0.1:20128/v1/chat/completions
WT_OMNI_MODEL    # default cron
WT_OMNI_CURL     # если задан — вызывается вместо curl (тесты)
```

---

### Task 1: `repos.yaml` + `lib.sh` + раннер тестов

**Files:**
- Create: `tools/wt/repos.yaml`
- Create: `tools/wt/lib.sh`
- Create: `tools/wt/tests/helpers.sh`
- Create: `tools/wt/tests/run.sh`
- Create: `tools/wt/tests/test_lib.sh`

**Interfaces:**
- Consumes: ничего
- Produces:
  ```bash
  wt_lib_dir                 # абсолютный tools/wt
  wt_load_lib                # no-op source guard
  wt_repo_keys               # stdout: ChronOS\nChronos-FM\n...
  wt_repo_get <key> <field>  # stdout field or exit 2
  wt_expand <pattern> <ticket> [slug]
  wt_status_dir              # $WT_STATUS_DIR or /home/neo/projects/chronos-ecosystem/.wt-status
  wt_atomic_write <path>     # stdin → path via tmp+mv same dir
  ```

- [ ] **Step 1: Write the failing test**

`tools/wt/tests/helpers.sh`:

```bash
#!/usr/bin/env bash
set -euo pipefail
assert_eq() {
  local got="$1" want="$2" msg="${3:-}"
  if [[ "$got" != "$want" ]]; then
    echo "FAIL ${msg}: got=$(printf %q "$got") want=$(printf %q "$want")" >&2
    return 1
  fi
}
assert_fail() {
  if "$@"; then
    echo "FAIL expected failure: $*" >&2
    return 1
  fi
}
```

`tools/wt/tests/test_lib.sh`:

```bash
#!/usr/bin/env bash
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=helpers.sh
source "$HERE/helpers.sh"
# shellcheck source=../lib.sh
source "$HERE/../lib.sh"

keys="$(wt_repo_keys | tr '\n' ' ')"
assert_eq "$keys" "ChronOS Chronos-FM Chronos-Engine Chronos-lm Source " "repo keys order"

assert_eq "$(wt_repo_get ChronOS default_branch)" "master"
assert_eq "$(wt_repo_get Chronos-Engine default_branch)" "chronos-main"
assert_eq "$(wt_repo_get Chronos-FM worktree_parent)" \
  "/home/neo/projects/chronos-ecosystem/Chronos-FM/.worktrees"
assert_eq "$(wt_repo_get ChronOS name_pattern)" "ChronOS-wt-t{ticket}"
assert_eq "$(wt_expand "$(wt_repo_get ChronOS branch_pattern)" 266 blur)" \
  "feat/t266-blur"
assert_eq "$(wt_expand "ChronOS-wt-t{ticket}" 266)" "ChronOS-wt-t266"
assert_fail wt_repo_get NoSuchRepo root

tmpdir="$(mktemp -d)"
printf 'hello\n' | wt_atomic_write "$tmpdir/out.md"
assert_eq "$(cat "$tmpdir/out.md")" "hello"
rm -rf "$tmpdir"
```

`tools/wt/tests/run.sh`:

```bash
#!/usr/bin/env bash
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
fail=0
for t in "$HERE"/test_*.sh; do
  echo "== $(basename "$t")"
  if bash "$t"; then
    echo "OK"
  else
    echo "FAIL $t"
    fail=1
  fi
done
exit "$fail"
```

- [ ] **Step 2: Run test to verify it fails**

```bash
chmod +x ChronOS/tools/wt/tests/run.sh ChronOS/tools/wt/tests/test_lib.sh
bash ChronOS/tools/wt/tests/run.sh
```

Expected: FAIL `source: .../lib.sh: No such file or directory`

- [ ] **Step 3: Write minimal implementation**

`tools/wt/repos.yaml` — скопировать блок `repos:` из spec (ChronOS / Chronos-FM / Chronos-Engine / Chronos-lm / Source) **байт-в-байт по полям spec**: `root`, `default_branch`, `worktree_parent`, `name_pattern`, `branch_pattern`, `task_glob`, `alias_bare`, `alias_legacy`, `exceptions`.

`tools/wt/lib.sh`:

```bash
#!/usr/bin/env bash
# Shared helpers for ChronOS/tools/wt. Source only.

wt_lib_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
: "${WT_REPOS_YAML:=$wt_lib_dir/repos.yaml}"
: "${WT_STATUS_DIR:=/home/neo/projects/chronos-ecosystem/.wt-status}"
: "${WT_OMNI_URL:=http://127.0.0.1:20128/v1/chat/completions}"
: "${WT_OMNI_MODEL:=cron}"

wt_status_dir() { printf '%s\n' "$WT_STATUS_DIR"; }

wt_repo_keys() {
  awk '
    /^repos:[[:space:]]*$/ { in_repos=1; next }
    in_repos && /^  [A-Za-z0-9_.-]+:[[:space:]]*$/ {
      k=$1; sub(/:$/, "", k); print k
    }
  ' "$WT_REPOS_YAML"
}

wt_repo_get() {
  local repo="$1" field="$2"
  local val
  val="$(
    awk -v repo="$repo" -v field="$field" '
      /^  [A-Za-z0-9_.-]+:[[:space:]]*$/ {
        k=$1; sub(/:$/, "", k); cur=k
      }
      cur==repo && $1==field":" {
        $1=""; sub(/^[[:space:]]+/, "", $0)
        print $0
        found=1
        exit
      }
      END { if (!found) exit 2 }
    ' "$WT_REPOS_YAML"
  )" || return 2
  if [[ "$val" == "null" ]]; then
    printf '\n'
    return 0
  fi
  # strip wrapping quotes
  val="${val#\"}"
  val="${val%\"}"
  printf '%s\n' "$val"
}

wt_expand() {
  local pat="$1" ticket="$2" slug="${3:-}"
  local out="$pat"
  out="${out//\{ticket\}/$ticket}"
  out="${out//\{slug\}/$slug}"
  # trailing hyphen if slug empty: feat/t266- → feat/t266
  out="${out%-}"
  printf '%s\n' "$out"
}

wt_atomic_write() {
  local dest="$1"
  local dir tmp
  dir="$(dirname "$dest")"
  mkdir -p "$dir"
  tmp="$(mktemp "$dir/.tmp.XXXXXX")"
  cat >"$tmp"
  mv -f "$tmp" "$dest"
}
```

- [ ] **Step 4: Run the tests and make sure they pass**

```bash
bash ChronOS/tools/wt/tests/run.sh
```

Expected: `== test_lib.sh` / `OK` / exit 0

- [ ] **Step 5: Commit**

```bash
git -C ChronOS add tools/wt/repos.yaml tools/wt/lib.sh tools/wt/tests/helpers.sh tools/wt/tests/run.sh tools/wt/tests/test_lib.sh
git -C ChronOS commit -m "chore : wt lib + repos.yaml + test runner"
```

---

### Task 2: Разбор тикета из имени ворктри

**Files:**
- Modify: `tools/wt/lib.sh`
- Create: `tools/wt/tests/test_names.sh`

**Interfaces:**
- Consumes: `wt_repo_get`, `wt_expand`
- Produces:
  ```bash
  # stdout ticket number or empty; exit 0 even if unmatched
  wt_ticket_from_name <repo_key> <basename>
  # 0 if basename is an exception for that repo
  wt_is_exception <repo_key> <basename>
  ```

Правила извлечения (в этом порядке, первое совпадение):

1. Канон: `name_pattern` с `{ticket}` → regex `ChronOS-wt-t([0-9]+[A-Za-z0-9]*)` (допускает `265A`).
2. Алиас без репо: `^wt-t([0-9]+[A-Za-z0-9]*)$` — только если `alias_bare: true` у этой записи (не сравнение `worktree_parent` с абсолютным путём экосистемы).
3. `alias_legacy: t-slug` → `^t([0-9]+[A-Za-z0-9]*)(-|$)` (`t051-dnd` → `051`). Не завязывать на имя репо `Chronos-FM`.

`exceptions`: поле yaml вида `[ChronOS-wt-measure]` — split по запятой, trim, сравнение с basename.

- [ ] **Step 1: Write the failing test**

`tools/wt/tests/test_names.sh`:

```bash
#!/usr/bin/env bash
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$HERE/helpers.sh"
source "$HERE/../lib.sh"

assert_eq "$(wt_ticket_from_name ChronOS ChronOS-wt-t266)" "266"
assert_eq "$(wt_ticket_from_name ChronOS wt-t285)" "285"
assert_eq "$(wt_ticket_from_name ChronOS ChronOS-wt-t265A)" "265A"
assert_eq "$(wt_ticket_from_name Chronos-FM FM-wt-t051)" "051"
assert_eq "$(wt_ticket_from_name Chronos-FM t051-dnd)" "051"
assert_eq "$(wt_ticket_from_name ChronOS t051-dnd)" ""
assert_eq "$(wt_ticket_from_name Source Source-wt-t12)" "12"
assert_eq "$(wt_is_exception ChronOS ChronOS-wt-measure && echo yes)" "yes"
assert_eq "$(wt_is_exception ChronOS ChronOS-wt-t266 && echo yes || echo no)" "no"

# alias_bare / alias_legacy — свойство yaml, не хардкод пути
scratch="$(mktemp -d)"
cat >"$scratch/repos.yaml" <<EOF
repos:
  Bare:
    root: /x
    default_branch: master
    worktree_parent: /scratch/parent
    name_pattern: "Bare-wt-t{ticket}"
    branch_pattern: "feat/t{ticket}"
    task_glob: null
    alias_bare: true
    alias_legacy: none
    exceptions: []
  Inside:
    root: /y
    default_branch: main
    worktree_parent: /scratch/inside/.worktrees
    name_pattern: "In-wt-t{ticket}"
    branch_pattern: "feat/t{ticket}"
    task_glob: null
    alias_bare: false
    alias_legacy: t-slug
    exceptions: []
EOF
WT_REPOS_YAML="$scratch/repos.yaml"
assert_eq "$(wt_ticket_from_name Bare wt-t9)" "9"
assert_eq "$(wt_ticket_from_name Inside wt-t9)" ""
assert_eq "$(wt_ticket_from_name Inside t051-dnd)" "051"
assert_eq "$(wt_ticket_from_name Bare t051-dnd)" ""
rm -rf "$scratch"
```

- [ ] **Step 2: Run test to verify it fails**

```bash
bash ChronOS/tools/wt/tests/test_names.sh
```

Expected: FAIL `wt_ticket_from_name: command not found`

- [ ] **Step 3: Write minimal implementation**

Append to `lib.sh`:

```bash
wt_is_exception() {
  local repo="$1" name="$2"
  local raw item
  raw="$(wt_repo_get "$repo" exceptions)" || return 1
  raw="${raw#[}"
  raw="${raw%]}"
  IFS=',' read -ra items <<<"$raw"
  for item in "${items[@]}"; do
    item="${item#"${item%%[![:space:]]*}"}"
    item="${item%"${item##*[![:space:]]}"}"
    [[ "$item" == "$name" ]] && return 0
  done
  return 1
}

wt_ticket_from_name() {
  local repo="$1" name="$2"
  local pat ticket bare legacy
  pat="$(wt_repo_get "$repo" name_pattern)"
  pat="${pat//\{ticket\}/__T__}"
  pat="${pat//\{slug\}/}"
  local re
  re="$(printf '%s' "$pat" | python3 -c '
import re,sys
p=sys.stdin.read()
parts=p.split("__T__")
print("".join(re.escape(a)+r"([0-9]+[A-Za-z0-9]*)"*(i<len(parts)-1) for i,a in enumerate(parts)))
')"
  if ticket="$(printf '%s' "$name" | sed -nE "s/^${re}\$/\1/p")" && [[ -n "$ticket" ]]; then
    printf '%s\n' "$ticket"
    return 0
  fi
  bare="$(wt_repo_get "$repo" alias_bare)"
  if [[ "$bare" == "true" ]] \
     && ticket="$(printf '%s' "$name" | sed -nE 's/^wt-t([0-9]+[A-Za-z0-9]*)$/\1/p')" \
     && [[ -n "$ticket" ]]; then
    printf '%s\n' "$ticket"
    return 0
  fi
  legacy="$(wt_repo_get "$repo" alias_legacy)"
  if [[ "$legacy" == "t-slug" ]] \
     && ticket="$(printf '%s' "$name" | sed -nE 's/^t([0-9]+[A-Za-z0-9]*)(-.*)?$/\1/p')" \
     && [[ -n "$ticket" ]]; then
    printf '%s\n' "$ticket"
    return 0
  fi
  printf '\n'
}
```

- [ ] **Step 4: Run the tests and make sure they pass**

```bash
bash ChronOS/tools/wt/tests/run.sh
```

Expected: `test_lib.sh` OK, `test_names.sh` OK

- [ ] **Step 5: Commit**

```bash
git -C ChronOS add tools/wt/lib.sh tools/wt/tests/test_names.sh
git -C ChronOS commit -m "chore : wt ticket name parser (canon + aliases)"
```

---

### Task 3: `wt-new.sh`

**Files:**
- Create: `tools/wt/wt-new.sh`
- Create: `tools/wt/tests/test_new.sh`
- Modify: `tools/wt/tests/helpers.sh` (фикстура git-репо)

**Interfaces:**
- Consumes: `wt_repo_get`, `wt_expand`
- Produces: CLI `wt-new.sh <repo> <ticket> [slug] [--base <sha>]` exit 0/1; worktree; sidecar `<name>-target/`; untracked `.envrc`; строка в `root/.git/info/exclude`

Поведение:
- Неизвестный `<repo>` → stderr «добавь запись в repos.yaml», exit 1.
- База: `--base` или `git -C root rev-parse default_branch`.
- `git -C root merge-base --is-ancestor "$sha" "$default_branch"` иначе exit 1 с текстом про вырез v1 (стек вручную).
- Путь уже существует → exit 1.
- `git -C root worktree add -b "$branch" "$path" "$sha"`
- `mkdir -p "${path}-target"`
- `.envrc` в ворктри: `export CARGO_TARGET_DIR=<abs sidecar>`
- Если `.envrc` нет в `root/.git/info/exclude` — дописать.

- [ ] **Step 1: Write the failing test + fixture**

Append to `helpers.sh`:

```bash
make_git_repo() {
  local d="$1" branch="${2:-master}"
  mkdir -p "$d"
  git -C "$d" init -q -b "$branch"
  git -C "$d" config user.email t@t
  git -C "$d" config user.name t
  echo ok >"$d/README"
  git -C "$d" add README
  git -C "$d" commit -q -m init
}
```

`tools/wt/tests/test_new.sh`:

```bash
#!/usr/bin/env bash
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$HERE/helpers.sh"
source "$HERE/../lib.sh"

scratch="$(mktemp -d)"
trap 'rm -rf "$scratch"' EXIT
make_git_repo "$scratch/repo" master
mkdir -p "$scratch/parent"
cat >"$scratch/repos.yaml" <<EOF
repos:
  Toy:
    root: $scratch/repo
    default_branch: master
    worktree_parent: $scratch/parent
    name_pattern: "Toy-wt-t{ticket}"
    branch_pattern: "feat/t{ticket}-{slug}"
    task_glob: null
    alias_bare: false
    alias_legacy: none
    exceptions: []
EOF
export WT_REPOS_YAML="$scratch/repos.yaml"

NEW="$HERE/../wt-new.sh"

assert_fail "$NEW" Nope 1 foo
"$NEW" Toy 1 demo
assert_eq "$(git -C "$scratch/repo" worktree list | wc -l | tr -d ' ')" "2"
[[ -d "$scratch/parent/Toy-wt-t1" ]]
[[ -d "$scratch/parent/Toy-wt-t1-target" ]]
[[ -f "$scratch/parent/Toy-wt-t1/.envrc" ]]
grep -q CARGO_TARGET_DIR "$scratch/parent/Toy-wt-t1/.envrc"
assert_eq "$(git -C "$scratch/parent/Toy-wt-t1" branch --show-current)" "feat/t1-demo"
# stacked commit not on master must fail
git -C "$scratch/repo" checkout -q -b side
echo x >>"$scratch/repo/README"
git -C "$scratch/repo" commit -q -am side
side="$(git -C "$scratch/repo" rev-parse HEAD)"
git -C "$scratch/repo" checkout -q master
assert_fail "$NEW" Toy 2 stacked --base "$side"
```

- [ ] **Step 2: Run test to verify it fails**

```bash
bash ChronOS/tools/wt/tests/test_new.sh
```

Expected: FAIL missing `wt-new.sh`

- [ ] **Step 3: Write minimal implementation**

`tools/wt/wt-new.sh`:

```bash
#!/usr/bin/env bash
set -euo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=lib.sh
source "$here/lib.sh"

usage() { echo "usage: wt-new.sh <repo> <ticket> [slug] [--base <sha>]" >&2; }

repo="" ticket="" slug="" base=""
while [[ $# -gt 0 ]]; do
  case "$1" in
    --base) base="${2:-}"; shift 2 ;;
    -h|--help) usage; exit 0 ;;
    *)
      if [[ -z "$repo" ]]; then repo="$1"
      elif [[ -z "$ticket" ]]; then ticket="$1"
      elif [[ -z "$slug" ]]; then slug="$1"
      else usage; exit 1
      fi
      shift
      ;;
  esac
done
[[ -n "$repo" && -n "$ticket" ]] || { usage; exit 1; }

if ! root="$(wt_repo_get "$repo" root)"; then
  echo "wt-new: unknown repo '$repo' — add it to $WT_REPOS_YAML" >&2
  exit 1
fi
branch_def="$(wt_repo_get "$repo" default_branch)"
parent="$(wt_repo_get "$repo" worktree_parent)"
name="$(wt_expand "$(wt_repo_get "$repo" name_pattern)" "$ticket" "$slug")"
branch="$(wt_expand "$(wt_repo_get "$repo" branch_pattern)" "$ticket" "$slug")"
path="$parent/$name"

if [[ -z "$base" ]]; then
  base="$(git -C "$root" rev-parse --verify "$branch_def")"
fi
if ! git -C "$root" merge-base --is-ancestor "$base" "$branch_def"; then
  echo "wt-new: $base is not an ancestor of $branch_def (v1: no stack on unmerged tickets; use git worktree add by hand)" >&2
  exit 1
fi
if [[ -e "$path" ]]; then
  echo "wt-new: $path already exists" >&2
  exit 1
fi
mkdir -p "$parent"
git -C "$root" worktree add -b "$branch" "$path" "$base"
sidecar="${path}-target"
mkdir -p "$sidecar"
printf 'export CARGO_TARGET_DIR=%q\n' "$sidecar" >"$path/.envrc"
exclude="$root/.git/info/exclude"
mkdir -p "$(dirname "$exclude")"
if ! grep -qxF '.envrc' "$exclude" 2>/dev/null; then
  printf '.envrc\n' >>"$exclude"
fi
echo "created $path (branch $branch, target $sidecar)"
```

`chmod +x tools/wt/wt-new.sh`

- [ ] **Step 4: Run the tests and make sure they pass**

```bash
bash ChronOS/tools/wt/tests/run.sh
```

Expected: all OK, including `test_new.sh`

- [ ] **Step 5: Commit**

```bash
git -C ChronOS add tools/wt/wt-new.sh tools/wt/tests/test_new.sh tools/wt/tests/helpers.sh
git -C ChronOS commit -m "chore : wt-new — worktree + sidecar target"
```

---

### Task 4: `wt-status.sh`

**Files:**
- Create: `tools/wt/wt-status.sh`
- Create: `tools/wt/tests/test_status.sh`
- Modify: `tools/wt/lib.sh` (`wt_task_state`)

**Interfaces:**
- Consumes: parser + `wt_repo_*`
- Produces:
  ```bash
  wt_task_state <repo_key> <ticket>   # active|check|pause|done|none
  # CLI writes $WT_STATUS_DIR/STATUS.md and echoes it
  ```

`wt_task_state`: взять `task_glob`; если пустой → `none`. Иначе корень оркестрации = `root` + dirname(dirname(glob)) т.е. `docs/orchestration/tasks`. Искать первый файл:

- `$tasks/active/T${ticket}-*.md` → active  
- `$tasks/active/pause/T${ticket}-*.md` → pause  
- `$tasks/active/check/T${ticket}-*.md` → check  
- `$tasks/done/T${ticket}-*.md` → done  

Нет файла → `none`.

Для каждого репо: `git -C root worktree list --porcelain`. Пропустить `worktree <path>` где `realpath path == realpath root`. Для остальных: basename, exception?, ticket, dirty (`git -C path status --short`), ahead (`git -C path log --oneline "$default_branch..HEAD"`), branch или `detached`, merged только если **не** detached и `git -C root merge-base --is-ancestor "$path_HEAD" "$default_branch"` **и** ahead пуст (ветка полностью в default). Не использовать голый `git branch --merged` на detached.

Формат STATUS.md (стабильный, его ест digest):

```markdown
# worktree status
generated: <ISO8601>

## ChronOS
- path: /.../ChronOS-wt-t266
  name: ChronOS-wt-t266
  ticket: 266
  branch: detached
  dirty: no
  ahead: 0
  merged: no
  exception: no
  task: active
```

- [ ] **Step 1: Write the failing test**

`tools/wt/tests/test_status.sh` обязан покрыть всё, что пишет таблица coverage: primary skip, detached ≠ merged, четыре каталога brief.

```bash
#!/usr/bin/env bash
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$HERE/helpers.sh"
source "$HERE/../lib.sh"
scratch="$(mktemp -d)"; trap 'rm -rf "$scratch"' EXIT
make_git_repo "$scratch/repo" master
mkdir -p "$scratch/parent" "$scratch/status" \
  "$scratch/repo/docs/orchestration/tasks/active/pause" \
  "$scratch/repo/docs/orchestration/tasks/active/check" \
  "$scratch/repo/docs/orchestration/tasks/done"
echo '# T1' >"$scratch/repo/docs/orchestration/tasks/active/T1-demo.md"
echo '# T2' >"$scratch/repo/docs/orchestration/tasks/active/pause/T2-hold.md"
echo '# T3' >"$scratch/repo/docs/orchestration/tasks/active/check/T3-qa.md"
echo '# T4' >"$scratch/repo/docs/orchestration/tasks/done/T4-old.md"
git -C "$scratch/repo" add docs && git -C "$scratch/repo" commit -q -m tasks
cat >"$scratch/repos.yaml" <<EOF
repos:
  Toy:
    root: $scratch/repo
    default_branch: master
    worktree_parent: $scratch/parent
    name_pattern: "Toy-wt-t{ticket}"
    branch_pattern: "feat/t{ticket}-{slug}"
    task_glob: docs/orchestration/tasks/active/*.md
    alias_bare: true
    alias_legacy: none
    exceptions: [Toy-keep]
EOF
export WT_REPOS_YAML="$scratch/repos.yaml" WT_STATUS_DIR="$scratch/status"

assert_eq "$(wt_task_state Toy 1)" "active"
assert_eq "$(wt_task_state Toy 2)" "pause"
assert_eq "$(wt_task_state Toy 3)" "check"
assert_eq "$(wt_task_state Toy 4)" "done"
assert_eq "$(wt_task_state Toy 99)" "none"

"$HERE/../wt-new.sh" Toy 1 demo
# detached at a commit already on master — naive --merged would say yes
git -C "$scratch/repo" worktree add --detach "$scratch/parent/Toy-wt-t9" HEAD

"$HERE/../wt-status.sh"
st="$scratch/status/STATUS.md"
grep -q 'name: Toy-wt-t1' "$st"
grep -q 'task: active' "$st"
if grep -q "name: repo$" "$st"; then
  echo FAIL primary listed; exit 1
fi
# extract the t9 block
python3 - "$st" <<'PY'
import sys
text = open(sys.argv[1]).read().split("- path:")
block = next(b for b in text if "name: Toy-wt-t9" in b)
assert "branch: detached" in block, block
assert "merged: no" in block, block
PY
```

- [ ] **Step 2: Run test to verify it fails**

Expected: missing `wt-status.sh` (или `wt_task_state: command not found`)

- [ ] **Step 3: Implement `wt_task_state` + `wt-status.sh`**

`wt_task_state` в `lib.sh`:

```bash
wt_task_state() {
  local repo="$1" ticket="$2"
  local root glob tasks
  root="$(wt_repo_get "$repo" root)"
  glob="$(wt_repo_get "$repo" task_glob)"
  [[ -n "$glob" ]] || { echo none; return 0; }
  tasks="$root/docs/orchestration/tasks"
  local f
  for f in \
    "$tasks/active/T${ticket}-"*.md \
    "$tasks/active/pause/T${ticket}-"*.md \
    "$tasks/active/check/T${ticket}-"*.md \
    "$tasks/done/T${ticket}-"*.md
  do
    [[ -e "$f" ]] || continue
    case "$f" in
      */active/pause/*) echo pause; return 0 ;;
      */active/check/*) echo check; return 0 ;;
      */done/*) echo done; return 0 ;;
      */active/*) echo active; return 0 ;;
    esac
  done
  echo none
}
```

`tools/wt/wt-status.sh` целиком (один файл, без второго варианта):

```bash
#!/usr/bin/env bash
set -euo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=lib.sh
source "$here/lib.sh"

wt_status_emit_one() {
  local repo="$1" path="$2" head="$3" branch_line="$4" detached="$5"
  local root branch_def rr rp name ticket dirty ahead_n merged exc task
  root="$(wt_repo_get "$repo" root)"
  branch_def="$(wt_repo_get "$repo" default_branch)"
  rp="$(realpath "$path")"
  rr="$(realpath "$root")"
  [[ "$rp" == "$rr" ]] && return 0
  name="$(basename "$path")"
  ticket="$(wt_ticket_from_name "$repo" "$name")"
  if wt_is_exception "$repo" "$name"; then exc=yes; else exc=no; fi
  if [[ -n "$(git -C "$path" status --short)" ]]; then dirty=yes; else dirty=no; fi
  ahead_n="$(git -C "$path" rev-list --count "${branch_def}..HEAD" 2>/dev/null || printf '0')"
  if [[ "$detached" == "1" ]]; then
    branch_line=detached
    merged=no
  else
    branch_line="${branch_line#refs/heads/}"
    if [[ "$ahead_n" == "0" ]] && git -C "$root" merge-base --is-ancestor "$head" "$branch_def"; then
      merged=yes
    else
      merged=no
    fi
  fi
  if [[ -n "$ticket" ]]; then
    task="$(wt_task_state "$repo" "$ticket")"
  else
    task=none
  fi
  printf -- '- path: %s\n' "$path"
  printf '  name: %s\n' "$name"
  printf '  ticket: %s\n' "$ticket"
  printf '  branch: %s\n' "$branch_line"
  printf '  dirty: %s\n' "$dirty"
  printf '  ahead: %s\n' "$ahead_n"
  printf '  merged: %s\n' "$merged"
  printf '  exception: %s\n' "$exc"
  printf '  task: %s\n' "$task"
  printf '\n'
}

wt_status_emit_repo() {
  local repo="$1" path="" head="" branch="" detached=0 line
  printf '## %s\n' "$repo"
  while IFS= read -r line; do
    case "$line" in
      worktree\ *)
        if [[ -n "$path" ]]; then
          wt_status_emit_one "$repo" "$path" "$head" "$branch" "$detached"
        fi
        path="${line#worktree }"
        head="" branch="" detached=0
        ;;
      HEAD\ *) head="${line#HEAD }" ;;
      branch\ *) branch="${line#branch }" ;;
      detached) detached=1 ;;
    esac
  done < <(git -C "$(wt_repo_get "$repo" root)" worktree list --porcelain)
  if [[ -n "$path" ]]; then
    wt_status_emit_one "$repo" "$path" "$head" "$branch" "$detached"
  fi
  printf '\n'
}

{
  printf '# worktree status\n'
  printf 'generated: %s\n\n' "$(date -Iseconds)"
  while IFS= read -r repo; do
    [[ -n "$repo" ]] || continue
    wt_status_emit_repo "$repo"
  done < <(wt_repo_keys)
} | wt_atomic_write "$(wt_status_dir)/STATUS.md"

cat "$(wt_status_dir)/STATUS.md"
```

`chmod +x tools/wt/wt-status.sh`

Merged: `detached` всегда `merged: no`. Иначе `ahead==0` и `merge-base --is-ancestor HEAD default_branch`.

- [ ] **Step 4: Run tests**

```bash
bash ChronOS/tools/wt/tests/run.sh
```

Expected: `test_status.sh` OK

- [ ] **Step 5: Commit**

```bash
git -C ChronOS add tools/wt/lib.sh tools/wt/wt-status.sh tools/wt/tests/test_status.sh
git -C ChronOS commit -m "chore : wt-status — STATUS.md across configured repos"
```

---

### Task 5: `wt-rm.sh`

**Files:**
- Create: `tools/wt/wt-rm.sh`
- Create: `tools/wt/tests/test_rm.sh`

**Interfaces:**
- CLI: `wt-rm.sh <repo> <ticket> [--force]`
- Резолв пути: сначала канон `name_pattern`, если нет — любое worktree того репо, чей `wt_ticket_from_name` совпал (алиасы `wt-t###` / FM `t051-dnd`).
- Без `--force`: отказ если dirty или `rev-list --count default..HEAD` > 0.
- С `--force`: напечатать список файлов `git status --short` и `git log --oneline default..HEAD`, затем `read -r` «type YES». В тестах: `WT_RM_CONFIRM=YES`.
- Удалить sidecar `<path>-target` если пустой или целиком (rm -rf sidecar).
- `git -C root worktree remove "$path"` затем `git worktree prune`.

- [ ] **Step 1: Failing test**

```bash
# dirty tree without --force → fail, tree remains
# clean tree → removed, sidecar gone
# WT_RM_CONFIRM=YES --force on dirty → removed
```

Полный `test_rm.sh`:

```bash
#!/usr/bin/env bash
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$HERE/helpers.sh"
scratch="$(mktemp -d)"; trap 'rm -rf "$scratch"' EXIT
make_git_repo "$scratch/repo" master
mkdir -p "$scratch/parent"
cat >"$scratch/repos.yaml" <<EOF
repos:
  Toy:
    root: $scratch/repo
    default_branch: master
    worktree_parent: $scratch/parent
    name_pattern: "Toy-wt-t{ticket}"
    branch_pattern: "feat/t{ticket}-{slug}"
    task_glob: null
    alias_bare: false
    alias_legacy: none
    exceptions: []
EOF
export WT_REPOS_YAML="$scratch/repos.yaml"
"$HERE/../wt-new.sh" Toy 3 x
echo dirt >"$scratch/parent/Toy-wt-t3/x"
if "$HERE/../wt-rm.sh" Toy 3; then echo FAIL expected refuse; exit 1; fi
[[ -d "$scratch/parent/Toy-wt-t3" ]]
WT_RM_CONFIRM=YES "$HERE/../wt-rm.sh" Toy 3 --force
[[ ! -d "$scratch/parent/Toy-wt-t3" ]]
[[ ! -d "$scratch/parent/Toy-wt-t3-target" ]]
"$HERE/../wt-new.sh" Toy 4 y
"$HERE/../wt-rm.sh" Toy 4
[[ ! -d "$scratch/parent/Toy-wt-t4" ]]
```

- [ ] **Step 2: Run — expect missing script**

- [ ] **Step 3: Implement `wt-rm.sh`**

```bash
#!/usr/bin/env bash
set -euo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$here/lib.sh"

force=0
repo="" ticket=""
while [[ $# -gt 0 ]]; do
  case "$1" in
    --force) force=1; shift ;;
    *)
      if [[ -z "$repo" ]]; then repo="$1"
      elif [[ -z "$ticket" ]]; then ticket="$1"
      else echo "usage: wt-rm.sh <repo> <ticket> [--force]" >&2; exit 1
      fi
      shift
      ;;
  esac
done
[[ -n "$repo" && -n "$ticket" ]] || { echo "usage: wt-rm.sh <repo> <ticket> [--force]" >&2; exit 1; }

root="$(wt_repo_get "$repo" root)" || { echo "unknown repo" >&2; exit 1; }
parent="$(wt_repo_get "$repo" worktree_parent)"
canon="$parent/$(wt_expand "$(wt_repo_get "$repo" name_pattern)" "$ticket")"
path=""
if [[ -d "$canon" ]]; then
  path="$canon"
else
  while IFS= read -r line; do
    [[ "$line" == worktree\ * ]] || continue
    p="${line#worktree }"
    [[ "$(realpath "$p")" == "$(realpath "$root")" ]] && continue
    t="$(wt_ticket_from_name "$repo" "$(basename "$p")")"
    if [[ "$t" == "$ticket" ]]; then path="$p"; break; fi
  done < <(git -C "$root" worktree list --porcelain)
fi
[[ -n "$path" && -d "$path" ]] || { echo "wt-rm: no worktree for $repo $ticket" >&2; exit 1; }

branch_def="$(wt_repo_get "$repo" default_branch)"
dirty="$(git -C "$path" status --short)"
ahead="$(git -C "$path" log --oneline "$branch_def..HEAD" || true)"
if [[ -n "$dirty" || -n "$ahead" ]]; then
  if [[ "$force" -ne 1 ]]; then
    echo "wt-rm: dirty or unmerged commits; pass --force" >&2
    echo "$dirty" >&2
    echo "$ahead" >&2
    exit 1
  fi
  echo "WILL LOSE:"
  echo "$dirty"
  echo "$ahead"
  if [[ "${WT_RM_CONFIRM:-}" != "YES" ]]; then
    printf 'type YES: '
    read -r ans
    [[ "$ans" == "YES" ]] || exit 1
  fi
fi
if [[ "$force" -eq 1 ]]; then
  git -C "$root" worktree remove --force "$path"
else
  git -C "$root" worktree remove "$path"
fi
git -C "$root" worktree prune
rm -rf "${path}-target"
echo "removed $path"
```

`git worktree remove --force` **только** когда пользователь передал `--force` и прошёл YES. Чистый путь (нет dirty/ahead) — `git worktree remove "$path"` без флага: вторая линия обороны git остаётся.

- [ ] **Step 4: Tests pass**

- [ ] **Step 5: Commit**

```bash
git -C ChronOS add tools/wt/wt-rm.sh tools/wt/tests/test_rm.sh
git -C ChronOS commit -m "chore : wt-rm — manual remove with dirty/ahead guard"
```

---

### Task 6: OmniRoute helper + `wt-drift.sh`

**Files:**
- Create: `tools/wt/wt-omni.sh`
- Create: `tools/wt/prompts/drift.txt`
- Create: `tools/wt/wt-drift.sh`
- Create: `tools/wt/tests/test_drift.sh`
- Modify: `tools/wt/lib.sh` (`wt_scope_block`, `wt_extract_scope_base`)

**Interfaces:**
- `wt_omni_complete <prompt-file-or-arg>` → stdout assistant text, exit ≠0 если пусто
- `wt_scope_block <brief-path>` → секция `## Scope (machine)` или пусто
- Drift пишет только `$WT_STATUS_DIR/DRIFT.md`

`prompts/drift.txt`:

```
You compare a git name-only diff to a machine scope block.
Reply in markdown only. No tools. No git commands.
If a path is outside allow or matches deny, list it under ## Drift.
If nothing drifts, write ## Drift\n- none
```

- [ ] **Step 1: Write failing tests**

`tools/wt/tests/test_omni.sh` — сеть без живого OmniRoute:

```bash
#!/usr/bin/env bash
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$HERE/helpers.sh"
OMNI="$HERE/../wt-omni.sh"

fake="$(mktemp)"
printf '#!/bin/sh\ncat >/dev/null\necho fake-ok\n' >"$fake"
chmod +x "$fake"
out="$(printf 'hi' | WT_OMNI_CURL="$fake" "$OMNI")"
assert_eq "$out" "fake-ok"

printf '#!/bin/sh\ncat >/dev/null\n' >"$fake"
if printf 'hi' | WT_OMNI_CURL="$fake" "$OMNI"; then
  echo FAIL expected empty fake to fail; exit 1
fi

# connection refused must exit 1 and not hang (timeout 2s)
if printf 'hi' | env -u WT_OMNI_CURL \
     WT_OMNI_URL=http://127.0.0.1:1/v1/chat/completions \
     WT_OMNI_MODEL=cron \
     WT_OMNI_TIMEOUT=2 \
     "$OMNI"; then
  echo FAIL expected refused; exit 1
fi
rm -f "$fake"
```

`tools/wt/tests/test_drift.sh` целиком (не outline):

```bash
#!/usr/bin/env bash
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$HERE/helpers.sh"
source "$HERE/../lib.sh"

scratch="$(mktemp -d)"; trap 'rm -rf "$scratch"' EXIT
make_git_repo "$scratch/repo" master
mkdir -p "$scratch/parent" "$scratch/status" \
  "$scratch/repo/docs/orchestration/tasks/active"
echo '# T1 no scope' >"$scratch/repo/docs/orchestration/tasks/active/T1-demo.md"
git -C "$scratch/repo" add docs && git -C "$scratch/repo" commit -q -m brief
cat >"$scratch/repos.yaml" <<EOF
repos:
  Toy:
    root: $scratch/repo
    default_branch: master
    worktree_parent: $scratch/parent
    name_pattern: "Toy-wt-t{ticket}"
    branch_pattern: "feat/t{ticket}-{slug}"
    task_glob: docs/orchestration/tasks/active/*.md
    alias_bare: false
    alias_legacy: none
    exceptions: []
EOF
export WT_REPOS_YAML="$scratch/repos.yaml" WT_STATUS_DIR="$scratch/status"

called="$scratch/omni-called"
rm -f "$called"
printf '#!/bin/sh\ntouch %q\ncat >/dev/null\necho SHOULD-NOT-RUN\n' "$called" >"$scratch/fake-omni"
chmod +x "$scratch/fake-omni"
export WT_OMNI_CURL="$scratch/fake-omni"

"$HERE/../wt-new.sh" Toy 1 demo
"$HERE/../wt-drift.sh"
grep -q 'no scope declared' "$scratch/status/DRIFT.md"
if [[ -e "$called" ]]; then
  echo FAIL omni called without scope; exit 1
fi

# now add scope + a denied file on the worktree
cat >"$scratch/repo/docs/orchestration/tasks/active/T1-demo.md" <<'MD'
# T1
## Scope (machine)
allow:
  - README
deny:
  - secret.txt
base: master
## Verification
do not include this heading in the scope block
MD
block="$(wt_scope_block "$scratch/repo/docs/orchestration/tasks/active/T1-demo.md")"
printf '%s\n' "$block" | grep -q '^## Scope (machine)'
if printf '%s\n' "$block" | grep -q '^## Verification'; then
  echo FAIL scope block leaked next heading; exit 1
fi
git -C "$scratch/repo" add docs && git -C "$scratch/repo" commit -q -m scope
# worktree is its own branch: commit deny file there
echo leak >"$scratch/parent/Toy-wt-t1/secret.txt"
git -C "$scratch/parent/Toy-wt-t1" add secret.txt
git -C "$scratch/parent/Toy-wt-t1" commit -q -m leak

printf '#!/bin/sh\ncat >/dev/null\necho DRIFT HIT\n' >"$scratch/fake-omni"
"$HERE/../wt-drift.sh"
grep -q 'DRIFT HIT' "$scratch/status/DRIFT.md"
grep -q '## T1 (Toy)' "$scratch/status/DRIFT.md"

assert_eq "$(wt_extract_scope_base "$scratch/repo/docs/orchestration/tasks/active/T1-demo.md")" "master"
```

- [ ] **Step 2: Run — expect missing scripts**

```bash
bash ChronOS/tools/wt/tests/test_omni.sh
bash ChronOS/tools/wt/tests/test_drift.sh
```

Expected: FAIL missing `wt-omni.sh` / `wt-drift.sh`

- [ ] **Step 3: Implement**

`lib.sh` — `wt_scope_block` + `wt_extract_scope_base`:

```bash
wt_scope_block() {
  local file="$1"
  [[ -f "$file" ]] || return 0
  awk '
    /^## Scope \(machine\)/ {p=1}
    p && /^## / && !/^## Scope \(machine\)/ {exit}
    p {print}
  ' "$file"
}

wt_extract_scope_base() {
  local file="$1" block
  block="$(wt_scope_block "$file")"
  printf '%s\n' "$block" | awk '/^base:[[:space:]]*/ {
    sub(/^base:[[:space:]]*/, "")
    print
    exit
  }'
}
```

`tools/wt/wt-omni.sh` целиком:

```bash
#!/usr/bin/env bash
set -euo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=lib.sh
source "$here/lib.sh"
: "${WT_OMNI_TIMEOUT:=120}"

wt_omni_nonempty() {
  local text="$1"
  [[ -n "${text//[$' \t\r\n']/}" ]]
}

if [[ -n "${WT_OMNI_CURL:-}" ]]; then
  out="$("$WT_OMNI_CURL")" || {
    echo "wt-omni: WT_OMNI_CURL failed" >&2
    exit 1
  }
  wt_omni_nonempty "$out" || { echo "wt-omni: empty fake output" >&2; exit 1; }
  printf '%s\n' "$out"
  exit 0
fi

export WT_OMNI_URL WT_OMNI_MODEL WT_OMNI_TIMEOUT
python3 - <<'PY'
import json, os, sys, urllib.error, urllib.request

url = os.environ["WT_OMNI_URL"]
model = os.environ["WT_OMNI_MODEL"]
timeout = float(os.environ.get("WT_OMNI_TIMEOUT", "120"))
prompt = sys.stdin.read()
body = json.dumps({
    "model": model,
    "stream": False,
    "messages": [{"role": "user", "content": prompt}],
}).encode()
req = urllib.request.Request(
    url,
    data=body,
    headers={"Content-Type": "application/json"},
    method="POST",
)
try:
    with urllib.request.urlopen(req, timeout=timeout) as resp:
        raw = resp.read().decode("utf-8")
except urllib.error.HTTPError as exc:
    err = exc.read().decode("utf-8", errors="replace")
    print(f"wt-omni: HTTP {exc.code}: {err}", file=sys.stderr)
    sys.exit(1)
except Exception as exc:
    print(f"wt-omni: request failed: {type(exc).__name__}: {exc}", file=sys.stderr)
    sys.exit(1)

try:
    data = json.loads(raw)
    text = data["choices"][0]["message"]["content"]
except (KeyError, IndexError, json.JSONDecodeError, TypeError) as exc:
    print(f"wt-omni: bad JSON: {exc}", file=sys.stderr)
    sys.exit(1)

if not isinstance(text, str) or not text.strip():
    print("wt-omni: empty content", file=sys.stderr)
    sys.exit(1)
sys.stdout.write(text if text.endswith("\n") else text + "\n")
PY
```

`chmod +x tools/wt/wt-omni.sh`

`tools/wt/wt-drift.sh` целиком:

```bash
#!/usr/bin/env bash
set -euo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=lib.sh
source "$here/lib.sh"

wt_find_live_brief() {
  local repo="$1" ticket="$2" root tasks f
  root="$(wt_repo_get "$repo" root)"
  tasks="$root/docs/orchestration/tasks"
  for f in \
    "$tasks/active/T${ticket}-"*.md \
    "$tasks/active/pause/T${ticket}-"*.md \
    "$tasks/active/check/T${ticket}-"*.md
  do
    [[ -e "$f" ]] || continue
    printf '%s\n' "$f"
    return 0
  done
  return 1
}

sections=""
while IFS= read -r repo; do
  [[ -n "$repo" ]] || continue
  root="$(wt_repo_get "$repo" root)"
  rr="$(realpath "$root")"
  while IFS= read -r path; do
    [[ -n "$path" ]] || continue
    [[ "$(realpath "$path")" == "$rr" ]] && continue
    name="$(basename "$path")"
    wt_is_exception "$repo" "$name" && continue
    ticket="$(wt_ticket_from_name "$repo" "$name")"
    if [[ -z "$ticket" ]]; then
      sections+="## ${name} (${repo})"$'\n'"no ticket parsed"$'\n\n'
      continue
    fi
    brief=""
    if ! brief="$(wt_find_live_brief "$repo" "$ticket")"; then
      sections+="## T${ticket} (${repo})"$'\n'"no scope declared"$'\n\n'
      continue
    fi
    scope="$(wt_scope_block "$brief")"
    if ! printf '%s\n' "$scope" | grep -q '^## Scope (machine)'; then
      sections+="## T${ticket} (${repo})"$'\n'"no scope declared"$'\n\n'
      continue
    fi
    base="$(wt_extract_scope_base "$brief")"
    [[ -n "$base" ]] || base="$(wt_repo_get "$repo" default_branch)"
    names="$(git -C "$path" diff --name-only "$base"..HEAD || true)"
    prompt="$(
      cat "$here/prompts/drift.txt"
      printf '\n%s\n\n# diff --name-only\n%s\n' "$scope" "$names"
    )"
    ans="$(printf '%s' "$prompt" | "$here/wt-omni.sh")" || exit 1
    sections+="## T${ticket} (${repo})"$'\n'"${ans}"$'\n\n'
  done < <(git -C "$root" worktree list --porcelain | awk '/^worktree /{print substr($0,10)}')
done < <(wt_repo_keys)

[[ -n "${sections//[$' \t\r\n']/}" ]] || { echo "drift: nothing to write" >&2; exit 1; }
printf '%s' "$sections" | wt_atomic_write "$(wt_status_dir)/DRIFT.md"
```

`chmod +x tools/wt/wt-drift.sh`

- [ ] **Step 4: Tests pass** (`test_omni.sh` и `test_drift.sh` не ходят на живой `:20128`, кроме refused на `:1`)

- [ ] **Step 5: Commit**

```bash
git -C ChronOS add tools/wt/wt-omni.sh tools/wt/wt-drift.sh tools/wt/prompts/drift.txt tools/wt/lib.sh tools/wt/tests/test_drift.sh tools/wt/tests/test_omni.sh
git -C ChronOS commit -m "chore : wt-drift — OmniRoute one-shot, DRIFT.md"
```

---

### Task 7: `wt-digest.sh`

**Files:**
- Create: `tools/wt/prompts/digest.txt`
- Create: `tools/wt/wt-digest.sh`
- Create: `tools/wt/tests/test_digest.sh`

**Interfaces:**
- Вход: `$WT_STATUS_DIR/STATUS.md` + `DRIFT.md` (нет файлов → stderr, exit 1, без HTTP)
- Один `wt-omni` вызов
- Атомарно `DIGEST.md`; пустой ответ → exit 1

`prompts/digest.txt`:

```
Summarize STATUS.md and DRIFT.md for a morning briefing in Russian.
No actions. No git. Markdown only.
```

- [ ] **Step 1: Failing test** — положить STATUS+DRIFT в tmp, `WT_OMNI_CURL` echo `digest-ok`, проверить DIGEST.md

- [ ] **Step 2: Run — missing script**

- [ ] **Step 3: Implement `wt-digest.sh`**

```bash
#!/usr/bin/env bash
set -euo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$here/lib.sh"
st="$(wt_status_dir)"
[[ -f "$st/STATUS.md" && -f "$st/DRIFT.md" ]] || { echo "digest: need STATUS.md and DRIFT.md" >&2; exit 1; }
prompt="$(cat "$here/prompts/digest.txt"; echo; echo '# STATUS'; cat "$st/STATUS.md"; echo; echo '# DRIFT'; cat "$st/DRIFT.md")"
out="$(printf '%s' "$prompt" | "$here/wt-omni.sh")"
[[ -n "${out// }" ]] || { echo "digest: empty model output" >&2; exit 1; }
printf '%s\n' "$out" | wt_atomic_write "$st/DIGEST.md"
```

- [ ] **Step 4: Tests pass**

- [ ] **Step 5: Commit**

```bash
git -C ChronOS add tools/wt/wt-digest.sh tools/wt/prompts/digest.txt tools/wt/tests/test_digest.sh
git -C ChronOS commit -m "chore : wt-digest — morning DIGEST.md"
```

---

### Task 8: README + регистрация cron (ручная)

**Files:**
- Create: `tools/wt/README.md`

**Interfaces:** нет кода, кроме уже существующих CLI.

Не вызывать `hermes cron create` из теста и не из автоматического шага агента без явного «создай джобы». В README — готовые команды.

- [ ] **Step 1: Write README**

```markdown
# tools/wt

Ручные ворктри + отчёты. Спека: `docs/superpowers/specs/2026-08-16-worktree-automation-design.md`.

## Команды

    tools/wt/wt-new.sh ChronOS 300 slug [--base <sha>]
    tools/wt/wt-status.sh
    tools/wt/wt-rm.sh ChronOS 300 [--force]
    tools/wt/wt-drift.sh
    tools/wt/wt-digest.sh

Отчёты: `/home/neo/projects/chronos-ecosystem/.wt-status/`.
`wt-new` не пишет brief. Scope-блок — в основном дереве.
`--base` только предок default_branch (стек v1 вручную).

## Тесты

    bash tools/wt/tests/run.sh

## Cron (gateway должен быть жив)

    hermes cron create "*/15 * * * *" --no-agent \
      --script /home/neo/projects/chronos-ecosystem/ChronOS/tools/wt/wt-status.sh \
      --name chronos-worktree-status
    hermes cron create "0 */2 * * *" --no-agent \
      --script /home/neo/projects/chronos-ecosystem/ChronOS/tools/wt/wt-drift.sh \
      --name chronos-worktree-drift
    hermes cron create "0 9 * * *" --no-agent \
      --script /home/neo/projects/chronos-ecosystem/ChronOS/tools/wt/wt-digest.sh \
      --name chronos-worktree-digest

Живая проверка OmniRoute (не часть unit):

    curl -sS http://127.0.0.1:20128/v1/models | head
    WT_STATUS_DIR=/tmp/wt-smoke tools/wt/wt-status.sh
```

- [ ] **Step 2: Нет автотеста на README** — прочитать глазами, что три cron-команды абсолютные и `--no-agent`.

- [ ] **Step 3: Commit**

```bash
git -C ChronOS add tools/wt/README.md
git -C ChronOS commit -m "docs : wt tools README and hermes cron recipes"
```

- [ ] **Step 4: Полный прогон**

```bash
bash ChronOS/tools/wt/tests/run.sh
```

Expected: все `test_*.sh` OK, exit 0.

- [ ] **Step 5: Не регистрировать cron**, пока архитектор не скажет. `PENDING: hermes cron create ×3 — awaiting authorization`

---

## Spec coverage

| Spec | Task |
|---|---|
| repos.yaml / 5 репо / out of scope | 1 |
| name_pattern + aliases `wt-t###` + FM `t051-dnd` | 2 |
| wt-new, --base ancestor, sidecar target, no brief | 3 |
| стек v1 отказ | 3 (`assert_fail --base` side commit) |
| wt-status, primary skip, detached ≠ merged, 4 task dirs | 4 |
| wt-rm dirty/ahead / --force | 5 |
| drift OmniRoute stream:false, no scope declared, DRIFT.md | 6 (`test_omni.sh` + полный `test_drift.sh`) |
| digest | 7 |
| cron recipes, abs --script, gateway dep | 8 |
| холодный старт briefs без scope | 6 (ветка без блока) |

## Self-review

- Плейсхолдеров нет: `wt-status.sh`, `wt-omni.sh` (urlopen + json + timeout + exit), `wt-drift.sh`, `test_status.sh` (detached + 4 dirs), `test_omni.sh`, `test_drift.sh` — литеральный код.
- `git worktree remove --force` только при пользовательском `--force`.
- Алиасы — поля `alias_bare` / `alias_legacy` в yaml, не абсолютный путь экосистемы.
- Имена функций совпадают между задачами (`wt_repo_get`, `wt_ticket_from_name`, `wt_atomic_write`).
- `WT_OMNI_CURL` не описан в spec — тестовый шов, в прод не нужен.
- Регистрация cron сознательно не в автоисполнении.
