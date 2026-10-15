# CONscripter-convert: notes for Claude

Converts visual novels the user owns into games for CONscripter (our
ONScripter-Yuri fork, the `engine/` submodule), especially its web
build.

| Game | Engine | Branch / worktree | Game document |
|---|---|---|---|
| 最果てのイマ COMPLETE (reopened: fixes, opcode audit) | Xuse | `xuse` | `docs/games/saihate-no-ima.md` |
| 装甲悪鬼村正 (in progress) | Nitroplus NSS | `nitroplus`, `../conscripter-convert-nitroplus` | `docs/games/muramasa.md` |
| 俺たちに翼はない, PS Vita (starting) | not identified yet | `oretsuba`, `../conscripter-convert-oretsuba` | `docs/games/oretsuba.md` |
| 続・殺戮のジャンゴ (starting) | Nitroplus NSS (older) | `django`, `../conscripter-convert-django` | `docs/games/django.md` |
| Full Metal Daemon Muramasa, English (starting) | Nitroplus Diesel (NPK3, Squirrel) | `muramasa-en`, `../conscripter-convert-muramasa-en` | `docs/games/muramasa-en.md` |

**Starting a new game? Read `docs/new-game.md` first.**

Read first, in order: the game's document (its "Status" section holds
the feature state and the next steps), `PLAN.md` §4.2 (engine
additions) and §13 (lessons). These are long: list the headings
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

- One commit per task item, each pushed on its own. Engine first: commit
  and push `engine/` (github.com/blankiekun/CONscripter, branch
  `master`), then commit the submodule pointer together with the
  converter change.
- Fetch before every push: the user and other agents push to both
  repos. Rebase only your own unpushed commits, onto the same branch's
  remote (`git pull --rebase origin <branch>`); bring `main` into a game
  branch by merging, never by rebasing (see "Working beside other
  agents"). Push with
  `git -c credential.helper= -c credential.helper='!gh auth git-credential' push`
  (git has no credential helper; `gh` is logged in). Don't change the
  global git config.
- Base game branches on `origin/main`, not local `main` (another agent
  may commit there without pushing). Stage your own paths by name, never
  `git add -A`: other agents leave uncommitted files in the tree.
- No Co-Authored-By or Claude attribution lines in commits.
- Use existing NScripter commands before adding to the engine.
  Engine additions are not opt-in (the user, 2026-10-14): a new
  behaviour applies to every script, with no command or flag to turn it
  on; new commands take effect where a script uses them. They live in
  `engine/src/onsyuri/` (preferably `ONScripter_cons.cpp`), work in the
  web build, and are documented in `engine/ADDITIONS.md` and `PLAN.md`
  §4.2. Check that a change of behaviour doesn't break another game's
  build. Bug fixes likewise: when the engine does something wrong (text
  misplaced, characters dropped), fix it for every script and list the
  fix in `engine/ADDITIONS.md`. Then drop any converter workaround for
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

## Working beside other agents

Several agents work at once, one per game branch (`xuse`, `nitroplus`,
each in its own worktree), and their work meets in shared code.

- **Start every session by merging `origin/main` into your game branch**
  (`git fetch && git merge origin/main && git submodule update engine`),
  before any edit, then build and run `cargo test`. Merge it again before a commit that touches a hot
  spot (below). Conflicts are small when they are fresh.
- **Hot spots**: files every game's work changes. Before editing one,
  check what main changed in it (`git log -p HEAD..origin/main -- <file>`).
  Keep each edit small and in its own place:

  | File | How to edit it |
  |---|---|
  | `crates/conv-core/src/ir/` (`mod.rs`: the ops and `Script`; `screens.rs`: the shared screens; `picture.rs`: the picture-drawn ones) | Add new types and `Op` variants at the end of their block, with a doc comment; don't reorder or reformat existing ones |
  | `crates/conv-cons/src/lib.rs` | Put a new feature's emitter in its own module (`puzzle.rs`, `menu.rs`, `replay.rs`) and add one `match` arm here. Sprite and variable ranges are the constants at the top: claim a new range there, with a comment, and check it against the ranges other modules document |
  | `crates/conv-cons/src/menu.rs` | Shared by every game's menus: change it only through what a game passes in its IR (`SaveMenu`, `ButtonBar`, …), not with game-specific branches |
  | `crates/conv-cons/src/switches.rs` | A new settings switch is one entry in `def` (its index and what applying it does), with the next free index in your engine's block (the table at the top: shared 20-99, xuse 100-199, mages 200-299, nitroplus 300-399, a new engine the next hundred); nothing in `menu.rs` |
  | `crates/conv-cli/src/main.rs` | One subcommand per engine; keep the engine's own code in `crates/conv-engines/src/<engine>/` |
  | `engine` (submodule pointer) | Two branches that bump it conflict, and a merge can't combine them: before bumping it, merge `origin/main` and rebase your engine commits onto CONscripter's `master`, so the pointer only moves forward. A merge that conflicts on it: `research/tools/merge-engine.sh` takes the newer of the two engine commits |
  | `PLAN.md`, `README.md`, `docs/new-game.md`, this file | Edit only your game's rows and paragraphs; general lessons go in `PLAN.md` §13 |
  | `Cargo.toml`, `Cargo.lock` | Add dependencies in their own commit; on a conflict, take both sides and let `cargo build` rewrite the lock file |

- **Shared refactors go to main at once**: when you change the shape of
  shared code (a helper's signature, a function split or moved, an IR
  type renamed, numbers renumbered), commit that change on its own,
  merge `origin/main` into your branch and push the branch to `main`
  right away, before building your feature on it. Other branches then
  write against the new shape; a refactor left on a game branch for a
  day collides with every change made to the old one meanwhile.
- **Game-specific code stays out of shared files**: an engine's reader
  and compiler live in `crates/conv-engines/src/<engine>/`, and a single
  game's own rules go in a module of their own there (for example
  `nitroplus/convert/muramasa.rs`, chosen by the CLI when it builds).
