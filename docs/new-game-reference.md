# Converting a new game: reference

Detail behind `docs/new-game.md`, by heading. Commands run from the repository root; game folders are
`<install>`, outputs `out/<game>-cons`.

## First contact

- Prior work first, as references, not dependencies: fan-translation tools for the game or its engine (they
  solved the same formats and text), open-source reimplementations of the engine (Committee of Zero's `impacto`
  for MAGES), extractors: GARbro `ArcFormats/`, arc_unpacker `src/dec/`, vgmstream `doc/FORMATS.md`
  (`vgmstream-cli -m` checks our audio). Read their code, write our own.
- Windows game: `research/tools/firstcontact/first_contact.py <install> [--ghidra <export> | --ghidra-run]`
  writes `out/<install name>-first-contact/report.md`: "Known family" (nearest game, readers that carry over;
  console dumps too), engine guesses, imports, files by extension, RTTI; with an export, `hooks.tsv` (candidate
  dispatch, wait-loop and present functions).

## A new engine's code

Follow `xuse/`, `nitroplus/`, `mages/` in `crates/conv-engines/src/`; one game's rules get a module of their own.

- **Module** `crates/conv-engines/src/<engine>/`: a file per format, `convert.rs` (lowering), `assets.rs` (asset
  source behind a trait, as Nitroplus' `Assets`, so lowering is testable without the game), `build.rs`.
- **Shared code**: `conv_engines::output::Output` (dry-run gate, errors, `write`, `write_png`),
  `conv_engines::media` (`ffmpeg`, `mp4_options`), `crates/conv-cli/src/build.rs` (`check_output`, `write_game`,
  `pack`). Throwaway Python goes in `research/<engine>/` (`python3 -I`, paths as arguments, no game data).
- **`build.rs`**: `build(game, ..., opts) -> Result<Built>` writes assets through `Output` and returns the IR;
  `OUTPUT_DIRS` lists the folders packed into `arc.nsa`. Images stay PNG (`--webp` via `build::webp_tree`).
- **CLI**: a subcommand in `crates/conv-cli/src/main.rs` (`#[command(flatten)] build: build::BuildArgs`) calling
  `build::run(...)` as the existing engines do; format tools in `conv-cli/src/tools/<engine>.rs`; `--dry-run`.
- **Variables, registry, GUI**: free ranges in `crates/conv-core/src/alloc.rs`, next free index in `switches.rs`
  (numbers stay once a build is out); `registry.rs`; `gui/games.py` with `gui/tests/test_games.py`. The step 9
  baseline writes `<game>.metrics.json`; on a conflict rerun `--update`.
- **Tests**: lowering on a tiny synthetic input (Nitroplus' `Fake` assets).
- **Engine reference** (optional): `docs/engines/<engine>.md` (`docs/engines/README.md`) with `engine_doc` set
  in `build.rs`.

## Decompiling the exe

- Find the dispatch table: per handler, the strings it uses, the source file it logs, the RTTI class it builds.
- `research/tools/ghidra/ghidra_export.sh <exe> research/<engine>/local/ghidra --names
  research/<engine>/names.tsv` writes a pseudo-C file per function (`decomp/<addr>_<name>.c`) plus
  `functions.tsv`, `strings.tsv`; grep those first. Record each identified name in `names.tsv`
  (`address<TAB>name<TAB>comment`) and rerun (~30 s). Options are in the script's header (`--rtti`;
  `-- -processor ...` for ARM; `-postScript ThumbPointerFunctions.java` for a Vita eboot).
- Packed exe (odd sections, entropy near 8): work on a copy in `local/`: an unpacked build the user owns, a
  standard unpacker, or a dump at the entry point under wine.
- Exe notes go in `docs/games/<game>.findings.tsv` (no lint). The export's Function ID analyzer already names
  MSVC CRT functions; read `functions.tsv` before naming by hand.

## Lifting from the exe

What the exe's tables and self-contained functions hold matters; how they work does not. Lift them:

- **Tables**: layouts, button rectangles, timings, text speeds, curves, CG and music lists often sit as constant
  arrays in the data sections. Find the array through the code that indexes it, then read it from the user's
  exe with a script; these numbers beat measuring screenshots.
- **Self-contained functions** (decompression, decryption, name hashing, curves, layout math): call them with
  `exe_call.py` (`docs/tools.md`, "The exe's own code") for the outputs or a sweep; when the build needs one,
  port it to Rust from the decompile and test the port against `exe_call.py` outputs. The repo holds our own
  code only; the decompiled C stays in `local/`.
- **Command log**: when many script commands stay unclear after the decompile, one `wine_bptrace.py` run with a
  breakpoint on the dispatcher (`hooks.tsv`) logs every command with its arguments and time; diff it against
  our trace. It is a wine run: `CLAUDE.md`, "Wine is the last resort".

## Engine pieces

Before a new command, check whether a general piece takes it as data:

- **Sprite animation**: `spmotion` keys (offset, zoom, angle, alpha), a curve per key from an `spcurve` table
  (from the exe or computed with `exe_call.py`); `spblend` adds or multiplies.
- **Transitions mixing two pictures in place**: `cons_rule.dll` with a strip of mask frames made by running the
  original's kernel on a black old and a white new picture. Pixel-moving transitions need a plugin.
- **Logic and parallel processes**: Lua (`luasub name` makes `NSCOM_name` a command; `cons_process(name, "fn",
  ...)` resumes a coroutine each frame).

## Checking a build with the debug trace

The engine plays itself and writes a JSON line per page, screen or sound command, choice, missing file, error or
crash (`engine/DEBUG.md`). Run headless under `timeout -k 5` with a fresh `--save-dir`:

```sh
cd out/<game>-cons
SDL_VIDEODRIVER=dummy SDL_AUDIODRIVER=dummy timeout -k 5 3600 \
    ../../engine/build/conscripter --root . --save-dir <scratch>/saves \
    --trace <scratch>/t.jsonl --trace-ignore <UI sprites> --auto-play 0
echo $?   # 0 end, 1 stuck, 2 script error, 3 startup failure, 128+n signal
python3 -I ../../engine/tools/cons_trace_check.py <scratch>/t.jsonl --script 0.txt
```

- The checker ranks errors, crashes, missing files, leftover sprites, loops, cut voices, leaked text, long
  waits, memory growth and unreached labels; add a false positive to `cons_trace_check.json` beside the trace
  after looking.
- `--auto-choice first|cycle|random|untaken`; `cons_explore.py` reruns until no new label is reached;
  `--replay t.jsonl`; start anywhere with `--goto <label> --vars <file>` (`%312=876` per line).
- Long runs: a renamed engine symlink (others `pkill -x conscripter`), `research/tools/capped.sh <GiB> <cmd>`,
  `--checkpoint-every N`. More: `engine/DEBUG.md`, `research/tools/scratch.sh`.

## Route simulation

1. Save a public walkthrough (routes, unlock and loop order, true-ending conditions) in the document's
   "Walkthrough" with its source.
2. Table the progression variables: meaning, where set, reset and read, save or system data.
3. Simulate the source script from decoded bytecode with the original's run-time rules (per page: text, links
   with targets, variables) and the converted `0.txt` on the same click path; diff page by page, wrong-order
   paths the original refuses included. Then `--replay` each path and compare `--trace-vars`. Model:
   `research/xuse/routesim.py`, `research/xuse/nstrace.py`.
