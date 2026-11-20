# Mac verification checklist

Phase 1 was built and tested on Linux against fake app CLIs. These items need one pass on the Mac with released apps; tick them off and fix recipes or docs where reality differs.

## Setup

- [ ] `sc flags enable AI-MCP` writes `~/Library/Application Support/ArtCraft/flags.toml`, and after a restart the launcher shows its agents panel.
- [ ] The launcher's switch is labelled "Let AI agents use these apps" (update `docs/mac-setup.md` and the doctor hint if not).
- [ ] The launcher app's path is `/Applications/ArtCraft Launcher.app` or `/Applications/ArtCraft.app` (doctor checks both).
- [ ] Each app bundle ships `<app>-cli` in `Contents/MacOS/` (`sc apps` shows source `AppBundle` or `LauncherList`).
- [ ] `apps.json` lists photocraft, vectorcraft, filmcraft after the switch is on; `gateway` check is ok.
- [ ] `sc doctor --fix` adds `STORYCRAFT` to `~/.cursor/mcp.json` without touching `ArtCraft`; Cursor shows both servers and the four `sc_*` tools.
- [ ] `--version` output of each CLI parses (`sc apps` shows a version, not `-`).

## youtube-thumbnail (PhotoCraft)

- [ ] `sc commands photocraft --search layerStyle` lists `layer.layerStyle.stroke` and `layer.layerStyle.dropShadow`.
- [ ] A run on a 4000×3000 JPEG gives a 1280×720 `thumbnail.jpg` under 2 MB and a PSD that opens in PhotoCraft with an editable title layer.
- [ ] `type.create` keys (`box`, `font`, `size`, `align`, `color` floats) are accepted; the title is centred in the lower third with outline and shadow.
- [ ] A missing font fails with a clear error (and `sc bug draft` produces a sensible draft from it).

## icon-set (VectorCraft)

- [ ] `vectorcraft-cli run --cmd file.new …` with no `--in` creates the document.
- [ ] `shape.rectangle` accepts `fill` / `stroke: "none"`; `text.create` accepts `size`, `font`, `color`, `justify`.
- [ ] `document.export` with `outlineText: true` writes `out/master.svg` with paths, not `<text>`; `run` may write there without automation roots.
- [ ] `AppIcon.icns` opens in Preview and looks the same as `iconutil -c icns out/icons/macos/AppIcon.iconset`.
- [ ] `windows/app.ico` and `web/favicon.ico` preview correctly (Quick Look).

## Other

- [ ] Exit codes for `vectorcraft-cli` and `filmcraft-cli` (PhotoCraft's are 0/1/2).
- [ ] `filmcraft-cli commands`: record the catalog in `apps/filmcraft/<version>.json` for the first FilmCraft recipe.
