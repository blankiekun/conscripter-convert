# Lessons learned and ways to speed up conversion

General lessons, collected while converting games. Each one is stated
for any engine, with the case that taught it as an example (untagged
ones come from 最果てのイマ, Xuse; its details are in
`docs/games/saihate-no-ima.md`), grouped by topic: add a new lesson to
its topic, not under a heading for its game.
The steps that apply them are in `docs/new-game.md`.

## Lessons learned

**Know the game before the bytecode.**
- **Learn the game's mechanics first** (wiki, playthrough video): the
  route choice mechanic, loops, endings. Example: 最果てのイマ's route
  choices are hyperlinks in the text. A no-click trace hit a dead end
  that restarts the chapter, which looked like a control-flow bug until
  the link targets were followed.
- **A loop is not necessarily a bug.** Before "fixing" one, check whether
  the original loops there too (dead ends, retries, back to the title).
- **Ask what to drop.** Debug menus, right-click menus and side rooms
  can each cost a session. The user decides.
- **A 100 % save unlocks the extras at once.** Download one instead of
  playing or reverse-engineering the unlock bits.
- **A route map straight from the scripts** (俺たちに翼はない: every NextScene, condition
  and choice per scene, one line each) is the walkthrough when no
  public one exists, and the route check follows it.

**Reading the exe.**
- **Rank opcodes by use count** and convert from the top: a handful of
  opcodes make most of the visible game.
- **Strings name handlers.** Error messages, logged source-file names
  (`Story.cpp` is bookkeeping, not display) and RTTI class names settle
  an opcode's meaning faster than tracing its logic. Dump them first.
- **MSVC RTTI maps factory ids to classes:** `.?AV<class>@@` string →
  type descriptor → complete object locator → vtable. The constructor
  that stores the vtable, with a static `{type id, factory}` table in
  `.rdata`, gives the class for each id.
- **Jump tables can be two-level:** a byte table indexed by the
  sub-number, then an address table. Reading the address table alone
  gives wrong handlers.
- **Look for divide-by-constant magic numbers.** A division compiles to
  a multiply and shift (`imul 0x14f8b589; sar 13` is ÷ 100000). Searching
  for the literal divisor finds nothing and wrongly suggests a lookup
  table.
- **Follow a field into the code before naming it**, and never name it
  after the values it takes. Examples: a 0/128/255 byte that looked like
  black/grey/white was an acceleration; a field whose values (1000, 500,
  0) looked like fade milliseconds was a sort depth, which the handler
  passed to the object creator (fixed in `6b22f6d`).
- **An operand can refer to state defined elsewhere.** An opcode may
  only name a preset that another opcode defines in a setup script; the
  context around the definitions (evening and night backgrounds next to
  colour presets) can fix their meaning.
- **Object lifetimes depend on mode bits.** Check across the corpus which
  modes keep an object (and what is attached to it) and which recreate
  it, before tracking state.
- **Map an exe from the corpus, not from its functions.** (村正) A coverage
  table (each built-in, system variable, text tag and action: corpus use
  count, what the exe does, what the converter does) turns "map the
  whole exe" into a bounded list of differences actually used; code
  reached only through function pointers has to be turned into functions
  before the decompiled output shows what a built-in does
  (`docs/new-game.md` §2, "Mapping the whole exe").
- **A built-in "converted" can still drop an argument** (村正, 2026-10-16):
  NSS `CreateWindow` was listed as converted because its children were
  placed, but its size, which clips them, was never read; the user saw
  the cockpit's tapes run the whole screen. Coverage is per argument and
  behaviour, and the built-in dispatch notes every argument it doesn't
  read (`docs/new-game.md` step 5).
- **Read the scripts' comments.** (俺たちに翼はない) Ports often keep the original's names
  in comments next to the numbers that replaced them (`:Texture 0 //
  BG_BLACK`, `:Voice 22562 // TAK1004`). Checking every commented number
  against the archives proved the ID scheme in one pass, and the
  comments gave the speaker-name table without the executable.

**Checking decoded data.**
- **Check the whole corpus, not one sample:** every message parses,
  every jump target exists, every checksum verifies. Corpus-wide checks
  catch edge cases that one scene hides.
- **Find out how many index spaces a jump uses** (per-script labels,
  global tables, implicit entries, labels on marker instructions).
  Getting it wrong still produces a script that runs, so check it with
  a trace, not by eye.
- **Cross-check references against asset names.** Name prefixes in the
  archives are a free oracle: checking each sprite reference's character
  against its image name found 623 mismatches in one pass.
- **A prefix check can't catch off-by-one inside a block.** The next
  image is usually the same character, so a 0-based misread still passed
  97 %. Count references to degenerate assets (empty files, 1×1
  placeholders) instead: 1,639 references dropped to 0 after the fix.
- **Count pages as they are displayed, not in file order.** Setup
  scripts and debug menus come first in the file but are never shown.
- **Check the routes against a walkthrough before calling a game done.**
  Simulate each route on both sides (the source with the original's
  run-time rules, the converted `0.txt`) and diff page by page,
  including the wrong-order clicks the walkthrough warns about. Static
  reading and spot runs missed a link that stayed clickable after a
  jump skipped its record (`docs/new-game.md` step 11).
