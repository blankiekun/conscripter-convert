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
- Frames mid-effect: the trace's `--shot-effects 25,50,75`. The older
  patched-script runs: `docs/new-game.md` step 9 and "When the trace
  can't do it". On Xvfb always set
  `WAYLAND_DISPLAY=`, or the window opens on the user's desktop.
- Test scripts: UTF-8 with `--enc:utf8` in `ons_args`, as the builds;
  a Shift-JIS one with `iconv -t cp932` (not `sjis`, which turns `\` into
  `＼`) and `--enc:sjis`.
- Read a variable headless: `lsp 999,"X:"+$1,0,0` prints "can't find
  file X:..." on stderr.

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
