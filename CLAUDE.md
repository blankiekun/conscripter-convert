# CONscripter-convert: notes for Claude

Converts visual novels the user owns into games for CONscripter (our
ONScripter-Yuri fork, the `engine/` submodule), especially its web
build.

| Game | Engine | Branch prefix, worktree | Game document |
|---|---|---|---|
| 最果てのイマ COMPLETE (reopened: fixes, opcode audit) | Xuse | `xuse/`, `../conscripter-convert-xuse` | `docs/games/saihate-no-ima.md` |
| 装甲悪鬼村正 (in progress) | Nitroplus NSS | `nitroplus/`, `../conscripter-convert-nitroplus` | `docs/games/muramasa.md` |
| 俺たちに翼はない, PS Vita (starting) | not identified yet | `oretsuba/`, `../conscripter-convert-oretsuba` | `docs/games/oretsuba.md` |
| 続・殺戮のジャンゴ (starting) | Nitroplus NSS (older) | `django/`, `../conscripter-convert-django` | `docs/games/django.md` |
| Full Metal Daemon Muramasa, English (starting) | Nitroplus Diesel (NPK3, Squirrel) | `muramasa-en/`, `../conscripter-convert-muramasa-en` | `docs/games/muramasa-en.md` |
| 月姫 -A piece of blue glass moon-, Switch (in progress) | TYPE-MOON (mrgd00/MZP archives, as in the 魔法使いの夜 remake) | `tsukihime/`, `../conscripter-convert-tsukihime` | `docs/games/tsukihime.md` |

**Starting a new game? Read `docs/new-game.md` first** (a lead: `docs/lead-new-game.md`) (and
`docs/new-engine.md` for a new engine).

Read first, in order: the game's document (its "Status" section holds
the feature state and the next steps) and `docs/lessons.md` (general
lessons); `engine/COMMANDS.md` is the reference for every engine
command. These are long: list the headings
(`grep -n '^##' <file>`), read the "Status" section and what the task
touches, and look up the rest by heading when you need it.

Each game's state is tracked in its `docs/games/<game>.md`, which is
public: no local paths (use placeholders such as `<game>`), only
reproducible commands. There are no git-ignored handoff files; keep
the document's "Status" current with every commit that changes it.
"Status" holds only the open state (what works, what is left, next
steps): when an item is done, move its write-up to the "History"
section after it, so a session reads Status alone.
Machine-local facts (where the user keeps a game) go in your memory,
not in the repo.

## Rules (from the user)

- Nobody pushes to `main` (here) or `master` (engine): both are
  protected, and every change goes in through a pull request that the
  team lead (or the user) reviews and merges. Don't merge your own PRs.
- One PR per feature (the user, 2026-10-18), on a short-lived branch
  `<game>/<topic>` made from `origin/main` (`git fetch && git switch -c
  <game>/<topic> origin/main`). Items that build on each other go in one
  PR, a commit each, not a stack of PRs. A change to any game's `0.txt`
  and an engine change keep a PR of their own; a doc-only lint or Status
  fix rides in the next related PR. Keep commit subjects short (under 72
  characters; detail goes in the body); the PR is squash-merged into one
  commit, titled after the PR.
- Engine changes: inside `engine/`, a branch `<game>/<topic>` from
  `origin/master`, with its own PR to CONscripter `master`
  (`gh pr create -R blankiekun/CONscripter --base master`). Point the
  submodule at your branch's commit while you work. The engine PR is
  merged first, with a merge commit (so its SHAs stay valid); then
  point the submodule at a commit on `master`: CI's `engine-on-master`
  check fails until you do.
- The branch is yours alone: before pushing, `git rebase origin/main`
  (never merge `main` into it) and push with
  `git -c credential.helper= -c credential.helper='!gh auth git-credential' push --force-with-lease`
  (git has no credential helper; `gh` is logged in). Don't change the
  global git config.
  After the agent reports, the lead may rebase and land the PR itself
  with `tests/land.sh <pr>` (user, 2026-10-18).
- The PR body says what changed and why, links the engine PR, and gives
  the `tests/pr-report.sh` block (tests, clippy, snapshots, lints; CI has no
  games: name every game whose `0.txt` changed and why) and how it was
  checked in the game. Update the game document's "Status" in the same
  PR.
