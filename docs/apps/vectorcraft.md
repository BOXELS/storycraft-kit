# VectorCraft

Repo: <https://github.com/storytold/vectorcraft>. Used by: `icon-set` (drawing the master). Minimum: 0.24.0.

## CLI we use (`vectorcraft-cli`)

| Command | Use |
|---|---|
| `-V` / `--version` | doctor and preflight |
| `commands` | preflight |
| `run [--in FILE] [--cmd ID [--params JSON] \| --commands FILE\|-]… [--export OUT.svg\|png]… [--scale N] [--ppi N]` | drawing; later params can use `"$N.id"` to name an earlier result |
| `convert IN OUT [--scale N] [--outline-text]` | format change |
| `mcp [--connect\|--headless] [--automation-read-root DIR] [--automation-write-root DIR]` | the gateway's per-app server |

## Command ids we use

`file.new`, `shape.rectangle`, `text.create`, `document.export`. What still needs a live check: [recipes/icon-set.md](../recipes/icon-set.md).

## Reference

VectorCraft's own `packaging/icons.sh` (resvg + `iconutil`, `.ico` sizes 16–256, Apple grid tile) is the model for the packer's targets; STORYCRAFT reimplements it in Rust rather than copying it.

## Known issues

None filed yet.
