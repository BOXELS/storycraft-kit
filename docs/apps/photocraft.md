# PhotoCraft

Repo: <https://github.com/storytold/photocraft>. Used by: `youtube-thumbnail`. Minimum: 0.23.0.

## CLI we use (`photocraft-cli`)

| Command | Use |
|---|---|
| `--version` | doctor and preflight |
| `commands` | preflight checks the recipe's command ids |
| `run <FILE> \| --new JSON  [--cmd ID [--params JSON]]…  [--out FILE] [--quality 1-100] [--format EXT]` | edits; exactly one of FILE/--new, at least one --cmd or --out, one --out per run |
| `convert IN OUT [--quality N] [--format EXT]` | flatten / change format |
| `mcp --headless --automation-read-root DIR --automation-write-root DIR` | the gateway's per-app server |

Exit codes: 0 ok, 1 operation failed, 2 usage error.

## Command ids we use

`image.imageSize`, `image.crop`, `type.create`, `layer.layerStyle.stroke`, `layer.layerStyle.dropShadow`. Parameter shapes and what still needs a live check: [recipes/youtube-thumbnail.md](../recipes/youtube-thumbnail.md).

Others seen in source, for later recipes: `file.new`, `image.canvasSize`, `file.placeEmbedded`, `layer.select`, `layer.align.*`.

## Known issues

None filed yet. Log upstream issues in `notes/contributions.md`.
