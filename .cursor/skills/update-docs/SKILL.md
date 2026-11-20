---
name: update-docs
description: Bring project documentation and .cursor/rules back in sync after a feature, refactor, or removal. Finds the affected docs, edits them in place, creates a new doc only when none exists, cross-references rules, and reports what changed. Use when the user says "update docs", "update documentation", "update the docs", "sync docs", or after shipping a change that alters architecture, conventions, or constraints.
---

# Update Docs

Run this whenever the user asks to update documentation, or after a change that other agents will need to know about. Do all five steps before reporting back. Do not ask the user which files to touch — find them.

Doc folder, naming convention, and what belongs in a doc are defined in `.cursor/rules/documentation.mdc`. Read it first if you haven't this session.

## Workflow

```
- [ ] 1. Find related docs
- [ ] 2. Update existing docs in place
- [ ] 3. Create a new doc only if none covers the area
- [ ] 4. Cross-reference from .cursor/rules/
- [ ] 5. Report every file touched
```

### 1. Find related docs

Search the doc folder for files related to the work just performed. Derive search terms from feature names, table names, routes, and component names touched. If several docs might be relevant, read their headers and update every one that mentions the affected area.

### 2. Update existing docs in place

Edit, don't append. Remove references to deleted patterns. Do not leave incomplete or contradictory information. Prefer editing an existing doc over creating a duplicate.

### 3. Create a new doc only when none exists

Only if no existing file covers the area. Follow the naming convention in `documentation.mdc`. A new feature doc includes: one-sentence purpose, files/routes/tables touched, how it works, critical constraints (things agents must NOT do), and links to related docs. Document only what was actually implemented — never invent behavior.

### 4. Cross-reference from `.cursor/rules/`

If the change affects patterns or conventions other agents need:

- Cross-cutting concern → `global-rules.mdc`.
- Domain-specific → the matching rule file (`tech.mdc`, `design.mdc`, `security.mdc`, `models.mdc`, `structure.mdc`), or a new focused `.mdc` file.
- Add a one-line "see `docs/FOO.md`" pointer from the rule when a longer reference exists.
- New rule files must use the `.mdc` extension with `description`, `globs`, and `alwaysApply` frontmatter. Scope with `globs` when the rule only matters for certain file types.

### 5. Report

End with a one-line list of every doc and rule file edited or created.

## Do not

- Ask "which docs should I update?"
- Create a new doc when an existing one covers the area.
- Write marketing copy. Write concise reference material agents will read.
- Invent feature behavior or document intent that wasn't implemented.
- Leave the old and new convention both described as current.
