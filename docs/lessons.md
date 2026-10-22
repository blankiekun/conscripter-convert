# Lessons learned and ways to speed up conversion

Flat reference: one lesson per bullet, its leading word first, grouped by
topic. Each is stated for any engine with the case that taught it
(untagged: 最果てのイマ). Add a new one to its topic. The steps that apply
them are in `docs/new-game.md`.

## Lessons learned

**Know the game before the bytecode**
- **Mechanics first**: wiki or playthrough video for the route choice
  mechanic, loops, endings. 最果てのイマ's route choices are hyperlinks in
  the text; a no-click trace hit a dead end that restarts the chapter and
  looked like a control-flow bug until the link targets were followed.
- **Loops**: check whether the original loops there too (dead ends,
  retries, back to the title) before "fixing" one.
- **Drops**: the user decides what to drop. Debug menus, right-click menus
  and side rooms can each cost a session.
- **100 % save**: download one to unlock the extras at once, instead of
  playing or reverse-engineering the unlock bits.
- **Route map**: every NextScene, condition and choice per scene, one line
  each, straight from the scripts (俺たちに翼はない), is the walkthrough
  when no public one exists, and the route check follows it.

**Reading the exe**
- **Rank opcodes by use count** and convert from the top: a handful make
  most of the visible game.
- **Strings name handlers**: error messages, logged source-file names
  (`Story.cpp` is bookkeeping, not display) and RTTI class names settle an
  opcode's meaning faster than tracing its logic. Dump them first.
- **MSVC RTTI maps factory ids to classes**: `.?AV<class>@@` string → type
  descriptor → complete object locator → vtable. The constructor that
  stores the vtable, with a static `{type id, factory}` table in `.rdata`,
  gives the class for each id.
- **Two-level jump tables**: a byte table indexed by the sub-number, then
  an address table; the address table alone gives wrong handlers.
- **Divide-by-constant magic numbers**: a division compiles to a multiply
  and shift (`imul 0x14f8b589; sar 13` is ÷ 100000). Searching for the
  literal divisor finds nothing and wrongly suggests a lookup table.
- **Follow a field into the code before naming it**, never after the
  values it takes: a 0/128/255 byte that looked like black/grey/white was
  an acceleration; a field with values 1000, 500, 0 that looked like fade
  milliseconds was a sort depth, which the handler passed to the object
  creator.
- **Operands refer to state defined elsewhere**: an opcode may only name a
  preset that another opcode defines in a setup script; the context around
  the definitions (evening and night backgrounds next to colour presets)
  can fix their meaning.
- **Object lifetimes depend on mode bits**: check across the corpus which
  modes keep an object (and what is attached to it) and which recreate it,
  before tracking state.
- **Map an exe from the corpus, not from its functions** (村正): a coverage
  table (each built-in, system variable, text tag and action: corpus use
  count, what the exe does, what the converter does) turns "map the whole
  exe" into a bounded list of differences actually used. Code reached only
  through function pointers has to become functions before the decompile
  shows what a built-in does (`research/tools/ghidra/`).
- **What the original does is a question for its code, not the user**:
  "does a new BGM cut the one fading out?" waited a day on the user; the
  decompiles answered it in an hour. Rebuild a missing Ghidra export from
  the game document's recipe (`ghidra_export.sh`, minutes); name a
  subsystem whole (channels, states, per-frame update) when you first
  touch it, not only the command's entry. Ask the user for decisions only.
- **"Converted" can still drop an argument** (村正): NSS `CreateWindow` was
  listed as converted because its children were placed, but its size,
  which clips them, was never read; the cockpit's tapes ran the whole
  screen. Coverage is per argument and behaviour; the built-in dispatch
  notes every argument it does not read (`docs/new-game-reference.md`,
  "Route simulation").
- **Script comments** (俺たちに翼はない): ports keep the original's names in
  comments next to the numbers that replaced them (`:Texture 0 //
  BG_BLACK`, `:Voice 22562 // TAK1004`). Checking every commented number
  against the archives proved the ID scheme in one pass and gave the
  speaker-name table without the executable.

**Checking decoded data**
- **Whole corpus, not one sample**: every message parses, every jump
  target exists, every checksum verifies.
- **Index spaces**: find out how many a jump uses (per-script labels,
  global tables, implicit entries, labels on marker instructions). A wrong
  guess still yields a script that runs, so check with a trace.
- **Asset names are a free oracle**: name prefixes in the archives checked
  against each sprite reference's character found 623 mismatches in one
  pass.
