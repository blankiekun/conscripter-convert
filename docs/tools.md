# Tools and recipes

Find your task's heading, read that bullet. Scripts print their options
with `--help`; this file holds what `--help` cannot say. Wrappers live in
`research/<engine>/` (`xuse/tracedll/xtrace.sh`,
`nitroplus/tracedll/ntrace.sh`, `oretsuba/vita-emu.sh`,
`typemoon/switch-emu.sh`).

## Runs, traces and saves

**Debug trace** (reference `engine/DEBUG.md`; recipes
`docs/new-game-reference.md`, "Checking a build with the debug trace").
- `--trace t.jsonl --auto-play 0 --save-dir <scratch>` plays the game by
  itself, headless with `SDL_VIDEODRIVER=dummy` (no Xvfb). One JSON line
  per page, per command that changed screen or sound, per choice, missing
  file, script error and crash.
- `--trace-ignore` leaves the game's own UI sprites out and keeps
  auto-play off their buttons.
- Check: `python3 -I engine/tools/cons_trace_check.py t.jsonl --script 0.txt`.
- `--real-time` samples each moving sprite every 4 frames
  (`--trace-motion-samples`) and times each effect; the check's `jump` (a
  sprite stepping much faster than its motion around it, or off and back)
  and `duration` (effect or motion off its asked time) catch jumpy motion
  no picture check sees.
- Start anywhere with `--goto <label> --vars <file>`, not a patched `0.txt`.
- Exit codes: 0 end, 1 auto-play stuck, 2 script error, 3 startup
  failure, 128+n a signal.
- `--trace-layers` writes, on each `print` or effect, what the new screen
  is drawn from, front to back (`lsp n`, `text`, `window`, ..., `bg`): a
  sprite behind the text window or a CG, or an `spz` gone wrong, seen
  without a picture.
- `--trace-perf` adds a `perf` record per page (frames presented,
  wall-clock ms worked, slowest frame); `--dump-frames frames.tsv` writes
  a line per present (game ms, work ms, present ms, page). Measure a
  sprite effect's cost (`sptrail`, `spfilter`) with these, not a patched
  engine or an LD_PRELOAD shim. Pages and `end` always count `presents`.
- `--roundtrip-every N` compares saves, `spmotion` keys and Lua state; a
  `save data left out` warning (save block from another engine version)
  is an error for the checker; `meta` gives `save_version`.
- Frames mid-effect: `--shot-effects 25,50,75`.
- Read a variable headless: `lsp 999,"X:"+$1,0,0` prints "can't find
  file X:..." on stderr.

