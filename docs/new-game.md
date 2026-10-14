# Converting a new game: steps and planning

A playbook for an agent starting a new game. It is what 最果てのイマ
COMPLETE (Xuse, finished 2026-10-14) taught us, put in the order that
would have been fastest. The general lessons behind each step are in
`PLAN.md` §13. The house rules are in `CLAUDE.md`.

For scale: 最果てのイマ took about two days of agent sessions, around
120 converter commits and ten CONscripter releases (0.8.0 to 0.17.0). The
story played end to end after the first day. The second day went on
system screens, extras and matching effects to the original.

## 0. Before writing any code

1. **Ask the user** what they want from this game: the whole thing with
   extras, or story first? Which platforms (the web build is the main
   one)? Anything to skip? Write the answers down in the game's document
   (step 0.4). Things the user dropped for 最果てのイマ: the debug menu,
   right-click menus, the WSS system-voice room, native Linux/macOS
   movies.
2. **Learn the game as a player.** Read a wiki or skim a playthrough for
   the route structure, the choice mechanic, endings and extras.
   最果てのイマ's route choices are hyperlinks in the text. Not knowing
   that cost a session: a no-click trace loops back to the chapter start,
   and that looked like a control-flow bug. Save a walkthrough in the
   game's document before converting control flow (step 11.1).
3. **Identify the engine** from the install: the exe's strings,
   `.cpp`/RTTI names, archive magic numbers, config files. If it is a
   known engine (KiriKiri, NScripter, Siglus, Nitroplus NSS, ...), find
   the existing format documentation and tools before reversing anything.
4. **Start the files that survive context resets**:
   - `docs/games/<game>.md`, committed: the game's record and its
     feature tracking (build command, formats, opcode or command table
     with handler addresses, how its screens map to CONscripter,
     testing and wine recipes, and a **"Status"** section with a table
     of what converts and the next steps, in order). Give it two
     sections from the start for step 11: **"Walkthrough"** (with its
     source) and **"Progression variables"** (the table of step 11.2).
     `docs/games/saihate-no-ima.md` and `docs/games/muramasa.md` are the
     models. It is public: no local paths (write `<install>` or
     `<game>`), only commands anyone with the game can run. Keep the
     "Status" current in the same commits as the work. There are no
     git-ignored handoff files; where the user keeps the game goes in
     your memory.
   - A branch per game (`xuse`, `nitroplus`), based on `origin/main`. A
     second agent works in its own worktree
     (`../conscripter-convert-<name>`). Merge `origin/main` into
     it at the start of every session, keep game-specific code out of
     the shared files, and merge back into `main` at good points
     (`CLAUDE.md`, "Working beside other agents").
   - Optional: a phone-visible progress page (an artifact whose shared
     database gets one log entry per commit). The user liked following
     the work that way.
5. **Plan in this order:** assets → script decoding → control flow →
   text and voices → pictures and sound in scripts → system screens →
   extras → matching effects to the original. Each phase ends with
   something the user can run. Plan a vertical slice first (one scene
   with text, a picture and a voice) before going broad.

## 1. Archives and assets

- Write the archive reader and per-format decoders in
  `crates/conv-engines/src/<engine>/`, one module per format, each
  with a `conscripter-convert <format> list|extract` subcommand for
  inspection.
- Keys and ciphers come from the exe. Keep the throwaway Python in
  `research/<engine>/` (run with `python3 -I`, game paths as arguments,
  no game data committed) and port only what holds up into Rust.
- **Check against the whole corpus:** every archive verifies (CRC/MD5),
  every image decodes, every audio file plays. Count degenerate assets
  (empty files, 1×1 images): references to them point to off-by-one bugs.
- Audio: Ogg Vorbis passes through. Put loop points into the `.ogg` as
  `LOOPSTART`/`LOOPEND` comments, which SDL_mixer loops on by itself and
  saves never lose. Movies: ffmpeg to H.264 + AAC MP4, played with
  `mpegplay`.
- Images stay PNG by default; `--webp` (near-lossless) is opt-in for
  the web build.

## 2. Scripts: decode, then measure

- Decode the script container fully (header, code, constant pool, label
  tables, checksums) and dump it: `conscripter-convert cd dump` is the
  model.
- **Rank opcodes (or built-ins) by use count** (`cd opstats`) and work
  from the top. A handful of opcodes make most of the visible game.
