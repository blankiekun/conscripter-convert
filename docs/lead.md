# Team lead notes

- **Delegate only what a few lines of prompt can fully describe**, to
  the cheapest type in `.claude/agents/`: `checker` (haiku) for reruns,
  checks, greps, a clean rebase + push + `gh pr checks --watch`;
  `worker` (sonnet) for mechanical edits, doc updates, build-test-commit
  loops, route runs, releases; `builder` (opus, medium effort) for
  features and engine additions with a clear spec (a screen, a setting,
  wiring existing commands); `engineer` (opus, high) only for reverse
  engineering, decompiles, hard debugging and design calls. The team
  lead picks the type for every agent it starts (the user, 2026-10-17).
- **Do small jobs yourself**: a fresh agent costs about 45k tokens
  before it does anything. A doc-only rebase, a doc edit of 10 lines or
  less, or anything needing what you already know (the session's
  findings, files read, a chain of reasoning) is cheaper done in place.
- **Idea agents** (`ideas`, opus, medium effort, read-only): dispatch one
  with a focus outline (primary goal, constraints, audience, out of
  scope) when the queue holds fewer than 5 items, when a game reaches a
  milestone (story playable, extras done, finished), before a release,
  or when the user names a focus. At most one at a time, and not the
  same focus twice in a week (check `out/lead-scratch/ideas/`). Its list
  lands in `out/lead-scratch/ideas/`; give the user the top 3 with your
  opinion and queue only what they approve.
- **A new game**: `docs/lead-new-game.md` (ask the user's plan first).
- **Offer an idea agent when an issue needs one**: a bug of a kind
  that keeps coming back (two lints this week found real bugs nobody
  saw), an agent stuck twice on the same item, a design choice with no
  clear option, or a gap no queued item covers. Ask the user in one
  line ("Run an idea agent on <focus>?") with the focus outline you
  would give it; start it only on a yes.
- **Landing a PR**: `tests/land.sh <pr>` (`--dry-run` stops before the
  push) rebases it in a scratch worktree, reruns `tests/snapshot.sh
  --update` when snapshots conflicted or shared code changed, pushes,
  waits for the checks and squash-merges. Use it instead of messaging the
  agent to rebase; on a real conflict it refuses and prints what to tell
  the agent.
- **Mechanical guards** (`.claude/hooks/bash_guards.py`, on every Bash
  call; self-test `--test`): refuses `pkill -f`/`pgrep -f`, a
  `git commit` carrying a Claude-Session line, and heavy commands
  (cargo, snapshot, pr-report, engine builds, convert-game) when under
  6 GiB of memory is free. `tests/land.sh` takes `out/land.lock` (one
  landing at a time) and refuses a draft or non-OPEN PR.
- **A bug the user found** (the PR's "Found by: user", from
  `tests/pr-report.sh`): land it only when the PR names the check that
  now catches its kind (or "none possible" and why) and what of the
  report is still open; queue what is open. A user report fixed "in
  part" without that came back the next day (#339, #436).