- **Merge your game branch into `main` at good points** (tests pass, the
  game document's "Status" is up to date), so the other
  agents start from it. After merging `origin/main` into the branch, the
  push is a fast-forward: check with
  `git merge-base --is-ancestor origin/main HEAD`, then
  `git push origin <branch>:main`. Never switch branches in another
  agent's worktree; the main checkout belongs to whoever works there.

## Layout

| Path | Contents |
|---|---|
| `crates/conv-core/src/ir/` | Engine-neutral IR; `alloc.rs`: the variables conv-cons keeps |
| `crates/conv-engines/src/<engine>/` | Source readers (`xuse/`, `nitroplus/`); a game's own rules in a module there (`nitroplus/convert/muramasa.rs`) |
| `crates/conv-cons/` | IR → `0.txt` (`lib.rs`), menus, extras, NSA writer |
| `crates/conv-cli/` | The `conscripter-convert` command |
| `engine/` | CONscripter (C++; our code in `src/onsyuri/ONScripter_cons.cpp`) |
| `research/<engine>/` | Throwaway Python; `local/` is git-ignored (disassembly, wine scripts, shots) |
| `docs/new-game.md` | Playbook for converting a new game |
| `docs/games/<game>.md` | Per-game record: formats, opcodes, progression variables, status |

## Build and test

- `cargo test`, `cargo clippy --all-targets`; `cargo build --release`.
- `tests/snapshot.sh`: dry-runs every game present on the machine (links
  in `$CONS_GAMES`, see the script) and says which games' `0.txt`
  changed. Run it after any change to shared code; a change to another
  game's script must be deliberate and said in the commit
  (`--update` then rewrites `tests/snapshots/`).
- Full build: `target/release/conscripter-convert convert-game <install
  dir> -o out/<game>-cons --font <a Japanese TTF, e.g. ipag.ttf> --title
  "<title>"`. Each game's exact command is in its document.
- Engine: `make -C engine/build -j16` → `engine/build/conscripter`.
  Web: `cd engine/script && EMSDK_HOME=<emsdk dir> bash ./cross_web.sh`.
- Headless: `SDL_VIDEODRIVER=dummy SDL_AUDIODRIVER=dummy timeout -k 5 N
  engine/build/conscripter -r .` with the game folder as the working
  directory (otherwise `default.ttf` isn't found). Screenshot with
  `print 1:getscreenshot 800,600:savescreenshot "x.bmp"`.
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
- Frames mid-effect and the older patched-script runs: `docs/new-game.md`
  step 9 and "When the trace can't do it". On Xvfb always set
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
  prints that section only (game docs, `PLAN.md`, this file);
  `label.py 0.txt <label> [lines]` prints one label's block with line
  numbers, `label.py 0.txt -l <text>` finds labels. Source code is the
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
  `compare -metric AE -fuzz 2% a.png b.png null:` (pixel count) or
  `-metric RMSE`, or crop to the region in question; view an image only
  when the numbers can't tell what is wrong. Screenshots are the biggest
  cost in past sessions: never Read a full-size PNG, look through
  `research/tools/view.py a.png [b.png] [--crop WxH+X+Y] [--scale 50]
  [-o out.png]`, which crops, halves and puts the shots side by side
  in one small image (original and ours in one read). A hook
  (`.claude/hooks/image_read.py`) refuses the first Read of an image
  over 250,000 pixels; if `view.py` really can't do the job, say why
  and Read it again.
- **Logic checks use the dry run** (`--dry-run`, script only, seconds);
  the full build re-renders every asset.
- **Film the original once per screen** and keep the shots in
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
- Rebuilding `0.txt` makes `kidoku.dat` stale (byte offsets): delete it.
- Python text-mode writes turn CRLF files into LF: check
  `grep -c $'\r$'` before committing engine files.
- `find /` and `locate` can be slow on machines with big network
  mounts: ask where the game is, or check your memory, first.