- Find the dispatch table in the exe. Then, for each handler: the
  strings it references (error messages name opcodes), the source-file
  name it logs, the RTTI class it builds. **Decompile with Ghidra**
  rather than reading raw disassembly:
  `research/tools/ghidra/ghidra_export.sh <exe> research/<engine>/local/ghidra
  --names research/<engine>/names.tsv` writes one pseudo-C file per
  function (`decomp/<address>_<name>.c`, headed by its callers, callees
  and the cp932 strings it uses) plus `functions.tsv` and `strings.tsv`.
  Grep those first. **Every function or global you identify goes into
  `research/<engine>/names.tsv`** (`address<TAB>name<TAB>comment`) and the
  export is rerun (~30 s; the first run analyzes for about a minute), so
  the next session reads named code instead of `FUN_00412a30`. Options
  (`--rtti` for MSVC C++ classes, `-- -processor …` for ARM) are in the
  script's header. For ARM C++ code (a Vita eboot), add `-postScript
  ThumbPointerFunctions.java` after `--`: analysis leaves virtual methods,
  reached only through vtables, outside any function, and the script makes
  one at every Thumb pointer stored as data. `objdump -d -M intel` is still
  handy for a quick look at the bytes.
- Work out **every index space a jump uses** (per-script labels or
  global tables, implicit entries, labels on marker instructions) and
  check all targets across the corpus. A wrong guess still produces a
  script that runs.

## 3. Lowering to the IR

- The reader produces the engine-neutral IR (`crates/conv-core/src/ir.rs`).
  Only `crates/conv-cons` knows NScripter. If the IR can't express
  something, add an IR op or a style variant with a doc comment that says
  what the picture does, not how the source engine does it.
- Anything not converted yet is emitted as a `;` comment naming the
  script and instruction. The converter prints a ranked list of what
  was left out, and that list is the to-do list.
- Keep converter variables and sprite numbers in documented ranges:
  write the map down (in the game's document) before the menus
  claim slots. Lower sprite numbers draw on top. Keep the game's own
  sprites at fixed numbers, so old saves still load after renumbering
  the menus.

## 4. Control flow: prove it before looking at pictures

- Trace the emitted `0.txt` statically (`research/xuse/nstrace.py`)
  to count displayed pages, find hot loops and follow each route (the
  "click link N" option). Pages count as displayed, not in file order.
- Run it headless (recipe in `CLAUDE.md`), always under `timeout -k 5`
  (ONScripter ignores SIGTERM in a script loop). Once the story runs,
  let it play itself with the debug trace and check the trace ("Checking
  a build with the debug trace", below): one run finds script errors,
  crashes, missing files and labels never reached.
- A loop isn't automatically a bug: check whether the original loops
  there too.
- Map the progression variables (step 11.2) as soon as conditions
  convert, and follow the walkthrough through the converted branches
  (step 11.3). Build the two-sided route simulation (step 11.4) at
  the same time and re-run it after every control-flow change.

## 5. Text and voices

- The text box and name plate are measured from screenshots of the
  original. Fonts must be fixed-width (`ipag.ttf`).
- Find the voice rule exactly (最果てのイマ hid it behind a
  divide-by-constant: look for magic multipliers) and check it plays
  every voiced line.
- Backlog names and voice replay: a `[name|voice]` tag per line with a
  `pretextgosub`, read back with `gettaglog`.
- Ruby, size and colour tags: `(base/reading)` with `rubyon`, `!zNNN`,
  `#RRGGBB`.

## 6. Pictures and sound in scripts

- Backgrounds, sprites, BGM, sound effects, transitions with their real
  durations. Check sprite references against asset names: name prefixes
  are a free oracle and find whole classes of bugs in one pass.
- Find where effect parameters are defined. An opcode may only name a
  preset that another opcode defined in a setup script.
- An effect ONScripter can't draw: first try existing commands (masks,
  `effect` numbers, `spfilter`), then add it to CONscripter (step 9).

## 7. System screens

Title, save/load (with thumbnails and delete), settings, backlog, the
button bar, confirmations. Build them as in-script menus from the
original's own parts (`menu.rs` explains the ONScripter mechanics). Where
the original uses a Windows dialog, design one from game parts and say
so. Check each one headless with `research/xuse/smoke.py` (scripted
button presses and screenshots). List the screens' sprites in the game's
trace config (below) so the trace leaves them out and auto-play never
presses the bar's save, load or skip buttons.

