# CONscripter-convert: notes for Claude

Converts visual novels the user owns into games for CONscripter (our
ONScripter-Yuri fork, `engine/` submodule), especially its web build.

| Game | Engine | Branch prefix | `docs/games/` |
|---|---|---|---|
| 装甲悪鬼村正 (JP+EN, active) | Nitroplus NSS; EN: Diesel NPK3 | `nitroplus/`, `muramasa-en/` | `muramasa.md`, `muramasa-en.md` |
| 続・殺戮のジャンゴ (active) | Nitroplus NSS (older) | `django/` | `django.md` |
| 最果てのイマ (finished, bugs only) | Xuse | `xuse/` | `saihate-no-ima.md` |
| 俺たちに翼はない, Vita (paused) | ? | `oretsuba/` | `oretsuba.md` |
| 月姫, Switch (paused) | TYPE-MOON | `tsukihime/` | `tsukihime.md` |

New game: `docs/new-game.md`.

Read first: the game's document ("Status": open state and next steps)
and `docs/lessons.md`; `engine/COMMANDS.md` lists every engine command.
They are long: `grep -n '^##' <file>`, then only the sections the task
touches.

Docs are public: `<game>` placeholders for local paths, reproducible
commands only. Keep "Status" current with every commit, open items
only; delete an item when it is done (git log is the record).
Machine-local facts go in your memory.

## Rules (from the user)

- **PRs only** (user, 2026-10-22): every change reaches `main` (and the
  engine's `master`) through a PR. Agents open the PR; the lead reviews
  it, then turns on auto-merge (`gh pr merge <n> --auto --squash`,
  engine PRs `--merge`) and GitHub merges once the required checks pass.
- **Branches**: one PR per goal phase, on the phase branch the brief
  names (from `origin/main`, then `git submodule update engine`); items
  are commits on it; the first agent on the branch opens its PR when
  its work is pushed. Small fixes
  ride in the session's `chores-<date>` PR (`docs/lead.md`, "Landing
  PRs"). Subjects under 72 chars; PRs are squash-merged. Commit
  messages and PR bodies carry no Co-Authored-By, Claude or session lines.
- **Engine changes**: a branch in `engine/` from `origin/master`, own PR
  (`gh pr create -R blankiekun/CONscripter --base master`), merged
  first with a merge commit; then point the submodule at `master` (CI
  `engine-on-master`).
- **Your branch is yours alone**: `git rebase origin/main` before
  pushing and before handing back (rebase, so `main` is never merged
  in); push with
  `git -c credential.helper= -c credential.helper='!gh auth git-credential' push --force-with-lease`.
  The global git config stays untouched.
- **PR body**: what and why, the engine PR link, the `tests/pr-report.sh`
  block (CI has no games: name every game whose `0.txt` changed, and
  why), how it was checked in the game.
- **Shared checkout**: stage paths by name (`git add <path>`); keep work
  in commits, since `git stash` is shared by all worktrees; stay on your
  own worktree's branch. Before editing a shared file read
  `docs/hot-spots.md`.
- **Existing NScripter commands first**, engine additions second. Engine
  additions and fixes apply to every script, no flag (2026-10-14); code
  in `engine/src/cons/` (preferably `ONScripter_cons.cpp`), works in the
  web build, listed in `engine/COMMANDS.md`, every other game still
  works; then drop the converter workaround.
- **Saves are the user's**: build into `out/<game>-cons`; the user's
  save folders (`out/saiima-flow`) are read-only to you. List a folder
  before deleting it.
- **Ask first** before anything big the user didn't ask for; they decide
  what is dropped.
- **Wine is the last resort** (2026-10-16): first the scripts, decompile
  (`research/tools/ghidra`), formats, and numbers (dry-run `0.txt`,
  traces, `compare -metric`); then the fewest frames, saying why. A
  few-frame check that a generated save or patched script reaches the
  moment may run at once (2026-10-18). Only via
  `research/tools/headless-run.sh` (hook `emu_run.py`): scratch prefix
  with real user folders, `LANG=ja_JP.UTF-8 LC_ALL=ja_JP.UTF-8`, reach
  the moment by patching scripts or saves; stop wine when done.
- **No agent past 120k context** (2026-10-23): at ~100k push the WIP,
  write HANDOFF.md in your worktree, report, stop (hook
  `context_cap.py`). Only a task planned over the cap before dispatch
  goes on.

## Testing (from the user, 2026-10-22)

Match verification to the size of the change:

- **Trivial** (copy, docs, styling, renames, config, comments, small UI
  tweaks): make the change; no tests, no CI check.
- **Logic in one area**: that module's tests only
  (`cargo test -p <crate> <filter>`).
- **Shared code, data handling, or anything risky**: the full suite,
  `cargo test` and `cargo clippy --all-targets`, plus `tests/snapshot.sh`
  (`--update` rewrites `tests/snapshots/`).

New tests only when the change adds real logic or the user asks. After
pushing, move on; watch CI only when the user asks. A failure unrelated
to your change: mention it in the report and leave it.

## Layout and build

IR: `crates/conv-core/src/ir/`; source readers and a game's own rules
(own module): `crates/conv-engines/src/<engine>/`; IR to `0.txt`, menus,
NSA: `crates/conv-cons/`; our engine code: `engine/src/cons/`;
throwaway Python: `research/<engine>/` (`local/` git-ignored).

`cargo build --release`. Game build commands: the game's document.
Engine, scratch runs, saves, traces: `docs/tools.md`. Releasing:
`docs/releasing.md`. Lead: `docs/lead.md`.

## Usage and gotchas

- **Read narrowly**: `0.txt`, traces, disassembly and full-size PNGs
  are queried, never dumped (`research/tools/view.py`); logs go to a
  file, then get queried. Helpers and "Numbers before pictures":
  `docs/tools.md`. Logic checks use `--dry-run`. Background runs: wait
  for the exit notification.
- **Stop at the end of the item asked for.** Reports: `key: value`
  lines, numbers first, at most 10; detail goes in the commit messages.
- ONScripter ignores SIGTERM in script loops: `timeout -k 5`. Kill by
  name with `pkill -x` (hook `bash_guards.py` refuses `-f`).
- Python text-mode writes turn CRLF into LF: `grep -c $'\r$'` before
  committing engine files.
