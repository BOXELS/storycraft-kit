# Architecture

## Layers

```
 agents (Cursor, Claude Code)          people (Terminal, later Finder / desktop app)
        |               \                         |
  ArtCraft gateway     storycraft-mcp ----------- sc
  (craft-ai-mcp,        (workflow tools)          (doctor, flags, run, verify, bug)
   per-app tools)            \                   /
        |                     storycraft-core (engine, verify, pack, bug)
        |                             |
        +----------> <app>-cli (photocraft-cli, vectorcraft-cli, filmcraft-cli)
```

- **App tools** (open, edit, export inside one app) come from the apps themselves through the launcher's gateway. STORYCRAFT does not wrap or duplicate them.
- **Workflows** (several steps or apps, a fixed output contract) are STORYCRAFT recipes. `sc` and `storycraft-mcp` are two front ends over the same `storycraft-core`.
- Apps are only ever driven through their own CLIs. No app crate is linked; no app process is patched.

## storycraft-core modules

| Module | Does |
|---|---|
| `paths` | Suite folders (`ArtCraft/flags.toml`, `ArtCraft/mcp`), job, cache and recipe folders, with the suite's env overrides |
| `flags` | Reads `flags.toml` + `CRAFT_FLAGS` like `craft-flags`; writes a flag keeping the rest of the file (`toml_edit`) |
| `apps` | Finds each app's CLI, reads `--version` and `commands`, runs commands with a timeout |
| `clients` | Reads/writes `mcpServers` entries in MCP client settings |
| `doctor` | Read-only checks plus explicit fixes |
| `recipe` | Recipe schema (unknown keys refused), presets, `{{…}}` templating with `json`/`rgb01` filters |
| `engine` | Job folder, input checks and copies, preflight (versions, command ids), steps, verify, `job.json`/`failure.json` |
| `verify` | Format sniffing, pixel size, bytes, alpha, image counts in `.ico`/`.icns` |
| `pack` | SVG master -> shaped PNGs (`full`, `macos` Apple grid, `opaque`), `.ico` and `.icns` writers, contact sheet |
| `bug` | `failure.json` -> scrubbed Markdown draft |

`craft-ai-agents` (storytold/craft-ai-integrations, pinned `rev`) reads the launcher's `apps.json`, so STORYCRAFT sees the same app list and consent the gateway does.

## Recipe run

1. Load the recipe; refuse unknown keys or steps that use an app not in `requires`.
2. Check inputs (required, type, file extension); copy file inputs into `in/`.
3. Work out which steps will run (`when`), then preflight only those apps: CLI found, version at least `min_version`, every listed command id present in `<app>-cli commands`.
4. Run steps in order. App steps run `<app>-cli` with the job folder as working directory and a timeout; a non-zero exit writes `failure.json` and stops. Built-ins: `cover-fit`, `monogram`, `pack-icons`, `previews`.
5. Measure every output named in `verify`; any miss fails the run (exit 3).
6. Write `job.json`: recipe version, app paths and versions, inputs, each step's command, exit code and time, verify results.

`--dry-run` prints every resolved command and writes nothing.

## storycraft-mcp

JSON-RPC 2.0 over stdio, one message per line: `initialize`, `ping`, `tools/list`, `tools/call`. Tools: `sc_doctor` (read-only), `sc_list_recipes` (read-only), `sc_run_recipe` (writes only into a new job folder), `sc_bug_draft` (writes `bug-draft.md` in the job folder; never files). Logs go to stderr only.

## Exit codes (`sc`)

0 ok, 1 operation failed (or doctor found a FAIL), 2 usage error, 3 verifier failed.