## 8. Extras

CG gallery, scene replay, music room. Their tables usually live in the
exe or in system archives. Get a 100% save (the user points to
sagaoz.net) and film the original's screens under wine before building
them.

## 9. Matching the original

- **Film the original under wine** for anything not obvious: transitions,
  effects, menus. Set up a scratch `WINEPREFIX` whose Documents, Desktop
  and other user folders are **real folders, not symlinks to the user's
  home** (games save there), use a symlink farm so patched scripts never
  touch the install, no audio, an Xvfb display of its own (never the
  user's desktop). Slow a transition down by patching its duration in a
  copy of the script, and take shots at fixed fractions of it. Stop
  wine and Xvfb afterwards. Recipes: "Running the original under
  wine" in `docs/games/saihate-no-ima.md`, and in
  `docs/games/muramasa.md` for a game that needs DirectSound.
- Then read the effect's kernel in the exe for the exact curve, and fit
  the filmed frames to it. Example: MeltDown, edge = h·((p − d)/(1 − d))².
- Check our side the same way: run CONscripter on its own Xvfb display
  with `SDL_RENDER_DRIVER=software` (otherwise the grab is black) and
  grab frames mid-effect with `import`.

## 10. Engine additions

When existing NScripter commands can't do it:
1. Add it opt-in in `engine/src/onsyuri/` (`ONScripter_cons.cpp`
   preferred); a bug fix instead applies to every script, with no command. Effects are `cons_*.dll` effect-99 plugins; screen
   effects use the `screenfilter`/`screenshake` slot machinery. It must
   build for the web (emscripten) and keep its state in saves.
2. Document it in `engine/ADDITIONS.md` and in `PLAN.md` §4.2.
3. Commit and push the engine first, then commit the submodule pointer
   together with the converter change that uses it.
4. Release when a batch is ready: bump the version in the 7 files that
   carry it (see `CLAUDE.md`), tag `vX.Y.0`, push the tag, and check that
   all 5 build workflows pass and the release has its 9 assets.

## 11. Route and progression check

A game can't be declared finished until this is done. A converted script
can run every page and still send the player to the wrong chapter, or
lose the loop state between sessions.

1. **Find a walkthrough first.** Before converting control flow, save a
   public walkthrough or flowchart of the routes and the unlock order in
   the game document's "Walkthrough" section: which choices or links lead
   to which chapters, the loop or cycle order, the true-ending
   conditions. Note its source (site and page).
2. **Map the progression variables.** Find every variable the script
   uses to gate routes, chapters, loops and endings. For each one,
   record its meaning, where it is set, reset and read, and whether the
   original keeps it in save data or in system data. Put the table in
   the game's document.
3. **Check the converted branches against the walkthrough.** For each
   step of the walkthrough, follow the converted `0.txt`, or run it
   headless with a scripted click path (`research/xuse/smoke.py`) and
   print the variables with the `lsp 999,"X:"+…` trick (`CLAUDE.md`),
   or record them with the engine's `--trace-vars` (below).
   Confirm the expected chapter is reached at every loop: for
   最果てのイマ, the first loop with no choice, each heroine, the second
   cycle and the final arc. Write down every place where the code and
   the walkthrough disagree, and settle each one by reading the
   original's disassembly or running the original under wine (step 9).
   Don't just pick one.
4. **Simulate every route, both sides, and compare.** Reading the
   output and spot runs miss bugs that only show on some paths: after
   最初の頃, the other heroine links in 「最初の日」 stayed clickable and
   switched the route (`fad5f9b`), because the converter put a link on
   its message even when a jump skipped the link record. So:
   - **Simulate the source script** from the decoded bytecode with the
     original engine's run-time rules (records collect until a message
     uses them, jumps, conditions, variables). The simulator lives in
     `research/<engine>/`. For each walkthrough path it prints, per
     page, the text, the clickable links and their targets, and the
     progression variables.
   - **Simulate the converted `0.txt`** on the same click path with
     `research/xuse/nstrace.py` (or the next engine's equivalent),
     printing the same things.
   - **Diff the two page by page.** Every page must show the same links
     with the same targets and leave the same variables. Also run
     wrong-order paths the walkthrough warns about, such as clicking a
     second heroine link in the same loop: the original must refuse them,
     and so must the conversion.
   - **Then in the engine.** The simulators check control flow; the
     engine's trace checks what the player sees on the same routes.
     Replay each path with `--replay`, or let `cons_explore.py` cover the
     routes (below), and compare the engine's `--trace-vars` records
     with the simulator's variables file by file: that tells you why
     two runs went different ways, not only that they did.
   - **Lint the output for the same class of bug.** Report every jump
     target that falls between a per-message record (link, ruby, style)
     and its message, every link the source only shows under a
     condition, and every page with two links on the same text.
   Keep the scripts and the click paths so the check can be re-run after
   any converter change that touches control flow, text or links.
   最果てのイマ's is `research/xuse/routesim.py` (paths in its `PATHS`,
   a few seconds per path). It normalizes what the conversion changes on
   purpose (full-width `(`, `/`, `!`; inline 【name】; padded chapter
   names) and detects a link page shown again after its scene by the
   jump back to its label, not by its text.
