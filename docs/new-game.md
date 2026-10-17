# Converting a new game: steps and planning

A playbook for an agent starting a new game. It is what 最果てのイマ
COMPLETE (Xuse, finished 2026-10-14) taught us, put in the order that
would have been fastest. The general lessons behind each step are in
`PLAN.md` §13. The house rules are in `CLAUDE.md`.

For scale: 最果てのイマ took about two days of agent sessions, around
120 converter commits and ten CONscripter releases (0.8.0 to 0.17.0). The
story played end to end after the first day. The second day went on
system screens, extras and matching effects to the original.
装甲悪鬼村正 (Nitroplus NSS) and 俺たちに翼はない (PS Vita, MAGES)
followed in parallel from 2026-10-13, each in its own branch: about 70
commits each in their first two days, by which time the whole story
converted, every ending reachable.

## Checklist

One line per step; the step's section has the how. Read a section when
you reach it, not the whole file up front
(`python3 -I research/tools/mdsec.py docs/new-game.md "9. Matching"`).

| Step | Do | Done when |
|---|---|---|
| 0 | Ask the user the scope; learn the game as a player; identify the engine; start `docs/games/<id>.md` from the template | Scope, walkthrough source and engine are in the document |
| 1 | Archive reader and format decoders, `<format> list\|extract` | The whole corpus verifies and decodes; degenerate assets counted |
| 2 | Decode the scripts; rank opcodes by use; Ghidra export with `names.tsv`; coverage table | Every script parses, every jump target exists |
| 3 | Lower to the IR; flags in `conv_core::alloc`'s free ranges | `--dry-run` writes a `0.txt`; what is left out is a ranked list |
| 3a | Registry entry, `tests/snapshot.sh` case and baseline (`docs/new-engine.md` 6) | `tests/snapshot.sh` covers the game |
| 4 | Dry run + `lint` + debug trace after every feature | Trace check clean; every ending reachable |
| 5 | Text box, name plate, voices, ruby; the longest message fits | Every voiced line plays |
| 6 | Pictures, sound, transitions with real durations | Sprite references match asset names |
| 7 | System screens from the IR's screens | Every screen opens, closes by mouse, keyboard and touch |
| 8 | Extras from a 100 % save | Locked and unlocked states both right |
| 9 | Match the original: code first, wine last | Differences measured (`compare -metric`), shots kept in `local/shots/` |
| 10 | Engine additions, engine PR first | Listed in `PLAN.md` §4.2; submodule on CONscripter `master` |
| 11 | Route, progression, unlocks, settings, input, saves across rebuilds | The document says "progression verified: yes, date, how" |
| 12 | Web build, loose files | Played in a browser; size and file count in the document |
| 13 | Finish: README, `PLAN.md` §6 and §13, GUI entry | The user agrees it's done |

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
   - `docs/games/<game>.md` (copied from `docs/games/TEMPLATE.md`; a
     new engine also follows `docs/new-engine.md`), committed: the game's record and its
     feature tracking (build command, formats, opcode or command table
     with handler addresses, how its screens map to CONscripter,
     testing and wine recipes, and a **"Status"** section with a table
     of what converts and the next steps, in order). Give it two
     sections from the start for step 11: **"Walkthrough"** (with its
     source) and **"Progression variables"** (the table of step 11.2).
     `docs/games/saihate-no-ima.md` and `docs/games/muramasa.md` are the
     models. It is public: no local paths (write `<install>` or
     `<game>`), only commands anyone with the game can run. Keep the
     "Status" current in the same commits as the work, and keep it to
     the open state: when an item is done, move its write-up to a
     **"History"** section after "Status" (the document's last two
     sections), so a session reads only what is open. There are no
     git-ignored handoff files; where the user keeps the game goes in
     your memory.
   - A worktree per game (`../conscripter-convert-<name>`), with one
     short-lived branch and pull request per task item, made from
     `origin/main`; keep game-specific code out of the shared files
     (`CLAUDE.md`, "Rules" and "Working beside other agents").
   - Keep each session to one item: do it, update "Status", open the
     PR, stop. A fresh session starts from "Status" and memory,
     which is cheaper than a long one (`CLAUDE.md`, "Keeping usage
     down").
5. **Plan in this order:** assets → script decoding → control flow →
   text and voices → pictures and sound in scripts → system screens →
   extras → matching effects to the original. Each phase ends with
   something the user can run. Plan a vertical slice first (one scene
   with text, a picture and a voice) before going broad.

## 1. Archives and assets

- **Reuse the shared code before writing your own.** The engines
  share how files are written (`conv_engines::output::Output`: the
  output folder, dry runs, collected errors, `write`, `write_png`,
  `image_size`), how sounds and movies are made
  (`conv_engines::media`: `ffmpeg` through a `.part` file,
  `mp4_options`) and the end of a build (`crates/conv-cli/src/build.rs`:
  `check_output`, `write_game`, `pack`, `summary` with the notes and the
  lint). A new engine's asset bank keeps only what is its own: reading
  its archives, its caches and its output names (saves record them).
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
  the web build. A new engine's convert command takes it with
  `#[command(flatten)] webp: build::WebpArgs` and calls
  `build::webp_tree` with its picture folders before writing `0.txt`.

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

### Marking what a finding rests on

Mark each finding **seen** (the code, a script or a trace shows it),
**inferred** (from names or layout) or **unknown**, and for an unknown
write the cheapest check that settles it (the code, a dry run, or a
wine frame). A negative search says what it searched ("not found in the
1.0.2 scripts"), never "absent". Name the executable build for every
decompile address (the game document's "Executables" table; the export
writes the input's sha256 to `sha256.txt`). The game's Status keeps a
short "Open questions" list, and reports and PR bodies end by marking
each original question answered, partly answered or open.

### Mapping the whole exe

When the scripts use more than the top of the ranking, extend the same
export and the same `names.tsv` in passes, from what the corpus uses
rather than by browsing:

1. **First pass:** the export as it stands. Name the dispatch table
   (statement built-ins), the value evaluator and the settings-variable
   setter first; every other function hangs off those.
2. **Recover the code that isn't a function yet.** The export misses
   code reached only through a function pointer. In Muramasa these were
   the per-object callbacks (`local_4 = &LAB_…`) behind `Fade`,
   `Request`, `SetShade`, `CreateEffect` and others: 76 labels, so the
   decompiled output never showed what those built-ins do. Collect the
   label addresses (`grep -ho '&LAB_[0-9a-f]*' decomp/*.c`: not only
   `= &LAB_`, since casts `(code *)&LAB_` and arguments are callbacks
   too), append them to the names file as `cb_<addr>` and re-run the
   export: `ApplyNames` disassembles each address and creates the
   function. Repeat until no new address turns up (the new functions
   hold pointers of their own). Do this as a standing step. Skip
   `puStack_`/`pcStack_` frame stores and `return &LAB_<odd>` (mid
   instruction). Some hits are junk: SEH handlers (around
   `004f8xxx` in Muramasa) and very large pixel blitters; don't read
   400 KB of blitter by mistake.
3. **Name the CRT helpers early.** Muramasa's `FUN_004d3be0` is
   `_ftol2`; Ghidra loses the x87 maths around it, so a computed value
   can look like it comes from nowhere.
4. **Find what the parser accepts.** Use `strings.tsv` plus the stack
   strings (`research/nitroplus/stackstrings.py`): most names are built
   on the stack and don't show up in `strings.tsv`. Look for
   character-comparison chains (`== 'R'`, `== 'U'` …): that is how the
   markup-tag parser was found. Note how constants are matched: Muramasa
   compares some by the sum of their characters (`NULL` = 315 = off),
   other places compare names exactly and case-sensitively (the
   `Request` actions).
5. **Plan from a coverage table, not from browsing.** For each built-in,
   `$SYSTEM_` variable, text tag and `Request` action, record its use
   count in the corpus, what the exe does (and where), what the
   converter does, and whether the dry run's "not converted" notes list
   it. Then work only on the rows where the exe and the converter differ
   and the corpus uses the feature: "map the entire exe" becomes a
   bounded list. A blanket walk through 2,400 functions is the expensive
   path; libraries such as libpng, zlib and libjpeg can be recognised
   from their strings and skipped.
   Coverage is per argument and behaviour, not per built-in: a row is
   "converted" only when every argument and behaviour of it is, and
   otherwise "partial", with what is left. Every engine's built-in
   dispatch notes each argument a call passes and the converter doesn't
   read (`<Builtin> argument <i> ignored`, whatever its value; an
   argument known to change nothing goes in a short table with the value
   it covers and why: Nitroplus's `UNREAD_HARMLESS`). The dry run's notes
   then list the partial rows by themselves.
   Give each row where it will go: data for an engine piece (`spmotion`
   keys, an `spcurve` table, `cons_rule.dll` masks), Lua (logic,
   processes), or a new engine command (§10). Only the last needs an
   engine PR.
6. **Write the results back.** Commit `names.tsv` with comments, fold the
   decoded semantics into the engine's built-ins reference
   (`research/nitroplus/builtins.md` for Nitroplus), and put the gaps
   found in the game document's Status.

### Covering the scripts too

The exe is half of what a game does; the rest is in its own scripts and
data files: the helper libraries the story calls (`function*.nss`), the
system screens (`sys_*.nss`), boot scripts, ini files (`System.ini`)
and, for a port, its own system scripts (the English Muramasa's
`sys_*.nut`). Full coverage means every one of them has an answer, in
the same coverage table as the exe's rows:

1. **List every script and data file** in the archives and give each
   one a status: compiled by the run (story files, helpers they call),
   replaced by a screen of ours (name the function: `sys_config.nss` →
   `screens.rs` `settings`), read for data (ini files, tables), or not
   used (say why: debug, unreachable). A file with no status is a gap.
   For NSS, `nss route` lists the unreachable story files.
2. **List every name the scripts call**, built-in or defined in a
   script, with its corpus count (`nss refs` for the names nothing
   defines, `nss builtins` for how they are called), and add what the
   converter does with it. The dry run's "not converted or approximated"
   notes give the gaps in the code the run reaches (`built-in X`,
   `skipped: no function X`, `Request …`), but only there: code the run
   never reaches (the system scripts, the functions only they call)
   needs its row from the static count instead.
3. **The same for the scripts' other vocabularies:** system variables
   (`$SYSTEM_…`), text tags (`<I>`, `<center>`, `<CLICK>`), `Request`
   actions, ini keys, and any string the scripts pass that the exe
   matches (names of effects, actions, sounds).
4. **Plan from the rows that differ**, as for the exe: a feature the
   corpus uses that the converter drops or approximates. Rows replaced
   by our own screens are checked against the original's screens
   (step 7), not line by line.
5. **Write it back** like the exe's results: the table's gaps in the game
   document's Status, the semantics in the engine's built-ins reference.

## 3. Lowering to the IR

- The reader produces the engine-neutral IR (`crates/conv-core/src/ir/`).
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
- **Claim them against what conv-cons already uses.** The menus write
  variables of their own (`grep -o '%[0-9]*' 0.txt`, and the constants
  in `menu/mod.rs` and at the top of `lib.rs`): pick the game's flags clear of
  them, locals below the global border, globals clear of the menus'.
  A new settings switch takes the next free index in `def` in
  `switches.rs`, which a test keeps unique: branches adding switches at
  the same time both took 9-11 once. Never move a number
  once a build is out (step 11.9).
- **Say where every hand-typed value comes from.** A literal that
  stands for a fact about the original (a position, size, colour,
  opacity, font size, timing, count, order) gets a one-line comment:
  `// src: exe 0x4fb808`, `// src: <file> <what>`, `// src: shot <name>`
  (`research/<engine>/local/shots/`), `// guess: <why this value>`, or
  `// ours` for the converter's own choice; one comment covers a block
  from one table. Check a value in that order (the game's data, the
  exe's tables, an exact capture of the original, a guess) and read it
  from the files at convert time when that is small. A scaled
  screenshot is a guess, not a source: Saiima's were off by up to 12 px.

## 4. Control flow: prove it before looking at pictures

- **After every feature, compile the whole game and lint it.** A dry
  run (`--dry-run`: the script only, no assets; under 20 s for a whole
  game; `convert-nss` and `mages convert` have it, give a new engine's
  command one too) then `conscripter-convert lint <dir>`: missing
  labels, effect numbers past 255, a `?` in text, half-width characters
  in vertical text, files missing from the archives. Two of 村正's engine-killing bugs only showed on the whole
  game, and lint's first run found a third.
- **Then let the engine play it** with the debug trace and check the
  trace ("Checking a build with the debug trace", below): one run finds
  script errors, crashes, missing files and labels never reached, and
  `cons_explore.py` covers every route. This is the default headless
  check; always under `timeout -k 5` (ONScripter ignores SIGTERM in a
  script loop).
- The older tools still help where the engine can't: a static tracer
  of `0.txt` (`research/xuse/nstrace.py`: displayed pages, hot loops,
  every route in seconds with "click link N"; pages count as displayed,
  not in file order) and the patched-`0.txt` recipe in `CLAUDE.md`.
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
- **Check that the longest text fits.** Find the longest message, with
  its name and its ruby, over the whole corpus and measure it against
  the window's lines and columns (俺たちに翼はない: 109 characters, four
  lines of 30). The trace's "text ran past the window" warning only
  covers the pages a run reached.

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
button bar, confirmations. Which buttons the original shows or
enables at each point (no save yet, no choice yet, nothing unlocked)
is checked in step 11.6. Build them as in-script menus from the
original's own parts (`menu/mod.rs` explains the ONScripter mechanics). Where
the original uses a Windows dialog, design one from game parts and say
so.

- **Start from the screens the IR already has**: `SaveMenu` and
  `SaveGrid` (save and load), `SettingsMenu` (with `PagedSettings` for
  pages drawn from the game's own pictures, option words and level bars),
  `PictureSettings` (one picture with sliders and on/off pairs),
  `ButtonBar`,
  `CommandMenu`, `Backlog`, `ConfirmDialog`, `PictureScreen`. A game
  changes them through what it puts in the IR, never with game-specific
  branches in `menu/`. Add a new screen type only for a screen none of
  them can draw, and make it general enough for the next game.
- **Two families of screens**: the picture ones (`SaveGrid`, `LineLog`,
  `PictureSettings`, `PictureExtras`, `CommandMenu`: Nitroplus's) are
  laid out from the original's own pictures, the classic ones
  (`SaveMenu`, `Backlog`, `SettingsMenu`, `ExtrasMenu`: Xuse, MAGES)
  arrange the game's parts in a layout of ours. Start from the picture
  family when the original draws its screens as pictures (most do);
  a third family needs the user's yes.
- **State the engine could keep goes in the engine.** When a screen needs
  something the engine can track (play time, counters, save details),
  add a CONscripter command instead of script arithmetic: 村正's save
  grid first counted play time in script, then got `getplaytime`.
- Check each screen headless with the trace's `--shot-pages` or
  `research/xuse/smoke.py` (scripted button presses and screenshots).
  List the screens' sprites in the game's trace config (below) so the
  trace leaves them out and auto-play never presses the bar's save, load
  or skip buttons.

## 8. Extras

CG gallery, scene replay, music room. Their tables usually live in the
exe or in system archives. Get a 100% save (the user points to
sagaoz.net) and film the original's screens under wine before building
them, with a fresh system data as well, to see what is locked before
anything is reached (step 11.6).

## 9. Matching the original

- **Read before you film.** Most questions about what the original does
  are answered faster from its code than from frames: the script
  commands' arguments, the decompiled function behind them (curves,
  times, alpha, coordinates), the format. Check our side with numbers
  first (the dry run's `0.txt`, a trace, `compare -metric`). Run the
  original under wine only for what that leaves open, and reuse shots
  in `research/<engine>/local/shots/` before filming new ones.
- **Reach the moment by patching, not by clicking.** To see a page,
  effect or menu in the original, in this order of preference:
  1. **Patch the game's scripts** in a copy: skip the logos and title,
     start straight at the scene, set the flags its branch needs, and let
     the pages advance by themselves (auto mode, or a timed wait added
     after each page). Then take Xvfb screenshots at known times. The
     install is never touched: the patched archive goes in the symlink
     farm in its place. Examples: 最果てのイマ's patched `.cd` files
     (`docs/games/saihate-no-ima.md`); for Muramasa, a repacked
     `nss.npa` (`docs/games/muramasa.md`, "Running the original under
     wine").
  2. **Write or edit a save file** that stands at the moment, when a
     script patch can't get there (state the scripts can't set). Work out
     the format from the exe's save and load code first.
  3. **Click through by hand** only when neither works. It is slow, and
     a click can miss or land during a fade.
- **Wine gotchas:** run every game with `LANG=ja_JP.UTF-8` and
  `LC_ALL=ja_JP.UTF-8`, or folder and file names come out garbled and
  saves can fail (Muramasa's new game went straight back to the title). Wine has no MS fonts, so a
  substitute font draws the text: compare where text starts, not the
  step between characters.
- **Filming the original under wine**, when the code leaves a real
  doubt (transitions, effects, menus). Set up a scratch `WINEPREFIX` whose Documents, Desktop
  and other user folders are **real folders, not symlinks to the user's
  home** (games save there), use a symlink farm so patched scripts never
  touch the install, no audio, an Xvfb display of its own (never the
  user's desktop). Slow a transition down by patching its duration in a
  copy of the script, and take shots at fixed fractions of it. Film
  each screen once and keep the shots in `research/<engine>/local/shots/`
  (git-ignored), named for what they show, so later checks reuse them
  instead of starting wine again. Compare ours with them by numbers
  first (`compare -metric AE -fuzz 2%`, on a crop), and look at a
  picture only when the numbers don't say what differs. Stop
  wine and Xvfb afterwards. Recipes: "Running the original under
  wine" in `docs/games/saihate-no-ima.md`, and in
  `docs/games/muramasa.md` for a game that needs DirectSound.
- Then read the effect's kernel in the exe for the exact curve, and fit
  the filmed frames to it. Example: MeltDown, edge = h·((p − d)/(1 − d))².
  Keep what you read as data where the engine can take it (§10): an
  easing as an `spcurve` table, an in-place transition as `cons_rule.dll`
  masks.
- Check our side with the trace: `--shot-effects 25,50,75` films every
  effect at those points of its own time, under `--auto-play` too, as
  `<shot-dir>/fx<pg>-<n>-<pct>.png` (with `--shot-pages` only on those
  pages); compare them with the original's frames at the same fractions.
  Watching it run is the old way: CONscripter on its own Xvfb display
  with `WAYLAND_DISPLAY= SDL_VIDEODRIVER=x11 SDL_RENDER_DRIVER=software`
  (the default renderer grabs black; without `WAYLAND_DISPLAY=` the
  window opens on the user's desktop), frames grabbed with
  `import -window root`.

## 10. Engine additions

Before adding a command, see whether the engine's general pieces take
the feature as data; most of an exe's presentation code is one of these:
- **Sprite animation** (moves, zooms, turns, fades, flashes, glows over
  time): `spmotion` keys (offset, zoom x/y, angle, alpha) with a curve
  per key; the exe's own easing goes in an `spcurve` table as read (or as
  computed by running its function, PLAN.md §13.2). `spblend` adds or
  multiplies.
- **Transitions that mix the two pictures in place** (wipes, dissolves,
  blinds, soft rules): `cons_rule.dll` with a strip of mask frames, made
  by running the original's kernel on a black old and a white new
  picture. Transitions that move pixels still get a plugin.
- **Logic** (rankings, unlock checks, counters, minigame rules) and
  **parallel processes** (an engine's per-frame threads): Lua, already in
  every build. `luasub name` makes `NSCOM_name` a command; the script's
  Lua globals are saved; `cons_process(name, "fn", ...)` resumes a
  coroutine once a frame. A process restarts from the top after a load,
  so keep its progress in globals. Port decompiled C almost line by line.
  The converter doesn't write a `system.lua` yet: the first game that
  needs one adds that to conv-cons, for every game.
A new command is for what none of these can do. Then:
1. Add it in `engine/src/cons/` (`ONScripter_cons.cpp` preferred).
   It isn't opt-in: a new behaviour, like a bug fix, applies to every
   script, with no command to turn it on. Effects are `cons_*.dll` effect-99 plugins; screen
   effects use the `screenfilter`/`screenshake` slot machinery. It must
   build for the web (emscripten) and keep its state in saves.
2. List it in `PLAN.md` §4.2.
3. Open the engine PR first (`CLAUDE.md`, "Rules"); point the
   submodule at your branch's commit while you work, and at a commit on
   CONscripter `master` once the engine PR is merged.
4. Release when a batch is ready: `docs/releasing.md`.

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
   step of the walkthrough, play it in the engine (a `--replay` file of
   its choices, below) and record the variables with `--trace-vars`; or
   follow the converted `0.txt` by hand, or with a scripted click path
   (`research/xuse/smoke.py`) and the `lsp 999,"X:"+…` trick
   (`CLAUDE.md`).
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
6. **Check when every button may be shown or used.** Many buttons
   appear, light up or open only after something has happened, and a
   conversion easily shows them all from the start. List every button
   and item on the title, the menus, the bars and the extras (gallery
   tabs and thumbnails, replay scenes, music and movie room entries,
   chapter or route entries, story menus) and for each one write down,
   in the game's document, what the original needs before it is shown,
   enabled or opened: an ending seen, a route cleared, a CG or scene
   reached, a save existing, a choice behind the player, the game's own
   unlock flag. Find the rule in the exe or the scripts, or see it
   under wine with a fresh system data and with a 100% save; where it
   can't be found, choose one, and say in the document that the rule is
   ours. Then check the conversion in both states: on a fresh start
   (nothing seen, no saves) every locked button must be hidden or greyed
   and refuse a click, as in the original; with the unlocks reached
   (by play, or the variables set with `--vars`) each one must appear.
   Examples: 俺たちに翼はない's after-stories show once an ending was
   seen and its music room opens every song (the original's rule is
   unknown); 村正's 選択肢まで戻る is greyed while there is no choice to
   go back to; 最果てのイマ's replay leaves out the scenes not unlocked
   yet. Locked state is usually in system data: check it survives as in
   step 5.
7. **Check that every setting does what the original's does.** For each
   setting: its default, what it changes, when it takes effect and that
   it is kept across quitting (system data). Compare in particular:
   skip (read text only or everything; stops at a choice or goes on),
   auto mode's speed and whether it waits for the voice, how the volume
   levels and the mute switches combine, whether a voice stops at the
   next click, the text speed's slowest and fastest. Write the
   original's rule next to each row of the game's settings table.
8. **Check every way of giving input.** Right-click, the mouse wheel and
   the original's keyboard shortcuts (Enter, Space, Ctrl for skip,
   Escape, the function keys) do what they do in the original, on the
   page and on every screen. Every screen can be left by mouse, by
   keyboard and by touch: the web build on a phone and the Android build
   have no right-click and no wheel, so no screen may need one to close
   (a dialog that only a right-click dismisses traps a phone player).
   Check that clicks during a fade, a movie or a transition don't skip
   more than the original's do.
9. **Check that saves survive a rebuild.** Players keep their saves
   across our builds, so before a build goes to the user, load the
   previous build's saves (story slots and system data) into it, natively
   and in the web build. Global variables, the game's sprite numbers,
   setting switches and save-slot numbers must not move between builds
   (`--roundtrip-every` only checks one build against itself). Saves
   and read marks (`kidoku.dat`) follow a rebuilt `0.txt` by label.
10. **Write it down.** Add a checklist entry to the game's document:
   "progression verified against walkthrough: yes/no, date,
   how" (which paths were simulated, and whether both sides matched). Add any new lesson to `PLAN.md` §13.

## 12. The web build

The web build is the main platform, so play it in a browser, not only
natively, at every milestone:

- Make it as loose files (the page fetches each file when it needs it;
  one big `arc.nsa` would be fetched whole), with the web engine and an
  index of the files:

      conscripter-convert <command> <game> -o out/<game>-web --font <ipag.ttf> --loose
      python3 -I engine/src/cons_web/conscripter_index.py -i out/<game>-web \
          -o out/<game>-web/onsyuri_index.json --title <game> --lazyload
      cp engine/build_web/conscripter.{html,js,wasm} engine/build_web/favicon.ico out/<game>-web/
      python3 -m http.server 8000 -d out/<game>-web   # then open /conscripter.html

  (the web engine: `cd engine/script && EMSDK_HOME=<emsdk> bash
  ./cross_web.sh`). `docs/games/oretsuba.md` has the whole recipe.
- Check: the first page's loading time; music and voices once the
  browser lets sound play (the first click), and movies (muted when the
  browser blocks sound); saves and system data surviving a page reload
  and a closed tab (step 11.5); the screens by touch (step 11.8); the
  picture memory on long routes (the trace's per-page image memory; the
  web build has less to spare). `?trace` in the URL and F7 give the same
  trace as natively.
- Note the package's size and file count in the game's document.

## 13. Finishing

- The finish line is set by the user: every story instruction converted,
  system screens and extras working, the route and progression check
  (step 11, unlocks included) done, the remaining differences written down as optional
  leftovers in the game's document.
- Update the game's document (status, leftovers), `README.md`'s support
  table, `PLAN.md` §6 (the list of games) and §13 (new lessons, stated
  for any engine), and this playbook if the process changed.
- **The game is in `crates/conv-engines/src/registry.rs`** (id, name,
  engine, marker files, command; `conscripter-convert games` lists it)
  and in `tests/snapshot.sh` from its first dry run (checklist 3a). Its `--update` also keeps
  `tests/snapshots/<game>.metrics.json`, the dry run's completion counts
  (each source call converted fully or with a gap, per command kind),
  which the convert-metrics page on blankiekun.github.io shows: the
  engine opens and closes each call on `conv_engines::metrics::Metrics`
  so its notes land on it; a call left out on purpose (unreachable code such
  as a debug menu) goes to `Metrics::left_out`, which the page counts as
  done but lists apart (`left_out`), so it isn't a gap.
- **Add the game to the GUI**: its entry in `gui/games.py` (the same
  markers as the registry, which a test checks; the marker
  files that recognise it, the command and font from the game's
  document, whether it needs ffmpeg, about how much it writes) and a fake
  layout with the expected command in `gui/tests/test_games.py`. When the
  document's build command changes, change the entry in the same commit.
  The converter's releases (a `v*` tag, `.github/workflows/release.yml`)
  ship the GUI.
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

- To run your own script or engine over someone else's full build,
  set up a scratch folder with `research/tools/scratch.sh` (links only
  the read-only archives, font and movies) and run its `run.sh`.
  `research/tools/scene_ab.py` builds on it: the same scene under two
  engines or two scripts, picture by picture, and save with one engine,
  load with another.
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
instead of an hour's replay). `*start`'s setup (the settings' defaults
and the text window) runs first, so game time matches a run from the
start. The vars file can come from:
- a simulator's or the converter's tracer (write it in that format),
- another run: `--dump-vars file` writes one at F8 and when the engine
  exits (F8's `dump` record lists the same variables),
- a run's `var` records (`--trace-vars`, below) or F8 dumps, turned
  into a vars file for any page or place by
  `engine/tools/cons_trace_vars.py t.jsonl --page N` (or `--at
  '*label:line'`). Global variables loaded from the save folder have no
  `var` records: take those from an F8 dump or `--dump-vars`.

**Long runs.** Other agents stop engines with `pkill -x conscripter`,
which ends every run on the machine: run long traces through a symlink
with another name (`ln -s conscripter out/conscripter-<game>`). Run
builds and engine runs under `systemd-run --user --scope -p
MemoryMax=…` so a memory regression can't take the desktop down (村正's
full build once grew to 7 GB). `--checkpoint-every N` saves a slot every N pages during
auto-play, and `--load n` starts a run from slot n, so a run that fails
deep in a route restarts from the last checkpoint. Checkpoints go to
slots 900 and up unless `--checkpoint-slot` says otherwise: move them if
the game keeps its own data in those slots. Keep the game's bar in
`--trace-ignore`, or auto-play presses its load button once a save
exists. `--roundtrip-every N`
saves, loads and compares the state every N pages; a `save/load
mismatch` warning means part of the state (filters, the `pushscreen`
stack, screen effects, variables) doesn't survive a save.

**Sound.** `sound` records say when a `chvolfade`/`chpan`/`chfreq` ramp
got there and when a sound ended, `ms_cpu` marks a command that held the
main thread over a frame, and `--audio-stats` adds an `audio` record per
100 ms per channel and for the mix (RMS, balance, frequency, largest
sample step, file position): sound plays in real time, so run it with
`--real-time` or without `--auto-play` (`engine/DEBUG.md`).

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

**A frame mid-motion.** `--real-time --shot-at '*label:line+ms'` takes a
picture that many ms of game time after the line has run (a `spmotion`
line: the sprite part of the way), `--shot-after ms` at a game time.
Under `SDL_VIDEODRIVER=dummy` the run has a virtual clock that moves
only as the engine waits, so two runs give the same frames
(`compare -metric AE` 0) and the waits cost no wall-clock time. A
`savescreenshot` that writes nothing is a `screenshot not written`
warning (with `--save-dir` it writes into the save folder).

**When the trace can't do it** (the older recipe): patch a copy of
`0.txt`: `textbtnwait` in `*conv_wait` → `mov %9,0`; comment out
`click` in `*conv_click_wait`; put `mov %101,1:goto *label` right after
`erasetextwindow 0` in `*start`. `research/xuse/smoke.py` does scripted
clicks and screenshots (its click queue is `%1000+`; keep converter
variables out of 1000-1099).

## Test saves at chosen scenes

To hand the user (or a later check) save files that stand at the
moments worth testing (battles, movies, minigames, route splits,
endings), list them in a slots file, one `slot label description` a
line, and run:

```sh
python3 -I research/tools/test_saves.py out/<game>-cons <slots.txt> -o <game>-test-saves.zip
```

For each line the engine starts at the label (`--goto`) and saves there
before any of the scene runs (`--checkpoint-start --checkpoint-slot
<slot>`), so loading the save plays the whole scene, its opening
effects too; it then plays to the first page for the thumbnail's
picture (`--shot-pages 1`, or `--shot-after 3000` when a run gets stuck
before it). A checkpoint is a bare save, without what our save
menu adds when the player saves, so the script adds both: a 160×90
`savethumbN.bmp` (the menu's thumbnail) and the slot's text, which
`savegame2` appends to the file as `"<date>|<play time>|<text>"*` (cp932)
and `getsavestr` reads back from the end; the description goes in the
text. The zip holds the files flat and uncompressed, as the web
player's Export Saves writes them (`save16.dat`, `savethumb16.bmp`);
its Import Saves takes that, and so does a native save folder (unzip
the `.dat` and `.bmp` files into it). Leave `gloval.sav` and
`kidoku.dat` out: they would replace the player's unlocks and read
text.

Without `--vars-dir` the saves carry none of the route's earlier
choices, so a scene that branches on them may go another way. With
`--vars-dir <dir>`, each label's `<dir>/<label>.txt` (a `--vars` file,
"Starting anywhere" in `engine/DEBUG.md`) sets the variables first, and
the save holds them. For Muramasa, `research/nitroplus/route_vars.py`
writes those files from the route paths, as nstrace has every variable
when a path first enters the label (a second, no engine run):

```sh
L=$(grep -v '^#' research/nitroplus/muramasa-test-saves.txt | awk '{print $2}' | paste -sd,)
python3 -I research/nitroplus/route_vars.py out/muramasa-cons <vars dir> research/nitroplus/paths/*.txt --labels $L
python3 -I research/tools/test_saves.py out/muramasa-cons research/nitroplus/muramasa-test-saves.txt \
    --vars-dir <vars dir> -o muramasa-test-saves.zip
```

Global variables (the `#` flags from finished routes, `%800` up) live in
`gloval.sav`, not in a save, so they are set for the run but don't come
with the zip.
