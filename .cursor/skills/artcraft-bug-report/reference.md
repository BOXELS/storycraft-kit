# ArtCraft report reference

## Repos

| App | Issues |
|---|---|
| VectorCraft | https://github.com/storytold/vectorcraft/issues |
| PhotoCraft | https://github.com/storytold/photocraft/issues |
| FilmCraft | https://github.com/storytold/filmcraft/issues |
| LightCraft | https://github.com/storytold/lightcraft/issues |
| PdfCraft | https://github.com/storytold/pdfcraft/issues |
| EffectCraft | https://github.com/storytold/effectcraft/issues |
| DesignCraft | https://github.com/storytold/designcraft/issues |
| FigCraft | https://github.com/storytold/figcraft/issues |
| WordCraft | https://github.com/storytold/wordcraft/issues |
| SoundCraft | https://github.com/storytold/soundcraft/issues |
| DeckCraft | https://github.com/storytold/deckcraft/issues |
| CADCraft | https://github.com/storytold/cadcraft/issues |
| GridCraft | https://github.com/storytold/gridcraft/issues |
| TextureCraft | https://github.com/storytold/texturecraft/issues |
| MaterialCraft | https://github.com/storytold/materialcraft/issues |
| Craft Anything | https://github.com/storytold/craft-anything/issues |
| SolidCraft | https://github.com/tangency-labs/solidcraft/issues |

Desktop logs live next to preferences: macOS `~/Library/Application Support/<AppName>/logs/` (PhotoCraft's folder on this Mac is `Photocraft`), Linux `$XDG_CONFIG_HOME/<app>/logs/` or `~/.config/<app>/logs/`, Windows `%APPDATA%\<AppName>\logs\`. The collector finds the macOS folder. Each launch rotates `app.log` to `app.1.log`, then `app.2.log`. VectorCraft's file stops growing at 16 MiB.

Maintainers keep asking for the same facts: version (Help › About), operating system, display setup, what was clicked, expected, actual, and a screenshot or the error line when the log has one.

## Issue body

```markdown
## What I was doing
One sentence.

## Steps to reproduce
1. 

## Expected

## Actual

## Environment
- App and version (Help › About):
- Release app, source build, or web build:
- OS and version:
- Machine and GPU:
- Display setup:

## Log
Quote only the error lines. No home-directory path.
```

## Commands

```bash
gh search issues --repo storytold/vectorcraft "NSSavePanel" --limit 10
gh issue create --repo storytold/vectorcraft --title "short specific title" --body-file /tmp/issue.md
gh issue comment 5980 --repo storytold/vectorcraft --body "Still happens on 0.22.0, macOS 15.7.4. Same log line."
```