5. **Check that state persists the way it does in the original.**
   Confirm that system-data flags survive (a) quitting and relaunching
   natively, (b) closing and reopening the web build, (c) loading an
   older save and (d) going back to the title and starting again.
   Compare each case with what the original does.
6. **Write it down.** Add a checklist entry to the game's document:
   "progression verified against walkthrough: yes/no, date,
   how" (which paths were simulated, and whether both sides matched). Add any new lesson to `PLAN.md` §13.

## 12. Finishing

- The finish line is set by the user: every story instruction converted,
  system screens and extras working, the route and progression check
  (step 11) done, the remaining differences written down as optional
  leftovers in the game's document.
- Update the game's document (status, leftovers), `README.md`'s support
  table, `PLAN.md` §6 (the list of games) and §13 (new lessons, stated
  for any engine), and this playbook if the process changed.
- Rebuild into a fresh output folder. Never touch folders holding the
  user's saves (`out/saiima-flow`), and list any folder before deleting
  it.

## Checking a build with the debug trace

CONscripter can play a converted game by itself and write a **trace**:
one JSON line per text page (the sprites with their boxes, the
background, music, sounds, filters and effects), per command that
changed the screen or sound, per choice, per missing file, script error
or crash. `engine/DEBUG.md` describes every record and option; this
section is the workflow for a conversion. Run everything headless and
under `timeout -k 5`, with the game folder as the working directory.
The trace needs a CONscripter newer than 0.18.0 (its `master`): move the
`engine` submodule forward first.

**Checker first, deep dives after.** After every new build, make a trace
and run `cons_trace_check.py` on it before digging into anything by
hand (disassembly, filming the original, reading the script). The
checker's list gives quick things to check and fix: a missing file, a
sprite left up, a looping sound, text with a leaked tag. Go deeper only
for what it can't explain.

**A first run.** No window and no Xvfb are needed, so several runs can go
at once, each with its own `--save-dir` and trace:

```sh
cd out/<game>-cons
SDL_VIDEODRIVER=dummy SDL_AUDIODRIVER=dummy timeout -k 5 3600 \
    ../../engine/build/conscripter --root . --save-dir /tmp/<game>-saves \
    --trace /tmp/t.jsonl --trace-ignore <UI sprites> --auto-play 0
echo $?   # 0 end, 1 auto-play stuck, 2 script error, 3 startup failure, 128+n signal
python3 -I ../../engine/tools/cons_trace_check.py /tmp/t.jsonl --script 0.txt
```

- Always give `--save-dir`: a game that saves as it goes would otherwise
  write to the player's saves (never `out/saiima-flow`). The engine
  (CONscripter 0.19.0 and later) makes the folder if it doesn't exist.
- `--trace-ignore` takes the game's own UI sprites (the ranges in the
  game document's sprite map). They are left out of the trace and
  auto-play never presses their buttons.
- `--auto-choice first|cycle|random` and `--seed n` pick the route;
  the same options and seed give the same run.
- The checker ranks what looks wrong (script errors and crashes,
  missing files, sprites left up across a background change, pictures
  a few pixels above the bottom edge, filters left over a CG, sounds
  left looping, the same sound started twice before a click, voices cut
  off, lines missing their voice, garbled or leaked text, very long
  waits, slow spots, memory growth, labels never reached), each at the script line
  `*label:line` to look at and with the command that showed the sprite.
  Its last line is a `--shot-pages` option: run again the same way with
  it to get pictures of just those pages.

