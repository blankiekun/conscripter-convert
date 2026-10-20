# CONscripter-convert: notes for Claude

Converts visual novels the user owns into games for CONscripter (our
ONScripter-Yuri fork, `engine/` submodule), especially its web build.

| Game | Engine | Branch prefix, worktree | Game document |
|---|---|---|---|
| 最果てのイマ COMPLETE (reopened: fixes, opcode audit) | Xuse | `xuse/`, `../conscripter-convert-xuse` | `docs/games/saihate-no-ima.md` |
| 装甲悪鬼村正 (in progress) | Nitroplus NSS | `nitroplus/`, `../conscripter-convert-nitroplus` | `docs/games/muramasa.md` |
| 俺たちに翼はない, PS Vita (starting) | not identified yet | `oretsuba/`, `../conscripter-convert-oretsuba` | `docs/games/oretsuba.md` |
| 続・殺戮のジャンゴ (starting) | Nitroplus NSS (older) | `django/`, `../conscripter-convert-django` | `docs/games/django.md` |
| Full Metal Daemon Muramasa, English (starting) | Nitroplus Diesel (NPK3, Squirrel) | `muramasa-en/`, `../conscripter-convert-muramasa-en` | `docs/games/muramasa-en.md` |
| 月姫 -A piece of blue glass moon-, Switch (in progress) | TYPE-MOON (mrgd00/MZP archives, as in the 魔法使いの夜 remake) | `tsukihime/`, `../conscripter-convert-tsukihime` | `docs/games/tsukihime.md` |

**New game?** Read `docs/new-game.md` first (lead: `docs/lead-new-game.md`;
new engine: `docs/new-engine.md`).

Read first: the game's document (its "Status" holds the feature state and
next steps) and `docs/lessons.md`; `engine/COMMANDS.md` lists every engine
command. They are long: `grep -n '^##' <file>`, read "Status" and what
the task touches.

Game documents are public: no local paths (use `<game>` placeholders),
only reproducible commands. Keep "Status" current with every commit; it
holds only the open state, finished write-ups move to
`docs/history/<game>.md`. Machine-local facts go in your memory, not the repo.

## Rules (from the user)

- Nobody pushes to `main` or the engine's `master` (protected). Every
  change is a PR that the lead or user reviews and merges; never merge
  your own.
- One PR per feature (2026-10-18), on a short-lived branch
  `<game>/<topic>` from `origin/main` (`git fetch && git switch -c
  <game>/<topic> origin/main && git submodule update engine`). Items
  that build on each other: one PR, a commit each. A game's `0.txt`
  change and an engine change each keep a PR of their own; a doc-only
  fix rides in the next related PR. Commit subjects under 72 chars; the
  PR is squash-merged, titled after the PR. No Co-Authored-By, Claude or
  session lines in commits or PR bodies.
- Engine changes: branch `<game>/<topic>` inside `engine/` from
  `origin/master`, own PR (`gh pr create -R blankiekun/CONscripter
  --base master`), merged first with a merge commit; then point the
  submodule at a commit on `master` (CI `engine-on-master`).
- The branch is yours alone: `git rebase origin/main` (never merge
  `main`) before pushing and again before handing back; push with
  `git -c credential.helper= -c credential.helper='!gh auth git-credential' push --force-with-lease`.
  Don't change global git config. The lead may land it with
  `tests/land.sh <pr>` (2026-10-18).
- PR body: what and why, the engine PR link, the `tests/pr-report.sh`
  block (CI has no games: name every game whose `0.txt` changed and
  why), how it was checked in the game. Update the game document's
  "Status" in the same PR.
- Stage your own paths by name, never `git add -A`. No `git stash`
  (the list is shared by all worktrees); save a patch or a scratch
  branch. Never switch branches in another agent's worktree.
