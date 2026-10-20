# CONscripter-convert: notes for Claude

Converts visual novels the user owns into games for CONscripter (our
ONScripter-Yuri fork, `engine/` submodule), especially its web build.

**Goal**: a game is done when it plays through in the web build with no
blocking or visible bug, then a release. Pixel, frame or trace parity
work only for a bug someone saw while playing.

**WIP limit**: no new game, and paused games stay paused, until
Muramasa (JP+EN) and Django have shipped (the user, 2026-10-20).

| Game | Engine | State | Branch prefix | Game document |
|---|---|---|---|---|
| 装甲悪鬼村正 / Full Metal Daemon Muramasa (JP+EN, one build) | Nitroplus NSS (JP), Diesel NPK3 (EN) | active | `nitroplus/`, `muramasa-en/` | `docs/games/muramasa.md`, `docs/games/muramasa-en.md` |
| 続・殺戮のジャンゴ | Nitroplus NSS (older) | active | `django/` | `docs/games/django.md` |
| 最果てのイマ COMPLETE | Xuse | finished (bugs only) | `xuse/` | `docs/games/saihate-no-ima.md` |
| 俺たちに翼はない, PS Vita | not identified yet | paused | `oretsuba/` | `docs/games/oretsuba.md` |
| 月姫 -A piece of blue glass moon-, Switch | TYPE-MOON (mrgd00/MZP) | paused | `tsukihime/` | `docs/games/tsukihime.md` |

Worktrees: `../conscripter-convert-<prefix without />`. **New game?**
`docs/new-game.md`.

Read first: the game's document ("Status": open state and next steps)
and `docs/lessons.md`; `engine/COMMANDS.md` lists every engine command.
List headings (`grep -n '^##' <file>`) and read only what the task
touches.

Docs are public: no local paths (`<game>` placeholders), only
reproducible commands, no session lines. They say what is, not how it
came to be. Keep the game's "Status" current with every commit; it holds
only open items: delete an item when it is done (git log and PR bodies
are the record). Machine-local facts go in your memory.

## Rules (from the user)

- Nobody pushes to `main` or the engine's `master`. Every change is a
  PR the lead or user merges; never merge your own.
- One PR per feature, on a short-lived branch `<game>/<topic>` from
  `origin/main` (`git fetch && git switch -c <game>/<topic> origin/main
  && git submodule update engine`). Dependent items: one PR, a commit
  each. A game's `0.txt` change and an engine change each get their own
  PR; a doc-only fix rides in the next related PR. Subjects under 72
  chars; PRs are squash-merged. No Co-Authored-By, Claude or session
  lines in commits or PR bodies.
- Engine: branch `<game>/<topic>` in `engine/` from `origin/master`,
  PR with `gh pr create -R blankiekun/CONscripter --base master`, merged
  first with a merge commit; then point the submodule at `master` (CI
  `engine-on-master`).
- The branch is yours alone: `git rebase origin/main` (never merge
  `main`) before pushing and before handing back; push with
  `git -c credential.helper= -c credential.helper='!gh auth git-credential' push --force-with-lease`.
  Don't change global git config. The lead may land with
  `tests/land.sh <pr>`.
- PR body: what and why, the engine PR link, the `tests/pr-report.sh`
  block (CI has no games: name every game whose `0.txt` changed and
  why), how it was checked in the game; the game's "Status" updated.
- Stage paths by name, never `git add -A`. No `git stash` (shared by all
  worktrees). Never switch branches in another agent's worktree.
- Existing NScripter commands before engine additions. Engine additions
  are not opt-in: new behaviour applies to every script, no flag. Code in
  `engine/src/cons/` (preferably `ONScripter_cons.cpp`), works on the
  web, listed in `engine/COMMANDS.md`, breaks no other game. Fix engine
  bugs for every script, then drop the converter workaround.
- Never touch the user's save folders (`out/saiima-flow`). Build into
  `out/<game>-cons`. List a folder before deleting it.
- Ask the user before anything big they didn't ask for; they decide
  what is dropped.
- Wine is the last resort (slow, many tokens): first the scripts, the
  decompile (`research/tools/ghidra`), the formats, then numbers (dry
  run `0.txt`, trace records, `compare -metric`). Wine only for a real
  doubt a frame settles; say so; film the fewest frames. A quick
  few-frame check that a generated save or patched script reaches the
  moment may run at once. Setup (`docs/tools.md`): scratch prefix with
  real user folders, own Xvfb via `-displayfd`, never the user's
  desktop, no audio, `LANG=ja_JP.UTF-8 LC_ALL=ja_JP.UTF-8`; stop wine
  and Xvfb when done.
- No agent past 100k context: at ~80k commit and push the WIP, write
  HANDOFF.md in your worktree, report, stop (unless the task was planned
  over the cap before dispatch).

## Working beside other agents

One agent per game, each in its own worktree; a lead hands out items and
lands PRs (`docs/lead.md`). Start every item from a fresh `origin/main`
and run `cargo test`. Before editing a shared file read
`docs/hot-spots.md`.

## Layout

| Path | Contents |
|---|---|
| `crates/conv-core/src/ir/` | Engine-neutral IR; `alloc.rs`: conv-cons variables |
| `crates/conv-engines/src/<engine>/` | Source readers and compilers; a game's own rules in a module there |
| `crates/conv-cons/` | IR to `0.txt`, menus, extras, NSA writer |
| `engine/` | CONscripter (C++; ours in `src/cons/`) |
| `research/<engine>/` | Throwaway Python; `local/` is git-ignored |

## Build and test

- `cargo test`, `cargo clippy --all-targets`, `cargo build --release`.
- `tests/snapshot.sh` after any shared-code change (`--update` rewrites
  `tests/snapshots/`).
- Each game's full build command is in its document. Engine builds,
  scratch runs, saves, traces: `docs/tools.md`.

## Keeping usage down

- Read narrowly (`grep -n`, then the lines needed; helpers in
  `docs/tools.md`). Never dump `0.txt`, traces or disassembly; never
  Read a full-size PNG (`research/tools/view.py`).
- Trim output (`tail -n 30`, `grep -E 'error|FAIL'`); logs to a file.
  Batch independent commands.
- Logic checks use `--dry-run`. No `/loop`, wakeups or polling.
- Stop at the end of the item. Reports at most 10 lines.

## Gotchas

- `pkill -x conscripter`, never `-f` (`.claude/hooks/bash_guards.py`
  refuses it, session lines in commits, and heavy builds on low memory).
- ONScripter ignores SIGTERM in script loops: `timeout -k 5`.
- Python text-mode writes turn CRLF into LF: `grep -c $'\r$'` before
  committing engine files.
- `find /` and `locate` are slow on network mounts: ask, or check memory.
- Releasing CONscripter: `docs/releasing.md`.