- Stage your own paths by name, never `git add -A`: other agents leave
  uncommitted files in the tree.
- No Co-Authored-By or Claude attribution lines in commits.
- Use existing NScripter commands before adding to the engine.
  Engine additions are not opt-in (the user, 2026-10-14): a new
  behaviour applies to every script, with no command or flag to turn it
  on; new commands take effect where a script uses them. They live in
  `engine/src/cons/` (preferably `ONScripter_cons.cpp`), work in the
  web build, and are listed in the engine's `COMMANDS.md` (the
  reference for every script command). Check that a change of behaviour doesn't break another game's
  build. Bug fixes likewise: when the engine does something wrong (text
  misplaced, characters dropped), fix it for every script. Then drop any converter workaround for
  it.
- Never touch the user's save folders (`out/saiima-flow`). Build into
  `out/<game>-cons`. List any folder before deleting it.
- Check with the user before starting anything big that they haven't
  asked for. They decide what is dropped.
- Running the original games under wine is allowed: a scratch prefix
  with real (not symlinked) user folders, its own Xvfb display, no
  audio, a Japanese locale (`LANG=ja_JP.UTF-8 LC_ALL=ja_JP.UTF-8`, for
  every game), never the user's desktop. Reach a moment by patching
  scripts or saves before clicking through (`docs/new-game.md` step 9).
  Xvfb with `-displayfd` (a free display), never a fixed number
  (recipe: `docs/tools.md`). Stop wine and Xvfb when done.
- Wine is the last resort (the user, 2026-10-16: it takes long and costs
  many tokens). Work out what the original does from its code first:
  the scripts, the decompile (`research/tools/ghidra`), the formats.
  Then check ours with numbers (the dry run's `0.txt`, the keys and
  times it emits, trace records, `compare -metric`). Start wine only
  when that leaves a real doubt that a frame can settle, say so in your
  report, and film the fewest frames that answer it.
  Exception (the user, 2026-10-18): a quick check that a generated save
  (or patched script) takes straight to the moment may run at once,
  without waiting for a filming session; keep it to a few frames and
  mind the tokens it costs.

## Working beside other agents

Several agents work at once, one per game, each in its own worktree
(`../conscripter-convert-<game>`, table above), and their work meets in
shared code. A team lead hands out the items and reviews the PRs.

- **Start every item from a fresh `origin/main`** (`git fetch && git
  switch -c <game>/<topic> origin/main && git submodule update engine`),
  then build and run `cargo test`. Rebase onto `origin/main` again before
  pushing a change to a hot spot (below), and again just before handing
  the PR back, so it is mergeable when the lead reads the report.
  Conflicts are small when they are fresh. Never switch branches in
  another agent's worktree.
- **No `git stash`**: the stash list is shared by every worktree, so a
  `pop` can apply another agent's stash into your tree. To set work
  aside, commit it on a scratch branch or save a patch
  (`git diff > <scratchpad>/x.patch`).
- **Hot spots**: files every game's work changes (IR, conv-cons, menus,
  switches, the CLI, the engine pointer, shared docs, Cargo files). Before
  editing a shared file, read `docs/hot-spots.md`.
- **Shared refactors go in first**: when you change the shape of
  shared code (a helper's signature, a function split or moved, an IR
  type renamed, numbers renumbered), open that change as its own PR and
  ask for it to be merged before you build your feature on it. Other
  branches then write against the new shape.
- **Game-specific code stays out of shared files**: an engine's reader
  and compiler live in `crates/conv-engines/src/<engine>/`, and a single
  game's own rules go in a module of their own there (for example
  `nitroplus/convert/muramasa.rs`, chosen by the CLI when it builds).

## Layout

| Path | Contents |
|---|---|
| `crates/conv-core/src/ir/` | Engine-neutral IR; `alloc.rs`: the variables conv-cons keeps |
| `crates/conv-engines/src/<engine>/` | Source readers (`xuse/`, `nitroplus/`); a game's own rules in a module there (`nitroplus/convert/muramasa.rs`) |
| `crates/conv-cons/` | IR → `0.txt` (`lib.rs`), menus, extras, NSA writer |
| `engine/` | CONscripter (C++; our code in `src/cons/ONScripter_cons.cpp`, the debug trace in `ONScripter_trace.cpp`) |
| `research/<engine>/` | Throwaway Python; `local/` is git-ignored (disassembly, wine scripts, shots) |

