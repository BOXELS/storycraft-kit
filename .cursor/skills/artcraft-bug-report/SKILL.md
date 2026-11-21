---
name: artcraft-bug-report
description: >-
  Collects local ArtCraft app details and files a GitHub bug report on the
  matching storytold (or SolidCraft) repository. Use when the user wants to
  report, file, or submit a bug or issue in VectorCraft, PhotoCraft, FilmCraft,
  LightCraft, PdfCraft, EffectCraft, DesignCraft, FigCraft, WordCraft,
  SoundCraft, DeckCraft, CADCraft, GridCraft, TextureCraft, MaterialCraft,
  Craft Anything, SolidCraft, or another ArtCraft / Crafting app.
---

# ArtCraft bug reports

File one public GitHub issue per bug, from this machine, as the logged-in `gh` user. Search first. A thin report wastes the maintainer's time; a duplicate wastes it twice.

Repo names, log folders, and the issue template are in [reference.md](reference.md).

## Workflow

1. **Which app.** Use the app the user named. If they did not name one, use the app whose log just changed. Run the collector (path is next to this file):

   ```bash
   bash "<this-skill-dir>/scripts/collect.sh" VectorCraft
   ```

   Omit the name to list installed Craft apps.

2. **What happened.** Prefer the user's words for the steps. Use the log for the error text, version, GPU, and timestamps. If steps, expected, and actual are still unclear and the log has no error, ask one question and wait. Do not invent a click path.

3. **Search.** `gh search issues` on that repo for the distinctive error and for a few plain words. Check closed issues too. `gh search issues --repo OWNER/REPO --author @me` catches one we already filed.

4. **Duplicate.** Comment on the existing issue with the version, OS, and the new log line. Do not open another issue. The VectorCraft save-sheet panic `unexpected NULL returned from +[NSSavePanel savePanel]` on 0.22.0 is already [#5980](https://github.com/storytold/vectorcraft/issues/5980).

5. **File.** `gh issue create --repo OWNER/REPO` with the template in [reference.md](reference.md). Skip labels if the API rejects them. Reply with the issue URL.

6. **Log.** If `/Users/jsmith/CursorApps/ArtCraftBOXELS/notes/contributions.md` exists, add a newest-first entry there (template at the top of that file) in the same turn.

7. **Several bugs.** One issue each. Say so before filing the set.

## Rules

- Confirm `gh auth status` first. File as that account. Do not switch accounts or ask for a token.
- Public text only. No home-directory path, username, document contents, or full log. Quote the error lines. Say "the release app" or "a source build", plus the version from Help › About / the collector.
- Crash logs rotate. `app.1.log` is the previous launch; a panic from the run that just quit is often there, not in the new file.
- A missing feature is a feature request. Title it that way and say what the user was trying to make. Do not label it a bug.
- Security vulnerabilities: look for `SECURITY.md` in the repo. Do not put exploit detail in a public issue. Tell the user where the project asks to report it.
- ArtCraft Launcher has no confirmed `storytold` issues repo. Search before filing. Do not file launcher bugs on a third-party fork.
- Do not clone the app repo into the current workspace just to file a report. `gh` can read a public file when the report needs a code pointer.