4. Lint for the bug class: links shown only under a condition, a per-message record jumping to another message,
   two links on one text.
5. Unlocks: list every button and item with what the original needs before showing or enabling it (from the
   code, or mark the rule as ours); check a fresh start and a 100 % state (`--vars`).
6. Compare with the original: settings (default, effect, kept across quitting: skip, auto, voice, volumes, text
   speed), input (keys, wheel, right-click; touch leaves every screen; clicks mid-fade skip no more) and
   persistence (relaunch, reopened tab, older save, return to title).

## The web build

```sh
conscripter-convert <command> <install> -o out/<game>-web --font <ipag.ttf> --loose
python3 -I engine/src/cons_web/conscripter_index.py -i out/<game>-web \
    -o out/<game>-web/cons_index.json --title <game> --lazyload
cp engine/build_web/conscripter.{html,js,wasm} engine/build_web/favicon.ico out/<game>-web/
python3 -m http.server 8000 -d out/<game>-web   # open /conscripter.html
```

Web engine: `cd engine/script && EMSDK_HOME=<emsdk> bash ./cross_web.sh`. Check loading time, sound, saves
across a reload and a closed tab, touch, picture memory on long routes. Headless trace: `docs/tools.md`.

## Test saves at chosen scenes

```sh
python3 -I research/tools/test_saves.py out/<game>-cons <slots.txt> -o <game>-test-saves.zip
```

`slots.txt` holds `slot label description` per line. The engine starts at each label (`--goto`) and saves before
it runs; the zip loads through the web player's Import Saves or a native save folder. `--vars-dir <dir>` sets
each label's `<dir>/<label>.txt` variables first.
