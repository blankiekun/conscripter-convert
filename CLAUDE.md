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

**Starting a new game? Read `docs/new-game.md` first.**

Read first, in order: the game's document (its "Status" section holds
the feature state and the next steps), `PLAN.md` §4.2 (engine
additions) and §13 (lessons).

Each game's state is tracked in its `docs/games/<game>.md`, which is
public: no local paths (use placeholders such as `<game>`), only
reproducible commands. There are no git-ignored handoff files; keep
the document's "Status" current with every commit that changes it.
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
  (`git fetch && git merge origin/main`), before any edit, then build and
  run `cargo test`. Merge it again before a commit that touches a hot
  spot (below). Conflicts are small when they are fresh.
- **Hot spots**: files every game's work changes. Before editing one,
  check what main changed in it (`git log -p HEAD..origin/main -- <file>`).
  Keep each edit small and in its own place:

  | File | How to edit it |
  |---|---|
  | `crates/conv-core/src/ir.rs` | Add new types and `Op` variants at the end of their block, with a doc comment; don't reorder or reformat existing ones |
  | `crates/conv-cons/src/lib.rs` | Put a new feature's emitter in its own module (`puzzle.rs`, `menu.rs`, `replay.rs`) and add one `match` arm here. Sprite and variable ranges are the constants at the top: claim a new range there, with a comment, and check it against the ranges other modules document |
  | `crates/conv-cons/src/menu.rs` | Shared by every game's menus: change it only through what a game passes in its IR (`SaveMenu`, `ButtonBar`, …), not with game-specific branches |
  | `crates/conv-cons/src/switches.rs` | A new settings switch is one entry in `def` (its index and what applying it does), with the next free index in your engine's block (the table at the top: shared 20-99, xuse 100-199, mages 200-299, nitroplus 300-399, a new engine the next hundred); nothing in `menu.rs` |
  | `crates/conv-cli/src/main.rs` | One subcommand per engine; keep the engine's own code in `crates/conv-engines/src/<engine>/` |
  | `engine` (submodule pointer) | Two branches that bump it conflict, and a merge can't combine them: before bumping it, merge `origin/main` and rebase your engine commits onto CONscripter's `master`, so the pointer only moves forward |
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
| `crates/conv-core/src/ir.rs` | Engine-neutral IR |
| `crates/conv-engines/src/<engine>/` | Source readers (`xuse/`, `nitroplus/`); a game's own rules in a module there (`nitroplus/convert/muramasa.rs`) |
| `crates/conv-cons/` | IR → `0.txt` (`lib.rs`), menus, extras, NSA writer |
| `crates/conv-cli/` | The `conscripter-convert` command |
| `engine/` | CONscripter (C++; our code in `src/onsyuri/ONScripter_cons.cpp`) |
| `research/<engine>/` | Throwaway Python; `local/` is git-ignored (disassembly, wine scripts, shots) |
| `docs/new-game.md` | Playbook for converting a new game |
| `docs/games/<game>.md` | Per-game record: formats, opcodes, progression variables, status |

## Build and test

- `cargo test`, `cargo clippy --all-targets`; `cargo build --release`.
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
- Headless game runs (the older recipe, when the trace can't do it):
  patch a copy of `0.txt`: `textbtnwait` in `*conv_wait` → `mov %9,0`;
  comment out `click` in `*conv_click_wait`; put `mov %101,1:goto
  *label` right after `erasetextwindow 0` in `*start`. `research/xuse/smoke.py` does scripted clicks and
  screenshots (its click queue is `%1000+`; keep converter variables out
  of 1000-1099).
- Frames mid-effect: run on your own Xvfb display with
  `WAYLAND_DISPLAY= SDL_VIDEODRIVER=x11 SDL_RENDER_DRIVER=software`
  (the default renderer grabs black) and `import -window root`. Without
  `WAYLAND_DISPLAY=` the window opens on the user's desktop.
- Test scripts: encode with `iconv -t cp932` (not `sjis`, which turns `\`
  into `＼`).
- Read a variable headless: `lsp 999,"X:"+$1,0,0` prints "can't find
  file X:..." on stderr.

## Releasing CONscripter

Bump the version in `asset/icon1.rc`, `src/onsyuri/version.h`,
`src/onsyuri_android/app/build.gradle` (also `versionCode` + 1),
`src/onsyuri_android/app/res/values/strings.xml`,
`src/onsyuri_libretro/conscripter_libretro.info`,
`src/onsyuri_libretro/libretro.cpp` and `src/onsyuri_web/conscripter.html`
(CRLF: edit it byte-safely). Leave `ONS_JH_VERSION` alone. Commit
"Version X.Y.0", tag `vX.Y.0`, push `master` and the tag. Five workflows
run on tags (Linux, Windows, macOS, web, Android); the release should
have 9 assets. If `build_web.yml`, `cross_web.sh` or `_fetch.sh`
changed, let the `master` run finish first (tags read its caches).
Then write the release notes, a changelog from `git log vOLD..vNEW`
grouped by what changed for a game or a player:
`gh release edit vX.Y.0 --notes-file notes.md` (the workflows keep the
notes since 0.19.0; 0.17.0 and 0.18.0 have none).

## Gotchas

- `pkill -f`/`pgrep -f` match your own shell when the pattern is in the
  command line: use `pkill -x conscripter`.
- ONScripter ignores SIGTERM in script loops: always `timeout -k 5`.
- Rebuilding `0.txt` makes `kidoku.dat` stale (byte offsets): delete it.
- Python text-mode writes turn CRLF files into LF: check
  `grep -c $'\r$'` before committing engine files.
- `find /` and `locate` can be slow on machines with big network
  mounts: ask where the game is, or check your memory, first.
