# CONscripter-convert: notes for Claude

Converts visual novels the user owns into games for CONscripter (our
ONScripter-Yuri fork, the `engine/` submodule), especially its web
build.

| Game | Engine | Branch / worktree | Game document | Handoff (git-ignored) |
|---|---|---|---|---|
| 最果てのイマ COMPLETE (finished) | Xuse | `xuse` | `docs/games/saihate-no-ima.md` | `saiima.md` |
| 装甲悪鬼村正 (in progress) | Nitroplus NSS | `nitroplus`, `~/Projects/conscripter-convert-nitroplus` | `docs/games/muramasa.md` | `muramasa.md` |

**Starting a new game? Read `docs/new-game.md` first.**

Read first, in order: the game's handoff file (state and next steps;
"Resume here" at the top), the game's document, `PLAN.md` §4.2 (engine
additions) and §13 (lessons).

## Rules (from the user)

- One commit per task item, each pushed on its own. Engine first: commit
  and push `engine/` (github.com/blankiekun/CONscripter, branch
  `master`), then commit the submodule pointer together with the
  converter change.
- Fetch and rebase before every push: the user and other agents push to
  both repos. Push with
  `git -c credential.helper= -c credential.helper='!gh auth git-credential' push`
  (git has no credential helper; `gh` is logged in). Don't change the
  global git config.
- Base game branches on `origin/main`, not local `main` (another agent
  may commit there without pushing). Stage your own paths by name, never
  `git add -A`: other agents leave uncommitted files in the tree.
- No Co-Authored-By or Claude attribution lines in commits.
- Use existing NScripter commands before adding to the engine.
  Engine additions are opt-in, live in `engine/src/onsyuri/` (preferably
  `ONScripter_cons.cpp`), work in the web build, and are documented in
  `engine/ADDITIONS.md` and `PLAN.md` §4.2.
- Never touch the user's save folders (`out/saiima-flow`). Build into
  `out/<game>-cons`. List any folder before deleting it.
- Check with the user before starting anything big that they haven't
  asked for. They decide what is dropped.
- Running the original games under wine is allowed: a scratch prefix
  with real (not symlinked) user folders, its own Xvfb display, no
  audio, never the user's desktop. Stop wine and Xvfb when done.

## Layout

| Path | Contents |
|---|---|
| `crates/conv-core/src/ir.rs` | Engine-neutral IR |
| `crates/conv-engines/src/<engine>/` | Source readers (`xuse/`; `nitroplus/` on its branch) |
| `crates/conv-cons/` | IR → `0.txt` (`lib.rs`), menus, extras, NSA writer |
| `crates/conv-cli/` | The `conscripter-convert` command |
| `engine/` | CONscripter (C++; our code in `src/onsyuri/ONScripter_cons.cpp`) |
| `research/<engine>/` | Throwaway Python; `local/` is git-ignored (disassembly, wine scripts, shots) |
| `docs/new-game.md` | Playbook for converting a new game |
| `docs/games/<game>.md` | Per-game record: formats, opcodes, progression variables, status |

## Build and test

- `cargo test`, `cargo clippy --all-targets`; `cargo build --release`.
- Full build: `target/release/conscripter-convert convert-game <install
  dir> -o out/<game>-cons --font /usr/share/fonts/OTF/ipag.ttf --title
  "<title>"`. Each game's exact command is in its document.
- Engine: `make -C engine/build -j16` → `engine/build/conscripter`.
  Web: `cd engine/script && EMSDK_HOME=~/emsdk bash ./cross_web.sh`.
- Headless: `SDL_VIDEODRIVER=dummy SDL_AUDIODRIVER=dummy timeout -k 5 N
  engine/build/conscripter -r .` with the game folder as the working
  directory (otherwise `default.ttf` isn't found). Screenshot with
  `print 1:getscreenshot 800,600:savescreenshot "x.bmp"`.
- Headless game runs: patch a copy of `0.txt`: `textbtnwait` in
  `*conv_wait` → `mov %9,0`; comment out `click` in `*conv_click_wait`;
  put `mov %101,1:goto *label` right after `erasetextwindow 0` in
  `*start`. `research/xuse/smoke.py` does scripted clicks and
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

## Gotchas

- `pkill -f`/`pgrep -f` match your own shell when the pattern is in the
  command line: use `pkill -x conscripter`.
- ONScripter ignores SIGTERM in script loops: always `timeout -k 5`.
- Rebuilding `0.txt` makes `kidoku.dat` stale (byte offsets): delete it.
- Python text-mode writes turn CRLF files into LF: check
  `grep -c $'\r$'` before committing engine files.
- `find /` and `locate` on this machine are slow (big NAS mounts): look
  in `~/Games`, `~/Z:/Media/InstalledVNs` and the notes first.
