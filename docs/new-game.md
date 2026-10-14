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
   one)? Anything to skip? Write the answers down in the handoff file
   (step 0.4). Things the user dropped for 最果てのイマ: the debug menu,
   right-click menus, the WSS system-voice room, native Linux/macOS
   movies.
2. **Learn the game as a player.** Read a wiki or skim a playthrough for
   the route structure, the choice mechanic, endings and extras.
   最果てのイマ's route choices are hyperlinks in the text. Not knowing
   that cost a session: a no-click trace loops back to the chapter start,
   and that looked like a control-flow bug. Save a walkthrough in the
   handoff file before converting control flow (step 11.1).
3. **Identify the engine** from the install: the exe's strings,
   `.cpp`/RTTI names, archive magic numbers, config files. If it is a
   known engine (KiriKiri, NScripter, Siglus, Nitroplus NSS, ...), find
   the existing format documentation and tools before reversing anything.
4. **Start the files that survive context resets**:
   - `<game>.md` in the repo root, git-ignored (add it to `.gitignore`),
     with a **"Resume here"** section at the top. It holds paths, recipes
     and findings. Look at `saiima.md` and `muramasa.md`. Give it two
     sections from the start for step 11: **"Walkthrough"** (with its
     source) and **"Progression variables"** (the table of step 11.2).
   - `docs/games/<game>.md`, committed: the game's record (install and
     build command, formats, opcode or command table with handler
     addresses, how its screens map to CONscripter, progression
     variables, status and leftovers). `docs/games/saihate-no-ima.md`
     is the model. Findings move there from the handoff file once they
     hold up.
   - A branch per game (`xuse`, `nitroplus`), based on `origin/main`. A
     second agent works in its own worktree
     (`~/Projects/conscripter-convert-<name>`).
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
  name it logs, the RTTI class it builds. Disassemble once with
  `objdump -d -M intel --no-show-raw-insn <exe>` into
  `research/<engine>/local/` (git-ignored) and grep that.
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
  (ONScripter ignores SIGTERM in a script loop).
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
button presses and screenshots).

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
  wine and Xvfb afterwards. Recipes: `saiima.md`, "Running the
  original"; `muramasa.md` for a game that needs DirectSound.
- Then read the effect's kernel in the exe for the exact curve, and fit
  the filmed frames to it. Example: MeltDown, edge = h·((p − d)/(1 − d))².
- Check our side the same way: run CONscripter on its own Xvfb display
  with `SDL_RENDER_DRIVER=software` (otherwise the grab is black) and
  grab frames mid-effect with `import`.

## 10. Engine additions

When existing NScripter commands can't do it:
1. Add it opt-in in `engine/src/onsyuri/` (`ONScripter_cons.cpp`
   preferred). Effects are `cons_*.dll` effect-99 plugins; screen
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
   the handoff file's "Walkthrough" section: which choices or links lead
   to which chapters, the loop or cycle order, the true-ending
   conditions. Note its source (site and page).
2. **Map the progression variables.** Find every variable the script
   uses to gate routes, chapters, loops and endings. For each one,
   record its meaning, where it is set, reset and read, and whether the
   original keeps it in save data or in system data. Put the table in
   the handoff file and in the game's document.
3. **Check the converted branches against the walkthrough.** For each
   step of the walkthrough, follow the converted `0.txt`, or run it
   headless with a scripted click path (`research/xuse/smoke.py`) and
   print the variables with the `lsp 999,"X:"+…` trick (`CLAUDE.md`).
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
