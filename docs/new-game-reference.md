# Converting a new game: reference

The detail behind `docs/new-game.md`'s checklist, by heading. Commands
run from the repository root; game folders are `<install>`, outputs
`out/<game>-cons`. Everything under "Matching the original" is optional:
use it only for a bug seen in play (`docs/new-game.md`, "The goal").

## First contact

- Look the format up first in other extractors' catalogues (references,
  not dependencies): [GARbro](https://github.com/crskycode/GARbro)
  `ArcFormats/`, [arc_unpacker](https://github.com/vn-tools/arc_unpacker)
  `src/dec/`, [vgmstream](https://github.com/vgmstream/vgmstream)
  `doc/FORMATS.md` (`vgmstream-cli -m` checks our audio).
- Windows game: `research/tools/firstcontact/first_contact.py <install>
  [--ghidra <export> | --ghidra-run]` writes
  `out/<install name>-first-contact/report.md`: a "Known family" block
  (nearest game, readers that carry over; console dumps too), engine
  guesses, imports, files by extension, RTTI; with an export, `hooks.tsv`
  (candidate dispatch, wait-loop and present functions).
- Then **one** logged run: `fc.sh <install> --hooks <out>/hooks.tsv`
  (headless, 300 s, auto clicks): file reads with offsets, DirectSound
  plays, presents, input, timers and their exe call sites in
  `trace.jsonl.gz`; `summary.txt` ranks the breakpoints. Shots are lossy
  WebP (`--lossless` for pixel compares). No script positions, text,
  XAudio2 or Vulkan. Answer later questions from this log.

## A new engine's code

Follow the shape of `xuse/`, `nitroplus/`, `mages/` in
`crates/conv-engines/src/`; one game's rules in a module of its own.

- **Module** `crates/conv-engines/src/<engine>/`: one file per format,
  `convert.rs` (lowering), `assets.rs` (an asset source behind a trait,
  as Nitroplus' `Assets`, so lowering is testable without the game),
  `build.rs`.
- **Shared code**: `conv_engines::output::Output` (dry-run gate, errors,
  `write`, `write_png`, `image_size`), `conv_engines::media` (`ffmpeg`
  via a `.part` file, `mp4_options`), `crates/conv-cli/src/build.rs`
  (`check_output`, `write_game`, `pack`, `summary`). Keys and ciphers
  come from the exe; throwaway Python in `research/<engine>/` (`python3
  -I`, paths as arguments, no game data committed).
- **`build.rs`**: `pub fn build(game: &Path, …, opts: &Opts) ->
  Result<Built>` writes assets through `Output` and returns the IR with
  the folders `--webp` and `--audio-codec` act on; `OUTPUT_DIRS` lists
  the asset folders packed into `arc.nsa`. Images stay PNG (`--webp`
  via `build::WebpArgs`/`build::webp_tree`); Ogg passes through with
  `LOOPSTART`/`LOOPEND` tags; movies to H.264/AAC MP4 for `mpegplay`.
- **CLI**: a subcommand in `crates/conv-cli/src/main.rs` with
  `#[command(flatten)] build: build::BuildArgs`, calling
  `build::run(build, &title, OUTPUT_DIRS, |opts| <engine>::build::build(…))`;
  format tools (`list`, `extract`, `dump`) in
  `conv-cli/src/tools/<engine>.rs`. Give it `--dry-run`.
- **Variables**: game flags in the free ranges at the top of
  `crates/conv-core/src/alloc.rs` (the build refuses a clash); a new
  settings switch takes the next free index in `switches.rs`. Never move
  a number once a build is out.
- **Registry and GUI**: `crates/conv-engines/src/registry.rs` (id, name,
  engine, marker files, command), the same in `gui/games.py` with a fake
  layout in `gui/tests/test_games.py`; `tests/snapshot.sh --update <id>`
  for the baseline and `<game>.metrics.json` (each source call opened and
  closed on `conv_engines::metrics::Metrics`; on purpose left out:
  `Metrics::left_out`). On a snapshot conflict rerun `--update`, never
  take one side.
- **Tests**: lowering on a tiny synthetic input (Nitroplus' `Fake`
  assets), the readers' corner cases.
- **Engine reference**: `docs/engines/<engine>.md`
  (`docs/engines/README.md`), one row per command the dry run counts,
  its `engine_doc` stem set in `build.rs`.

## Decompiling the exe

- Find the dispatch table: per handler, the strings it uses, the source
  file it logs, the RTTI class it builds.
- `research/tools/ghidra/ghidra_export.sh <exe>
  research/<engine>/local/ghidra --names research/<engine>/names.tsv`
  writes one pseudo-C file per function (`decomp/<addr>_<name>.c`, with
  callers, callees, strings) plus `functions.tsv`, `strings.tsv`,
  `sha256.txt`. Grep those first. Every name you identify goes in
  `names.tsv` (`address<TAB>name<TAB>comment`); rerun (~30 s). Options
  in the script's header (`--rtti`; `-- -processor …` for ARM;
  `-postScript ThumbPointerFunctions.java` for a Vita eboot).
- **Packed exe** (odd sections, entropy near 8, entry in the last
  section): work on a copy in `local/`: (1) an unpacked build of the same
  version the user owns, (2) a standard unpacker, (3) dump at the
  original entry point under wine and rebuild imports.
- **Library signatures**: the export's Function ID analyzer names MSVC
  CRT functions (`_ftol2`, `memcpy`); check `functions.tsv` before naming
  by hand. zlib/libpng/libjpeg need an extra `.fidbf` (untested).
- **Carrying names between builds** (JP/EN, versions): Ghidra Version
  Tracking, `ghidriff` or BinDiff; take high-similarity matches into the
  other build's `names.tsv` as inferred (untested).

### Running one exe function

`research/tools/exe_call.py` maps the exe into Unicorn and calls one
address with int, float and pointer arguments (imports and `--stub`
addresses return 0 or a chosen value); `Exe(path).call` sweeps. Setup:
`research/tools/setup-venv.sh`. Example:
`research/nitroplus/en_tempo_exe.py`. No OS: state the game set up must
be written in first (`Exe.load_dump`); stub the clock
(`stub_import("WINMM.dll!timeGetTime", …)`) to step frames. Works on
x86-32 PE, Linux ELF, Vita eboot (SCE relocations not read), Switch NSO;
not Windows x86-64.

### Snapshot oracle

For "what does the original do over time", step its own code on a
memory dump instead of filming:

1. A probe scene with a marker statement around each step (Nitroplus:
   `Delete("__dump_<label>")`; `research/nitroplus/sched_probe.nss`).
2. Dumps at each marker: `ntrace.sh --probe <file> --dumps <dir>` (1x),
   or `--scene-dumps` at every scene start (compare with our sprites:
   `research/tools/scene_check.py`). Vita: `gdbdump.py --scan`; Switch:
   `gdbdump.py --get-info` (modules only).
3. `Exe.load_dump(prefix)` maps it; `research/tools/sched_harness.py`
   stubs clock, heap and TLS and steps the frame function
   (`research/nitroplus/sched_django.py <exe> <dumps> --cases`).
4. The numbers become test fixtures and seen `(scheduler)` rows.
   Per-frame state: `ntrace.sh --frames`, `research/tools/frame_diff.py`.

## Mapping the whole exe

Extend the export in passes, from what the corpus uses:

1. Name the dispatch table, the value evaluator, the settings setter.
2. Recover callbacks that aren't functions yet: `grep -ho
   '&LAB_[0-9a-f]*' decomp/*.c`, append as `cb_<addr>`, rerun until none
   is new (skip `puStack_`/`pcStack_` stores, SEH handlers).
3. Name CRT helpers early: Ghidra loses x87 maths around `_ftol2`.
4. What the parser accepts: `strings.tsv` plus stack strings
   (`research/nitroplus/stackstrings.py`), character-comparison chains;
   note how constants match (exact, case, character sums).
5. **Coverage table** per built-in, system variable, text tag, script
   file: corpus count, what the exe does, what we do, "not converted"
   notes. Work only where they differ and the corpus uses it. A row is
   converted only when every argument is; unread arguments are noted by
   the dispatch (`<Builtin> argument <i> ignored`). Each row says where
   it goes: engine-piece data, Lua, or a new command.
6. **Special values and units** per argument: comparisons against
   constants before use (`-1`, `-32766`, `0x8000`, `-100000`), units (ms
   or frames, px at which scale, percent or 1/256, degrees).
7. `python3 -I research/tools/argcensus.py out/<game>/0.txt`: explain
   every flagged value.
8. An off-screen or out-of-range finding is "the original's data" only
   when seen in its code or data.
9. Also: what runs during waits, easing curves, branches per argument
   (counted in the metrics), state a new picture inherits or a load
   resets, anchors and blend formulas, out-of-range input, randomness,
   sound volume scale, text line breaks and speed units.
10. **Scripts too**: every script and data file gets a status (compiled,
    replaced by a screen of ours, read for data, unused and why); every
    name the scripts call with its count (`nss refs`, `nss builtins`,
    `nss route`), since the dry run only notes code it reaches.
11. Write back: `names.tsv` (grep it before reading a function), the
    engine's built-ins reference, gaps in Status. When done, a second
    agent re-derives 10 random seen rows (fixed seed) from the decompile.

### Timing and overlap

Per timed command: does the script wait, go on, or wait part; ms or
frames at what rate; what a click does mid-effect. Then overlap: what an
unwaited action does while later commands run, what a wait or a 0 ms
action does to actions in flight, replace or delete while moving. Record
these as `(scheduler)` rows, one per question in `findings_lint.py`'s
`QUESTIONS` (`research/tools/ghidra/callmap.py <export> --names
<names.tsv> --tsv` drafts them); the last column names the check (a test
`file.rs::fn` or `ledger: "<reason>"`). Test the game's own helpers that
chain timed built-ins on the composed output.

## Findings table

`docs/games/<game>.findings.tsv`: one row per kind the dry run's metrics
count (`tests/snapshots/<game>.metrics.json`), the kind in the first
column, each marked **seen** (code, script or trace shows it),
**inferred** (from names or layout) or **unknown** (with the cheapest
check that settles it). Inferred becomes seen only by a decompile
lookup. A negative search says what it searched. Name the exe build for
every address. `research/tools/findings_lint.py` (CI) ties table and
metrics; per game `NO_ROW_MAX`, `OPEN_MAX`, `SCHED_MAX` only go down; a
new game starts at 0. Every counted command also needs its row in
`docs/engines/<engine>.md`.

## Engine pieces

Before a new command, see whether a general piece takes it as data:

- **Sprite animation**: `spmotion` keys (offset, zoom, angle, alpha),
  a curve per key from an `spcurve` table (read from the exe or computed
  with `exe_call.py`); `spblend` adds or multiplies.
- **Transitions mixing two pictures in place**: `cons_rule.dll` with a
  strip of mask frames made by running the original's kernel on a black
  old and a white new picture. Pixel-moving transitions need a plugin.
- **Logic and parallel processes**: Lua. `luasub name` makes
  `NSCOM_name` a command; globals are saved; `cons_process(name, "fn",
  …)` resumes a coroutine each frame and restarts after a load (keep
  progress in globals). conv-cons writes no `system.lua` yet.

A new command: in `engine/src/cons/` (`ONScripter_cons.cpp`), not
opt-in, building for the web, state kept in saves; effects are
`cons_*.dll` effect-99 plugins, screen effects use the
`screenfilter`/`screenshake` slots; documented in the engine's
`COMMANDS.md`; engine PR first (`CLAUDE.md`, "Rules").

## Checking a build with the debug trace

The engine plays the game by itself and writes one JSON line per page,
screen or sound command, choice, missing file, error or crash
(`engine/DEBUG.md`). Headless, under `timeout -k 5`, a fresh
`--save-dir` per run:

```sh
cd out/<game>-cons
SDL_VIDEODRIVER=dummy SDL_AUDIODRIVER=dummy timeout -k 5 3600 \
    ../../engine/build/conscripter --root . --save-dir <scratch>/saves \
    --trace <scratch>/t.jsonl --trace-ignore <UI sprites> --auto-play 0
echo $?   # 0 end, 1 stuck, 2 script error, 3 startup failure, 128+n signal
python3 -I ../../engine/tools/cons_trace_check.py <scratch>/t.jsonl --script 0.txt
```

- The checker ranks errors, crashes, missing files, leftover sprites,
  filters and loops, cut voices, leaked text, long waits, memory growth,
  unreached labels at `*label:line`, and prints a `--shot-pages` option
  for those pages. False positives go in `cons_trace_check.json` beside
  the trace, only after looking. A `crash` record has the gosub chain and
  last 50 commands.
- `--auto-choice first|cycle|random|untaken`, `--seed n`;
  `engine/tools/cons_explore.py` reruns until no new label is reached.
  `--replay t.jsonl` repeats a run's choices (`{"t":"choice","at":
  "*label:line","picked":2}`); compare with `cons_trace_check.py --diff`.
- Start anywhere: `--goto <label> --vars <file>` (`%312=876` per line);
  vars from `--dump-vars`, F8, or `engine/tools/cons_trace_vars.py
  t.jsonl --page N`.
- Long runs: a renamed engine symlink (others `pkill -x conscripter`),
  `research/tools/capped.sh <GiB> <cmd>`, `--checkpoint-every N` and
  `--load n`; `--roundtrip-every N` finds state that doesn't survive a
  save.
- More in `engine/DEBUG.md` (`--trace-vars`, `--shot-at`,
  `--shot-effects`, …). Over another agent's build:
  `research/tools/scratch.sh`, `research/tools/scene_ab.py`.

## Route simulation

1. Save a public walkthrough (routes, unlock and loop order, true-ending
   conditions) in the document's "Walkthrough", with its source.
2. Table the progression variables: meaning, where set, reset and read,
   save or system data.
3. Simulate the source script from the decoded bytecode with the
   original's run-time rules (per page: text, links with targets,
   variables), and the converted `0.txt` on the same click path; diff
   page by page, including the wrong-order paths the original refuses.
   Then `--replay` each path and compare `--trace-vars`. Model:
   `research/xuse/routesim.py`, `research/xuse/nstrace.py`.
4. Lint for the class of bug: links shown only under a condition, a
   per-message record jumping to another message, two links on one text.
5. Unlocks: list every button and item; write what the original needs
   before showing or enabling it (from the code, or say the rule is
   ours); check a fresh start and a 100 % state (`--vars`).
6. Settings (default, effect, kept across quitting: skip, auto, voice,
   volumes, text speed), input (keys, wheel, right-click; touch leaves
   every screen; clicks mid-fade skip no more) and persistence (relaunch,
   reopened tab, older save, return to title) as the original.

## The web build

```sh
conscripter-convert <command> <install> -o out/<game>-web --font <ipag.ttf> --loose
python3 -I engine/src/cons_web/conscripter_index.py -i out/<game>-web \
    -o out/<game>-web/cons_index.json --title <game> --lazyload
cp engine/build_web/conscripter.{html,js,wasm} engine/build_web/favicon.ico out/<game>-web/
python3 -m http.server 8000 -d out/<game>-web   # open /conscripter.html
```

Web engine: `cd engine/script && EMSDK_HOME=<emsdk> bash
./cross_web.sh`. Check loading time, sound once allowed, saves across a
reload and a closed tab, touch, picture memory on long routes. Headless
trace: `engine/tools/web_run.py --query 'auto-play=0'` (`docs/tools.md`).
Note size and file count in the document.

## Test saves at chosen scenes

```sh
python3 -I research/tools/test_saves.py out/<game>-cons <slots.txt> -o <game>-test-saves.zip
```

`slots.txt`: one `slot label description` a line. The engine starts at
each label (`--goto`), saves before it runs (`--checkpoint-start
--checkpoint-slot`), adds a thumbnail and the slot's text. The zip is
what the web player's Import Saves and a native save folder take; it
leaves out `gloval.sav` and `kidoku.dat`. `--vars-dir <dir>` sets each
label's `<dir>/<label>.txt` variables first
(`research/nitroplus/route_vars.py` writes them for Muramasa).

## Matching the original

Optional: only for a bug seen in play; code and oracle first.

- **Manual first**: extract it to `research/<engine>/local/manual/`;
  note which screens it shows (`docs/tools.md`, "Manuals").
- **Reach the moment by patching**: (0) a save from
  `research/tools/<engine>_save.py` (`xuse_save.py`,
  `nitroplus_save.py`; one template save from the game); (1) patched
  scripts in a copy, in a symlink farm; (2) an edited save; (3) clicking
  by hand last.
- **Filming**: only through `research/tools/headless-run.sh` or a
  wrapper (`xtrace.sh`, `ntrace.sh`, `vita-emu.sh`, `switch-emu.sh`); a
  scratch prefix with real user folders, `LANG=ja_JP.UTF-8`, no audio.
  Slow a transition by patching its duration; compare by numbers
  (`compare -metric AE -fuzz 0` on a crop). Fit the kernel's curve from
  the exe to the frames; ours: `--shot-effects 25,50,75`. Consoles:
  `docs/games/tsukihime.md`, `docs/games/oretsuba.md` ("Testing").

### Wine API log

`WINEDEBUG=+timestamp,+relay` with `RelayInclude` set in the prefix
(`wine reg add 'HKCU\Software\Wine\Debug' /v RelayInclude /d
'kernel32.Sleep;winmm.timeGetTime;…'`), into `research/<engine>/local/`;
count with `grep`/`awk`, group `Call` lines by `ret=` to find the frame
loop. COM calls (`Present`, buffer `Play`) are invisible: use
`+dsound` for sound.

### Breakpoint trace

`research/tools/wine_bptrace.py names.tsv [name …] -o bp.gdb`, then
`winedbg --gdb --no-start 'C:\<dir>\game.exe'` and `gdb -batch -ex
'target remote localhost:PORT' -x bp.gdb > bp.log`; fold with
`wine_bptrace.py --fold bp.log`. 32-bit stack arguments only. Break on
handlers and per-frame callbacks, not the frame loop (each stop costs
ms). A waited command takes one frame more than its time.

### Trace DLL

The debug trace's records from the original: a mingw DLL injected by a
launcher (`CreateRemoteThread(LoadLibraryA)`, no game file touched)
hooks the dispatch table, counts frames at the loop's `Sleep`, and turns
on the game's auto mode. `research/xuse/tracedll/` (`xtrace.sh`),
`research/nitroplus/tracedll/` (`ntrace.sh`; NSS: hook the
interpreter's calls, `at` is `<file>:<line>`, times in ms). Switch:
`research/typemoon/switchtrace/`; Vita: `research/oretsuba/vpatch.py`.

### Trace diff

Build ours with `--source-map <file>` (filled from `Script::source_map`),
trace with `--trace --auto-play N --real-time` and the original's
choices, then `research/tools/trace_diff.py orig.jsonl ours.jsonl --map
<file> [--fps N | --ms] [--merge-into OP] [--top N]`: per page, text that
differs, instructions one side ran (`missing`, `merged`, `quiet`), times
off by more than a frame. Triage time findings over 1 s first; read
`missing` timed ops too.

### Reference traces

`research/tools/reftrace.sh <game>` builds our dry run, map and trace,
then runs the original on the same route in the background (3 GB cap)
into `out/<game>-reftrace/`. `reftrace.sh status <game>` ~5 min after
the start and every 30-60 min; `summary <game>` at the end. Route inputs
(`*-route.arg`, `*-choose.txt`, `*-patch/`, `keys.sh`) live in `$CD`
(default `out/reftrace`). Status records how far it got and the top
`trace_diff.py` kinds.

### Trace speeds

`SPEED=N` (`xtrace.sh`, `ntrace.sh`; `research/tools/trace_speed.h`)
runs the original's clock N times fast; records keep game time, so they
diff as 1x. Good values: Saihate 4, Muramasa and Django 8, Tsukihime
`IDLEA=0.3`; Django's reference run uses 1. Sound and loading aren't
scaled: `trace_diff.py` (`--orig-speed N` or the meta record) counts
those steps as `fast-voice`.

### Picture diff

Original `ntrace.sh --shots <dir> [--shot-delay MS]`; ours `--trace
--auto-play N --real-time --shot-every 1 --shot-dir <dir>`. `research/tools/picdiff.py orig.jsonl <orig dir>
ours.jsonl <our dir> --map <map> --ms --thread 0 --mask WxH+X+Y`: per
page differing pixels (`--fuzz`, 15 for lossy shots), RMSE and box,
worst first. A difference that moves with `--shot-delay` is timing.

### Branch coverage

The trace DLL arms one-shot `int3` breakpoints on the addresses in
`names.tsv` and the findings table's source column (instruction starts
only, check with `objdump -d`) and logs `hit` records; a script marks
each row `runs:`/`not:` or `never runs` (`research/xuse/cover.py`).

### Film list

Search shots first: `research/tools/shots.py find <words>` (indexes
`research/*/local/shots/INDEX.tsv`; add with `shots.py add <file> <game>
<scene> <page> <what>`). A doubt only the original settles goes on
`research/<engine>/local/film-list.md` (`| game | scene / page |
question | asked by |`); carry on without running it. A quick check that
a generated save or patched script reaches its moment may run at once.
`.claude/hooks/emu_run.py` refuses direct runs.
