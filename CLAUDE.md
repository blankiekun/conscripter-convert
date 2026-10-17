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

**Starting a new game? Read `docs/new-game.md` first** (and
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
- One PR per task item, on a short-lived branch `<game>/<topic>` made
  from `origin/main` (`git fetch && git switch -c <game>/<topic>
  origin/main`). Keep it to a few commits with short subjects (under 72
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
- The PR body says what changed and why, links the engine PR, and gives
  the `cargo test`/`clippy` and `tests/snapshot.sh` results (CI has no
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
  Stop wine and Xvfb when done.
- Wine is the last resort (the user, 2026-10-16: it takes long and costs
  many tokens). Work out what the original does from its code first:
  the scripts, the decompile (`research/tools/ghidra`), the formats.
  Then check ours with numbers (the dry run's `0.txt`, the keys and
  times it emits, trace records, `compare -metric`). Start wine only
  when that leaves a real doubt that a frame can settle, say so in your
  report, and film the fewest frames that answer it.

## Working beside other agents

Several agents work at once, one per game, each in its own worktree
(`../conscripter-convert-<game>`, table above), and their work meets in
shared code. A team lead hands out the items and reviews the PRs.

- **Start every item from a fresh `origin/main`** (`git fetch && git
  switch -c <game>/<topic> origin/main && git submodule update engine`),
  then build and run `cargo test`. Rebase onto `origin/main` again before
  pushing a change to a hot spot (below). Conflicts are small when they
  are fresh. Never switch branches in another agent's worktree.
- **No `git stash`**: the stash list is shared by every worktree, so a
  `pop` can apply another agent's stash into your tree. To set work
  aside, commit it on a scratch branch or save a patch
  (`git diff > <scratchpad>/x.patch`).
- **Hot spots**: files every game's work changes. Before editing one,
  check what main changed in it (`git log -p HEAD..origin/main -- <file>`).
  Keep each edit small and in its own place:

  | File | How to edit it |
  |---|---|
  | `crates/conv-core/src/ir/` (`mod.rs`: the ops and `Script`; `screens.rs`: the shared screens; `picture.rs`: the picture-drawn ones) | Add new types and `Op` variants at the end of their block, with a doc comment; don't reorder or reformat existing ones |
  | `crates/conv-cons/src/lib.rs`, `ops.rs` | Put a new feature's emitter in its own module (`puzzle.rs`, `replay.rs`) and add one `match` arm in `ops.rs` (`Emitter::op`). Sprite ranges are the constants at the top of `lib.rs`; a new variable range goes into `conv-core/src/alloc.rs` first (a test checks every `%N` conv-cons names is there, and the build refuses a game variable inside one) |
  | `crates/conv-cons/src/menu/` (one file per screen; the shared constants in `mod.rs`) | Shared by every game's menus: change it only through what a game passes in its IR (`SaveMenu`, `ButtonBar`, …), not with game-specific branches |
  | `crates/conv-cons/src/switches.rs` | A new settings switch is one entry in `def` (its index and what applying it does), with the next free index; nothing in `menu/` |
  | `crates/conv-cli/src/main.rs` | One subcommand per engine; keep the engine's own code in `crates/conv-engines/src/<engine>/` |
  | `engine` (submodule pointer) | Merge the engine PR first, then point the submodule at a commit on CONscripter `master` that contains every engine commit `origin/main` points at (rebase onto `origin/main` first, so the pointer only moves forward) |
  | `README.md`, `docs/new-game.md`, `docs/lessons.md`, `docs/design.md`, this file | Edit only your game's rows and paragraphs; general lessons go in `docs/lessons.md` |
  | `Cargo.toml`, `Cargo.lock` | Add dependencies in their own commit; on a conflict, take both sides and let `cargo build` rewrite the lock file |

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
| `crates/conv-cli/` | The `conscripter-convert` command |
| `engine/` | CONscripter (C++; our code in `src/cons/ONScripter_cons.cpp`, the debug trace in `ONScripter_trace.cpp`) |
| `research/<engine>/` | Throwaway Python; `local/` is git-ignored (disassembly, wine scripts, shots) |
| `docs/new-game.md` | Playbook for converting a new game |
| `docs/games/<game>.md` | Per-game record: formats, opcodes, progression variables, status |

## Build and test

- `cargo test`, `cargo clippy --all-targets`; `cargo build --release`.
- `tests/snapshot.sh`: dry-runs every game present on the machine (links
  in `$CONS_GAMES`, see the script) and says which games' `0.txt`
  changed. Run it after any change to shared code; a change to another
  game's script must be deliberate and said in the commit
  (`--update` then rewrites `tests/snapshots/`). `--diff origin/main`
  builds that ref in its own worktree and shows each game's 0.txt
  changes; `--map <rules>` checks a change is only a rename (below).
  On a conflict in `tests/snapshots/` never take one side: rebase, rerun
  `--update`, and explain each changed game's 0.txt in the PR body.
- Full build: `target/release/conscripter-convert convert-game <install
  dir> -o out/<game>-cons --font <a Japanese TTF, e.g. ipag.ttf> --title
  "<title>"`. Each game's exact command is in its document.
- Engine: `research/tools/build-engine.sh [engine dir] [build dir]`
  (cmake + make; `SDL2_MIXER_PREFIX=<prefix>` when the system has no
  SDL2_mixer) → `engine/build/conscripter`.
  Web: `cd engine/script && EMSDK_HOME=<emsdk dir> bash ./cross_web.sh`.
- Headless: `SDL_VIDEODRIVER=dummy SDL_AUDIODRIVER=dummy timeout -k 5 N
  engine/build/conscripter -r .` with the game folder as the working
  directory (otherwise `default.ttf` isn't found). Screenshot with
  `print 1:getscreenshot 800,600:savescreenshot "x.bmp"`.
- Scratch run beside a full build (yours or another agent's):
  `research/tools/scratch.sh out/<game>-cons <scratch> [0.txt [engine]]`
  links only the archives, `default.ttf` and `movie/`, copies in your
  script and a renamed engine, and writes `<scratch>/run.sh [options]`
  (headless, `--save-dir <scratch>/save`).
- A/B frames and saves: `research/tools/scene_ab.py frames|saves`
  plays a scene under two engines (or with one line dropped) in such
  scratch folders and prints `compare -metric AE` per shot; `saves`
  saves with engine A and loads with B (every save-version PR).
- Debug trace (CONscripter after 0.18.0; reference: `engine/DEBUG.md`;
  recipes: `docs/new-game.md`, "Checking a build with the debug
  trace"): `--trace t.jsonl --auto-play 0 --save-dir <scratch>` plays
  the game by itself, headless with `SDL_VIDEODRIVER=dummy` (no Xvfb),
  and writes one JSON line per page, per command that changed the
  screen or sound, per choice, missing file, script error and crash. `--trace-ignore` leaves the game's own UI
  sprites out and keeps auto-play off their buttons. Check a trace with
  `python3 -I engine/tools/cons_trace_check.py t.jsonl --script 0.txt`.
  Start anywhere with `--goto <label> --vars <file>` instead of patching
  `0.txt`. Exit codes: 0 end, 1 auto-play stuck, 2 script error, 3
  startup failure, 128+n a signal.
- Frames mid-effect: the trace's `--shot-effects 25,50,75`. The older
  patched-script runs: `docs/new-game.md` step 9 and "When the trace
  can't do it". On Xvfb always set
  `WAYLAND_DISPLAY=`, or the window opens on the user's desktop.
- Test scripts: encode with `iconv -t cp932` (not `sjis`, which turns `\`
  into `＼`).
- Read a variable headless: `lsp 999,"X:"+$1,0,0` prints "can't find
  file X:..." on stderr.

## Keeping usage down

Every byte read or printed stays in the context and is paid for again
on each later turn; several agents run at once.

- **Read narrowly.** `grep -n` first, then read only the lines you need
  (`sed -n a,bp`, Read with offset/limit). Don't dump whole big files
  (generated `0.txt`, CG lists, disassembly, traces). For the docs and
  scripts read most often, use the helpers in `research/tools/`
  (run with `python3 -I`):
  `mdsec.py <file.md>` lists the headings, `mdsec.py <file.md> Status`
  prints that section only (game docs, `docs/lessons.md`, this file);
  `label.py 0.txt <label> [lines]` prints one label's block with line
  numbers, `label.py 0.txt -l <text>` finds labels; `txtdiff.py old.txt
  new.txt [--map 's/old/new/'|--rules <file>]` prints changed hunks under
  their labels, counts them by command (numbers masked) and says whether
  the rules make it only a rename (`--self-test` tests it). Source code is the
  biggest text cost: `code.py <file.rs|.cpp>` lists its items with line
  numbers, `code.py <file> <name> [lines]` prints one fn, type, impl or
  C++ `Class::name` with its doc comments; use it instead of paging
  through a file with `sed`.
- **Trim command output.** Pipe builds and long runs through
  `tail -n 30` or `grep -E 'error|warning|FAIL'`; write traces and logs
  to a file and query them (`cons_trace_check.py`, `grep`, `jq`).
  Show the checker few findings per kind (`--max 5`) and, after a fix,
  only what changed (`--diff before.jsonl after.jsonl`).
- **Numbers before pictures.** Compare frames with
  `compare -metric AE -fuzz 0 a.png b.png null:` (pixel count) or
  `-metric RMSE`, or crop to the region in question; view an image only
  when the numbers can't tell what is wrong. Where the expected pixels
  are known exactly (a source picture against the drawn sprite, a colour
  from the exe) use no fuzz or read the pixel values: 2 % hid sprites
  drawn 1 % dark. Keep a small fuzz only for frames that legitimately
  differ (video, scaled pictures) and say so in the report. Screenshots are the biggest
  cost in past sessions: never Read a full-size PNG, look through
  `research/tools/view.py a.png [b.png] [--crop WxH+X+Y] [--scale 50]
  [-o out.png]`, which crops, halves and puts the shots side by side
  in one small image (original and ours in one read). A hook
  (`.claude/hooks/image_read.py`) refuses the first Read of an image
  over 250,000 pixels; if `view.py` really can't do the job, say why
  and Read it again.
- **Logic checks use the dry run** (`--dry-run`, script only, seconds);
  the full build re-renders every asset.
- **Code before wine**: the decompile and the dry run answer most
  questions (Rules, "Wine is the last resort"). When you do film the
  original, film it once per screen and keep the shots in
  `research/<engine>/local/shots/` (git-ignored) with a name that says
  what they show; reuse them before starting wine again.
- **Batch** independent commands into one call; keep replies short, no
  progress narration between steps.
- **Stop at the end of the item asked for** (commit, push, update the
  game document's "Status") instead of scoping ahead; the Status and
  memory carry the state into a fresh session.
- No `/loop`, scheduled wakeups or polling unless the user asks; wait
  for background runs to notify.
- **Delegate small jobs to cheaper subagents** when you judge it fits
  (no need to ask): `model: "haiku"` for searches, greps across the
  tree, reading a long file for one fact, summarising a log or trace;
  `model: "sonnet"` for mechanical edits, doc updates, build-test-commit
  loops and route runs. Keep design, engine work, reverse engineering
  and hard debugging yourself. Give the subagent the exact task and
  the files, and ask for a short report, not file dumps. Don't
  delegate when the job needs much of what you already know (the
  session's findings, many files read, a long chain of reasoning):
  a subagent starts empty and re-reading all that costs more than
  doing the job yourself. Delegate only what a few lines of prompt
  can fully describe.

## Releasing CONscripter

Steps in `docs/releasing.md`; read it only when releasing.

## Gotchas

- `pkill -f`/`pgrep -f` match your own shell when the pattern is in the
  command line: use `pkill -x conscripter`.
- ONScripter ignores SIGTERM in script loops: always `timeout -k 5`.
- Python text-mode writes turn CRLF files into LF: check
  `grep -c $'\r$'` before committing engine files.
- `find /` and `locate` can be slow on machines with big network
  mounts: ask where the game is, or check your memory, first.
