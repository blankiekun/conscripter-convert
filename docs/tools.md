# Tools and recipes

Moved out of `CLAUDE.md` so every agent does not load them; read the
bullet you need.

## Runs, traces and saves

- Scratch run beside a full build (yours or another agent's):
  `research/tools/scratch.sh out/<game>-cons <scratch> [0.txt [engine]]`
  links only the archives, `default.ttf` and `movie/`, copies in your
  script and a renamed engine, and writes `<scratch>/run.sh [options]`
  (headless, `--save-dir <scratch>/save`).
- A/B frames and saves: `research/tools/scene_ab.py frames|saves`
  plays a scene under two engines (or with one line dropped) in such
  scratch folders and prints `compare -metric AE` per shot; `saves`
  saves with engine A and loads with B (every save-version PR).
- Saves at chosen scenes: `research/tools/test_saves.py`; every route:
  `engine/tools/cons_explore.py`; value census of a `0.txt`: `argcensus.py`.
- A new Windows game's first look and its one logged wine run:
  `research/tools/firstcontact/first_contact.py <install>` (report,
  hooks) and `fc.sh <install>` (API-level trace, capped WebP shots);
  `docs/new-game.md` step 0.3.
- Debug trace (CONscripter after 0.18.0; reference: `engine/DEBUG.md`;
  recipes: `docs/new-game.md`, "Checking a build with the debug
  trace"): `--trace t.jsonl --auto-play 0 --save-dir <scratch>` plays
  the game by itself, headless with `SDL_VIDEODRIVER=dummy` (no Xvfb),
  and writes one JSON line per page, per command that changed the
  screen or sound, per choice, missing file, script error and crash. `--trace-ignore` leaves the game's own UI
  sprites out and keeps auto-play off their buttons. Check a trace with
  `python3 -I engine/tools/cons_trace_check.py t.jsonl --script 0.txt`.
  With `--real-time` the trace samples each moving sprite every 4 frames
  (`--trace-motion-samples`) and times each effect: the check's `jump`
  (a sprite stepping much faster than its motion around it, or off and
  back) and `duration` (an effect or motion off its asked time) catch
  jumpy motion no picture check sees.
  Start anywhere with `--goto <label> --vars <file>` instead of patching
  `0.txt`. Exit codes: 0 end, 1 auto-play stuck, 2 script error, 3
  startup failure, 128+n a signal.
- Against the original (a trace of it, `docs/new-game.md` steps 22-23):
  `research/tools/trace_diff.py orig.jsonl ours.jsonl --map <the
  build's --source-map file>` prints the pages' text, command and time
  differences (`--self-test` tests it). A wine original's trace
  (`xtrace.sh`, `ntrace.sh`) runs N times as fast with `SPEED=N`, its
  times still comparable (4 for 最果てのイマ, 8 for the Nitroplus games);
  the Switch one with `IDLEA=0.3` (`docs/new-game.md`, "Trace speeds").
- Picture diff per page against the original: `ntrace.sh --shots <dir>`
  (a frame per text block) and our `--shot-every 1`, then
  `research/tools/picdiff.py orig.jsonl <dir> ours.jsonl <dir> --map
  <map> --ms --thread 0 --mask <text window>`: differing pixels, RMSE
  and their box per page, worst first, views of the worst
  (`docs/new-game.md`, "Picture diff"; lossy WebP: `--fuzz 15`).
- Frames mid-effect: the trace's `--shot-effects 25,50,75`. The older
  patched-script runs: `docs/new-game.md` step 9 and "When the trace
  can't do it". Anything that needs a display (Xvfb, wine, Vita3K,
  Ryujinx) runs only through `research/tools/headless-run.sh <cmd>
  [args...]`: it starts its own Xvfb on a free display (`-displayfd`),
  unsets `WAYLAND_DISPLAY`, gives an empty `XDG_RUNTIME_DIR`, no D-Bus,
  no-op `xdg-open`/`gio`, dummy SDL audio, no pulse/oss in wine (the trace scripts keep a null ALSA device, so voices still time their waits), and kills Xvfb
  on exit. It sets `HEADLESS=1` and `DISPLAY`; `xtrace.sh`, `ntrace.sh`,
  `vita-emu.sh` and `switch-emu.sh` re-run themselves under it. Never
  `pkill Xvfb` (other agents' displays).
  Xvfb also dies if the wrapper is SIGKILLed (a pid watcher). 2026-10-18: all four
  scripts started, ran and exited cleanly through it (short capped runs); `vita-emu.sh` and
  `switch-emu.sh` cap memory with `capped.sh`, which takes the user bus for `systemd-run` only (`HEADLESS_USER_RUNTIME`).
- What a frame costs: `--trace-perf` adds a `perf` record per page
  (frames presented, the wall-clock ms worked for them, the slowest
  frame); `--dump-frames frames.tsv` writes a line per present (game ms,
  work ms, present ms, page). Measure a sprite effect's cost (`sptrail`,
  `spfilter`) with these, not a patched engine or an LD_PRELOAD shim.
  Pages and `end` count their `presents` always.
- Drawing order: `--trace-layers` writes, on each `print` or effect, what
  the new screen is drawn from, front to back (`lsp n`, `text`, `window`,
  ..., `bg`): a sprite behind the text window or a CG, or an `spz` gone
  wrong, without a picture.
- Saves: `--roundtrip-every N` also compares `spmotion` keys and the Lua
  state; a `save data left out` warning (save block from another engine
  version) is an error for the checker; `meta` gives `save_version`.
- The web build headless: `python3 -I engine/tools/web_run.py
  engine/build_web out/<game>-web -o web.jsonl --query
  'auto-play=200&seed=3'` serves the build and the loose-file game on
  127.0.0.1 (a free port), plays it in headless Chromium with `?trace`
  and writes the trace as it comes, until `end` or `crash`. Check it
  like a native one.
- Test scripts: UTF-8 with `--enc:utf8` in `ons_args`, as the builds;
  a Shift-JIS one with `iconv -t cp932` (not `sjis`, which turns `\` into
  `＼`) and `--enc:sjis`.
- One function of the exe without the game: `research/tools/.venv/bin/python
  -I research/tools/exe_call.py <exe> <addr|*table+4*i> [args] --ret
  float` (PE, Vita ELF, Switch NSO under Unicorn; `docs/new-game.md`,
  "Running one exe function"). Setup, once per machine:
  `research/tools/setup-venv.sh` (makes `research/tools/.venv` with
  unicorn 2.1.4 and runs exe_call's self-test,
  `research/tools/test_exe_call.py`, which CI runs too).
- Read a variable headless: `lsp 999,"X:"+$1,0,0` prints "can't find
  file X:..." on stderr.

## Manuals

Each platform keeps the manual in a known place; check it before asking
for screenshots (`docs/new-game.md` step 9). Extract into
`research/<engine>/local/manual/`.

- **PC**: a PDF/HTML/CHM in the install or on the disc (`find <install>
  -iname '*manual*' -o -iname '*.pdf' -o -iname '*.chm'`);
  `pdftoppm -r 80 -png Manual.pdf p` renders the pages.
- **Vita**: `sce_sys/manual/NNN.png` in the decrypted app (the
  encrypted copy's PNGs do not open); 960x544 pages.
- **Switch**: the NSP's NCA of Content Type `Manual` (find it with
  `hactool -k <keys> -i <nca>`), extracted with `hactool -k <keys>
  --romfsdir=<empty dir> <nca>`. Many games' Manual NCA is only the
  legal/support HTML (`legalinfo.xml`, `support.htdocs/`), not a game
  manual.

## PR report

`tests/pr-report.sh [--base REF]` (about 5 minutes) runs `cargo test`,
`clippy -D warnings`, `findings_lint.py`, `dropped_lint.py`,
`docpaths.py`, `doclint.py` (template headings and Status length per game
against ceilings that only go down) and `tests/snapshot.sh --diff`, and
prints a Markdown block for the PR body: per changed game the txtdiff
counts, metrics totals before and after, and a `why it changed: <fill in>`
line. It also fails when a branch changes `crates/conv-engines/src/<engine>/`
without a `docs/games/*.md`, and flags (without failing) a change to an
engine's sync code with no change to its tests (`research/tools/sync_check.py`:
Nitroplus `convert/sync.rs` against `convert/tests/`; Xuse, MAGES and
TYPE-MOON `convert.rs` against its own `#[cfg(test)]` module). `docpaths` skips paths that the
engine's own `.gitignore` ignores, so a worktree that has not built the
engine passes.

## Reading files cheaply

For the docs and scripts read most often, use the helpers in
`research/tools/` (run with `python3 -I`):
`mdsec.py <file.md>` lists the headings, `mdsec.py <file.md> Status`
prints that section only (game docs, `docs/lessons.md`, `CLAUDE.md`);
`label.py 0.txt <label> [lines]` prints one label's block with line
numbers, `label.py 0.txt -l <text>` finds labels; `txtdiff.py old.txt
new.txt [--map 's/old/new/'|--rules <file>]` prints changed hunks under
their labels, counts them by command (numbers masked) and says whether
the rules make it only a rename (`--self-test` tests it);
`cmdcheck.py engine/src/cons/ONScripter_lut.cpp 0.txt` lists the
commands a script uses that the engine's table lacks (`tests/snapshot.sh`
runs it on every game). Source code is the
biggest text cost: `code.py <file.rs|.cpp>` lists its items with line
numbers, `code.py <file> <name> [lines]` prints one fn, type, impl or
C++ `Class::name` with its doc comments; use it instead of paging
through a file with `sed`.

## Numbers before pictures

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

## Code before wine

- **Code before wine**: the decompile and the dry run answer most
  questions (Rules, "Wine is the last resort"). When you do film the
  original, film it once per screen and keep the shots in
  `research/<engine>/local/shots/` (git-ignored) with a name that says
  what they show; reuse them before starting wine again. Search
  `research/tools/shots.py find <words>` first, `shots.py add` every
  new shot; a quick check from a generated save may run at once (Rules),
  anything longer: park a doubt only the original can settle in
  `research/<engine>/local/film-list.md` (docs/new-game.md, "Film
  list"). A hook refuses wine/Xvfb/Vita3K/Ryujinx run outside
  `research/tools/headless-run.sh`.

## Build and test details

- Memory caps: run builds, engine runs and emulators that may grow
  through `research/tools/capped.sh <GiB> <cmd> [args...]`, never a raw
  `systemd-run -p MemoryMax` (the bash hook refuses it): a kernel OOM
  kill pops a "killed" notification on the user's desktop. capped.sh
  runs cmd in its own `systemd-run --user` scope (MemoryHigh=<GiB>,
  MemoryMax=<GiB>+1 as a backstop), stops it itself at 90 % of <GiB>
  (SIGTERM, SIGKILL after 5 s; prints `capped.sh: stopped at X MiB`,
  exit 137), refuses to start with exit 75 when less than <GiB>+2 GiB
  is available, and passes cmd's exit code through otherwise. It works
  inside `headless-run.sh`. Self-test: `research/tools/test_capped.sh`.
- `tests/snapshot.sh`: dry-runs every game present on the machine (links
  in `$CONS_GAMES`, see the script) and says which games' `0.txt`
  changed. Run it after any change to shared code; a change to another
  game's script must be deliberate and said in the commit
  (`--update` then rewrites `tests/snapshots/`). `--diff origin/main`
  builds that ref in its own worktree and shows each game's 0.txt
  changes; `--map <rules>` checks a change is only a rename (below).
  The dry runs go two at a time when memory allows (each game's last
  peak plus 2 GB must be free), and a game's dry run is reused while
  the binary, its arguments, the engine's command table and the script
  are unchanged (`--no-cache` reruns them; after editing a game install,
  use it).
  On a conflict in `tests/snapshots/` never take one side: rebase, rerun
  `--update`, and explain each changed game's 0.txt in the PR body.
  It prints each game's `gaps`/`left_out` change and exits 2 when either
  rose against `<game>.metrics.json`, unless `--accept-gaps` (say why in
  the PR). CI also runs `dropped_lint.py` and `findings_lint.py`, each
  with a per-game ceiling that only goes down.
  The snapshot holds a sha256 per other file the dry run writes
  (`screens/*.lua`, `ons_args`), so it names the screen a shared change
  touched (`--diff` shows the first lines of each changed file's diff).
  It fails a game when a count of the dry run's lint (the `lint:` lines
  at the end of its log: a jump to no label, a label defined twice, a
  string that runs past its line, `?` in text, a file not written, …)
  rises above its ceiling in `<game>.lint` (`N  kind`; a kind not there
  is 0). A count below its ceiling prints `lower to N`; `--update`
  writes the lower counts but never raises one: a new ceiling goes in
  by hand, with the reason in the PR.
  Among the lint's kinds, a file the script names that the build wouldn't
  write: the dry run's lint checks 0.txt's file names against the files
  each engine records it would write (`conv_engines::output::record`;
  `Output::write`, `Output::have` do it), not against the folder. Code
  that writes a file outside `Output` (its own `fs::write`, ffmpeg, a
  dry-run shortcut that skips making it) calls `record` with the
  output-relative name, in a dry run too. Movies (`.mp4`) and `.webp`
  aren't among the names the lint checks.
- `python3 -I tests/extras_smoke.py <full build>`: opens the extras'
  menu headless with every lock open and checks each mode is a button
  and opens (the trace); run it on every full build handed to the user
  (the snapshot sees only the screens' text, route runs skip them).
- Engine: `research/tools/build-engine.sh [engine dir] [build dir]`
  (cmake + make; `SDL2_MIXER_PREFIX=<prefix>` when the system has no
  SDL2_mixer) → `engine/build/conscripter`.
  Web: `cd engine/script && EMSDK_HOME=<emsdk dir> bash ./cross_web.sh`.
- Headless: `SDL_VIDEODRIVER=dummy SDL_AUDIODRIVER=dummy timeout -k 5 N
  engine/build/conscripter -r .` with the game folder as the working
  directory (otherwise `default.ttf` isn't found). Screenshot with
  `print 1:getscreenshot 800,600:savescreenshot "x.bmp"`.
