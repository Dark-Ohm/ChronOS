# Documentation Generation Pattern

## Parallel Subagent Approach

When creating a documentation suite (3+ related .md files), use `delegate_task` in batches of 3:

```
Batch 1: README.md + ARCHITECTURE.md + ROADMAP.md  (dispatched together)
Batch 2: PITFALLS.md + ACP-PROTOCOL.md + GPUI-GUIDE.md  (dispatched after batch 1)
```

Each subagent gets:
- **Goal**: Write [filename] at [path]. Use write_file tool.
- **Context**: Full project context (what it does, tech stack, key decisions, current status)
- **Role**: leaf (no delegation)

### Why batches of 3?
- `delegation.max_concurrent_children` defaults to 3
- More than 3 causes "Too many tasks" error
- 2 batches of 3 = 6 docs in ~3 minutes

### What each doc needs in context:
- The actual project facts (don't make subagents guess)
- Language (Russian, English, etc.)
- Tone (professional technical, casual, etc.)
- Structure expectations (sections, tables, code blocks)
- Cross-references to other docs (for wikilinks)

### Obsidian vault integration:
- Docs go in project's `docs/` folder
- Use `[[Note Name]]` wikilinks between docs
- If project has `.obsidian/`, the docs are auto-discovered
- Vault path: check `find /home -name ".obsidian" -type d`