- **Whole-corpus runs find what single scenes don't.** (村正) Two engine-killing
  bugs only showed on the full game: more than 254 transition effects
  (`effect` numbers stop at 255) and a half-width character in vertical
  text (CONscripter exits silently). Run the whole game, at least as a
  dry run, after every feature.
- **Check duplicate definitions for comment blocks** before deciding which
  one is live (Muramasa keeps old helper versions inside `/* */`).
- **Skip runs per ending find what logic checks can't** (村正, 2026-10-14): a
  `?` in text (NScripter reads an array and exits), a 270 s cross-fade,
  heap corruption after 500 files. Patch the choices into the script
  (one `if` per line: a false `if` skips the rest of its line) or write
  them as a `--replay` file, play headless with `--auto-play` and a
  trace (no Xvfb, no held Ctrl; a crash leaves a `crash` record), and
  start deep in a route from the tracer's variables (`--goto` and
  `--vars`, or a patched start) to reproduce in seconds.

**Matching the original.**
- **Read the effect's code, then film only what it leaves open.** The
  exe's kernel gives the exact curve; static reading of the scripts
  alone had got transitions wrong. Wine is the last resort (the user,
  2026-10-16: it takes long and costs many tokens): when a real doubt
  is left, run it silent on its own Xvfb display with a script copy
  patched to slow the effect down, take the fewest shots at fixed
  fractions and fit them to the curve (`CLAUDE.md`, "Rules").
- **Use reference screenshots of the original** for layout: measuring
  pixels settles the text box and name plate in one pass.
- **A wine prefix shares the user's home by default.** Its Documents,
  Desktop etc. are symlinks to the real folders, and games save there.
  Replace them with real folders before the first run.
- **Effects ONScripter can't draw: approximate in the converter first,
  then add them to the engine.** Pre-tinted sprite copies (~930 images,
  130 MB) were replaced by the engine's `spfilter`, which also follows
  the game's control flow instead of a file-order guess.
- **Small effect plugins beat general machinery.** Each transition family
  became a short `cons_*.dll` effect-99 plugin, checkable on Xvfb
  mid-effect. Release them in batches: a CONscripter release is a
  version bump and a tag, and CI builds every platform.
- **Atlases without their layout table:** (俺たちに翼はない) measure the parts from the
  picture's alpha (`research/oretsuba/atlas.py`) and say in the game's
  document that the placement is ours.

**Target engine facts (ONScripter / CONscripter).**
- **Sprite numbers set the stacking order, and saves record them.**
  Lower `lsp` numbers draw on top, so dialogs need the lowest slots.
  Renumber menus and bars freely, but keep the game's own sprites at
  fixed numbers so old saves still load.
- **The font must be fixed-width.** ONScripter draws every character in
  a full-width cell, so a proportional font leaves gaps after narrow
  glyphs. Use IPA Gothic (`ipag.ttf`).
- **The target's image decoder limits the asset formats.** Check a new
  format with one test file in every target build (the web build
  especially) before changing the converter. WebP needed CONscripter's
  own libwebp on the web.
- **Judge lossy image quality on the worst region,** not the average:
  crop the worst block of the difference image and compare it enlarged.
  Lossy WebP's 4:2:0 subsampling blurred small coloured detail at every
  quality; near-lossless was fine at half the PNG size, so lossy is only
  an option (`--webp-quality`).
- **Measure completion per source call, not per note.** One call with
  three unread arguments is one gap; a note list alone ranks a command
  by how many arguments it has. Each engine opens a call on the shared
  `Metrics`, and every note made while it is open is that call's gap
  (`tests/snapshots/<game>.metrics.json`).
- **Engine facts worth knowing early:** (村正) `:a` on a JPEG uses its right
  half as the mask; loose files with Japanese names were found only
  once CONscripter looked for their UTF-8 names (4d94b92; before, pack
  them); `;$Sw,h` sets any screen size, `,valueN` raises the global
  border so more locals are saved per slot; `print 10,ms` works without
  an `effect` definition; `%%n` and `$%n` (the variable whose number is
  in `%n`) work as values and as targets.
- **Button loops:** (村正) `btnwait` forgets its buttons once it returns, so
  every way back into the loop defines them again. Sprite buttons are hit
  by rectangle unless `transbtn` is on (then by alpha; `btndef` turns it
  off); with it, a "transparent" button keeps a little alpha (8/1000) to
  stay clickable.
- **CONscripter's errors go to `stderr.txt`** (村正) in the game folder
  (`USE_FILELOG`), not to the terminal: an exit that looks silent has
  its message there.