- Use existing NScripter commands before adding to the engine. Engine
  additions are not opt-in (2026-10-14): a new behaviour applies to
  every script, no flag. Code lives in `engine/src/cons/` (preferably
  `ONScripter_cons.cpp`), works in the web build, is listed in the
  engine's `COMMANDS.md`, and must not break another game's build.
  Engine bugs are fixed for every script; then drop the converter
  workaround.
- Never touch the user's save folders (`out/saiima-flow`). Build into
  `out/<game>-cons`. List any folder before deleting it.
- Check with the user before anything big they haven't asked for. They
  decide what is dropped.
- Wine is the last resort (2026-10-16: slow, many tokens). First work
  it out from scripts, the decompile (`research/tools/ghidra`) and
  formats, then check ours with numbers (dry-run `0.txt`, emitted keys
  and times, trace records, `compare -metric`). Start wine only for a
  real doubt a frame can settle; say so in the report; film the fewest
  frames. Exception (2026-10-18): a quick few-frame check that a
  generated save or patched script reaches the moment may run at once.
  Allowed setup: scratch prefix with real (not symlinked) user folders,
  own Xvfb via `-displayfd` (never a fixed number, never the user's
  desktop), no audio, `LANG=ja_JP.UTF-8 LC_ALL=ja_JP.UTF-8`; reach a
  moment by patching scripts or saves (`docs/new-game.md` step 9;
  recipe in `docs/tools.md`). Stop wine and Xvfb when done.

## Working beside other agents

One agent per game, each in its own worktree; a lead hands out items and
reviews PRs. Start every item from a fresh `origin/main` and run `cargo
test`. Before editing a shared file read `docs/hot-spots.md` (hot spots,
shared refactors go in first as their own PR, game-specific code stays
in its own module). A lead: `docs/lead.md`.

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
- `tests/snapshot.sh` dry-runs every game present and says which `0.txt`
  changed; run it after any shared-code change (`--update` rewrites
  `tests/snapshots/`).
- Full build: `target/release/conscripter-convert convert-game <install
  dir> -o out/<game>-cons --font <Japanese TTF> --title "<title>"`;
  each game's exact command is in its document.
- Engine builds, scratch runs, saves, traces, mid-effect frames, reading
  a variable headless: `docs/tools.md`.

## Keeping usage down

- **Read narrowly**: `grep -n` first, then only the lines needed; never
  dump big files (`0.txt`, CG lists, disassembly, traces). Helpers
  (`mdsec.py`, `label.py`, `txtdiff.py`, `code.py`): `docs/tools.md`.
- **Trim output**: `tail -n 30` / `grep -E 'error|warning|FAIL'`; logs
  and traces to a file, then query them (`cons_trace_check.py --max 5`,
  `--diff`). Batch independent commands in one call.
- **Numbers before pictures**: never Read a full-size PNG
  (`research/tools/view.py`; `docs/tools.md`, "Numbers before pictures").
- **Logic checks use `--dry-run`**; code before wine ("Code before wine"
  in `docs/tools.md`). Reference traces: `research/tools/reftrace.sh`
  (`docs/new-game.md`, "Reference traces").
- No `/loop`, wakeups or polling unless asked; wait for background runs.
- **Stop at the end of the item asked for**; keep the game document's
  "Status" current (open state only; finished items move to its history).
- **Reports at most 10 lines**; detail goes in the PR body.
- **No agent past 100k context** (2026-10-19): at ~80k commit and push
  the WIP, write HANDOFF.md in your worktree, report, stop. Only a task planned over the cap
  before dispatch goes on.

## Gotchas

- `pkill -x conscripter`, not `-f` (it matches your own shell); the hook
  `.claude/hooks/bash_guards.py` refuses `-f`, session lines in commits
  and heavy builds on low memory.
- ONScripter ignores SIGTERM in script loops: use `timeout -k 5`.
- Python text-mode writes turn CRLF files into LF: check
  `grep -c $'\r$'` before committing engine files.
- `find /` and `locate` can be slow on network mounts: ask where the
  game is, or check memory.
- Releasing CONscripter: `docs/releasing.md`.
