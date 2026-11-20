# Recipe: youtube-thumbnail

Photo + title -> a YouTube thumbnail, made in PhotoCraft. File: `recipes/youtube-thumbnail.yaml`; look: `recipes/presets/youtube-thumbnail.yaml`.

```sh
sc run youtube-thumbnail --set image=photo.jpg --set title="Big News Today"
sc run youtube-thumbnail --set image=photo.jpg --set title="Big News" --set text_color="#FFD400"
```

## Inputs

| Input | Type | Default | Notes |
|---|---|---|---|
| `image` | file (png, jpg, jpeg) | required | scaled and cropped to fill 16:9 |
| `title` | text | required | 2–5 words read best at phone size |
| `text_color` | colour | `#FFFFFF` | |
| `outline_color` | colour | `#000000` | |

## Outputs (in `out/`)

| File | Checked |
|---|---|
| `thumbnail.jpg` | JPEG, 1280×720, at most 2,000,000 bytes (YouTube's limit is 2 MB) |
| `thumbnail.psd` | layered source (PSD signature, not empty) |
| `previews/preview-168x94.png`, `preview-320x180.png` | how it reads in search results and the mobile feed |

## Steps

| Step | Kind | What |
|---|---|---|
| `fit` | built-in `cover-fit` | reads the photo size; works out the resize and centred crop that cover 1280×720 |
| `compose` | `photocraft-cli run <photo>` | `image.imageSize` -> `image.crop` -> `type.create` (title in the lower third) -> `layer.layerStyle.stroke` -> `layer.layerStyle.dropShadow`, `--out thumbnail.psd` |
| `export` | `photocraft-cli convert` | PSD -> JPEG at `--quality` from the preset (88) |
| `previews` | built-in `previews` | downscaled copies |

Requires PhotoCraft 0.23.0 or later with all five command ids (checked before anything runs).

## Verify on the Mac

These parameter shapes come from PhotoCraft's source, not from a live run yet:

- `image.imageSize {width, height, resample: "bicubic"}` with both sides given resizes exactly (no aspect lock).
- `type.create` accepts `box: [x, y, w, h]`, `font` as a family name (`Helvetica Neue`), `size` in px, `align: "center"`, `color` as `[r, g, b]` floats 0–1, and leaves the new text layer selected so the two layer styles apply to it.
- `layer.layerStyle.stroke` takes `color` as `"#rrggbb"`, `position: "outside"`, `opacity` 0–100.
- `layer.layerStyle.dropShadow` with `{}` uses sensible defaults.
- `convert IN.psd OUT.jpg --quality 88` flattens layer styles the same way the app does.

## Later

Brand presets (font, colours, logo corner via `file.placeEmbedded`), A/B title variants, a face-aware crop.
