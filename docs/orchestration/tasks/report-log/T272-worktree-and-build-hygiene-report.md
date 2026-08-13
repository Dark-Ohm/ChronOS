# T272 — worktree and build hygiene report

## Outcome

**PARTIAL / NO DELETIONS NEEDED.** The one-time cleanup was audited on
2026-08-13. No currently registered worktree passed the “dead + clean + zero
commits outside `master`” removal gate, so no worktree, branch, or source file
was deleted.

The hygiene rule is already present in `docs/ARCHITECT.md` at commit `4a276fe`.

## Verification

### Worktree inventory before cleanup

Command:

```bash
git worktree list
```

Output:

```text
/home/neo/projects/chronos-ecosystem/ChronOS            4a276fe [master]
/home/neo/projects/chronos-ecosystem/ChronOS-wt-measure bd0274f [measure/component-bench]
/home/neo/projects/chronos-ecosystem/ChronOS-wt-t266    77428c6 (detached HEAD)
```

The previously reported `ChronOS-wt-t267` and `verify-0b3aed7` paths were
already absent when this run started. Their old status/log evidence was not
available in this run, so this report does **not** claim that this execution
removed them.

Per-worktree audit:

| Tree | HEAD | Status | `master..HEAD` | Decision |
|---|---|---|---|---|
| `ChronOS` | `4a276fe` | dirty, pre-existing WIP | empty | Keep; never clean a shared checkout |
| `ChronOS-wt-measure` | `bd0274f` | clean | `bd0274f`, `ec02946` | Keep; two commits outside `master`, documented long-lived measurement exception |
| `ChronOS-wt-t266` | `77428c6` | clean | empty | Keep; T266 is still active, but this base is stale and must not be used for code |

The T266 base check was run with:

```bash
git -C ../ChronOS-wt-t266 merge-base --is-ancestor 4a276fe HEAD
```

It returned exit `1`: the tree does not contain current `master`. Before T266
code work resumes, create a fresh sibling worktree from the commit that
actually contains accepted T263. Do not repair this tree by rebasing or
checking out over foreign work.

### Stale metadata and branches

Command:

```bash
git worktree prune --dry-run -v
```

Output: empty. There were no stale worktree metadata entries to prune.

Command:

```bash
git branch --merged master
```

Output:

```text
  feat/t267-edge-separator
* master
  measure/gpui-component
```

Merged branches were listed only. T272 explicitly does not delete branches;
no branch was removed.

### Build target audit

Command:

```bash
echo "$CARGO_TARGET_DIR"
ls -d ../ChronOS-wt-*
```

Output showed an empty `CARGO_TARGET_DIR` and these candidates:

```text
../ChronOS-wt-measure
../ChronOS-wt-t266
```

Both registered worktrees have no local `target/` directory. The main tree
has a local target of `31G`:

```text
31G  /home/neo/projects/chronos-ecosystem/ChronOS/target
```

For context, the same ecosystem-level scan observed:

```text
1.1G  ../Chronos-IDE/target
31G   ../Source/target
210G  ../Chronos-FM/target
22G   ../Chronos-lm/target
```

These are not ChronOS worktree cleanup candidates and were not touched. The
scan confirms that a shared/default target arrangement is expensive enough to
remain an explicit build-debugging suspect.

### Repository scope

The T272 changes themselves are documentation-only (`docs/ARCHITECT.md` and
the T272 brief at `4a276fe`). The main checkout still contains unrelated
pre-existing dirty files, including `crates/`; this report does not attribute
those files to T272 and did not stage or modify them.

## Risks / follow-up

1. The current `ChronOS-wt-t266` is clean but based before the accepted T263
   dependency. It is retained because T266 remains active, not because it is a
   valid implementation base.
2. The release-build incident remains recorded as an environment risk:
   `cargo build --release -p chronos` first returned exit `101`, and the
   repeated invocation timed out after 600 seconds while trees shared the
   default target area. A future code change must first print
   `CARGO_TARGET_DIR` and enumerate sibling worktrees, then retry in an
   isolated target before blaming source code.
3. `ChronOS-wt-measure` remains intentionally long-lived for the component
   measurement record. Its branch and two commits outside `master` are the
   recorded reason it was not removed.

---

## Приёмка архитектора (2026-08-13): ПРИНЯТО

Проверено мной, не по тексту отчёта: `git worktree list` — три дерева,
`prune --dry-run` пуст, `target/` основного дерева 31G, собственных
`target/` в воркетри нет ни одного. Правило записано в
`docs/ARCHITECT.md` (коммит `4a276fe`, раздел «Инструмент подозревается
раньше кода»).

Отдельно отмечу дисциплину: безопасных кандидатов на снос не нашлось, и
исполнитель НЕ приписал себе удаление `ChronOS-wt-t267` / `verify-0b3aed7`,
хотя они исчезли в ту же сессию (снёс их я вручную до старта аудита).
Ветки не удалялись, грязный WIP не тронут. Ровно то поведение, которого
мы добиваемся.

## Что аудит изменил в наших выводах

**Гипотеза «общий `target/` уронил сборку T266» НЕ подтвердилась.** В
`ChronOS-wt-t266` каталога `target/` нет вовсе — значит сборка там либо не
запускалась, либо падала до его создания. Причина `exit 101` на doc-only
правке остаётся неустановленной; правило №2 в `ARCHITECT.md` верно само по
себе, но этот случай не объясняет. Требование к исполнителю T266: приложить
полный вывод первой упавшей команды, без него разбор превращается в
гадание.

## Побочная находка (вне зоны тикета)

Экосистемный замер: `Chronos-FM/target` — **210 ГБ**, против 31 ГБ у
ChronOS, 31 ГБ у Source, 22 ГБ у Chronos-lm, 1.1 ГБ у Chronos-IDE. Это
почти столько же, сколько все остальные вместе. Действие — за пределами
ChronOS-сессии, вынесено пользователю как факт, а не как задача.
