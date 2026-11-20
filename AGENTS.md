# Agent instructions

Project conventions for AI coding agents live in `.cursor/rules/*.mdc` and `.cursor/skills/*/SKILL.md`. Cursor loads them automatically. If you are a different agent (Claude Code, Codex, Copilot, etc.), read these before making changes:

- `.cursor/rules/global-rules.mdc` — how to behave: ask vs. act, scope discipline, verify before reporting
- `.cursor/rules/product.mdc` — what STORYCRAFT is and who it's for
- `.cursor/rules/tech.mdc` — Rust workspace, recipes, process safety, git, tests
- `.cursor/rules/security.mdc` — file roots, MCP, secrets, public GitHub text
- `.cursor/rules/structure.mdc` — where files go
- `.cursor/rules/documentation.mdc` — doc locations and what to keep current
- `.cursor/rules/craft-apps.mdc` — how to drive the ArtCraft apps (gateway, CLIs, roots, verification)
- `.cursor/rules/upstream-contrib.mdc` — filing issues and PRs on storytold repos
- `.cursor/rules/recipes.mdc` — recipe authoring (scoped to `recipes/**`)
- `.cursor/rules/design.mdc` — Mac front-end UI (scoped to `mac/**`)
- `.cursor/rules/models.mdc` — Cursor model guidance, including plan-then-delegate

Skills: `update-docs`, `artcraft-bug-report` in `.cursor/skills/`.

Subagent: `.cursor/agents/worker.md` — low-cost executor for fully specified steps (Cursor only).

Don't duplicate rule content here. This file is intentionally a pointer.
