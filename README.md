# storycraft-kit

Agent control and automated workflows for the [ArtCraft Crafting Apps](https://getartcraft.com/apps) (project STORYCRAFT).

The Craft apps already ship a command-line tool each (`photocraft-cli`, `vectorcraft-cli`, `filmcraft-cli`, …) and the ArtCraft Launcher ships an MCP gateway (`craft-ai-mcp`) that lets agents use them. storycraft-kit adds the layer above:

- **`sc doctor`**: checks a Mac is ready for agents to drive the apps (the `AI-MCP` flag, the launcher's agents switch, each app's CLI and version, MCP client settings) and offers fixes, each applied only with your OK.
- **`sc flags`**: turns suite flags such as `AI-MCP` on or off in `~/Library/Application Support/ArtCraft/flags.toml`.
- **Recipes**: workflows as YAML, run by `sc run`, with every output measured before the run counts as done.
  - `youtube-thumbnail`: photo + title -> 1280×720 JPEG under 2 MB + layered PSD + small-size previews (PhotoCraft).
  - `icon-set`: app name (or your SVG) -> `.icns`, `.ico`, iOS, Android, web and Linux icons + a contact sheet (VectorCraft draws the master; the built-in packer does the rest).
- **`storycraft-mcp`**: the same workflows as MCP tools (`sc_doctor`, `sc_list_recipes`, `sc_run_recipe`, `sc_bug_draft`) next to the ArtCraft gateway.
- **Bug drafts**: a failed app step leaves `failure.json`; `sc bug draft` turns it into a scrubbed report. Nothing is filed without per-issue approval.

First apps: PhotoCraft, VectorCraft, FilmCraft.

## Quick start (Mac)

```sh
# Rust stable (rust-toolchain.toml pins the channel)
cargo install --path crates/sc
cargo install --path crates/mcp

sc doctor                # what is missing
sc flags enable AI-MCP   # asks first; restart the launcher and apps afterwards
sc doctor --fix          # offers each remaining fix

sc run youtube-thumbnail --set image=~/Desktop/photo.jpg --set title="Big News Today"
sc run icon-set --set name="StoryCraft"
sc run icon-set --set name=X --set master=~/Desktop/logo.svg   # no app needed
```

Every run gets its own folder under `~/StoryCraft/jobs/` (`in/`, `out/`, `job.json`). Use `--dry-run` to see the exact app commands first.

Full setup, including the exact AI-MCP steps: [docs/mac-setup.md](docs/mac-setup.md). Things still to confirm against real apps: [docs/mac-verification.md](docs/mac-verification.md).

## Layout

| Path | What |
|---|---|
| `crates/core` | `storycraft-core`: paths, flags, app discovery, doctor, recipes, engine, verify, icon packer, bug drafts |
| `crates/sc` | the `sc` command line |
| `crates/mcp` | `storycraft-mcp` (stdio MCP server) |
| `recipes/` | recipe YAML; `recipes/presets/` holds looks and output specs |
| `docs/` | architecture, Mac setup, per-recipe and per-app notes, bug loop |
| `.cursor/`, `.claude/`, `AGENTS.md` | agent rules, skills and subagents for this repo |

## Development

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Tests run every recipe end to end against fake app CLIs (shell scripts), so they pass on Linux CI without the apps.

## License

MIT OR Apache-2.0, matching the Craft suite.
