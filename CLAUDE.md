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
They are long: `grep -n '^##' <file>`, then only what the task touches.

Docs are public: no local paths (`<game>` placeholders), only
reproducible commands. Keep "Status" current with every commit, open
items only; delete an item when done (git log is the record).
Machine-local facts go in your memory.

## Rules (from the user)

- Nobody pushes to `main` or the engine's `master`. Every change is a
  PR the lead or user merges; never merge your own.
- One PR per goal phase, on the phase branch the brief names (from
  `origin/main`, then `git submodule update engine`); items are commits
  on it; open a PR only if the brief says so. Small fixes ride in the
  session's `chores-<date>` PR (`docs/lead.md`, "Landing PRs").
  Subjects under 72 chars; PRs are squash-merged. No Co-Authored-By,
  Claude or session lines.
- Engine changes: a branch in `engine/` from `origin/master`, own PR
  (`gh pr create -R blankiekun/CONscripter --base master`), merged
  first with a merge commit; then point the submodule at `master` (CI
  `engine-on-master`).
- The branch is yours alone: `git rebase origin/main` (never merge
  `main`) before pushing and before handing back; push with
  `git -c credential.helper= -c credential.helper='!gh auth git-credential' push --force-with-lease`.
  Never edit the global git config.
- PR body: what and why, the engine PR link, the `tests/pr-report.sh`
  block (CI has no games: name every game whose `0.txt` changed, and
  why), how it was checked in the game.
- Stage paths by name, never `git add -A`. No `git stash` (shared by
  all worktrees). Never switch branches in another agent's worktree.
  Before editing a shared file read `docs/hot-spots.md`.
- Existing NScripter commands before engine additions. Engine additions
  and fixes apply to every script, no flag (2026-10-14); code in
  `engine/src/cons/` (preferably `ONScripter_cons.cpp`), works in the
  web build, listed in `engine/COMMANDS.md`, breaks no other game; then
  drop the converter workaround.
- Never touch the user's save folders (`out/saiima-flow`). Build into
  `out/<game>-cons`. List a folder before deleting it.
- Ask the user before anything big they didn't ask for; they decide
  what is dropped.
- Wine is the last resort (2026-10-16): first the scripts, decompile
  (`research/tools/ghidra`), formats, and numbers (dry-run `0.txt`,
  traces, `compare -metric`); then the fewest frames, saying why. A
  few-frame check that a generated save or patched script reaches the
  moment may run at once (2026-10-18). Only via
  `research/tools/headless-run.sh` (hook `emu_run.py`): scratch prefix
  with real user folders, `LANG=ja_JP.UTF-8 LC_ALL=ja_JP.UTF-8`, reach
  the moment by patching scripts or saves; stop wine when done.
- No agent past 100k context (2026-10-19): at ~80k push the WIP, write
  HANDOFF.md in your worktree, report, stop (hook `context_cap.py`).
  Only a task planned over the cap before dispatch goes on.

## Layout and build

IR: `crates/conv-core/src/ir/`; source readers and a game's own rules
(own module): `crates/conv-engines/src/<engine>/`; IR to `0.txt`, menus,
NSA: `crates/conv-cons/`; our engine code: `engine/src/cons/`;
throwaway Python: `research/<engine>/` (`local/` git-ignored).

`cargo test`, `cargo clippy --all-targets`, `cargo build --release`;
`tests/snapshot.sh` after any shared-code change (`--update` rewrites
`tests/snapshots/`). Game build commands: the game's document. Engine,
scratch runs, saves, traces: `docs/tools.md`. Releasing:
`docs/releasing.md`. Lead: `docs/lead.md`.

## Usage and gotchas

- Read narrowly; never dump `0.txt`, traces, disassembly or full-size
  PNGs (`research/tools/view.py`); logs to a file, then query them.
  Helpers and "Numbers before pictures": `docs/tools.md`. Logic checks
  use `--dry-run`. No `/loop` or polling; wait for background runs.
- Stop at the end of the item asked for. Reports: `key: value` lines,
  numbers first, at most 10; detail goes in the commit messages.
- ONScripter ignores SIGTERM in script loops: `timeout -k 5`. Use
  `pkill -x`, never `-f` (hook `bash_guards.py`).
- Python text-mode writes turn CRLF into LF: `grep -c $'\r$'` before
  committing engine files.
