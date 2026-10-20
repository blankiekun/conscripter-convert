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
  (`docs/engines/README.md`); optional, its `engine_doc` stem set in
  `build.rs`.

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

## Findings notes

`docs/games/<game>.findings.tsv` is optional notes on what the exe does
(seen, inferred or unknown); no lint checks it.

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

Optional, for a bug seen in play: `docs/tools.md`, "Matching the original"
(manual, patching, filming, traces, picture diff, oracle, one exe function).