- **Prefix checks miss off-by-one inside a block**: the next image is
  usually the same character, so a 0-based misread still passed 97 %. Count
  references to degenerate assets (empty files, 1×1 placeholders) instead:
  1,639 references dropped to 0 after the fix.
- **Pages as displayed**: count them in play order, not file order; setup
  scripts and debug menus come first in the file but are never shown.
- **Routes against a walkthrough** before calling a game done: simulate
  each route on both sides (the source with the original's run-time rules,
  the converted `0.txt`) and diff page by page, wrong-order clicks the
  walkthrough warns about included. Static reading and spot runs missed a
  link that stayed clickable after a jump skipped its record
  (`docs/new-game-reference.md`, "Route simulation").
- **Whole-corpus runs find what single scenes do not** (村正): two
  engine-killing bugs showed only on the full game, more than 254
  transition effects (`effect` numbers stop at 255) and a half-width
  character in vertical text (CONscripter exits silently). Run the whole
  game, at least as a dry run, after every feature.
- **Duplicate definitions**: check for comment blocks before deciding which
  is live (Muramasa keeps old helper versions inside `/* */`).
- **Skip runs per ending** (村正) find what logic checks cannot: a `?` in
  text (NScripter reads an array and exits), a 270 s cross-fade, heap
  corruption after 500 files. Patch the choices into the script (one `if`
  per line: a false `if` skips the rest of its line) or write a `--replay`
  file; play headless with `--auto-play` and a trace (no Xvfb, no held
  Ctrl; a crash leaves a `crash` record); start deep in a route from the
  tracer's variables (`--goto` and `--vars`, or a patched start) to
  reproduce in seconds.

**Matching the original**
- **Effect code first, then film only what it leaves open**: the exe's
  kernel gives the exact curve; static reading of the scripts alone had
  got transitions wrong. When wine is still needed (`CLAUDE.md`, "Rules"),
  patch a script copy to slow the effect, take the fewest shots at fixed
  fractions and fit them to the curve.
- **Snapshot run for timing** (続・殺戮のジャンゴ): one 40 s wine run of a
  probe scene that dumps the process's memory before and after each
  overlap case answered the FadeFade question (the pan's step counter went
  on through `Fade(0)`); the exe's own frame function then steps the dumps
  under Unicorn with no further wine (`docs/tools.md`, "Matching the
  original").
- **Reference screenshots**: measure them for layout (text box, name
  plate).
- **Wine prefix** shares the user's home by default (Documents etc. are
  symlinks, and games save there): make them real folders first.
- **Effects ONScripter cannot draw**: approximate in the converter first,
  then add them to the engine. Pre-tinted sprite copies (~930 images,
  130 MB) were replaced by the engine's `spfilter`, which also follows the
  game's control flow instead of a file-order guess.
- **Effect plugins**: small ones beat general machinery. Each transition
  family became a short `cons_*.dll` effect-99 plugin, checkable on Xvfb
  mid-effect. Release them in batches: a CONscripter release is a version
  bump and a tag, and CI builds every platform.
- **Atlases without their layout table** (俺たちに翼はない): measure the
  parts from the picture's alpha (`research/oretsuba/atlas.py`) and say in
  the game's document that the placement is ours.

**Target engine facts (ONScripter / CONscripter)**
- **Sprite numbers set stacking order, and saves record them**: lower
  `lsp` numbers draw on top, so dialogs need the lowest slots. Renumber
  menus and bars freely; keep the game's own sprites at fixed numbers so
  old saves still load.
- **Fixed-width font**: ONScripter draws every character in a full-width
  cell, so a proportional font leaves gaps after narrow glyphs
  (`docs/cli.md`, `--font`). Use IPA Gothic (`ipag.ttf`).
- **Image decoder limits asset formats**: test a new format with one file
  in every target build (the web build especially) before changing the
  converter. WebP needed CONscripter's own libwebp on the web.
- **Lossy image quality**: judge the worst region, not the average; crop
  the worst block of the difference image and compare it enlarged. Lossy
  WebP's 4:2:0 subsampling blurred small coloured detail at every quality;
  near-lossless was fine at half the PNG size, so lossy is only an option
  (`--webp-quality`).
- **Completion per source call, not per note**: one call with three unread
  arguments is one gap; a note list alone ranks a command by how many
  arguments it has. Each engine opens a call on the shared `Metrics`, and
  every note made while it is open is that call's gap
  (`tests/snapshots/<game>.metrics.json`).
