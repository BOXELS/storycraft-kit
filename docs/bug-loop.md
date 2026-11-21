# Bug loop

From an app misbehaving in a workflow to an issue (or fix PR) on its storytold repo, with a person approving every public step.

1. **Capture**: a failed app step writes `failure.json` in the job folder: recipe, step, app and version, the exact command, exit code or timeout, stdout/stderr tails, OS.
2. **Draft**: `sc bug draft <job dir>` (or the `sc_bug_draft` MCP tool) writes `bug-draft.md`: title, environment, steps to reproduce (the command, shell-quoted), expected, actual, log tails. The home folder and user name are scrubbed.
3. **Review**: reproduce on the latest release, cut the input down to a minimal file, search the repo's issues for duplicates, and add app logs with `.cursor/skills/artcraft-bug-report/scripts/collect.sh`. Read the draft again for anything personal.
4. **Approve**: the user approves **this specific issue**. No approval, no filing.
5. **File**: `gh issue create --repo storytold/<app>` with the approved text; add the link to `notes/contributions.md` and to the app's `docs/apps/<app>.md`.
6. **Fix PR (optional, allowed)**: clean-room only, one issue per PR, following the repo's own contribution rules (`craft-contrib/standards`). Never commit `.cursor/` or `.claude/` to a storytold repo. See `.cursor/rules/upstream-contrib.mdc`.