**Scratch runs and saves**
- Beside a full build (yours or another agent's):
  `research/tools/scratch.sh out/<game>-cons <scratch> [0.txt [engine]]`
  links the archives, `default.ttf` and `movie/`, copies in your script and
  a renamed engine, writes `<scratch>/run.sh [options]` (headless,
  `--save-dir <scratch>/save`).
- A/B frames and saves: `research/tools/scene_ab.py frames|saves` plays a
  scene under two engines (or with one line dropped) in such scratch
  folders and prints `compare -metric AE` per shot; `saves` saves with
  engine A and loads with B (every save-version PR).
- Saves at chosen scenes: `research/tools/test_saves.py`; every route:
  `engine/tools/cons_explore.py`; value census of a `0.txt`:
  `research/tools/argcensus.py`.
- Test scripts: UTF-8 with `--enc:utf8` in `ons_args`, as the builds;
  Shift-JIS with `iconv -t cp932` (`sjis` turns `\` into `＼`) and
  `--enc:sjis`.

**Web build headless**: `python3 -I engine/tools/web_run.py
engine/build_web out/<game>-web -o web.jsonl --query 'auto-play=200&seed=3'`
serves the build and the loose-file game on 127.0.0.1 (free port), plays
it in headless Chromium with `?trace`, writes the trace as it comes until
`end` or `crash`. Check it like a native one.

**Nightly checks**: the 03:30 user timer `conscripter-snapshot.timer` runs
`tests/snapshot.sh` and `tests/playback.sh` on `origin/main` (log
`out/nightly/<date>.log`).
- `tests/playback.sh [--update] [--pages N] [game ...]`: per game it
  auto-plays the dry run's `0.txt` from `*conv_title_start` (`--goto`),
  headless, `--real-time`, 3,000 pages, no pictures or sounds (no full
  build; motions and effect times are traced all the same), renamed engine
  copy, 3 GiB and 400 s at most. Then `cons_trace_check.py`, and the
  `jump` and `duration` counts against `tests/snapshots/<game>.playback`
  (ceilings like the `.lint` files: a rise fails, `--update` lowers).
- A game stops where it must press a button with no picture (Django at its
  first choice plate, page 559). Runs stay in `out/playback/<game>`;
  `--self-test` checks the compare.

**Displays and emulators** (Xvfb, wine, Vita3K, Ryujinx): run only through
`research/tools/headless-run.sh <cmd> [args...]`. It starts its own Xvfb on
a free display (`-displayfd`), unsets `WAYLAND_DISPLAY`, gives an empty
`XDG_RUNTIME_DIR`, no D-Bus, no-op `xdg-open`/`gio`, dummy SDL audio, no
pulse/oss in wine (the trace scripts keep a null ALSA device, so voices
still time their waits), sets `HEADLESS=1` and `DISPLAY`, and kills its
Xvfb on exit (not when SIGKILLed). `xtrace.sh`, `ntrace.sh`, `vita-emu.sh`
and `switch-emu.sh` re-run themselves under it; the emulator wrappers cap
memory with `capped.sh`. Other agents own the other Xvfb displays, so leave
them running.

**New Windows game, first look**: `research/tools/firstcontact/first_contact.py
<install>` (report, hooks) and `fc.sh <install>` (API-level trace, capped
WebP shots), the one logged wine run; `docs/new-game.md` step 3.

## Manuals

Check the manual before asking for screenshots. Extract into
`research/<engine>/local/manual/`.

- **PC**: a PDF/HTML/CHM in the install or on the disc (`find <install>
  -iname '*manual*' -o -iname '*.pdf' -o -iname '*.chm'`);
  `pdftoppm -r 80 -png Manual.pdf p` renders the pages.
- **Vita**: `sce_sys/manual/NNN.png` in the decrypted app (the encrypted
  copy's PNGs do not open); 960x544 pages.
- **Switch**: the NSP's NCA of Content Type `Manual` (find it with
  `hactool -k <keys> -i <nca>`), extracted with `hactool -k <keys>
  --romfsdir=<empty dir> <nca>`. Many games' Manual NCA is only the
  legal/support HTML (`legalinfo.xml`, `support.htdocs/`).

## PR report

`tests/pr-report.sh [--base REF]` (about 5 minutes) runs `cargo test`,
`clippy -D warnings`, `dropped_lint.py`, `docpaths.py`, `doclint.py` and
`tests/snapshot.sh --diff`, and prints the Markdown block for the PR body
(per changed game: txtdiff counts, metrics before and after, a `why it
changed: <fill in>` line). It fails when `crates/conv-engines/src/<engine>/`
changes without a `docs/games/*.md`, and flags an engine's sync code changed
without its tests (`research/tools/sync_check.py`). `docpaths` skips paths
the engine's `.gitignore` ignores.

## Reading files cheaply

Helpers in `research/tools/` (run with `python3 -I`):
- Docs: `mdsec.py <file.md>` lists headings, `mdsec.py <file.md> Status`
  prints that section (game docs, `docs/lessons.md`, `CLAUDE.md`).
- `0.txt`: `label.py 0.txt <label> [lines]` prints one label's block with
  line numbers, `label.py 0.txt -l <text>` finds labels.
- Script changes: `txtdiff.py old.txt new.txt [--map 's/old/new/'|--rules
  <file>]` prints changed hunks under their labels, counts them by command
  (numbers masked) and says whether the rules make it only a rename
  (`--self-test`).
- Missing commands: `cmdcheck.py engine/src/cons/ONScripter_lut.cpp 0.txt`
  lists commands a script uses that the engine's table lacks
  (`tests/snapshot.sh` runs it on every game).
- Source code is the biggest text cost: `code.py <file.rs|.cpp>` lists its
  items with line numbers, `code.py <file> <name> [lines]` prints one fn,
  type, impl or C++ `Class::name` with its doc comments; use it, not `sed`
  paging.

## Numbers before pictures

- Compare frames with `compare -metric AE -fuzz 0 a.png b.png null:`
  (pixel count) or `-metric RMSE`, or crop to the region in question; view
  an image only when numbers cannot tell what is wrong.
- Exact expectations (a source picture against the drawn sprite, a colour
  from the exe): no fuzz, or read the pixel values; 2 % fuzz hid sprites
  drawn 1 % dark. A small fuzz is for frames that legitimately differ
  (video, scaled pictures); say so.
- Screenshots cost the most. Look through `research/tools/view.py a.png
  [b.png] [--crop WxH+X+Y] [--scale 50] [-o out.png]`: it crops, halves and
  puts shots side by side in one small image (original and ours in one
  read). The hook `.claude/hooks/image_read.py` refuses the first Read of
  an image over 250,000 pixels; if `view.py` cannot do the job, say why and
  Read it again.

## Code before wine

The decompile and the dry run answer most questions (`CLAUDE.md`, "Wine is
the last resort").
- **Shots**: film the original once per screen; keep shots in
  `research/<engine>/local/shots/` (git-ignored) under a name that says what
  they show. `research/tools/shots.py find <words>` first (indexes
  `research/*/local/shots/INDEX.tsv`), then `shots.py add <file> <game>
  <scene> <page> <what>` for each new one.
- **Film list**: a doubt only the original can settle goes in
  `research/<engine>/local/film-list.md` (`| game | scene / page | question |
  asked by |`) and the work carries on. A quick check that a generated save
  or patched script reaches its moment may run at once. The hook
  `.claude/hooks/emu_run.py` refuses direct wine/Xvfb/Vita3K/Ryujinx runs.
- **Wine prefix**: a scratch `WINEPREFIX` per game; replace its
  `drive_c/users/<user>/{Documents,Desktop,...}` symlinks with real folders
  before the first run (games save there, and wine shares the user's home
  otherwise); run through `headless-run.sh` with
  `LANG=ja_JP.UTF-8 LC_ALL=ja_JP.UTF-8`; reach the moment by patching
  scripts or saves; `wineserver -k` when done.

## Build and test details

**Memory caps**: run builds, engine runs and emulators that may grow
through `research/tools/capped.sh <GiB> <cmd> [args...]` (the bash hook
refuses a raw `systemd-run -p MemoryMax`: a kernel OOM kill pops a
notification on the user's desktop). It stops cmd at 90 % of GiB (exit
137), refuses to start (exit 75) with less than GiB+2 available, else
passes cmd's exit code through; works inside `headless-run.sh`. Self-test:
`research/tools/test_capped.sh`.

**Engine**
- Build: `research/tools/build-engine.sh [engine dir] [build dir]` (cmake +
  make; `SDL2_MIXER_PREFIX=<prefix>` when the system has no SDL2_mixer) →
  `engine/build/conscripter`. Web: `cd engine/script && EMSDK_HOME=<emsdk
  dir> bash ./cross_web.sh`.
- Headless run: `SDL_VIDEODRIVER=dummy SDL_AUDIODRIVER=dummy timeout -k 5 N
  engine/build/conscripter -r .` with the game folder as working directory
  (else `default.ttf` isn't found). Screenshot: `print 1:getscreenshot
  800,600:savescreenshot "x.bmp"`.
- `python3 -I tests/extras_smoke.py <full build>`: opens the extras' menu
  headless with every lock open and checks each mode is a button and opens
  (the trace). Run it on every full build handed to the user; the snapshot
  sees only the screens' text and route runs skip them.

**`tests/snapshot.sh`**: dry-runs every game present (links in
`$CONS_GAMES`, see the script) and says which games' `0.txt` changed. Run
it after any shared-code change; a change to another game's script must be
deliberate and said in the commit.
- Flags: `--update` rewrites `tests/snapshots/`; `--diff origin/main` builds
  that ref in its own worktree and shows each game's 0.txt changes (and the
  first lines of each changed `screens/*.lua`/`ons_args` diff: the snapshot
  holds a sha256 per such file); `--map <rules>` checks a change is only a
  rename; `--no-cache` reruns dry runs (use it after editing a game
  install); `--accept-gaps` passes the gap/score gates (say why in the PR).
- Speed: dry runs go two at a time when memory allows (each game's last peak
  plus 2 GB free); a game's run is reused while binary, arguments, engine
  command table and script are unchanged.
- Conflict in `tests/snapshots/`: rebase, rerun `--update`, explain each
  changed game's 0.txt in the PR body.
- Gates, exit 2: a game's `gaps`/`left_out` rose against
  `<game>.metrics.json`, or a channel of its `score` (or the total) fell by
  over 0.01 (`research/tools/score_fell.py`); the same for `parity`
  `short_ms` (per scene, the motion time the source asks for against what
  `0.txt` animates there, shortfalls summed; a rise is a motion or fade the
  conversion lost; `--parity-report <file>` lists the scenes; it cannot see
  a loss hidden by extra motion elsewhere).
- Gate, exit 3: `timing` (calls noted with a reason that loses time,
  `metrics.rs` `TIMING`) rose. `--accept-gaps` does not pass it: fix it, or
  `--update` and say why.
- Lint ceilings: a game fails when a count of the dry run's `lint:` lines
  (jump to no label, label defined twice, string past its line, `?` in
  text, file not written, ...) rises above its ceiling in `<game>.lint`
  (`N  kind`; a kind not there is 0). A count below prints `lower to N`;
  `--update` writes lower counts but never raises one: a new ceiling goes in
  by hand, reason in the PR. CI also runs `dropped_lint.py`, with a per-game
  ceiling that only goes down.
- "File not written" checks 0.txt's file names against what each engine
  records it writes (`conv_engines::output::record`, done by
  `Output::write`/`Output::have`); code writing a file outside `Output`
  calls `record` with the output-relative name, in a dry run too. Movies and
  `.webp` aren't checked.

## Matching the original

Only for a bug seen in play, with the user's yes; code and oracle first
(order in `docs/new-game.md`). Wine rules: "Code before wine".

**Reaching the moment**
- Patch: a save from `research/tools/<engine>_save.py` (`xuse_save.py`,
  `nitroplus_save.py`), patched scripts in a symlink farm, an edited save;
  clicking by hand last.
- Filming runs only through `headless-run.sh` or a wrapper; scratch prefix,
  real user folders, `LANG=ja_JP.UTF-8`, no audio. Slow a transition by
  patching its duration; compare by numbers (`compare -metric AE -fuzz 0` on
  a crop); ours with `--shot-effects 25,50,75`. Consoles:
  `docs/games/tsukihime.md`, `docs/games/oretsuba.md` ("Testing").

**The exe's own code**
- `research/tools/exe_call.py <exe> <addr|*table+4*i> [args] --ret float`
  (run as `research/tools/.venv/bin/python -I`; setup once per machine:
  `research/tools/setup-venv.sh`, unicorn plus self-test) maps the exe into
  Unicorn and calls one address (int, float, pointer arguments; imports and
  `--stub` addresses return 0 or a chosen value); `Exe(path).call` sweeps.
  Example: `research/nitroplus/en_tempo_exe.py`. Formats: x86-32 PE, Linux
  ELF, Vita eboot (no SCE relocations), Switch NSO; not Windows x86-64.
  No OS: write the state in first (`Exe.load_dump`); stub the clock
  (`stub_import("WINMM.dll!timeGetTime", ...)`). Unstubbed imports return 0, except `memset`/`memcpy`/`memmove`,
  which do their work.
- **Snapshot oracle** (what the original does over time): (1) a probe scene
  with a marker around each step (Nitroplus: `Delete("__dump_<label>")`;
  `research/nitroplus/sched_probe.nss`); (2) dumps at each marker:
  `ntrace.sh --probe <file> --dumps <dir>`, or `--scene-dumps` (compare with
  `research/tools/scene_check.py`); Vita: `gdbdump.py --scan`; Switch:
  `gdbdump.py --get-info`; (3) `Exe.load_dump(prefix)`, then
  `research/tools/sched_harness.py` stubs clock, heap and TLS and steps the
  frame function (`research/nitroplus/sched_django.py <exe> <dumps>
  --cases`); (4) the numbers become test fixtures. Per-frame state:
  `ntrace.sh --frames`, `research/tools/frame_diff.py`.
- **Wine API log**: `WINEDEBUG=+timestamp,+relay` with `RelayInclude` set
  (`wine reg add 'HKCU\Software\Wine\Debug' /v RelayInclude /d
  'kernel32.Sleep;winmm.timeGetTime;...'`), into `research/<engine>/local/`;
  group `Call` lines by `ret=` for the frame loop. COM calls are invisible
  (`+dsound` for sound).
- **Breakpoint trace**: `research/tools/wine_bptrace.py names.tsv [name ...]
  -o bp.gdb`, `winedbg --gdb --no-start 'C:\<dir>\game.exe'`, `gdb -batch -ex
  'target remote localhost:PORT' -x bp.gdb > bp.log`; fold with
  `wine_bptrace.py --fold bp.log`. 32-bit stack arguments only; break on
  handlers, not the frame loop.
- **Branch coverage**: the trace DLL arms one-shot `int3` breakpoints on the
  `names.tsv` addresses (instruction starts only) and logs `hit` records;
  `research/xuse/cover.py` marks each row `runs:`/`not:`/`never runs`.

**Traces of the original**
- **Trace DLL**: the debug trace's records from the original: a mingw DLL
  injected by a launcher hooks the dispatch table, counts frames at the
  loop's `Sleep`, turns on auto mode. `research/xuse/tracedll/` (`xtrace.sh`),
  `research/nitroplus/tracedll/` (`ntrace.sh`; `at` is `<file>:<line>`, times
  in ms); Switch `research/typemoon/switchtrace/`; Vita
  `research/oretsuba/vpatch.py`.
- **Reference traces**: `research/tools/reftrace.sh <game>` builds our dry
  run, map and trace, then runs the original on the same route in the
  background (3 GB cap) into `out/<game>-reftrace/`. `reftrace.sh status
  <game>` ~5 min after the start and every 30-60 min; `summary <game>` at the
  end. Route inputs (`*-route.arg`, `*-choose.txt`, `*-patch/`, `keys.sh`)
  live in `$CD` (default `out/reftrace`).
- **Trace speeds**: `SPEED=N` (`xtrace.sh`, `ntrace.sh`) runs the original's
  clock N times fast, records keep game time: Saihate 4, Muramasa and Django
  8, Django's reference run 1; Switch `IDLEA=0.3`. Sound and loading aren't
  scaled: `trace_diff.py` (`--orig-speed N`) counts them as `fast-voice`.
- **Trace diff**: ours with `--source-map <file>`, `--trace --auto-play N
  --real-time` and the original's choices, then `research/tools/trace_diff.py
  orig.jsonl ours.jsonl --map <file> [--fps N | --ms] [--merge-into OP]
  [--top N]` (`--self-test`): per page, differing text, instructions one
  side ran (`missing`, `merged`, `quiet`), times off by over a frame. Triage
  times over 1 s first.
- **Picture diff**: original `ntrace.sh --shots <dir> [--shot-delay MS]`
  (a frame per text block); ours `--trace --auto-play N --real-time
  --shot-every 1 --shot-dir <dir>`; then `research/tools/picdiff.py
  orig.jsonl <orig dir> ours.jsonl <our dir> --map <map> --ms --thread 0
  --mask WxH+X+Y` (`--fuzz 15` for lossy WebP shots): per page differing
  pixels, RMSE and box, worst first, with views of the worst. A difference
  that moves with `--shot-delay` is timing.