## Build and test

- `cargo test`, `cargo clippy --all-targets`; `cargo build --release`.
- `tests/snapshot.sh`: dry-runs every game present and says which games'
  `0.txt` changed; run it after any change to shared code (`--update`
  rewrites `tests/snapshots/`). More: `docs/tools.md` ("Build and test
  details").
- Engine build and headless runs: `docs/tools.md` ("Build and test details").
- Full build: `target/release/conscripter-convert convert-game <install
  dir> -o out/<game>-cons --font <a Japanese TTF, e.g. ipag.ttf> --title
  "<title>"`. Each game's exact command is in its document.
- Scratch runs, A/B frames, saves at chosen scenes, the debug trace,
  frames mid-effect, test scripts, reading a variable headless:
  `docs/tools.md` (read the bullet you need).

## Keeping usage down

Every byte read or printed stays in the context and is paid for again
on each later turn; several agents run at once.

- **Read narrowly.** `grep -n` first, then read only the lines you need
  (`sed -n a,bp`, Read with offset/limit). Don't dump whole big files
  (generated `0.txt`, CG lists, disassembly, traces). Helpers for the files
  read most often (`mdsec.py`, `label.py`, `txtdiff.py`, `code.py`):
  `docs/tools.md`; `code.py` instead of paging source with `sed`.
- **Trim command output.** Pipe builds and long runs through
  `tail -n 30` or `grep -E 'error|warning|FAIL'`; write traces and logs
  to a file and query them (`cons_trace_check.py`, `grep`, `jq`).
  Show the checker few findings per kind (`--max 5`) and, after a fix,
  only what changed (`--diff before.jsonl after.jsonl`).
- **Numbers before pictures.** Compare frames by numbers; never Read a
  full-size PNG (use `research/tools/view.py`). Details: `docs/tools.md`
  ("Numbers before pictures").
- **Logic checks use the dry run** (`--dry-run`, script only, seconds);
  the full build re-renders every asset.
- **Code before wine**: the decompile and the dry run answer most
  questions (Rules, "Wine is the last resort"). Shots, `shots.py` and the
  film list: `docs/tools.md` ("Code before wine").
- **Reference traces of the originals**: `research/tools/reftrace.sh <game>` starts one, `reftrace.sh status <game>` after ~5 min and every 30-60 min, `summary <game>` at the end (docs/new-game.md, "Reference traces"); don't poll or hand-run the wrappers.
- **Batch** independent commands into one call; keep replies short, no
  progress narration between steps.
- **Stop at the end of the item asked for** (commit, push, update the
  game document's "Status") instead of scoping ahead; the Status and
  memory carry the state into a fresh session.
- No `/loop`, scheduled wakeups or polling unless the user asks; wait
  for background runs to notify.
- **Delegation** (team lead only): `docs/lead.md`.
- **Reports at most 10 lines**; the detail goes in the PR body. Every
  line of a report stays in the lead's context.
- **No agent past 100k context** (the user, 2026-10-19): at ~80k commit
  and push the WIP, write HANDOFF.md in your worktree, report, stop; the
  lead starts a fresh agent from it. `.claude/hooks/context_cap.py` says
  so after every tool call. Only a task planned over the cap before
  dispatch (its prompt says so) goes on.

## Releasing CONscripter

Steps in `docs/releasing.md`; read it only when releasing.

## Gotchas

- `pkill -f`/`pgrep -f` match your own shell when the pattern is in the
  command line: use `pkill -x conscripter`.
  The hook `.claude/hooks/bash_guards.py` refuses `-f`, session lines in
  commits and heavy builds on low memory.
- ONScripter ignores SIGTERM in script loops: always `timeout -k 5`.
- Python text-mode writes turn CRLF files into LF: check
  `grep -c $'\r$'` before committing engine files.
- `find /` and `locate` can be slow on machines with big network
  mounts: ask where the game is, or check your memory, first.