- **`setwindow` flushes the backlog** (村正; NScripter's rule): a converter
  that sets the window again for each text block keeps no history. The
  converter emits `setwindow3`, which doesn't. And `btndef` turns
  `btndown` off: a screen that wants presses sets `btndown 1` after each
  `btndef clear`, and in that mode `btnwait` also returns on the release
  and on moves with the button held (tell them apart with `isdown`).
- **`lsp2` sprites are no place for things over the text window** (村正): with
  `windowback` they are drawn under it, and ONScripter doesn't step
  their animation. An animated icon over the window is an `lsp` sprite
  numbered at or below `humanz`.
- **Mind the output stage's variables:** (俺たちに翼はない) pick the game's flag ranges
  against the `%N` that conv-cons already writes (`grep -o '%[0-9]*'`),
  locals below the global border and globals clear of the menus'.

**Getting the files.**
- **Console packages decrypt offline.** A Vita package plus its license
  (`work.bin`) unpacks with pkg2zip; the PFS layer with Vita3K's fork of
  psvpfsparser, which has the F00D step built in (the web service the
  original tool needs is gone). Build both from source.

**Compiling from source scripts** (Nitroplus NSS: 装甲悪鬼村正).
- **With source scripts, compile by running them at convert time.** Story
  scenes are straight runs of helper calls. Inlining the helpers, tracking
  objects and variables, and emitting what the player sees at each wait
  gave a readable build in one session. Only what really depends on the
  player has to become runtime code.
- **Find the runtime state statically, then keep it small.** Story flags
  = variables the story files both set and test (about 190 of thousands).
  Everything else (helper scratch, positions, names) stays convert-time.
  A choice or a flag test forks the run; branches that lead to different
  files dispatch at the end of the file, the rest merge.
- **Pure-logic library code that loops over flags** (the route ranking)
  can't be traced. Port it as a small intrinsic and test it with an IR
  interpreter against a hand-worked example.
- **Merging fork states:** a variable set again after the fork must keep
  that later value. Small synthetic file sets in unit tests caught this.

**Builds.**
- **Measure memory before blaming it on mapped files:** (村正) the kernel's OOM
  line gives anon-rss. The full build's 7 GB was heap (60x zooms scaled
  whole, 8 GB bitmaps), not the mapped archives; cutting big zooms to the
  screen brought it to 1.2 GB. Run builds under `research/tools/capped.sh
  <GiB> <cmd>` so a regression can't take the desktop down.
- **An opaque WebP has no alpha channel** (月姫): libwebp leaves it out when
  every pixel is opaque, and on native builds (SDL_image) `lsp ":a;…"`
  took such a picture's right half as the mask; a PNG without alpha was
  already exempt. CONscripter now treats a WebP as it treats a PNG
  (engine PR #55); the web build's own decoder was never affected.
- **Check a muxer's output with the strict reader** (月姫): Ogg Opus files
  with a wrong OpusTags vendor length played in ffmpeg and failed in
  opusfile, which the engine uses for chunks; music, through SDL_mixer,
  played. The trace's "broken sound" found it.
- **No HashMap or HashSet iteration into output** (村正): its order changes
  from run to run, so two dry runs of one commit gave different `0.txt`
  (`csp` lines swapped). Iterate a `BTreeMap`/`BTreeSet`, or sort first;
  a hash map is fine for lookups only. Check: dry-run twice, same hash.

## Ways to speed up conversion

Done, use them: the engine's debug trace with `--auto-play`, `--goto`
`--vars`, `--shot-pages`, `--shot-effects` (replaces a static tracer, the
smoke-test recipe and filming our side's effects on Xvfb;
`engine/DEBUG.md`); `--dry-run` for logic (seconds, script only);
`conscripter-convert lint`; `convert-script` for one scene; `nss route`
for a route map; `tests/snapshot.sh`, whose dry runs also check that
every file 0.txt names would be written (docs/tools.md).

Still worth building, in rough order of payoff:
1. **Incremental full builds**: keep extracted assets between runs and
   repack only what changed, or stream archive entries straight into the
   `.nsa`; extract in parallel. A full build is minutes, nearly all of it
   assets.
2. **Corpus tests in `cargo test`**: every message parses. (Every jump
   target exists and every referenced asset is written: the dry run's
   lint in `tests/snapshot.sh`, done.)
3. **A frontier report**: compare the files a run reaches with the route
   map's successors and print the condition that blocks each.
4. **Game knowledge up front**: route structure, the choice mechanic,
   known endings, given to an agent before it infers opcodes.
5. **Run exe kernels at convert time**: call a transition's or an
   easing's function from the user's own exe in an x86 emulator (for
   example the `unicorn` crate) and keep its output as data: mask frames
   for `cons_rule.dll`, curve tables for `spcurve`. The original's exact
   pixels without porting its code, and nothing from the exe goes into
   the engine or the repo. Works for self-contained functions; one that
   reaches into the exe's objects or renderer still needs reading.

## Open items from the October 2026 audit

The audit itself is in git history (docs/audit-2026-10.md, removed). Still open:

- **Colours into the IR (step 17):** skipped while one game has links;
  move them into the IR when a second game needs other ones.
- **Rename `onsyuri` (step 21):** done: the web index is
  `cons_index.json`, saves `/cons_save`, the JS factory `cons`, the
  Android package `io.github.blankiekun.conscripter`. The stock page does
  not move old `/onsyuri_save` saves (the site's page does).
