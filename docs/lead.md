# Team lead workflow

The lead picks and orders the work, writes briefs, reviews PRs and merges them. Project state lives in the
handoff file (`out/lead-scratch/NEXT-LEAD-PROMPT.md`, gitignored); this file holds the durable rules.
A new game: `docs/lead-new-game.md` (ask the user's plan first).

**Autonomy is bounded** by a goal, a definition of done and a session budget, all set by the user and recorded at
the top of the handoff file. Ask for any missing one before starting. At done or at budget, stop and report.

## Spend where it pays

- **Ask the user how much weekly usage is left** at session start; size the session to fit.
- **Do small jobs yourself**: a fresh agent costs ~45k tokens before it does anything. An edit of 10 lines or
  less, a doc-only rebase, or anything that needs what you already know is cheaper in place.
- **Run commands yourself** with `run_in_background` (~1k tokens, notified on exit); an agent that only runs
  given commands costs ~45k.
- **Cheapest agent that fits** (`.claude/agents/`; the lead picks the type for every agent):
  `checker` (haiku) for reruns, greps, one fact from a long file or trace; `worker` (sonnet) for mechanical
  edits, docs, build-test-commit loops, route runs, releases; `builder` (opus, medium) for features and engine
  additions with a clear spec; `engineer` (opus, high) for reverse engineering, decompiles, experiment design,
  hard bugs, design calls. No idea agents.
- **Start agents by value per cost**; launch independent items together in one message.
- **Split design from measurement**: one agent builds; the lead or another agent runs and reports (agents that
  did both passed 130k context).
- **Machines do the mechanical checks** (tests, lint, type checks run in CI or a hook, not in a checker agent);
  anything done twice becomes a script with one command. Verification depth follows CLAUDE.md's testing rule.
- **Keep CLAUDE.md short**: build/test commands and the conventions agents get wrong; every agent reads it.
- **Size items to one agent**: finished under ~100k context (CLAUDE.md, "No agent past 100k context"); split
  bigger ones before briefing.

## Briefs

- **Every constraint goes in the first brief**: a later SendMessage can arrive after the agent has acted.
- Include only the rules, numbers and paths the task needs, plus "follow docs/lead.md Team rules"; CLAUDE.md
  already reaches every agent.
- Name the phase branch and worktree, and whether the agent opens the PR.
- State the task's real shape ("N questions follow"), or the agent optimizes for the wrong case.
- End with: **"Report as `key: value` lines, numbers first, no recap; detail goes in the commit messages."**

## Evidence

- **Claims need numbers**: a fixed input set, a control run the same day as the variant (tools and overhead
  drift), n>=3, real token counts or costs, raw results committed.
- **Pilot every new harness at n=1** and read the raw per-call data before the full run.
- **Test what a real session does**; a haiku smoke run checks plumbing, not behaviour.
- **An unmeasured estimate goes in the queue**, not in shipped docs.
- Report null and negative results as they are; report every cost a change moves, not only its target.

## Before starting a line of work

- **Dumbest baseline first**: try the cheapest alternative (an existing tool or NScripter command, a cheaper
  model, a config change, a manual step, nothing); build only what beats it.
- **Pre-mortem**, one minute: "Two weeks from now this wasted the budget. Why?" Write the top reason next to the
  item in the handoff file and check it first.
- **Write the assumptions down** in the handoff file, so the user can veto one at a glance.
- **Budget each line of work** in sessions; when spent, decide: ship, stop, or ask the user for more.

## Checking the work

- **Use the thing yourself** after each feature: play the web build or run the CLI end to end as a user would.
- **Read real usage** each session: error logs, the issue tracker, 2-3 real session transcripts.
- **Review each diff with one question**, "how does this break?" (`/code-review`, or a read-only sonnet agent).
- **Fresh-eyes setup test** at milestones: a haiku agent follows the README literally on a clean checkout and
  reports each failing step.
- **A bug the user found** (the PR's "Found by: user", from `tests/pr-report.sh`): land it only when the PR names
  the check that now catches its kind (or "none possible" and why) and what of the report is still open; queue
  what is open. (A fix "in part" without that came back the next day: #339, #436.)

## Where work comes from

- **Demand drives the queue**: the user's goal, the friction log, what the last result changed. An empty queue
  means ship or stop: bring it to the user ("goal X reached at Y; next goal, or stop?").
- **Friction log** (in the handoff file): whoever hits waste in real work (time, tokens, a re-explanation) adds a
  line; next session's queue is drawn from it.
- **An item enters the queue** with its expected win, how it will be measured, and what result kills it.
- **Stuck?** Find the single measurement that would change what we do next and run it; if none exists, drop the item.
- **Outside view = critique**: a read-only sonnet agent answers "which of these queue items would you cut, and why?"

## Know when to stop

- **Ship the smallest useful version** and see whether it gets used before building infrastructure around it.
- End-of-session review: is the process (harness, rules, handoffs) bigger than the product? Is the queue only
  micro-optimizations? Did the last two items both come back null? On any yes, stop, write up the findings, tell
  the user.
- Record each choice and rejected idea in the handoff file's Decisions list, one line with its reason.

## Team rules (every agent)

- Work in the worktree and branch the brief names; commit at each logical step; open a PR only if the brief says
  so. CLAUDE.md's Rules hold.
- Never merge PRs. Before handing back, rebase on origin/main and push.
- A job that needs a decision the brief didn't cover: stop and report it.
- Run long commands in the foreground (timeout up to 10 min) or wait with Monitor.

## Landing PRs

- **One PR per goal phase**, a unit the user reads as a whole, not one queue item. One branch and worktree per
  phase (`git worktree add ../conscripter-convert-<phase> -b <phase> origin/main`); items are commits on it, from
  one agent or several in turn. An engine change gets its own CONscripter PR first (CLAUDE.md, "Engine
  changes"); the phase PR then points the submodule at `master`.
- **One chores PR per session**: typos, doc tweaks and fixes under ~20 lines go on one `chores-<date>` branch,
  merged once. Handoff and lead-notes edits stay out of git, or ride in it.
- **The user reviews milestones**: at the end of each phase post a 5-line summary (what changed, why, what to
  look at) pointing at that PR.
- **Tripwire**: more than ~5 PRs opened in a session means stop and ask the user why before opening another.
- **Review the diff.** A PR that claims a number: rerun one arm (yourself or a checker) before merging. A PR
  that changes another game's `0.txt`: an independent `tests/snapshot.sh --diff origin/main` rerun.
- **Land with `tests/land.sh <pr>`** (`--dry-run` stops before the push): it rebases in a scratch worktree,
  reruns `tests/snapshot.sh --update` when snapshots conflicted or shared code changed, pushes, waits for the
  checks (merging needs them green) and squash-merges. It takes `out/land.lock` (one landing at a time), refuses
  a draft or non-OPEN PR, and on a real conflict prints what to tell the agent. Run it as a command on its own;
  if denied, ask the user.
- **The author fixes a PR**: your own commits added before merging read as an unreviewed merge, and auto mode
  then blocks even `gh pr view` / `git pull`. Send fixes back to the author, or get a checker or the user to
  review.
- **Pre-approve routine steps**: ask the user to add allow rules (tests, build, `tests/land.sh`) to the
  git-ignored `.claude/settings.local.json`; the user edits settings.

## Mechanical guards

`.claude/hooks/bash_guards.py` (every Bash call; self-test `--test`) refuses `pkill -f`/`pgrep -f`, a
`git commit` carrying a Claude-Session line, and heavy commands (cargo, snapshot, pr-report, engine builds,
convert-game) when under 6 GiB of memory is free. `.claude/hooks/context_cap.py` tells an agent past 80k context
to hand off.

## Session mechanics

- **Handoff file**: under ~10 KB; delete done items (git log is the record); record the exact command of
  anything still running. Hand off before the lead's context gets long.
- **At session start** check ListAgents, `ps`, `git worktree list`, `gh pr list`: background agents and runs
  survive /clear.
- **The scratchpad dies with the session**: copy results into the repo before a handoff.
- **Before removing a worktree**, copy its untracked outputs out (the `cleanup` skill keeps any worktree with
  unmerged work, untracked files or saves); deletes go through a backup dir.
- Auto mode blocks subagents from reading credentials, even when the lead may.
- `claude -p` in scripts: leave ANTHROPIC_API_KEY unset (it bills the API instead of the plan); under
  `--resume`, `total_cost_usd` / `modelUsage` are session totals.

## End of session

Tell the user what was merged, what is open and why, what was learned (with numbers), and what waits on them.
