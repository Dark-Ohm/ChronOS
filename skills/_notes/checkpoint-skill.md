# Checkpoint — Skill Index (chronos/skills)

> Index skill. Every other skill in this vault should link here so an agent
> entering any skill can find the full map. Companion to [[start-here]].

## Purpose

`start-here` is the session-entry router (what to read, which skill to invoke).
`checkpoint` is the **static index** — a durable list of every skill with a
one-line comment, so navigation doesn't depend on reading `start-here`'s
routing table. An agent in any leaf skill can jump here and see the whole graph.

## Status

- [ ] Not yet created. This is Task 1 of the skills-graph hardening.
- [ ] Every skill needs a reverse wikilink to this note (or to `start-here`).

## Skill list (with comments)

### Project-specific (chronos)
- [[chronos-shell]] — how the code is actually laid out/wired (crates/app, services, luau, plugins)
- [[gpui]] — generic GPUI framework API knowledge (upstream gpui-ce, not chronos-specific usage)
- [[start-here]] — session orientation router + sibling-disambiguation

### Process (superpowers family)
- [[brainstorming]] — design before implementation
- [[writing-plans]] — write the plan
- [[executing-plans]] — execute a written plan with checkpoints
- [[subagent-driven-development]] — parallel same-session execution
- [[dispatching-parallel-agents]] — 2+ independent tasks, no shared state
- [[test-driven-development]] — tests before code
- [[systematic-debugging]] — root-cause before fixes
- [[using-git-worktrees]] — isolated workspace
- [[requesting-code-review]] / [[receiving-code-review]] — review gate
- [[verification-before-completion]] — prove it works before claiming
- [[finishing-a-development-branch]] — merge/PR/cleanup
- [[writing-skills]] — author/edit skills (TDD for docs)
- [[using-superpowers]] — invoke skills before any response

### Reference / cross-cutting
- [[mcp-memory-workflow]] — 3 MCP memory layers, universal + chronos scopes
- [[documentation-investigation]] — extract technical docs across a codebase
- [[philip-main]] — audit/rewrite docs from code evidence
- [[rust-skills-master]] — Rust coding guidelines
- [[dogfood]] — web-app QA (almost never applicable here)

### Isolated island (needs wiring)
- `engram/*` (21 skills) — see [[engram-island]] — currently `related_skills: []`, no graph participation

## Related
- [[start-here]] — the entry router this index complements
- [[engram-island]] — Task 2
- [[architecture-crates-ui-discrepancy]] — Task 3
- [[documentation-investigation-dead-links]] — Task 4
