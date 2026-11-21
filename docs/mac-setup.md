# Mac setup

What a Mac needs before agents (Cursor, Claude Code) or `sc` can drive the Craft apps, and how `sc doctor` checks each piece.

## 1. Install the apps

Install the **ArtCraft Launcher** from <https://getartcraft.com/apps>, then install PhotoCraft, VectorCraft and FilmCraft from it. The launcher puts each app in `/Applications/<Name>.app`; each app's command-line tool (`<app>-cli`) ships beside the main executable.

Without the launcher (or to test a newer build), unpack the app's `<app>-cli` release zip from its GitHub release and either put it on `PATH` or point `sc` at it:

```sh
export SC_PHOTOCRAFT_CLI=~/bin/photocraft-cli
sc run youtube-thumbnail --app photocraft=~/bin/photocraft-cli ...   # one run only
```

`sc apps` shows which CLI each app resolves to and its version. Lookup order: `SC_<APP>_CLI`, the launcher's `apps.json`, `/Applications/<Name>.app/Contents/MacOS/<app>-cli` (then `~/Applications`), `PATH`.

## 2. Turn on the AI-MCP flag

The suite gates agent access behind the `AI-MCP` feature flag. Every Craft app and the launcher read it from one per-user file:

```
~/Library/Application Support/ArtCraft/flags.toml
```

```toml
[flags]
AI-MCP = true
```

Either:

```sh
sc flags enable AI-MCP      # shows the file, asks, then writes (keeps any other flags and comments)
sc flags status             # AI-MCP: on (file …: Some(true); CRAFT_FLAGS: None)
```

or edit the file by hand with the two lines above (create the folder if it does not exist).

Then **quit and reopen the ArtCraft Launcher and any open Craft app**; flags are read at start.

Notes:
- `CRAFT_FLAGS=AI-MCP` turns it on for one run of a command started from that shell (`CRAFT_FLAGS=AI-MCP,-AI-MODELS` style). Apps started from the Dock or Finder do not see it, so use the file.
- `CRAFT_FLAGS_FILE=/path/flags.toml` points the suite (and `sc`) at another file, for tests.
- `sc doctor` reports `flag.ai-mcp` as `ok` only when the file says `true`.

## 3. Turn on the launcher's agents switch

In the ArtCraft Launcher: **Settings -> "Let AI agents use these apps"** (visible once AI-MCP is on). This writes:

- `~/Library/Application Support/ArtCraft/mcp/apps.json`: which installed apps agents may use, and each app's CLI;
- `~/Library/Application Support/ArtCraft/mcp/bin/craft-ai-mcp`: the gateway copy MCP clients start.

From the same panel add the clients you use (Cursor, Claude Code, Claude Desktop, LM Studio). Each gets an `ArtCraft` server entry, for example in `~/.cursor/mcp.json`:

```json
{ "mcpServers": { "ArtCraft": { "command": "…/ArtCraft/mcp/bin/craft-ai-mcp", "args": ["--stdio"] } } }
```

The gateway offers `craft_list_apps`, `craft_find_tools`, `craft_describe_tool`, `craft_call_tool`.

## 4. Add the STORYCRAFT server (optional)

```sh
cargo install --path crates/sc
cargo install --path crates/mcp
sc doctor --fix     # offers: add the STORYCRAFT MCP server to Cursor / Claude Code
```

This adds `"STORYCRAFT": { "command": "~/.cargo/bin/storycraft-mcp", "args": [] }` beside `ArtCraft`, leaving every other entry alone. Start the server from the repo folder, or set `SC_RECIPES_DIR` to the `recipes/` folder, so it finds the recipes.

## 5. Check

```sh
sc doctor            # table; exit 1 if anything FAILs
sc doctor --json     # for agents
```

| Check | Means |
|---|---|
| `flag.ai-mcp` | `AI-MCP = true` in `flags.toml` (fix: `sc flags enable AI-MCP`) |
| `launcher` | the launcher app is in `/Applications` |
| `launcher.agents` | the agents switch is on (`apps.json` exists) and which apps it lists |
| `gateway` | the gateway copy exists |
| `app.photocraft`, `app.vectorcraft`, `app.filmcraft` | CLI found, answers `--version`, and is listed in `apps.json` |
| `client.cursor.*`, `client.claude-code.*` | `ArtCraft` / `STORYCRAFT` entries in that client's MCP settings |
| `gh` | GitHub CLI signed in (only needed to file approved bug reports) |

## Where things go

| What | Where | Override |
|---|---|---|
| Job folders | `~/StoryCraft/jobs/<recipe>-<time>-<pid>/` | `SC_JOBS_DIR` or `sc run --job-dir` |
| Recipes | `./recipes`, else `~/.config/storycraft/recipes` | `SC_RECIPES_DIR` |
| App logs (bug reports) | `~/Library/Application Support/<AppName>/logs` | |