- **Engine facts worth knowing early** (村正): `:a` on a JPEG uses its
  right half as the mask; loose files with Japanese names are found only
  by CONscripter's UTF-8 lookup; `;$Sw,h` sets any screen size; `,valueN`
  raises the global border so more locals are saved per slot; `print
  10,ms` works without an `effect` definition; `%%n` and `$%n` (the
  variable whose number is in `%n`) work as values and as targets.
- **Button loops** (村正): `btnwait` forgets its buttons once it returns,
  so every way back into the loop defines them again. Sprite buttons are
  hit by rectangle unless `transbtn` is on (then by alpha; `btndef` turns
  it off); with it, a "transparent" button keeps a little alpha (8/1000)
  to stay clickable.
- **Errors go to `stderr.txt`** (村正) in the game folder (`USE_FILELOG`),
  not the terminal: an exit that looks silent has its message there.
- **`setwindow` flushes the backlog** (村正; NScripter's rule): setting
  the window for each text block keeps no history, so the converter emits
  `setwindow3`, which does not. `btndef` turns `btndown` off: a screen that
  wants presses sets `btndown 1` after each `btndef clear`, and in that
  mode `btnwait` also returns on the release and on moves with the button
  held (tell them apart with `isdown`).
- **`lsp2` sprites under the text window** (村正): with `windowback` they
  are drawn under it, and ONScripter does not step their animation. An
  animated icon over the window is an `lsp` sprite numbered at or below
  `humanz`.
- **Output stage's variables** (俺たちに翼はない): pick the game's flag
  ranges against the `%N` conv-cons already writes (`grep -o '%[0-9]*'`):
  locals below the global border, globals clear of the menus'.

**Getting the files**
- **Console packages decrypt offline**: a Vita package plus its license
  (`work.bin`) unpacks with pkg2zip; the PFS layer with Vita3K's fork of
  psvpfsparser, which has the F00D step built in (the web service the
  original tool needs is gone). Build both from source.

**Compiling from source scripts** (Nitroplus NSS: 装甲悪鬼村正)
- **Compile by running at convert time**: story scenes are straight runs
  of helper calls. Inlining the helpers, tracking objects and variables,
  and emitting what the player sees at each wait gave a readable build in
  one session. Only what depends on the player becomes runtime code.
- **Runtime state, found statically and kept small**: story flags =
  variables the story files both set and test (about 190 of thousands);
  everything else (helper scratch, positions, names) stays convert-time. A
  choice or a flag test forks the run; branches that lead to different
  files dispatch at the end of the file, the rest merge.
- **Pure-logic library code that loops over flags** (the route ranking)
  cannot be traced: port it as a small intrinsic and test it with an IR
  interpreter against a hand-worked example.
- **Merging fork states**: a variable set again after the fork keeps that
  later value. Small synthetic file sets in unit tests caught this.

**Builds**
- **Memory**: measure before blaming mapped files (村正). The kernel's OOM
  line gives anon-rss; the full build's 7 GB was heap (60x zooms scaled
  whole, 8 GB bitmaps), not the mapped archives; cutting big zooms to the
  screen brought it to 1.2 GB. Run builds under `research/tools/capped.sh
  <GiB> <cmd>` so a regression cannot take the desktop down.
- **Opaque WebP has no alpha channel** (月姫): libwebp leaves it out when
  every pixel is opaque, and on native builds (SDL_image) `lsp ":a;…"`
  took such a picture's right half as the mask; a PNG without alpha was
  already exempt. CONscripter treats a WebP as it treats a PNG.
- **Muxer output**: check it with the strict reader (月姫). Ogg Opus files
  with a wrong OpusTags vendor length played in ffmpeg and failed in
  opusfile, which the engine uses for chunks; music, through SDL_mixer,
  played. The trace's "broken sound" found it.
- **Deterministic output** (村正): iterate a `BTreeMap`/`BTreeSet` (or
  sort first) into output; a hash map is for lookups only. Hash iteration
  order changes per run, and two dry runs of one commit gave different
  `0.txt` (`csp` lines swapped). Check: dry-run twice, same hash.

## Ways to speed up conversion

The engine's debug trace with `--auto-play`, `--goto`, `--vars`,
`--shot-pages`, `--shot-effects` (`engine/DEBUG.md`); `--dry-run` for
logic; `conscripter-convert lint`; `convert-script` for one scene; `nss
route` for a route map; `tests/snapshot.sh` (`docs/tools.md`).
