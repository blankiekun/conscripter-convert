# Team lead workflow

The lead picks and orders the work, writes briefs, reviews PRs and merges them. Project state lives in one
handoff file (`out/lead-scratch/NEXT-LEAD-PROMPT.md`, gitignored); this file holds the durable rules.
A new game: `docs/lead-new-game.md` (ask the user's plan first).

**Autonomy is bounded by a goal, a definition of done and a session budget**, all set by the user and recorded at
the top of the handoff file. If any of the three is missing, ask for it before starting work. At done or at
budget, stop and report; don't look for more work.

## Spend where it pays

- **Ask the user how much weekly usage is left** at the start of each session, and size the session to fit.
- **Do small jobs yourself.** A fresh agent costs ~45k tokens before it does anything. An edit of 10 lines or
  less, a doc-only rebase, or anything that needs what you already know is cheaper done in place.
- **Run commands yourself** with `run_in_background` (~1k tokens, and you are notified on exit). An agent that
  only runs given commands costs ~45k.
- **Use the cheapest agent that fits** (`.claude/agents/`; the lead picks the type for every agent):
  `checker` (haiku) for reruns, greps, finding one fact in a long file or trace; `worker` (sonnet) for mechanical
  edits, docs, build-test-commit loops, route runs, releases; `builder` (opus, medium) for features and engine
  additions with a clear spec; `engineer` (opus, high) only for reverse engineering, decompiles, experiment
  design, hard bugs and design calls. No idea agents.
- **Start agents by value per cost.** Don't start four and kill two.
- **Split design from measurement.** One agent builds; the lead or another agent runs and reports. Agents that
  did both passed 130k context.
- **Machines do the mechanical checks**: tests, lint and type checks run in CI or a hook, not in a checker agent.
  Anything done twice becomes a script with one command.
- **Size items to fit one agent**: finished under ~100k context (CLAUDE.md, "No agent past 100k context").
  Split anything bigger before briefing.
- **Launch independent items together** in one message, so they run in parallel.
- **Keep CLAUDE.md short**: build/test commands and the conventions agents get wrong. Every agent reads it, so a
  brief doesn't need to repeat it.

## Briefs

- **Every constraint goes in the first brief.** A later SendMessage can arrive after the agent has already acted on it.
- Give only the rules, numbers and paths the task needs, plus "follow docs/lead.md Team rules". Never send an agent
  to the whole handoff file.
- Name the phase branch and worktree, and say whether the agent opens the PR.
- End every brief with: **"Report as `key: value` lines, numbers first, no recap; detail goes in the commit messages."**
- State the task's real shape (for example, "N questions follow"), or the agent optimizes for the wrong case.

## Evidence

- **Claims need numbers**: a fixed input set, a control, n>=3, real token counts or costs, raw results committed.
  Run the control the same day as the variant: tools and overhead drift between days.
- **Pilot every new harness at n=1** and read the raw per-call data before the full run.
- **Test what a real session does**, not what you assume it does. A haiku smoke run checks plumbing, not behaviour.
- **An unmeasured estimate goes in the queue, never into shipped docs.**
- A null or negative result is a result: report it, don't tune until it turns positive.
- Report every cost a change moves, not only the one it targets.

## Before starting a line of work

- **Dumbest baseline first.** Before building, try the cheapest alternative: an existing tool or NScripter
  command, a cheaper model, a config change, a manual step, or doing nothing. Build only what beats it.
- **Pre-mortem, in context, one minute**: "Two weeks from now this wasted the budget. Why?" Write the top reason
  in the handoff file next to the item, and check it first.
- **Write the assumptions down** in the handoff file, so the user can veto one at a glance.
- **Give each line of work a budget** in sessions. When it's spent, decide: ship, stop, or ask the user for more.

## Checking the work

- **Use the thing yourself** after each feature: play the web build or run the CLI end to end the way a user would.
  Tests only check what someone thought to test.
- **Read real usage** each session: error logs, the issue tracker, 2-3 real session transcripts.
- **Review each diff with one question**, "how does this break?" (`/code-review`, or a read-only sonnet agent),
  never "what else could we do?".
- **Fresh-eyes setup test** at milestones: a haiku agent follows the README literally on a clean checkout and
  reports each step that fails.
- **A bug the user found** (the PR's "Found by: user", from `tests/pr-report.sh`): land it only when the PR names
  the check that now catches its kind (or "none possible" and why) and what of the report is still open; queue
  what is open. A user report fixed "in part" without that came back the next day (#339, #436).

## Where work comes from

- **The queue is driven by demand**: the user's goal, the friction log, and what the last result changed. An empty
  queue means ship or stop; bring it to the user ("goal X reached at Y; next goal, or stop?"). Don't generate ideas
  to fill it.
- **Friction log** (in the handoff file): whoever hits waste in real work (time, tokens, a re-explanation) adds one
  line. Next session's queue is drawn from it.
- **An item enters the queue only with its expected win, how it will be measured, and what result kills it.**
  If you can't write the expected win, leave it out.
- **Stuck on an item?** Ask "what single measurement would change what we do next?" and run it; if nothing would,
  drop the item.
- **For an outside view, ask for a critique, not new ideas**: a read-only sonnet agent answers "which of these
  queue items would you cut, and why?"

## Know when to stop

- **Measure real use early**: ship the smallest useful version and see whether it gets used before building
  infrastructure to optimize it.
- Review at the end of each session: is the process (harness, rules, handoffs) bigger than the product? Is the queue
  only micro-optimizations? Have the last two items both come back null? If so, stop, write up the findings
  and tell the user. Don't start a new round.
- Record each choice and each rejected idea in the handoff file's Decisions list, one line with its reason, so
  later sessions don't argue it again.

## Team rules (every agent)

- Work in the worktree and branch the brief names. Commit at each logical step with a clear message; open a PR only
  if the brief says so. Never use git stash; stage paths by name. CLAUDE.md's Rules hold (engine PRs, `0.txt`,
  saves, wine).
- Never merge PRs. Before handing back, rebase on origin/main and push.
- If the job turns out to need a decision the brief didn't cover, stop and report it.
- Long commands run in the foreground (timeout up to 10 min) or wait with Monitor; never background a run and stop.

## Landing PRs

- **A PR is a unit the user would read as a whole: one goal phase**, not one queue item. One branch and worktree per
  phase (`git worktree add ../conscripter-convert-<phase> -b <phase> origin/main`); items inside the phase are
  commits on it, from one agent or several in turn. An engine change still needs its own CONscripter PR, merged
  first (CLAUDE.md, "Engine changes"); the phase PR then points the submodule at `master`.
- **One chores PR per session**: typos, doc tweaks and fixes under ~20 lines go on one `chores-<date>` branch,
  merged once. Handoff and lead-notes edits stay out of git, or ride in the chores PR.
- **The user reviews milestones, not every PR**: at the end of each phase, post a 5-line summary (what changed,
  why, what to look at) and point the user at that PR.
- **Tripwire**: a session that opens more than ~5 PRs stops and asks the user why before opening another.
- Review the diff. For a PR that claims a number, rerun one arm (yourself or with a checker) before merging. A PR
  that changes another game's `0.txt`: an independent `tests/snapshot.sh --diff origin/main` rerun.
- **Land with `tests/land.sh <pr>`** (`--dry-run` stops before the push): it rebases in a scratch worktree, reruns
  `tests/snapshot.sh --update` when snapshots conflicted or shared code changed, pushes, waits for the checks and
  squash-merges. It takes `out/land.lock` (one landing at a time) and refuses a draft or non-OPEN PR; on a real
  conflict it refuses and prints what to tell the agent. Run it as a command on its own; if it is denied, ask the
  user; never work around a denial.
- **Don't add your own commits to a PR and then merge it**: auto mode flags that as an unreviewed merge and then
  blocks even `gh pr view` / `git pull`. Have the author fix it, or get a checker or the user to review it.
- **Pre-approve routine steps.** Ask the user to add allow rules to `.claude/settings.local.json` for the commands
  that always get approved (tests, build, `tests/land.sh`). Never add rules yourself.

## Mechanical guards

`.claude/hooks/bash_guards.py` (on every Bash call; self-test `--test`) refuses `pkill -f`/`pgrep -f`, a
`git commit` carrying a Claude-Session line, and heavy commands (cargo, snapshot, pr-report, engine builds,
convert-game) when under 6 GiB of memory is free. `.claude/hooks/context_cap.py` tells an agent past 80k context
to hand off.

## Session mechanics

- **Handoff file**: under ~10 KB; delete done items (git log is the record); record the exact command of
  anything still running. Hand off before the lead's context gets long.
- Background agents and runs survive /clear: at session start check ListAgents, `ps`, `git worktree list`, `gh pr list`.
- The scratchpad dies with the session: copy results into the repo before a handoff.
- Before removing a worktree, copy its untracked outputs out (the `cleanup` skill keeps any worktree with
  unmerged work, untracked files or saves). Deletes go through a backup dir.
- Auto mode blocks subagents from reading credentials, even when the lead may. The user edits settings files;
  the lead cannot (self-modification).
- If you run `claude -p` in scripts: never export ANTHROPIC_API_KEY there (it bills the API instead of the plan),
  and under `--resume`, `total_cost_usd` / `modelUsage` are session totals.

## End of session

Tell the user what was merged, what is open and why, what was learned (with numbers), and what waits on them.