**Per-game checker config.** Put the game's UI sprite ranges, its CG
name pattern and its known false positives in a JSON file beside the
trace or the script (`cons_trace_check.json`, or `--config FILE`), so a
check needs no long command line. Keep each game's file next to its
document's sprite map, and add a false positive only after looking at
the page. For 最果てのイマ that is sprite 61 (`bg\frame01.png`, the
game's frame overlay) for `linger`.

**A crash.** A trace that ends in a `crash` record has the signal, the
script line, the gosub chain and the last 50 commands run: read those
before the core dump. A script error has an `error` record with the same
place and chain; with `USE_FILELOG` builds the message also goes to the
terminal.

**Every route, not one.** One auto-play run follows one route, and a
hub that is always answered the same way leaves whole routes unreached
(最果てのイマ's first runs missed 1,062 labels that way).

- `engine/tools/cons_explore.py` runs the engine again and again with
  `--auto-choice untaken`, each run steered toward the options no
  earlier trace took, until no run reaches a new label. It reports the
  labels reached by all runs together against the script's.
- `--replay t.jsonl` repeats an earlier run's choices exactly (and its
  seed and `--auto-choice`), so a route the checker complained about
  can be played again after a fix. Give each run a fresh `--save-dir`:
  the game's global variables stay in the save folder, and a second run
  in the same one starts from a different state. A `replay diverged` warning marks the first place the new
  build went a different way. Compare the before and after traces with
  `cons_trace_check.py --diff before.jsonl after.jsonl`.
- A route simulator (step 11.4) can write the `--replay` file itself:
  the engine reads only its `choice` records, in order, each with the
  place and the answer (`{"t":"choice","at":"*label:line","picked":2}`,
  or `"result"` for a `btnwait`), so a walkthrough path plays in the
  engine without patching `0.txt`. The places are `0.txt` line numbers:
  write them from the simulator's run over the same `0.txt`.

**Start anywhere.** `--goto <label> --vars <file>` starts at a label
with `%`/`$` variables preset (`%312=876`, `$5=text`, one per line). It
replaces patching `0.txt` (a scene deep in a route reproduced in seconds
instead of an hour's replay). The vars file can come from:
- a simulator's or the converter's tracer (write it in that format),
- another run: `--dump-vars file` writes one at F8 and when the engine
  exits (F8's `dump` record lists the same variables),
- a run's `var` records (`--trace-vars`, below) or F8 dumps, turned
  into a vars file for any page or place by
  `engine/tools/cons_trace_vars.py t.jsonl --page N` (or `--at
  '*label:line'`). Global variables loaded from the save folder have no
  `var` records: take those from an F8 dump or `--dump-vars`.

**Long runs.** `--checkpoint-every N` saves a slot every N pages during
auto-play, and `--load n` starts a run from slot n, so a run that fails
deep in a route restarts from the last checkpoint. Checkpoints go to
slots 900 and up unless `--checkpoint-slot` says otherwise: move them if
the game keeps its own data in those slots. Keep the game's bar in
`--trace-ignore`, or auto-play presses its load button once a save
exists. `--roundtrip-every N`
saves, loads and compares the state every N pages; a `save/load
mismatch` warning means part of the state (filters, the `pushscreen`
stack, screen effects, variables) doesn't survive a save.

**Story flags.** `--trace-vars 300-899` writes a `var` record each time
one of those variables changes, with the script line. Compare them with
the route simulator's variables (step 11.4) to see where and why the
engine's flags part from the source's, file by file:
`engine/tools/cons_trace_vars.py --diff a.jsonl b.jsonl` prints the
first change that differs (`--only 300-899` to narrow it).

**What a player would notice but auto-play hides.** Auto-play shortens
effects and waits, so the trace also records what each command asked
for: a `long wait` warning flags any `print`, `delay`, `wait` or effect
asking for more than 10 s (`print 10,270000` is a 4.5-minute fade). A
`large image` warning flags a single picture that takes more than 64 MB
(`--trace-max-image`), and each page records the image memory in use, so
a leak shows as a curve before it crashes. A picture's width alone is no
problem: CONscripter draws pictures in software, the web build too, so
a 37600x600 animation strip works in a browser.

**Seeing a page.** `--shot-pages 412,980-990` or `--shot-every n` saves
pictures of chosen pages during the same run; F8 while playing with a
trace on writes the state at that moment (and natively a picture). The
web build takes `?trace` in its URL and downloads the trace with F7.
