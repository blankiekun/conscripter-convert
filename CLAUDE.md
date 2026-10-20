# CONscripter-convert: notes for Claude

Converts visual novels the user owns into games for CONscripter (our
ONScripter-Yuri fork, `engine/` submodule), especially its web build.

**Goal**: a game is done when its web build plays every route to its
ending (the route runs), saves and loads, its settings work, and it
shows no visible glitch in a one-hour play by the user; then a release.
Anything beyond that (pixel, frame or trace parity) needs the user's yes.

**WIP limit**: no new game, and paused games stay paused, until
Muramasa (JP+EN) and Django have shipped (the user, 2026-10-20).

| Game | Engine | State | Branch prefix | `docs/games/` |
|---|---|---|---|---|
| 装甲悪鬼村正 (JP+EN, one build) | NSS (JP), Diesel NPK3 (EN) | active | `nitroplus/`, `muramasa-en/` | `muramasa.md`, `muramasa-en.md` |
| 続・殺戮のジャンゴ | NSS (older) | active | `django/` | `django.md` |
| 最果てのイマ | Xuse | finished (bugs only) | `xuse/` | `saihate-no-ima.md` |
| 俺たちに翼はない (Vita) | ? | paused | `oretsuba/` | `oretsuba.md` |
| 月姫 (Switch) | TYPE-MOON | paused | `tsukihime/` | `tsukihime.md` |

Worktrees: `../conscripter-convert-<prefix without />`. New game:
`docs/new-game.md`.

Read first: the game's document ("Status") and `docs/lessons.md`;
`engine/COMMANDS.md` lists every engine command.

Docs are public (no local paths: `<game>` placeholders) and say what
is, not how it came to be. Keep the game's "Status" current with every
commit, open items only (delete an item when done). Machine-local facts
go in your memory.

## Rules (from the user)

A new rule comes with a hook or test when it can be checked; otherwise
it replaces an old one (hooks: `.claude/hooks/`; CI: `check.yml`).

- Nobody pushes to `main` or the engine's `master`. Every change is a
  PR the lead or user merges; never merge your own. PRs are
  squash-merged; the lead may land with `tests/land.sh <pr>`.
- One PR per feature, on a branch `<game>/<topic>` from a fresh
  `origin/main` (then `git submodule update engine`; `cargo test`).
  Dependent items: one PR, a commit each. A game's `0.txt` change and
  an engine change each get their own PR; a doc-only fix rides in the
  next related PR.
  Subjects under 72 chars. No Co-Authored-By, Claude or session lines
  (hook and CI).
- Engine: branch `<game>/<topic>` in `engine/` from `origin/master`, PR
  with `gh pr create -R blankiekun/CONscripter --base master`, merged
  first with a merge commit; then point the submodule at `master` (CI
  `engine-on-master`).
- The branch is yours alone: `git rebase origin/main` (never merge
  `main`) before pushing and before handing back; push with
  `git -c credential.helper= -c credential.helper='!gh auth git-credential' push --force-with-lease`
  (never edit the global git config).
- PR body: what and why, the engine PR link, the `tests/pr-report.sh`
  block (name every game whose `0.txt` changed, and why), how it was
  checked in the game.
- Stage paths by name, never `git add -A`. No `git stash` (shared by all
  worktrees). Never switch branches in another agent's worktree. Shared
  files: read `docs/hot-spots.md` first.
- Existing NScripter commands before engine additions. Engine additions
  and fixes apply to every script, no flag; code in `engine/src/cons/`,
  works on the web, listed in `engine/COMMANDS.md`, breaks no other
  game; then drop the converter workaround.
- Never touch the user's save folders (`out/saiima-flow`). Build into
  `out/<game>-cons`. List a folder before deleting it.
- Ask the user before anything big they didn't ask for; they decide
  what is dropped.
- Wine is the last resort: first the scripts, decompile, formats and
  numbers; then the fewest frames, saying why. A few-frame check that a
  save or patched script reaches the moment may run at once. Setup:
  `docs/tools.md` (hook `emu_run.py`).
- Context cap 100k unless planned over it: at ~80k push the WIP, write
  HANDOFF.md, report, stop (hook `context_cap.py`). Lead: `docs/lead.md`.

## Layout and build

IR: `crates/conv-core/src/ir/`; source readers and a game's own rules:
`crates/conv-engines/src/<engine>/`; IR to `0.txt`, menus, NSA:
`crates/conv-cons/`; our engine code: `engine/src/cons/`; throwaway
Python: `research/<engine>/` (`local/` is git-ignored).

`cargo test`, `cargo clippy --all-targets`, `cargo build --release`;
`tests/snapshot.sh` after any shared-code change (`--update` rewrites
`tests/snapshots/`). Game build commands: the game's document. Engine,
scratch runs, saves, traces: `docs/tools.md`. Releasing:
`docs/releasing.md`.

## Keeping usage down, gotchas

- Read narrowly (`grep -n '^##'`, then the section; helpers:
  `docs/tools.md`); never dump `0.txt`, traces or disassembly; logs to a
  file. Logic checks: `--dry-run`. No `/loop` or polling. Stop at the
  end of the item; reports <= 10 lines.
- ONScripter ignores SIGTERM in script loops: `timeout -k 5`.
- Python text-mode writes turn CRLF into LF: `grep -c $'\r$'` before
  committing engine files.
