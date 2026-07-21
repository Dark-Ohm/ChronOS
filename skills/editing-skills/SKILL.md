---
name: editing-skills
description: Use when changing the content of an existing skill — reorganizing, fixing, extending, or merging SKILL.md files in the skills vault. For authoring a brand-new skill, use writing-skills.
platforms: [linux, macos, windows]
---

# Editing Skills

Editing an existing skill is two things at once: a **discipline** (what change is legitimate) and a **mechanic** (how to touch the file). They come from two places and used to contradict — this skill reconciles them.

- **Discipline — from `writing-skills`.** The Iron Law extends to edits: a behavior-changing edit needs a baseline first, you never weaken a check to make it pass, and you verify after. Editing is not exempt from the method just because the file already exists.
- **Mechanic — filesystem-first, via native file tools.** Prefer `read_file` / `search_files` / `patch` / `write_file` over shell (`cat` / `grep` / `find` / `ls` / heredocs / `echo`): line numbers, pagination, no shell-quoting traps, structured results, and paths may contain spaces.

**REQUIRED BACKGROUND:** Use writing-skills for the full RED-GREEN-REFACTOR method, SDO, and bulletproofing. This skill does not repeat it — it scopes it to edits and adds the file mechanics.

## When to use

- Changing an existing SKILL.md: fixing wording, adding/removing a section, closing a rationalization loophole, merging two skills, correcting frontmatter.
- **When NOT to use:** authoring a new skill from scratch (→ writing-skills); a mechanical constraint better enforced by validation than documentation.

## Discipline gate — before you touch the file

1. **Classify the edit.** Reference/mechanics fix (typo, path, restructure, cross-reference) → proceed to mechanics; the TDD-baseline ceremony is N/A for pure reference content (writing-skills says so explicitly). Behavior-shaping change (a new rule, a loosened/tightened requirement, a changed description that alters when the skill loads) → it needs the writing-skills method: baseline, minimal change, re-verify.
2. **Never weaken to make green.** Do not delete or soften a rule, checklist item, or red-flag to resolve a conflict. If two skills contradict, reconcile by scoping (who owns what), not by deleting the harder rule.
3. **Read before you overwrite.** Open the whole file (or the whole span) before `write_file`/`patch`. A large unfamiliar block is not automatically garbage — read it and understand intent before replacing it.

## Vault path

Resolve a concrete absolute vault path before calling file tools. Convention: `/home/neo/projects/chronos-ecosystem/ChronOS/skills/`.

File tools do not expand shell variables — never pass `$OBSIDIAN_VAULT_PATH` to `read_file`/`write_file`/`patch`/`search_files`; resolve it first. If the path is unknown, `terminal` is acceptable only to resolve `OBSIDIAN_VAULT_PATH` or check the fallback exists, then switch back to file tools.

## Read a skill

`read_file` with the resolved absolute path — over `cat` — for line numbers and pagination. Read the **whole** file before a structural edit; read the relevant span before a targeted one.

## List / search skills

`search_files` over `find`/`grep`/`ls`:
- List: `target: "files"`, `pattern: "*.md"` under the vault (or a subfolder's absolute path).
- Content: `target: "content"`, the regex as `pattern`, `file_glob: "*.md"` to restrict to markdown.

Use content search to find contradictions before merging — e.g. two SKILL.md declaring the same `name:`, or a rule stated two different ways.

## Make the edit

- **Targeted change with stable context** → `patch`: replace the anchor (a heading or a known trailing block) with the anchor plus the new content. Prefer this over shell text rewriting.
- **Whole-file rewrite** → `write_file`, but only when you have fully read the file this session and a rewrite is clearer than a fragile patch.
- **Simple append, no stable anchor** → `terminal` is acceptable only if it is the clearest safe option.

## Frontmatter integrity

When editing, keep the skill's identity coherent — a scrambled `name:` is the most damaging edit there is:
- `name:` matches the directory and is unique across the vault (two dirs with one `name:` is a collision — fix it, don't ship it).
- `description:` is third-person, starts with "Use when…", states triggering conditions only, and does **not** overlap another skill's triggers (overlap makes the loader pick wrong). If you split one skill into two, split the triggers cleanly.

## Wikilinks

Obsidian links skills with `[[Note Name]]`. When adding cross-references between skills, prefer a named requirement marker (`**REQUIRED BACKGROUND:** Use writing-skills`) over `@`-links, which force-load and burn context.

## Verify after

- Re-read the changed span (or re-run `search_files` for the thing you claimed to fix) — observation, not assumption.
- Behavior-shaping edit → the writing-skills verification applies (re-run the scenario / micro-test). Reference edit → confirm the file still parses and no dangling reference (a `readme =`/link/path pointing at a file that isn't there).
