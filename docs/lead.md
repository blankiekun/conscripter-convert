# Team lead workflow

The lead orders the work, writes briefs, reviews and merges PRs. State lives in the gitignored handoff file
`out/lead-scratch/NEXT-LEAD-PROMPT.md`; this file holds the durable rules. New game: `docs/lead-new-game.md`.

**Autonomy is bounded** by a goal, a definition of done and a session budget, set by the user and recorded atop
the handoff file. Ask for a missing one before starting; at done or budget, stop and report.

## Spend where it pays

- Ask the user at session start how much weekly usage is left; size the session to it.
- A fresh agent costs ~45k tokens before it acts: do in place edits of 10 lines or less, doc-only rebases and
  anything needing what you know; run bare commands with `run_in_background` (~1k tokens).
- Cheapest agent that fits (`.claude/agents/`; the lead picks the type): `checker` (haiku) for reruns, greps,
  one fact from a long file; `worker` (sonnet) for mechanical edits, docs, build-test-commit loops, route runs,
  releases; `builder` (opus, medium) for features and engine additions with a clear spec; `engineer` (opus,
  high) for reverse engineering, decompiles, experiment design, hard bugs, design calls. No idea agents.
- Launch independent items in one message. One agent builds, the lead or another measures (agents that did both
  passed 130k context).
- Machines do the mechanical checks (CI or a hook, not a checker agent); a task done twice becomes a one-command
  script. Verification depth follows CLAUDE.md's testing rule.
- Size an item to finish under ~100k context (CLAUDE.md, "No agent past 100k context").
- CLAUDE.md holds build/test commands and the conventions agents get wrong.

## Briefs

- The first brief carries every constraint (a later SendMessage can arrive after the agent has acted): the
  rules, numbers and paths the task needs, "follow docs/lead.md Team rules", phase branch, worktree, whether
  the agent opens the PR, and the task's real shape ("N questions follow").
- Close with: **"Report as `key: value` lines, numbers first, no recap; detail goes in the commit messages."**

## Evidence

- A claim carries numbers: fixed inputs, a control run the same day as the variant (tools drift), n>=3, real
  token counts or costs, raw results committed. Estimates stay in the queue, out of shipped docs.
- Pilot a new harness at n=1 and read the raw per-call data first.
- Test what a real session does; a haiku smoke run covers plumbing only.
- Report null results and every cost the change moves.

## Starting a line of work

- Dumbest baseline first: the cheapest alternative (existing tool or NScripter command, cheaper model, config
  change, manual step, nothing); build what beats it.
- Pre-mortem: "Two weeks from now this wasted the budget. Why?" Record the top reason and the assumptions beside
  the item in the handoff file, so the user can veto at a glance.
- Budget the line in sessions; when spent: ship, stop, or ask for more.

## Checking the work

- Use the feature after building it: play the web build or run the CLI end to end.
- Read real usage each session: error logs, the issue tracker, 2-3 session transcripts.
- Review a diff with "how does this break?" (`/code-review` or a read-only sonnet agent).
- At milestones a haiku agent follows the README literally on a clean checkout and reports each failing step.
- A bug the user found ("Found by: user" in `tests/pr-report.sh`) lands when the PR names the check that now
  catches its kind (or "none possible" and why) and what of the report is still open; queue that remainder.
  (#339, #436: partial fixes came back the next day.)

## The queue

- Demand fills it: the user's goal, the friction log, what the last result changed. Empty means ship or stop:
  ask the user ("goal X reached at Y; next goal, or stop?").
- Friction log (handoff file): whoever hits waste (time, tokens, a re-explanation) adds a line.
- An item enters with expected win, measurement and the result that kills it. When stuck, run the single
  measurement that would change the next move, or drop the item.
- Outside view: ask a read-only sonnet agent "which items would you cut, and why?"

## Stopping

- Ship the smallest useful version and watch whether it gets used before building infrastructure.
- Each session end, ask: is the process bigger than the product? is the queue only micro-optimizations? did the
  last two items come back null? Any yes: write up the findings and tell the user.
- The handoff file's Decisions list holds each choice and rejected idea, one line with its reason.

## Team rules (every agent)

- Work in the worktree and branch the brief names; commit at each logical step; open a PR only if the brief says
  so. CLAUDE.md's Rules hold.
- Never merge PRs. Rebase on origin/main and push before handing back.
- A decision the brief didn't cover: stop and report it.
- Long commands run in the foreground (timeout up to 10 min) or under Monitor.

## Landing PRs

- One PR per goal phase, a unit the user reads whole, on one branch and worktree
  (`git worktree add ../conscripter-convert-<phase> -b <phase> origin/main`); items are commits, from one agent
  or several in turn. An engine change gets its CONscripter PR first (CLAUDE.md, "Engine changes").
- One chores PR per session: typos, doc tweaks and fixes under ~20 lines on one `chores-<date>` branch; handoff
  and lead-notes edits stay out of git or ride in it.
- At each phase end post a 5-line summary (what, why, what to look at) pointing at the PR.
- Tripwire: after ~5 PRs opened in a session, ask the user why before opening another.
- Review the diff. A PR claiming a number: rerun one arm (yourself or a checker). A PR changing another game's
  `0.txt`: rerun `tests/snapshot.sh --diff origin/main` independently.
- Land with `tests/land.sh <pr>` (`--dry-run` stops before the push), as a command on its own. It rebases in a
  scratch worktree, reruns `tests/snapshot.sh --update` when snapshots conflicted or shared code changed,
  pushes, waits for green checks and squash-merges. It holds `out/land.lock`, refuses a draft or non-OPEN PR,
  and on a real conflict prints what to tell the agent. If denied, ask the user.
- Route fixes to the PR's author, or have a checker or the user review: your own commits before merging read as
  an unreviewed merge, and auto mode then blocks even `gh pr view` / `git pull`.
- The user adds allow rules (tests, build, `tests/land.sh`) to the git-ignored `settings.local.json` in `.claude/`;
  ask for them.

## Mechanical guards

`.claude/hooks/bash_guards.py` (every Bash call; self-test `--test`) refuses `pkill -f`/`pgrep -f`, a
`git commit` with a Claude-Session line, and heavy commands (cargo, snapshot, pr-report, engine builds,
convert-game) under 6 GiB free memory. `.claude/hooks/context_cap.py` tells an agent past 80k context to hand off.

## Session mechanics

- Handoff file: under ~10 KB, done items deleted (git log is the record), the exact command of anything still
  running.
- Session start: check ListAgents, `ps`, `git worktree list`, `gh pr list` (agents and runs survive /clear).
- The scratchpad dies with the session: copy results into the repo first.
- Before removing a worktree copy its untracked outputs out (the `cleanup` skill keeps worktrees with unmerged
  work, untracked files or saves); deletes go through a backup dir.
- Auto mode blocks subagents from reading credentials, even when the lead may.
- `claude -p` in scripts: leave ANTHROPIC_API_KEY unset (it bills the API, not the plan); under `--resume`,
  `total_cost_usd` / `modelUsage` are session totals.

## End of session

Tell the user what merged, what is open and why, what was learned (with numbers), what waits on them.
