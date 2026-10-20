# Team lead notes

- **Delegate only what a few lines of prompt fully describe**, to the
  cheapest type in `.claude/agents/`: `checker` (haiku) for reruns,
  checks, greps, a clean rebase + push + `gh pr checks --required
  --watch`; `worker` (sonnet) for mechanical edits, doc updates,
  build-test-commit loops, route runs, releases; `builder` (opus,
  medium) for features with a clear spec; `engineer` (opus, high) only
  for reverse engineering, decompiles, hard debugging and design calls.
  The lead picks the type for every agent.
- **Do small jobs yourself**: a fresh agent costs about 45k tokens
  before it starts. A doc-only rebase, a short doc edit, or anything
  needing what you already know is cheaper in place.
- **Idea agents** (`ideas`, read-only) only when the user asks. Give the
  user every idea on its list (title, one line, effort) except those you
  advise against, each named with a one-line reason; queue only what
  they approve.
- **A new game**: ask the user's plan first, then `docs/new-game.md`.
- **Landing a PR**: `tests/land.sh <pr>` (`--dry-run` stops before the
  push) rebases in a scratch worktree, reruns `tests/snapshot.sh
  --update` when snapshots conflicted or shared code changed, pushes,
  waits for the checks and squash-merges. On a real conflict it refuses
  and prints what to tell the agent. It takes `out/land.lock` and
  refuses a draft or non-OPEN PR.
- **Guards** (`.claude/hooks/bash_guards.py`, every Bash call; `--test`):
  refuses `pkill -f`/`pgrep -f`, a commit with a Claude-Session line, and
  heavy commands (cargo, snapshot, pr-report, engine builds,
  convert-game) under 6 GiB free memory.
- **A bug the user found** ("Found by: user" in the PR): land it only
  when the PR names the check that now catches its kind (or "none
  possible" and why) and what is still open; queue what is open.
