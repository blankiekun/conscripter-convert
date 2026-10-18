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
- Against the original (a trace of it, `docs/new-game.md` steps 22-23):
  `research/tools/trace_diff.py orig.jsonl ours.jsonl --map <the
  build's --source-map file>` prints the pages' text, command and time
  differences (`--self-test` tests it). A wine original's trace
  (`xtrace.sh`, `ntrace.sh`) runs N times as fast with `SPEED=N`, its
  times still comparable (4 for 最果てのイマ, 8 for the Nitroplus games);
  the Switch one with `IDLEA=0.3` (`docs/new-game.md`, "Trace speeds").
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
  `switch-emu.sh` get the user bus for `systemd-run` only (`HEADLESS_USER_RUNTIME`).
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

## Reading files cheaply

For the docs and scripts read most often, use the helpers in
`research/tools/` (run with `python3 -I`):
`mdsec.py <file.md>` lists the headings, `mdsec.py <file.md> Status`
prints that section only (game docs, `docs/lessons.md`, `CLAUDE.md`);
`label.py 0.txt <label> [lines]` prints one label's block with line
numbers, `label.py 0.txt -l <text>` finds labels; `txtdiff.py old.txt
new.txt [--map 's/old/new/'|--rules <file>]` prints changed hunks under
their labels, counts them by command (numbers masked) and says whether
the rules make it only a rename (`--self-test` tests it). Source code is the
biggest text cost: `code.py <file.rs|.cpp>` lists its items with line
numbers, `code.py <file> <name> [lines]` prints one fn, type, impl or
C++ `Class::name` with its doc comments; use it instead of paging
through a file with `sed`.
