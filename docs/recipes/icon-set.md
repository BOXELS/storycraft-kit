# Recipe: icon-set

App name (or your own SVG) -> every icon file a Mac, iOS, Windows, Android, web or Linux app needs. File: `recipes/icon-set.yaml`; look: `recipes/presets/icon-set.yaml`; outputs: `recipes/presets/icon-targets.yaml`.

```sh
sc run icon-set --set name="StoryCraft"                       # VectorCraft draws an "SC" monogram tile
sc run icon-set --set name="Film Studio" --set background="#111111" --set foreground="#FFCC00"
sc run icon-set --set name=X --set letters=XY                  # choose the letters
sc run icon-set --set name=X --set master=logo.svg             # your SVG; no app needed
```

## Inputs

| Input | Type | Default | Notes |
|---|---|---|---|
| `name` | text | required | initials become the monogram (`PhotoCraft` -> `PC`, `my cool app` -> `MC`) |
| `letters` | text | | overrides the monogram |
| `master` | file (svg) | | a square SVG with text outlined; skips the VectorCraft step |
| `background` | colour | `#2F5BFF` | tile colour |
| `foreground` | colour | `#FFFFFF` | letter colour |

## Steps

| Step | Kind | What |
|---|---|---|
| `mono` | built-in `monogram` | the letters |
| `draw` | `vectorcraft-cli run` (skipped with `master`) | `file.new` 1024×1024 -> `shape.rectangle` full-bleed fill -> `text.create` monogram -> `document.export` SVG with `outlineText`, plus a PNG preview |
| `pack-drawn` / `pack-given` | built-in `pack-icons` | rasterizes the master (resvg) into every target |

The master is full-bleed; platform shaping happens in the packer:

- `macos`: Apple's grid, a rounded tile 824/1024 of the canvas (corner radius 22.37% of the tile) with a transparent margin; `.iconset` PNGs plus `AppIcon.icns` (written directly, same entry types as `iconutil`).
- `opaque`: flattened onto white, no alpha (iOS 1024 App Store icon, `apple-touch-icon`).
- `full`: as drawn (Windows `.ico` 16–256, Android, web, Linux).

## Outputs (in `out/icons/`)

`macos/AppIcon.icns`, `macos/AppIcon.iconset/*`, `ios/AppIcon-1024.png`, `windows/app.ico`, `android/play-store-512.png`, `android/mipmap-*/ic_launcher.png`, `web/favicon.ico`, `web/favicon-16.png`, `web/favicon-32.png`, `web/apple-touch-icon.png`, `web/icon-192.png`, `web/icon-512.png`, `web/icon.svg`, `linux/hicolor/*`, and `contact-sheet.png` (every size up to 256 on light and dark).

Checked: `.icns` holds 10 images, `.ico` holds 9, iOS icon is 1024×1024 with no alpha, favicon and Play Store sizes are exact.

## Verify on the Mac

From VectorCraft's source, not yet a live run:

- `run` with no `--in` and `file.new {width, height}` as the first command creates the document.
- `shape.rectangle` accepts `fill: "#rrggbb"` and `stroke: "none"` (rounded corners are not needed: the packer shapes the tile).
- `text.create` accepts `size`, `font`, `color`, `justify` (these names come from the MCP `add_text` tool; `text.create` refuses unknown keys, so a wrong name fails fast).
- `document.export {path, format: "svg", outlineText: true}` writes outlined text, and `run` may write to the job folder without automation roots.
- Open `AppIcon.icns` in Finder / Preview and compare it with an `iconutil -c icns` build of `AppIcon.iconset`.
