---
name: obsidian
description: Filesystem-first vault work (reading, searching, editing skill files) has moved into editing-skills. Use editing-skills instead.
platforms: [linux, macos, windows]
---

# Obsidian Vault → editing-skills

The file-tool mechanics for vault work (resolve vault path; prefer `read_file` / `search_files` / `patch` / `write_file` over shell; read / list / search / append / targeted edits / wikilinks) now live in **editing-skills**, together with the discipline that governs when an edit is legitimate.

**REQUIRED:** Use editing-skills for editing an existing skill, and writing-skills for authoring a new one.

This pointer is kept so that anything still invoking `obsidian` is redirected rather than broken; it can be deleted once no workflow references it.
