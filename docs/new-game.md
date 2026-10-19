# Converting a new game: steps and planning

A playbook for an agent starting a new game, in the order 最果てのイマ
COMPLETE (Xuse, finished 2026-10-14) showed would have been fastest.
General lessons: `docs/lessons.md`; house rules: `CLAUDE.md`.

For scale: 最果てのイマ took two days of agent sessions (~120 converter
commits, ten releases); the story played end to end after day one. 装甲悪鬼村正
(NSS) and 俺たちに翼はない (PS Vita) converted their whole story, every ending
reachable, in about 70 commits each over two days.

## Checklist

One line per step; the section has the how. Read a section when you reach
it, not the whole file (`python3 -I research/tools/mdsec.py docs/new-game.md "9. Matching"`).
"Step 14" to "step 24" elsewhere (game documents, `docs/tools.md`) are the
items of section 2's list "Mapping the whole exe", not rows of this table.

| Step | Do | Done when |
|---|---|---|
| 0 | Ask the user the scope; learn the game as a player; identify the engine (first-contact kit: report, then one logged run); start `docs/games/<id>.md` from the template | Scope, walkthrough source and engine are in the document |
| 1 | Archive reader and format decoders, `<format> list\|extract` | The whole corpus verifies and decodes; degenerate assets counted |
| 2 | Decode the scripts; rank opcodes by use; Ghidra export with `names.tsv`; coverage table; map the exe (section 2 list, items 1-17) | Every script parses, every jump target exists; findings table marks each behaviour seen/inferred |
| 3 | Lower to the IR; flags in `conv_core::alloc`'s free ranges | `--dry-run` writes a `0.txt`; what is left out is a ranked list |
| 3a | Registry entry (`tests/snapshot.sh` reads it) and baseline (`docs/new-engine.md` 6) | `tests/snapshot.sh` covers the game |
| 4 | Dry run + `lint` + debug trace after every feature | Trace check clean; every ending reachable |
| 5 | Text box, name plate, voices, ruby; the longest message fits | Every voiced line plays |
| 6 | Pictures, sound, transitions with real durations | Sprite references match asset names |
| 7 | System screens from the IR's screens; boot warnings and brand logos (`Script::boot`) | Every screen opens, closes by mouse, keyboard and touch; launch to title matches the original's steps, times and skip rules |
| 8 | Extras from a 100 % save | Locked and unlocked states both right |
| 9 | Match the original: the manual first, then code, then the oracle, wine or an emulator last; trace DLL and trace diff (section 2 list, items 22-23; "Trace speeds") | Differences measured (`compare -metric`, `trace_diff.py`), shots kept in `local/shots/` |
| 10 | Engine additions, engine PR first | Documented in the engine's `COMMANDS.md`; submodule on CONscripter `master` |
| 11 | Route, progression, unlocks, settings, input, saves across rebuilds | The document says "progression verified: yes, date, how" |
| 12 | Web build, loose files | Played in a browser; size and file count in the document |
| 13 | Finish: README, CLAUDE.md's table, `docs/lessons.md`, GUI entry | The user agrees it's done |

## 0. Before writing any code

1. **Ask the user** what they want: everything with extras, or story first?
   Which platforms (the web build is the main one)? Anything to skip? Write
   the answers in the game's document (step 0.4). (最果てのイマ: the user
   dropped the debug menu, right-click menus, the WSS system-voice room,
   native Linux/macOS movies.)
2. **Learn the game as a player.** Read a wiki or skim a playthrough for the
   route structure, choice mechanic, endings and extras; save a walkthrough
   in the game's document before converting control flow (step 11.1).
   (最果てのイマ: route choices are hyperlinks in the text; a no-click trace
   looped to the chapter start and looked like a control-flow bug.)
3. **Identify the engine** from the install: exe strings, `.cpp`/RTTI names,
   archive magic, config files. For a known engine (KiriKiri, NScripter,
   Siglus, Nitroplus NSS, ...), find its format documentation and tools first.
   For a Windows game, start with the **first-contact kit**
   (`research/tools/firstcontact/`): `first_contact.py <install>
   [--ghidra <export> | --ghidra-run]` writes
   `out/<install name>-first-contact/report.md`: a "Known family" block when
   we already convert the family (nearest game's document, readers and modules
   that carry over, the command to start from; works on console dumps too),
   then engine guesses from file names, archive magic and exe strings, imports
   by use, files by extension, RTTI. With a Ghidra export it also writes
   `hooks.tsv`: candidate dispatch, wait-loop and present functions as count
   breakpoints. Then **one** logged run, `fc.sh <install> --hooks
   <out>/hooks.tsv` (headless, 300 s, a click and Enter every 2 s):
   `trace.jsonl.gz` holds every file opened and read (offsets, so archive
   entries can be named), DirectSound buffers and plays with the file read
   just before, presents with times, input, timers, per-second counts of
   sleeps, clock calls and breakpoint hits, and the exe call sites on the
   stack of each open, sound and present. `summary.txt` ranks each
   breakpoint against the presents. Answer later questions from this log, not
   new runs. Screenshots are lossy WebP (quality 80: for looking and coarse
   compares, RMSE or a fuzz, not AE 0; `--lossless` for pixel compares), every
   120 presents, unchanged screens left out, at most 500, none once the folder
   passes 200 MB (`--max-shots`, `--cap-mb`; `shots.log` says so). It can't
   see script positions, text or variables (the engine trace's job), XAudio2
   or Vulkan presents; 64-bit games are built but untried. (装甲悪鬼村正 with
   `ntrace.dll` (`--dll`): every sound statement matched a DirectSound buffer
   12 ms later, every literal CG name a `cg.npa` read ~20 ms later.)
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
   - A worktree per game (`../conscripter-convert-<name>`), one short-lived
     branch and PR per feature from `origin/main`; game-specific code stays
     out of shared files (`CLAUDE.md`, "Rules", "Working beside other agents").
   - One item per session: do it, update "Status", open the PR, stop
     (`CLAUDE.md`, "Keeping usage down").
5. **Plan in this order:** assets, script decoding, control flow, text and
   voices, pictures and sound in scripts, system screens, extras, matching
   effects to the original. Each phase ends with something the user can run.
   Build a vertical slice first (one scene with text, a picture and a voice).

## 1. Archives and assets

- **Reuse the shared code.** `conv_engines::output::Output` (output folder,
  dry runs, collected errors, `write`, `write_png`, `image_size`);
  `conv_engines::media` (`ffmpeg` through a `.part` file, `mp4_options`);
  `crates/conv-cli/src/build.rs` (`check_output`, `write_game`, `pack`,
  `summary` with the notes and the lint). A new engine's asset bank keeps only
  what is its own: its archives, caches and output names (saves record them).
- Archive reader and per-format decoders go in
  `crates/conv-engines/src/<engine>/`, one module per format, each with a
  `conscripter-convert <format> list|extract` subcommand.
- Keys and ciphers come from the exe. Throwaway Python goes in
  `research/<engine>/` (`python3 -I`, game paths as arguments, no game data
  committed); port only what holds up into Rust.
- **Check against the whole corpus:** every archive verifies (CRC/MD5), every
  image decodes, every audio file plays. Count degenerate assets (empty
  files, 1×1 images): references to them mean off-by-one bugs.
- Audio: Ogg Vorbis passes through; loop points go into the `.ogg` as
  `LOOPSTART`/`LOOPEND` comments (SDL_mixer loops on them; saves never lose
  them). Movies: ffmpeg to H.264 + AAC MP4, played with `mpegplay`.
- Images stay PNG; `--webp` (near-lossless) is opt-in for the web build. A new
  engine's convert command takes it with `#[command(flatten)] webp:
  build::WebpArgs` and calls `build::webp_tree` with its picture folders
  before writing `0.txt`.

## 2. Scripts: decode, then measure

Order for every question about the original: **code** (scripts, decompile),
then the **oracle** (`exe_call`, memory dumps, a reference trace), then
**filming under wine** last (`CLAUDE.md`, "Wine is the last resort").

- Decode the script container fully (header, code, constant pool, label
  tables, checksums) and dump it (`conscripter-convert cd dump` is the model).
- **Rank opcodes (or built-ins) by use count** (`cd opstats`) and work from
  the top; a handful make most of the visible game.
- Find the dispatch table in the exe. Per handler: the strings it references
  (error messages name opcodes), the source-file name it logs, the RTTI class
  it builds. **Decompile with Ghidra**, not raw disassembly:
  `research/tools/ghidra/ghidra_export.sh <exe> research/<engine>/local/ghidra
  --names research/<engine>/names.tsv` writes one pseudo-C file per function
  (`decomp/<address>_<name>.c`, headed by callers, callees and the cp932
  strings it uses) plus `functions.tsv` and `strings.tsv`. Grep those first.
  **Every function or global you identify goes into
  `research/<engine>/names.tsv`** (`address<TAB>name<TAB>comment`); rerun the
  export (~30 s; the first run ~1 min) so the next session reads named code.
  Options (`--rtti` for MSVC C++ classes, `-- -processor …` for ARM) are in the
  script's header. For ARM C++ (a Vita eboot) add `-postScript
  ThumbPointerFunctions.java` after `--` (vtable-only virtual methods sit
  outside any function; it makes one at every Thumb pointer stored as data).
  `objdump -d -M intel` is fine for a quick look at bytes.
- **A packed exe** (section names gone or odd, entropy near 8 bits/byte,
  entry point in the last section, stub strings such as "cannot be run with
  debugger") gives Ghidra nothing. Work on a copy only, unpacked binaries in a
  git-ignored `local/`. In order: (1) an unpacked build the user already owns
  of the same version (a no-CD exe is an image dump with rebuilt imports:
  plain entropies, an extra import section; check its code against the
  other copies and its addresses against the scripts' behaviour); (2) a
  standard unpacker for the packer the stub names; (3) dump under wine once
  the stub reaches the original entry point (stop on the first execution in
  the first section; a disc check in the stub stops this without the disc)
  and rebuild the imports (Scylla's method). Hooks for the unpacked exe
  (step 22) then go into that exe run under wine. (続・殺戮のジャンゴ: (1),
  docs/games/django.md "The exe".)
- Work out **every index space a jump uses** (per-script labels or global
  tables, implicit entries, labels on marker instructions) and check all
  targets across the corpus: a wrong guess still produces a script that runs.

### Running one exe function

For a value from a small function (a curve, a table lookup, a sprite quad's
corners), run it instead of reading its maths: `research/tools/exe_call.py`
maps the exe into Unicorn (Ghidra's addresses throughout) and calls one
address with int, float and pointer arguments, printing the return value and
buffers; calls into imports or `--stub` addresses return 0 or a chosen value
and are listed. Seconds, no game, no wine. `Exe(path).call` runs thousands of
calls for a sweep. Setup once per machine: `research/tools/setup-venv.sh`
(ends with exe_call's self-test). Examples: `research/nitroplus/en_tempo_exe.py`
(every tempo curve of the English Muramasa, checked by a test in
`muramasa.rs`); 俺たちに翼はない's sprite quad with a negative scale
(`docs/games/oretsuba.findings.tsv`).

Limits: no OS, so a function that allocates, does file or GPU work, or reads
state the game set up needs that state written into memory first (an object's
fields as a buffer); a stdcall import's stub pops as many arguments as the
mingw-w64 import libraries give it (unknown: none); the Vita's SCE relocations
and import stubs are not read (only an import call is wrong: stub it). For
game-set state, a trace DLL can dump the process's memory at chosen statements
(Django's `ntrace.sh --dumps`, docs/games/django.md "Running the original
under wine"); `load_dump` maps a dump so the exe's own functions run on real
objects. Stub the clock (`stub_import("WINMM.dll!timeGetTime", …)`) to step
frames.

**What each platform has** (2026-10-19):

| Platform | Executable | `exe_call` | Memory snapshots | Per-frame state |
|---|---|---|---|---|
| Windows x86-32 (Xuse, Nitroplus NSS) | PE | yes (cdecl, stdcall, thiscall, fastcall; FS block, CRT TLS) | trace DLL under wine: `ntrace.sh --dumps`/`--scene-dumps` (NSS) | `ntrace.sh --frames` (Django, Muramasa), `xtrace.sh --frames` (Saihate) |
| Windows x86-64 | PE32+ | no (not read yet) | not yet | not yet |
| Linux x86-64 | ELF | yes (System V; RELA relocations) | not yet (no game yet) | not yet |
| PS Vita (ARM) | decrypted eboot ELF | yes (AAPCS; SCE relocations not read) | Vita3K's GDB stub: `gdbdump.py --scan` (docs/games/oretsuba.md) | not yet |
| Switch (AArch64) | NSO | yes (relocations applied) | Ryubing's GDB stub, modules only: `gdbdump.py --get-info`; its heap hangs the stub (docs/games/tsukihime.md) | not yet |

### Snapshot oracle

For a question about what the original does over time ("does an unwaited move
go on through a 0 ms fade?"), take the state from one short run and step the
exe's own code on it instead of filming:

1. **A probe scene**: statements that make the case, with a marker statement
   before and after each step (Django: `research/nitroplus/sched_probe.nss`;
   a marker is `Delete("__dump_<label>")`, a name nothing has).
2. **Dumps**: the trace DLL writes the process's committed memory at each
   marker (`ntrace.sh --probe <file> --dumps <dir>`; `ntrace.c`,
   `NTRACE_DUMPS`), holding the engine's object mutex, suspending other
   threads and stopping the game clock. Run at 1x (script waits follow the
   clock, motions step per frame). Dumps go under `research/<engine>/local/`
   and often answer the question alone: read the object's fields before and
   after. `--scene-dumps` dumps at every scene start (a base plus changed
   pages per scene, docs/games/django.md "Memory snapshots", packed by
   `research/tools/dumppack.py`); faster than playing the route is a save at
   each scene start, loaded one after another in one run
   (docs/games/muramasa.md "Per-scene snapshots": 881 scenes in 41 min).
   `research/tools/scene_check.py` compares each scene's objects at its first
   page with our engine's sprites there through a per-game adapter
   (Muramasa's: `research/nitroplus/scene_objects.py`).
3. **load_dump**: `Exe.load_dump(prefix)` maps a dump over the image, putting
   the exe's code and import slots back from the file (imports stay stubs;
   Win32 stubs pop their stdcall arguments; FS points at a thread block for
   MSVC's SEH prologs).
4. **Step the frame function**: `research/tools/sched_harness.py` stubs the
   clock to +1/fps per frame, the heap from an arena and the CRT's TLS. An
   engine adapter queues actions through the exe's own job code, calls the
   per-frame stepper N times and reads x, y, alpha per frame. Django's:
   `research/nitroplus/sched_django.py <exe> <dumps> [--jobs N] [--cases]`;
   by default it steps every dump in a pool of worker processes
   (`sched_harness.pool_map`, one Unicorn per process, ~130 MB each; 20
   dumps: 2.1 s with 1 worker, 0.3 s with 20); `--cases` runs the four overlap
   cases and checks each against the case's later dump.
5. **Into tests**: the numbers become fixtures of the converter's unit tests
   and `(scheduler)` rows marked seen in the findings table.

**Per-frame state of the original** (Django's `ntrace.sh --frames`,
docs/games/django.md "Per-frame state trace"): objects' x, y, alpha each
frame, compared with our motion samples by `research/tools/frame_diff.py`.

**Further: the interpreter from a dump** (timeboxed prototype, 2026-10-19,
`research/nitroplus/interp.py`): scene dumps name the process and script
context of the statement they stopped on (`th`, `ctx` in the `dump` record).
It runs Muramasa.exe's `run_statement` (`0x470a50`) on them in the thread
loop's order (`0x4708b0`) and gets past the chain's `Load(i+1);`, but the new
frame then runs nothing: script files are read on another thread, text goes
through the main thread's message queue and frames step on the render thread.
It needs a cooperative scheduler over one Unicorn (context and stack per
thread, switched at waits), file calls served from the real archives, and
sound lengths. Generic fixes already in `exe_call.py`: stubs inside the image
at odd x86 addresses (a Thumb mask), stdcall argument counts from the
mingw-w64 import libraries, `EncodePointer`/`DecodePointer`, the CRT's
per-thread data.

Worked example, Django's FadeFade (dja010_01:76-78: an unwaited 16 s `Move`,
then `Fade(0)` and a 1 s fade in): the probe run took 40 s and wrote 20 dumps
of ~94 MB; `load_dump` maps one in 0.1 s. The Move is queued with 960 steps,
still at step 0 after the 0 ms fade (no frame passed), at step 63 after the
1 s fade in and 126 after another second: the pan goes on (#564 fixed our
converter). Stepping `0x436040` from that dump moves x one step a frame.
Prior art: AFL-Unicorn's `unicorn_dumper`, "what the fuzz", Snapchange.

### Marking what a finding rests on

Mark each finding **seen** (code, a script or a trace shows it), **inferred**
(from names or layout) or **unknown**, and for an unknown write the cheapest
check that settles it (code, then a dry run or oracle, then a wine frame). A
negative search says what it searched ("not found in the 1.0.2 scripts"),
never "absent". Name the executable build for every decompile address (the
game document's "Executables" table; the export writes the input's sha256 to
`sha256.txt`). The game's Status keeps a short "Open questions" list; reports
and PR bodies end by marking each original question answered, partly answered
or open.

### Mapping the whole exe

When the scripts use more than the top of the ranking, extend the same export
and `names.tsv` in passes, from what the corpus uses, not by browsing:

1. **First pass:** the export as it stands. Name the dispatch table (statement
   built-ins), the value evaluator and the settings-variable setter first.
2. **Recover code that isn't a function yet.** Code reached only through a
   function pointer is missed (Muramasa: 76 per-object callbacks behind
   `Fade`, `Request`, `SetShade`, `CreateEffect`). Collect label addresses
   (`grep -ho '&LAB_[0-9a-f]*' decomp/*.c`, not only `= &LAB_`: casts
   `(code *)&LAB_` and arguments are callbacks too), append them to the names
   file as `cb_<addr>` and rerun (`ApplyNames` disassembles each and creates
   the function); repeat until no new address appears (a standing step). Skip
   `puStack_`/`pcStack_` frame stores and `return &LAB_<odd>` (mid
   instruction). Some hits are junk: SEH handlers (`004f8xxx` in Muramasa)
   and very large pixel blitters (don't read 400 KB of blitter).
3. **Name the CRT helpers early** (Muramasa's `FUN_004d3be0` is `_ftol2`;
   Ghidra loses the x87 maths around it, so a value can look like it comes
   from nowhere). See step 20.
4. **Find what the parser accepts.** `strings.tsv` plus the stack strings
   (`research/nitroplus/stackstrings.py`): most names are built on the stack.
   Look for character-comparison chains (`== 'R'`, `== 'U'` …; that is how the
   markup-tag parser was found). Note how constants are matched: Muramasa
   compares some by the sum of their characters (`NULL` = 315 = off), others
   exactly and case-sensitively (the `Request` actions).
5. **Plan from a coverage table.** For each built-in, `$SYSTEM_` variable,
   text tag and `Request` action: use count in the corpus, what the exe does
   (and where), what the converter does, whether the dry run's "not
   converted" notes list it. Work only on rows where exe and converter differ
   and the corpus uses the feature. Skip libraries (libpng, zlib, libjpeg:
   recognise them from strings). Coverage is per argument and behaviour: a
   row is "converted" only when every argument and behaviour is, else
   "partial" with what is left. Every engine's built-in dispatch notes each
   argument a call passes that the converter doesn't read (`<Builtin> argument
   <i> ignored`; an argument known to change nothing goes in a short table
   with the value it covers and why: Nitroplus's `UNREAD_HARMLESS`), so the
   dry run's notes list the partial rows. Give each row where it goes: data
   for an engine piece (`spmotion` keys, an `spcurve` table, `cons_rule.dll`
   masks), Lua (logic, processes), or a new engine command (§10; only this
   needs an engine PR).
6. **Special values and units, per argument.** For every argument or field the
   converter reads, look in the decompile for comparisons against constants
   before use (`== -1`, `-32766`/`-32767`, `0x8000`, `0xFFFF`, `-100000`, `0`),
   record each special value's meaning in the game document, and handle each
   (Tsukihime: motion key fields of -32766 mean "no key, interpolate"; we used
   them as values and pictures flew to -86829 while the metrics counted the
   field as converted). Record each argument's unit (ms or frames; px at which
   scale; percent or 1/256; degrees) and packed fields (two halves of one
   value, e.g. fields 8 and 9 as one i32 whose skip value is -100000).
7. **Census the output.** `python3 -I research/tools/argcensus.py
   out/<game>/0.txt` lists per command argument the count, range, top values
   and sentinel-like or huge values (`--self-test`). Run it on each new dry
   run and explain every flagged value from the exe or the data; the value
   audit only covers literals in our own code.
8. **Triage rule.** An off-screen, invisible or out-of-range trace finding is
   "the original's data" only when you have SEEN that in its code or data. A
   picture more than a screen away mid-motion is always checked.
9. **Dry-run parity.** A dry run's `0.txt` must equal the full build's with the
   same flags, file names included (once a dry run named UI pictures `.png`
   while the build wrote `.webp`). Check once per new media option.
10. **What keeps running.** List what the original updates every frame during
    effects, waits and fades (movies, motions, particles, sounds) and check
    ours does too (a movie stood still during a fade).
11. **Motion easing.** Record each motion's and fade's curve (linear, ease
    in/out, the exe's table), not only its key values.
12. **Branches, not just arguments.** Per handler, list the branches an
    argument picks (`_FADS` type 0/1, `_STEC` modes), which the game's data
    reaches, and count each (command, branch) in the metrics (a follow-up for
    the metrics code).
13. **Hidden state.** What a new picture inherits (alpha, zoom, blend,
    priority), what each command leaves unchanged, what a scene change or a
    load resets.
14. **Coordinates and colour from the drawing code.** Origin, anchor of zoom
    and rotation, the original's resolution against ours, rounding; sRGB or
    linear blending (the `_STEC` bug), premultiplied alpha, each blend's
    formula from the shader.
15. **Timing and blocking.** Per timed command: does the script wait, go on,
    or wait part of the time; ms or frames, at what frame rate; what a click
    does mid-effect (finish, cancel, skip). Then **overlap**, which a
    per-argument row can't show: what an unwaited action does while later
    commands run (keeps going, snaps to its end, restarts), what a wait or
    sync does to actions in flight, what a 0 ms action does to them. Map it
    from the exe's scheduler as `(scheduler)` rows in the findings table
    (`research/tools/ghidra/callmap.py <export> --names <names.tsv> --tsv`
    drafts them: per built-in, the queue, job, wait and frame functions it
    reaches, callbacks stored as pointers included; rows stay inferred until
    code or a snapshot shows them). List the game's own script helpers that
    chain timed built-ins (Django's `FadeFade`, function.nss:887) and give the
    converter a test or dry-run check of the composed output (Django lost 16 s
    pans to a 0 ms fade's sync; only the trace diff caught it). One
    `(scheduler)` row per question of `findings_lint.py`'s `QUESTIONS`
    (unwaited goes on, 0 ms on in-flight, wait on in-flight, replace or delete
    while moving), the question in the second column; the last column names
    its check, a test (`file.rs::fn`) or a ledger reason (`ledger: "<reason>"`),
    and the lint checks the name exists. Missing rows warn up to the game's
    `SCHED_MAX`, which only goes down.
16. **Track inferred findings to the end.** A machine-readable table per game
    (`docs/games/<game>.findings.tsv`, or a section): every argument and
    branch marked seen, inferred or unknown; an inferred row becomes seen only
    by a decompile lookup. It starts as soon as the dry run's metrics exist
    (`tests/snapshots/<game>.metrics.json`, from `tests/snapshot.sh
    --update`): one row per kind, the kind in the first column.
    `research/tools/findings_lint.py` (CI's check job) ties the two: every
    kind needs a row, every row names a kind, a kind in `NO_ROW` that gets a
    row must leave `NO_ROW`, `NO_ROW_MAX` per game only goes down, and
    `OPEN_MAX` does the same for rows marked inferred or unknown. A new game
    has no `NO_ROW` entry and a ceiling of 0: its table covers every kind from
    the first PR.
17. **Write the results back.** Commit `names.tsv` with comments, fold the
    decoded semantics into the engine's built-ins reference
    (`research/nitroplus/builtins.md` for Nitroplus), put the gaps in the game
    document's Status. Every command the dry run counts gets a row in
    `docs/engines/README.md` (what the original does in one plain sentence,
    and how it converts; `findings_lint.py` checks it).
    - **names.tsv is the index, a standing rule**: before reading a function,
      `grep research/<engine>/names.tsv` for it; for every function you read,
      add a line (address, name, one-line comment) in the same PR as the
      finding. Findings rows cite names from it.
    - **Audit when the mapping is done**: a second agent re-derives 10 random
      "seen" rows (fixed seed, stated in the report) and up to 10 `left_out`
      kinds from the decompile and checks each row's converter column against
      the code. Verdicts: confirmed, wrong, unsupported, stale; a wrong row is
      downgraded to inferred or unknown. (Done 2026-10-18 for Django, Saihate,
      Tsukihime: 30 of 30 confirmed.)
    - **Tools for reaching a moment and comparing builds**:
      `engine/tools/cons_explore.py` plays every route by taking the untaken
      choices; `engine/tools/cons_trace_vars.py` turns a trace into a `--vars`
      file for `--goto <label> --vars F`; `research/tools/test_saves.py` makes
      saves at chosen scenes; `scene_ab.py` compares a scene under two engines
      or scripts; `txtdiff.py` compares two `0.txt` by label; `argcensus.py`
      (step 7).

Items 18-24 reach the original itself; use them only after code and the
oracle above leave a real doubt (wine rules: `CLAUDE.md`).

18. **Wine API log with timestamps** (steps 10 and 15, sound in the
    checklist). Log only the calls that answer the question, to a file in
    `research/<engine>/local/`: `WINEDEBUG=+timestamp,+relay wine game.exe 2>
    local/relay.log` (lines carry the thread id), with `RelayInclude` set in
    the prefix (`wine reg add 'HKCU\Software\Wine\Debug' /v RelayInclude /d
    'kernel32.Sleep;kernel32.QueryPerformanceCounter;winmm.timeGetTime;user32.PeekMessageA;dsound.DirectSoundCreate;ddraw.DirectDrawCreate;d3d9.Direct3DCreate9'`;
    `module.function`, `;`-separated, no spaces). Count and space the calls
    with `grep`/`awk`; never read the log whole. The `ret=` address of each
    `Call` line tells call sites apart: group by it to find the frame loop
    (the site called once a frame). COM methods (`Present`, `Flip`, a sound
    buffer's `Play`) are vtable calls the relay never sees: count frames from
    the game's own pacing call and get sound from `WINEDEBUG=+timestamp,+dsound`
    (`IDirectSoundBufferImpl_Play`, `_Stop`, `_SetVolume`; drop the mixer's
    lines). (最果てのイマ, wine 11: 2.3 MB a minute; the frame loop `0x406f30`
    calls `Sleep(period - work)` from one site, 30.1 frames a second, the
    period from a table by a frame-rate setting
    (`docs/games/saihate-no-ima.findings.tsv`); `QueryPerformanceCounter` never
    called. Source: CodeWeavers, "Working on Wine, part 4".)
19. **Breakpoint trace of the original** (argument values, call order and
    frame counts, in place of screenshots).
    `research/tools/wine_bptrace.py names.tsv [name ...] -o bp.gdb` turns
    `names.tsv` into a gdb command file: a breakpoint per function printing
    its address, the time and the first stack arguments as JSON, then
    continuing. Run `winedbg --gdb --no-start 'C:\<dir>\game.exe'` (the Windows
    path; a bare name fails; it prints the port on stderr), then `gdb -batch
    -ex 'target remote localhost:PORT' -x bp.gdb > bp.log` (connect first: the
    file ends with `continue`), drive the game, and fold the log with
    `wine_bptrace.py --fold bp.log` (one line per run of hits of one function:
    first and last time, count). A row it confirms is "seen" (step 16). Only
    32-bit x86 stack arguments; works on a 32-bit exe under wine 11's WoW64.
    (最果てのイマ, `docs/games/saihate-no-ima.md` "Running the original under
    wine".)
    - Break on the script's handlers and their per-frame callbacks: the time
      from one handler to the next is what the script waits, and the
      callback's count is that wait in frames.
    - Don't break on the frame loop: each stop costs a few ms (30.7 frames a
      second with handler breakpoints only, 24 with one in the frame loop) and
      Saihate's clock measures from the end of the last `Sleep`, so waits
      count right in frames but stretch in seconds.
    - Every handler that waits hands over on the next frame: each waited
      command takes one frame more than its time (0x66 500 ms: 530 ms, 16
      frames).
20. **Library signatures in Ghidra** (steps 3 and 5). The export's analysis
    already runs the Function ID analyzer (the bundled `vs2012`..`vs2019` and
    `vsOlder` `.fidbf` databases in `Ghidra/Features/FunctionID/data` name MSVC
    CRT functions such as `_ftol2`, `memcpy`, `_strlen`); look for those labels
    in `functions.tsv` before naming a CRT helper by hand. zlib, libpng and
    libjpeg are not bundled: attach the extra `.fidbf` once in the Ghidra GUI
    (Tools, Function ID, Attach existing FidDb; kept in the user's settings, so
    the headless run uses it; delete `<outdir>/project` to re-analyze). The
    extra databases are untested. (最果てのイマ, VS2008: it named 18 statically
    linked CRT helpers (`__allmul`, `__alldiv`, `__SEH_prolog4`,
    `__security_check_cookie`, `_atexit` …); `ghidra.log` says "Function ID".)
21. **Carrying names between builds** (Muramasa JP/EN, Django versions,
    Tsukihime 1.0.0/1.0.2). Match the two executables' functions with Ghidra's
    Version Tracking (exact-bytes and exact-instructions correlators first,
    then the combined-reference ones), `ghidriff old.exe new.exe`, or BinDiff
    on two exports; take high-similarity matches, write the new address next to
    the old name in the other build's `names.tsv`, review the rest by hand.
    Every finding says which build it was seen in (the game document's
    executables table); a name carried over by matching is "inferred" (step 16)
    until its decompile is read. Untested (none of these tools is installed
    here; 最果てのイマ has one build).
22. **A trace DLL in the original** (the debug trace's records from the
    original itself: every script command with its time, page and choice,
    while the game plays by itself). Once the dispatch table and the wait's
    input check are named (steps 1 and 17), a small DLL (mingw
    `i686-w64-mingw32-gcc`) replaces each table entry with a stub that logs the
    instruction and jumps on, counts frames by hooking the frame loop's `Sleep`
    in the import table, and patches the input check's first bytes to turn on
    the game's own auto mode and take choices as its click code does. A
    launcher starts the exe suspended and loads the DLL with
    `CreateRemoteThread(LoadLibraryA)`: no game file is touched; works under
    wine 11's WoW64. Cheaper than step 19 (no stop per hit, frame times stay
    true) and runs for hundreds of pages unattended. 最果てのイマ:
    `research/xuse/tracedll/` (`xtrace.sh`, docs/games/saihate-no-ima.md
    "Running the original under wine").
    - An engine that runs source text (Nitroplus NSS) has no dispatch table:
      redirect the interpreter's calls (a 5-byte `call rel32` at the call site
      to a stub that logs and jumps to the callee) and take "at" as
      `<file>:<line>` in the text the exe loaded (check it against the file:
      装甲悪鬼村正's loaded text turns the CR LF of a text block's header and
      code lines into LF LF). Its waits are timed in ms by the process's own
      thread, not frames: log `ms`, diff with `--ms`. Turn auto mode on by
      patching one script line in place (lines must not move) where the script
      itself unlocks it. 装甲悪鬼村正: `research/nitroplus/tracedll/`
      (`ntrace.sh`, docs/games/muramasa.md "Running the original under wine").
      A packed exe: hook the unpacked one under wine.
    - Random hangs under wine: find what the threads wait on (`winedbg`,
      `attach`, `bt all`; hook the waits in the DLL and log who takes the lock)
      before blaming timing. (続・殺戮のジャンゴ pauses script threads with
      `SuspendThread`, and wine hands a mutex to a suspended thread that was
      waiting for it, so every thread waits; the DLL makes the threads suspend
      themselves between waits, docs/games/django.md.)
    - A Switch game under Ryubing: an ExeFS mod, a `main` NSO whose text
      segment is lengthened over the padding before the rodata page to hold a
      hook branched to from the dispatcher's loop head, logging each command
      through `svcOutputDebugString` (Ryubing's log, with its clock: `--ms`). No
      GDB stub (a stop per command stalls the game). Input from the log: press
      A when the last command has sat on a text wait a second. 月姫:
      `research/typemoon/switchtrace/` (docs/games/tsukihime.md "Testing").
    - A Vita game under Vita3K: an eboot patch (`patch/<TITLEID>.txt`)
      branching from each hooked function's first push to code in a function
      nothing calls, which `printf`s the record (a TTY line in Vita3K's log).
      Its GDB stub polls every 100 ms and its breakpoints re-hit. 俺たちに翼は
      ない: `research/oretsuba/vpatch.py` (docs/games/oretsuba.md "Testing").
23. **Diff the two traces**: with the converter's `--source-map <file>` (each
    0.txt line's `<script>:<instruction>`; the engine's lowering fills
    `Script::source_map`, as `xuse/convert.rs`'s `lower_script` does),
    `research/tools/trace_diff.py orig.jsonl ours.jsonl --map <file>` pairs
    pages by the instruction that shows them and prints, per page, the text
    that differs, the instructions one side ran and the other didn't
    (`--merge-into OP[,OP]`: a `missing` one whose ops ours emits under the
    next command of the page, if that is one of those ops, counts as `merged`;
    one the map has lines from that gave no trace record, as `quiet`), and
    times from one command to the next off by more than a frame. Ours: a
    `--dry-run` build's `--trace --auto-play N --real-time` (no click then ends
    an effect, so times are real), with the original's choices. Times use the
    original's frame count (`--fps`), not its clock (wine once stalled 134 s
    between two frames). A `missing` command that had nothing to change (a
    `csp -1` with no sprites) is no finding: ours writes an `ev` only for a
    change. 最果てのイマ: docs/games/saihate-no-ima.md "Trace diff". Where the
    compiler runs the script at convert time (Nitroplus NSS), mark each op
    with the statement being run when it is pushed (a function's ops with the
    function's own line), a text page with its text block, and sounds flushed
    after a statement with that statement (docs/games/muramasa.md "Trace
    diff"). Triage by size: `--top N` prints the N `time` findings with the
    largest |difference| first (`reftrace.sh summary` shows the top 5); triage
    anything over 1 s before the rest. A lost motion can also show as a
    `missing` `Move` (Django's 16 s `FadeFade`, dja010_01:77): read the
    `missing` timed ops of those pages too.

    **Reference traces** (a full trace of the original, few tokens):
    `research/tools/reftrace.sh <game>` (saihate, muramasa, muramasa-en,
    django, oretsuba, tsukihime) builds our dry run, `ours.map` and our trace on
    the same route, then starts the original's run in the background under a
    3 GB `systemd-run --user --scope` memory cap through `headless-run.sh` into
    `out/<game>-reftrace/` (git-ignored). Game folders come from the
    `~/.config/conscripter-convert/games` links; per-machine route inputs
    (`saihate-route.arg`, `<mura|django>-choose.txt`, the patched `*-patch/`
    dirs, oretsuba's `keys.sh`) from `$CD` (default `out/reftrace`); other
    variables are at the top of the script. `reftrace.sh status <game>` prints
    at most 5 lines (running, pages, last scene, minutes since progress, free
    RAM); `reftrace.sh summary <game>` at most 20 (pages, how far, top
    `trace_diff.py` kinds, the 5 largest `time` findings). Rhythm: one `status`
    ~5 min after the start (setup problems show then), then every 30-60 min,
    `summary` at the end. A watchdog resumes 最果てのイマ from its last save
    after 10 min without progress; for the others it logs the stall to
    `watchdog.log`. 続・殺戮のジャンゴ runs at `SPEED=1` (at `SPEED=8` its full
    run stalled on several pages, 2026-10-18; the speed hook is the suspected
    cause, unconfirmed). 装甲悪鬼村正's first H-mode screen (`sys_hmode.nss`,
    near page 4884) waits in a `select` for a click a clickless run never
    gives: `mura-patch/` holds a copy with that line's `select{` made `if(0){`
    (one line, so the trace's lines hold), and `function_magiccube.nss` (the
    立方陣 puzzle after `md05_025`) one whose line 261 `select{` is
    `$立方陣正解=true;$Loop_end=false;$Answer=true;if(0){` (solved, so the route
    goes on to `md05_025end`). The other selects outside the story files are
    menus, the title or the quick bar, which a clickless run never opens.

    **Trace speeds** (2026-10-17, 40 pages from each trace's start; wall
    seconds a page once the game runs, plus 20-30 s of start-up). A wine
    original runs faster with `SPEED=N` (`xtrace.sh`, `ntrace.sh`):
    `research/tools/trace_speed.h` gives the exe a clock N times as fast
    (`timeGetTime`, `GetTickCount`, `QueryPerformanceCounter`) and divides its
    waits by N (`Sleep`, `Wait*` timeouts, `timeSetEvent`, `SetTimer`); the
    records' `ms` (and xtrace's frame count `f`) are in that game time, so they
    diff as a 1x run's. "Same times": the share of command-to-command steps of
    the fast run within two frames of a 1x run's (`difflib`-paired by `at`).
    Not scaled, so off in a fast run: what the exe doesn't time by those
    clocks, wine's sound (a wait on a voice lasts its real length, N times too
    long in game time) and loading (a `CreateTexture` costs N times its real
    time). `trace_diff.py` reads `speed` from the trace's meta record (or
    `--orig-speed N`) and counts a time finding whose step holds a `WaitText`,
    `CreateSound`/`SE`/`BGM`, `CreateTexture`, `CreateMovie` or `LoadImage` as
    `fast-voice` (not listed).

    | Game | Run | 1x | Fast | Same times |
    |---|---|---|---|---|
    | 最果てのイマ | original, `xtrace.sh` | 3.8 | `SPEED=4`: 1.0 | 17/17 steps within 2 frames (`f`); `SPEED=8` (0.6) is off by up to 10 frames: the frame loop can't keep up |
    | 装甲悪鬼村正 | original, `ntrace.sh` | 3.1 | `SPEED=8`: 0.45 (`SPEED=4`: 0.8) | 97 % (two 1x runs: 100 %, worst 75 ms); off: `WaitText` on a voiced page, `CreateTexture` |
    | 続・殺戮のジャンゴ | original, `ntrace.sh` | 7.0 | `SPEED=8`: 0.9 (`SPEED=4`: 1.8) | 93 %; off: `WaitAction` of the text decoration, `CreateTexture`, `CreateMovie` |
    | 月姫 | original, `switch-emu.sh` + hook | 2.5 (`IDLEA=1`) | `IDLEA=0.3`: 1.4 | 91 %, step ratio 1.00 (the clock is untouched); a refresh rate of 400 % (`--vsync-mode Custom --custom-refresh-rate 400`) gives 1.1 but the game then runs only ~1.7x, by no fixed factor: times not comparable, not used |
    | any | ours, `--trace --auto-play N` | 0.08 (`--real-time`) | 0.03 (without) | `--real-time` keeps the times (headless, its waits take no wall time); without it times are not comparable |

    A whole 装甲悪鬼村正 route (23k pages) is ~3 h of original trace at
    `SPEED=8`, against 20 h at 1x. Skip mode (text and order only) wasn't
    needed.

    **Full-route reference trace** (2026-10-18): one canonical route of the
    original, traced whole, as the reference later builds are diffed against.
    Pick the route from the game's route map (fewest forced choices, e.g. every
    first option where that reaches an ending); force the original's choices by
    its script, a save or the key driver (a choice wait the trace hook logs:
    press to the wanted option), never by hand; auto mode or the trace's own
    idle presses, shots off (or one every 10 minutes to see a stall); stop
    after 10 minutes without a new page and resume from a save. Ours: the same
    choices (`--choose '*label:line=N'`, or `--taken` with `avoid` lines),
    `--trace --auto-play 0` (`--real-time` only when times are wanted: as long
    as the original's run). Keep the traces gzipped in `out/<game>-reftrace/`
    (never committed); in the game document: how far the route got, the
    commands, the top `trace_diff.py` kinds as Status items.

    **Picture diff** (2026-10-18): the same pages' frames, as numbers. The
    original's: `ntrace.sh --shots <dir> [--shot-delay MS]` saves the game
    window as lossless WebP `MS` of game time (default 1000) after each text
    block starts, as `<dir>/<pg>.webp` (pg: the block's first page; one frame
    per block, not per page). Ours: `--trace --auto-play N --real-time
    --shot-every 1 --shot-dir <dir>` (`--real-time`: without it auto-play cuts
    every effect and motion to its end; ~0.9 MB a page: keep N small, or
    `--shot-pages`). Then `research/tools/picdiff.py orig.jsonl <orig dir>
    ours.jsonl <our dir> --map <source map> --ms --thread 0 --mask WxH+X+Y`
    pairs pages as `trace_diff.py` does, paints the `--mask` boxes black in both
    (the text window and our own UI, which differ on purpose) and prints per
    page the differing pixels (AE: a channel off by more than `--fuzz`, default
    2 %; lossless shots of one picture give 0), their percent, RMSE and the box
    they lie in (one text window wide: the window sits elsewhere; whole frame:
    the picture), worst first, with ORIG | OURS | differing-in-red views of the
    `--top` 5. Lossy WebP (fc.sh's default, q80) needs `--fuzz 15` (our own
    frames re-encoded at q80 have 4 % of pixels off by over 6 %, 0.2 % over
    15 %). Limits: the original's frame is a fixed time into the block, ours the
    page's click wait (text typed out, click-wait icon up), so text still
    typing or a move still running differs by timing; a second run with another
    `--shot-delay` tells (a difference at both is real). (装甲悪鬼村正, first 80
    pages, mask `130x576+447+0`: 24 compared, 13 identical, 3 differ only in a
    text-window-wide box outside the mask, 8 over the whole frame, same at 1 s
    and 4 s. Causes: a `Plain` cover shown again drew the live screen (fixed);
    a pan cut by auto-play (right with `--real-time`); the exe's blur floors;
    typing progress; `docs/games/muramasa.md`.)
24. **Which branches the original runs** (marks for the findings table, step
    16). The trace DLL (step 22) also arms one-shot breakpoints: an `int3` on
    every code address of `names.tsv` and of the findings table's source
    column, restored at its first hit by a vectored exception handler, which
    logs a `hit` record. Each address costs one exception once, so the run
    keeps its speed. Arm instruction starts only (check each against `objdump
    -d`; an `int3` inside an instruction breaks the game). A script then marks
    every row by its addresses (`runs: <hit>; not: <never hit>`, or `never
    runs`), or by its opcodes in the `cmd` records when it cites no address. A
    "never runs" holds for the traced runs only: it settles a `left_out` row
    when the runs cover where the script uses it. 最果てのイマ:
    `research/xuse/cover.py` (`addrs`, which `xtrace.sh` runs, and `mark
    trace.jsonl... --write`, which fills the table's last column).

### Checklist: behaviours to map

Each needs an answer from the exe or the data (or "not used") in the game
document:

6. **Out-of-range input:** clamp, wrap or ignore; negative times; missing files.
7. **Run-time-only behaviour:** randomness, clock and date checks.
8. **Persistent state:** global flags, unlock conditions, read marks, what a
   save holds.
9. **Sound:** channel choice, loop points, dB or linear volume, fade shapes,
   load latency.
10. **Text:** line-break rules, ruby placement, speed units, inline wait codes.
11. **Golden frames:** one capture of the original per distinct screen in
    `research/<engine>/local/shots/`, compared numerically with ours; only
    where code and the oracle leave a real doubt (wine is last).

### Covering the scripts too

The rest of a game is in its own scripts and data: the helper libraries the
story calls (`function*.nss`), system screens (`sys_*.nss`), boot scripts, ini
files (`System.ini`) and, for a port, its own system scripts (the English
Muramasa's `sys_*.nut`). Each gets an answer in the same coverage table as the
exe's rows:

1. **List every script and data file** in the archives with a status: compiled
   by the run (story files, helpers they call), replaced by a screen of ours
   (name the function: `sys_config.nss` → `screens.rs` `settings`), read for
   data (ini files, tables), or not used (say why: debug, unreachable). A file
   with no status is a gap. For NSS, `nss route` lists unreachable story files.
2. **List every name the scripts call**, built-in or script-defined, with its
   corpus count (`nss refs` for names nothing defines, `nss builtins` for how
   they are called) and what the converter does with it. The dry run's "not
   converted or approximated" notes (`built-in X`, `skipped: no function X`,
   `Request …`) cover only code the run reaches; code it never reaches (system
   scripts, functions only they call) needs its row from the static count.
3. **The same for other vocabularies:** system variables (`$SYSTEM_…`), text
   tags (`<I>`, `<center>`, `<CLICK>`), `Request` actions, ini keys, and any
   string the scripts pass that the exe matches (names of effects, actions,
   sounds).
4. **Plan from the rows that differ**, as for the exe. Rows replaced by our own
   screens are checked against the original's screens (step 7), not line by
   line.
5. **Write it back** like the exe's results: gaps in the game document's
   Status, semantics in the engine's built-ins reference.

## 3. Lowering to the IR

- The reader produces the engine-neutral IR (`crates/conv-core/src/ir/`);
  only `crates/conv-cons` knows NScripter. If the IR can't express
  something, add an IR op or style variant with a doc comment saying what
  the picture does, not how the source engine does it.
- Anything not converted yet is emitted as a `;` comment naming the script
  and instruction; the converter's ranked list of them is the to-do list.
- Keep converter variables and sprite numbers in documented ranges (map in
  the game's document before the menus claim slots); the game's own
  sprites keep fixed numbers so old saves load.
- **Claim them against what conv-cons already uses** (`grep -o '%[0-9]*'
  0.txt`, the constants in `menu/mod.rs` and the top of `lib.rs`): game
  flags clear of the menus', locals below the global border. A new
  settings switch takes the next free index in `def` in `switches.rs` (a
  test keeps them unique; two branches once both took 9-11). Never move a
  number once a build is out (step 11.9).
- **Say where every hand-typed value comes from.** A literal standing for
  a fact about the original (position, size, colour, timing, count, order)
  gets a one-line comment: `// src: exe 0x4fb808`, `// src: <file> <what>`,
  `// src: shot <name>`, `// guess: <why>` or `// ours`; one comment covers
  a block from one table. Check in that order (game data, exe tables, an
  exact capture, a guess); a scaled screenshot is a guess (Saiima's were
  off by up to 12 px).
- **Say why a feature is left out**: `// dropped: <reason>` in code; in
  `docs/games/*.md` the reason in the same sentence.
  `python3 -I research/tools/dropped_lint.py [--max N]` lists sentences
  with "left out", "not converted", "dropped" or "skipped" and no reason;
  CI runs it with a ceiling per game (`CEIL`): lower it, never raise it.

## 4. Control flow: prove it before looking at pictures

- **After every feature, compile the whole game and lint it**: a dry run
  (`--dry-run`: script only, under 20 s; give a new engine's command one)
  then `conscripter-convert lint <dir>` (missing labels, effect numbers
  past 255, a `?` in text, half-width characters in vertical text, files
  missing from the archives). 村正's engine-killing bugs showed only on the
  whole game.
- **Then let the engine play it** with the debug trace (below);
  `cons_explore.py` covers every route; always under `timeout -k 5`.
- A loop isn't automatically a bug: check whether the original loops too.
- Map the progression variables (step 11.2) as soon as conditions
  convert, follow the walkthrough through the converted branches (11.3),
  and build the two-sided route simulation (11.4) at the same time;
  re-run it after every control-flow change.

## 5. Text and voices

- The text box and name plate are measured from screenshots of the
  original. Fonts must be fixed-width (`ipag.ttf`).
- Find the voice rule exactly (最果てのイマ hid it behind a
  divide-by-constant: look for magic multipliers) and check it plays every
  voiced line. Backlog names and voice replay: a `[name|voice]` tag per
  line with a `pretextgosub`, read back with `gettaglog`. Ruby, size and
  colour tags: `(base/reading)` with `rubyon`, `!zNNN`, `#RRGGBB`.
- **Check that the longest text fits**: find the longest message (name and
  ruby included) over the whole corpus and measure it against the window
  (俺たちに翼はない: 109 characters, four lines of 30); the trace's "text
  ran past the window" warning covers only pages a run reached.

## 6. Pictures and sound in scripts

- Backgrounds, sprites, BGM, sound effects, transitions with their real
  durations. Check sprite references against asset names: name prefixes
  are a free oracle.
- Find where effect parameters are defined: an opcode may only name a
  preset that another opcode defined in a setup script.
- An effect ONScripter can't draw: existing commands first (masks,
  `effect` numbers, `spfilter`), then the engine (step 10).

## 7. System screens

Title, save/load (thumbnails, delete), settings, backlog, button bar,
confirmations, built as in-script menus from the original's own parts
(`menu/mod.rs` explains the mechanics); which buttons the original shows
or enables when is step 11.6. Where it uses a Windows dialog, design one
from game parts and say so.

- **Start from the screens the IR already has**: `SaveMenu`, `SaveGrid`,
  `SettingsMenu` (`PagedSettings`), `PictureSettings`, `ButtonBar`,
  `CommandMenu`, `Backlog`, `ConfirmDialog`, `PictureScreen`. A game
  changes them through what it puts in the IR, never with game-specific
  branches in `menu/`; a new screen type only for what none can draw.
- **Two families**: the picture ones (`SaveGrid`, `LineLog`,
  `PictureSettings`, `PictureExtras`, `CommandMenu`: Nitroplus; use them
  when the original draws its screens as pictures) are laid out from the
  original's pictures; the classic ones (`SaveMenu`, `Backlog`,
  `SettingsMenu`, `ExtrasMenu`: Xuse, MAGES) arrange the game's parts in
  our layout. A third family needs the user's yes.
- **State the engine could keep goes in the engine** (play time, counters,
  save details): a CONscripter command, not script arithmetic (村正's
  save grid counted play time in script, then got `getplaytime`).
- Check each screen headless with the trace's `--shot-pages` or
  `research/xuse/smoke.py`; list the screens' sprites in the trace config
  so auto-play never presses the bar's buttons.

## 8. Extras

CG gallery, scene replay, music room. Their tables usually live in the exe
or in system archives. Get a 100% save (the user points to sagaoz.net)
and film the original's screens under wine before building them, with a
fresh system data as well, to see what is locked (step 11.6).

## 9. Matching the original

Fallback order: code (scripts, decompile, formats), then the oracle (our
numbers against dumps and reference traces), wine or an emulator last.

- **Check the game's manual before asking for screenshots**: it shows the
  title, menu, save/load, config and backlog screens. Extract it into
  `research/<engine>/local/manual/` (git-ignored), note in the game
  document which screens it covers (`docs/tools.md`, "Manuals"), or that
  there is none.
- **Read before you film.** Script arguments, the decompiled function
  behind them (curves, times, alpha, coordinates) and the format answer
  most questions. Check our side by numbers first (dry run's `0.txt`, a
  trace, `compare -metric`), run the original only for what remains, and
  reuse `research/<engine>/local/shots/` first.
- **Reach the moment by patching, not by clicking**, in this order:
  0. **A save from `research/tools/<engine>_save.py`** where one exists
     (`xuse_save.py`, `nitroplus_save.py`): it stands at any label with
     the variables you give, and the original loads it from its own load
     screen. Make one save in the game first as the template. Write such a
     tool once the save format is known (the document's "Save format").
  1. **Patch the game's scripts** in a copy (skip logos and title, set
     the flags, let pages advance by themselves, Xvfb screenshots at known
     times). The install is never touched: the patched archive goes in
     the symlink farm (`docs/games/saihate-no-ima.md`;
     `docs/games/muramasa.md`).
  2. **Write or edit a save file** when a script patch can't get there;
     work out the format from the exe's save and load code first.
  3. **Click through by hand** only when neither works.
- **Console games: an emulator instead of wine**, same rules (last
  resort, own Xvfb, never started bare even for `--help`, own data dir, no
  audio, memory capped; firmware and keys must match). Recipes:
  `docs/games/tsukihime.md` and `docs/games/oretsuba.md` ("Testing"),
  `research/oretsuba/vita-emu.sh`.
- **Filming under wine**, when the code leaves a real doubt
  (`LANG=ja_JP.UTF-8 LC_ALL=ja_JP.UTF-8` always; wine has no MS fonts, so
  compare where text starts): a scratch
  `WINEPREFIX` with **real** (not symlinked) Documents, Desktop and other
  user folders; a symlink farm so patched scripts never touch the install;
  no audio; an Xvfb of its own from `-displayfd` (`docs/tools.md`). Slow a
  transition by patching its duration in a copy, shoot at fixed fractions,
  film each screen once, keep shots in `research/<engine>/local/shots/`.
  Compare by numbers first (`compare -metric AE -fuzz 0` on a crop; a
  fuzz only for video or scaled pictures, and say so: 2 % hid sprites
  drawn 1 % dark). Stop wine and Xvfb afterwards. Recipes:
  `docs/games/saihate-no-ima.md`, `docs/games/muramasa.md`.
- Then read the effect's kernel in the exe for the exact curve and fit the
  filmed frames (MeltDown: edge = h·((p − d)/(1 − d))²); keep it as data
  where the engine can take it (step 10).
- Check our side with the trace: `--shot-effects 25,50,75` films every
  effect at those fractions of its own time as
  `<shot-dir>/fx<pg>-<n>-<pct>.png`, to compare with the original's frames.

## 10. Engine additions

Before adding a command, see whether the engine's general pieces take the
feature as data; most of an exe's presentation code is one of these:
- **Sprite animation**: `spmotion` keys (offset, zoom, angle, alpha) with
  a curve per key; the exe's easing goes in an `spcurve` table as read (or
  computed by running its function, `docs/lessons.md`, "Ways to speed up
  conversion"). `spblend` adds or multiplies.
- **Transitions that mix the two pictures in place** (wipes, dissolves,
  blinds): `cons_rule.dll` with a strip of mask frames made by running the
  original's kernel on a black old and a white new picture. Transitions
  that move pixels still get a plugin.
- **Logic** (rankings, unlock checks, minigame rules) and **parallel
  processes**: Lua, in every build. `luasub name` makes `NSCOM_name` a
  command; Lua globals are saved; `cons_process(name, "fn", ...)` resumes
  a coroutine once a frame and restarts from the top after a load, so keep
  progress in globals. Port decompiled C almost line by line. The
  converter doesn't write a `system.lua` yet: the first game that needs
  one adds that to conv-cons.

A new command is for what none of these can do. Then:
1. Add it in `engine/src/cons/` (`ONScripter_cons.cpp` preferred), not
   opt-in, building for the web (emscripten) and keeping its state in
   saves. Effects are `cons_*.dll` effect-99 plugins; screen effects use
   the `screenfilter`/`screenshake` slots.
2. Document it in the engine's `COMMANDS.md`.
3. Open the engine PR first (`CLAUDE.md`, "Rules"); point the submodule
   at your branch's commit, then at a commit on `master` once merged.
4. Release when a batch is ready: `docs/releasing.md`.

## 11. Route and progression check

A game isn't finished until this is done: a converted script can run every
page and still send the player to the wrong chapter or lose the loop state
between sessions.

1. **Find a walkthrough first.** Before converting control flow, save a
   public walkthrough or flowchart (routes, unlock order, loop order,
   true-ending conditions) in the game document's "Walkthrough" section,
   with its source.
2. **Map the progression variables** that gate routes, chapters, loops
   and endings: meaning, where set, reset and read, and whether the
   original keeps each in save data or system data. Table in the game's
   document.
3. **Check the converted branches against the walkthrough.** Play each
   step in the engine (a `--replay` file, below) with `--trace-vars`, or
   follow `0.txt` by hand or with `research/xuse/smoke.py` and the
   `lsp 999,"X:"+…` trick (`CLAUDE.md`). Confirm the expected chapter at
   every loop. Settle every disagreement from the original's disassembly
   or wine (step 9); don't just pick one.
4. **Simulate every route, both sides, and compare.** Spot runs miss bugs
   that show only on some paths (最果てのイマ: other heroine links in
   「最初の日」 stayed clickable and switched the route, `fad5f9b`).
   - **Simulate the source script** from the decoded bytecode with the
     original engine's run-time rules (in `research/<engine>/`): per page
     it prints the text, the links with targets, and the progression
     variables, for each walkthrough path.
   - **Simulate the converted `0.txt`** on the same click path
     (`research/xuse/nstrace.py` or the engine's equivalent) and **diff
     page by page**: same links, targets, variables. Also run the
     wrong-order paths the walkthrough warns about: the original refuses
     them, so must the conversion.
   - **Then in the engine**: `--replay` each path (or `cons_explore.py`)
     and compare `--trace-vars` with the simulator's variables.
   - **Lint for the same class of bug**: jump targets between a
     per-message record (link, ruby, style) and its message, links shown
     only under a condition, pages with two links on the same text.
   Keep the scripts and click paths to re-run after any control-flow,
   text or link change (最果てのイマ: `research/xuse/routesim.py`, paths in
   `PATHS`).
5. **Check that state persists as in the original**: system-data flags
   survive quitting and relaunching natively, closing and reopening the
   web build, loading an older save, and going back to the title to start
   again.
6. **Check when every button may be shown or used.** Conversions easily
   show everything from the start. List every button and item on the
   title, menus, bars and extras (gallery, replay scenes, music and movie
   entries, chapter or route entries) and write down in the game's
   document what the original needs before it is shown, enabled or opened
   (an ending seen, a CG reached, a save existing, a choice behind the
   player, its own unlock flag). Find the rule in the exe or scripts, or
   under wine with a fresh system data and with a 100% save; where it
   can't be found, choose one and say the rule is ours. Check both
   states: on a fresh start every locked button is hidden or greyed and
   refuses a click; with the unlocks reached (by play, or `--vars`) each
   appears. Locked state is usually system data: check it survives as in
   step 5.
7. **Check every setting does what the original's does**: default, effect,
   when it applies, kept across quitting; especially skip (read only or
   all; stops at a choice), auto speed and voice wait, volumes against
   mute, whether a voice stops at the next click, text speed extremes.
   Write the original's rule next to each row of the settings table.
8. **Check every way of giving input**: right-click, wheel and the
   original's keys do what they do in the original, on every screen. Every
   screen can be left by mouse, keyboard and touch (the web build on a
   phone and the Android build have no right-click or wheel). Clicks
   during a fade, movie or transition must not skip more than the
   original's.
9. **Check that saves survive a rebuild.** Before a build goes to the
   user, load the previous build's saves (story slots and system data)
   into it, natively and in the web build. Global variables, the game's
   sprite numbers, setting switches and save-slot numbers must not move
   between builds (`--roundtrip-every` checks only one build). Saves and
   read marks (`kidoku.dat`) follow a rebuilt `0.txt` by label.
10. **Write it down**: a checklist entry in the game's document,
    "progression verified against walkthrough: yes/no, date, how" (paths
    simulated, whether both sides matched). Add new lessons to
    `docs/lessons.md`.

## 12. The web build

The web build is the main platform: play it in a browser at every
milestone.

- Make it as loose files (one big `arc.nsa` would be fetched whole), with
  the web engine and an index:

      conscripter-convert <command> <game> -o out/<game>-web --font <ipag.ttf> --loose
      python3 -I engine/src/cons_web/conscripter_index.py -i out/<game>-web \
          -o out/<game>-web/cons_index.json --title <game> --lazyload
      cp engine/build_web/conscripter.{html,js,wasm} engine/build_web/favicon.ico out/<game>-web/
      python3 -m http.server 8000 -d out/<game>-web   # open /conscripter.html

  (web engine: `cd engine/script && EMSDK_HOME=<emsdk> bash
  ./cross_web.sh`; whole recipe in `docs/games/oretsuba.md`).
- Check: loading time; music, voices and movies once the browser lets
  sound play; saves and system data across a reload and a closed tab
  (11.5); screens by touch (11.8); picture memory on long routes.
  `engine/tools/web_run.py` makes a trace headless (`docs/tools.md`): run
  it with `--query 'auto-play=0'` at each milestone and check it like a
  native one.
- Note the package's size and file count in the game's document.

## 13. Finishing

- The finish line is set by the user: every story instruction converted,
  system screens and extras working, the route and progression check
  (step 11, unlocks included) done, remaining differences written down as
  optional leftovers in the game's document.
- Update the game's document, `README.md`'s games table, `gui/README.md`,
  CLAUDE.md's game table, `docs/lessons.md` and this playbook.
- **The game is in `crates/conv-engines/src/registry.rs`** (id, name,
  engine, marker files, command; `conscripter-convert games` lists it)
  from its first dry run (checklist 3a); `tests/snapshot.sh` reads it. On
  a conflict in `tests/snapshots/` never take one side: rebase, rerun
  `tests/snapshot.sh --update`, and explain each changed game's 0.txt in
  the PR body. `--update` also keeps `<game>.metrics.json` (source calls
  converted fully or with a gap, shown on the convert-metrics page): the
  engine opens and closes each call on `conv_engines::metrics::Metrics`;
  a call left out on purpose (unreachable code) goes to
  `Metrics::left_out`.
- **Add the game to the GUI**: its entry in `gui/games.py` (the registry's
  markers, checked by a test; command and font from the game's document;
  ffmpeg need; rough write size) and a fake layout with the expected
  command in `gui/tests/test_games.py`. Change the entry in the same commit
  as the document's build command.
- Rebuild into a fresh output folder; never touch folders holding the
  user's saves (`out/saiima-flow`); list any folder before deleting it.

## Checking a build with the debug trace

CONscripter plays a converted game by itself and writes a **trace**: one
JSON line per text page (sprites with boxes, background, music, sounds,
filters, effects), per screen- or sound-changing command, choice, missing
file, script error or crash. `engine/DEBUG.md` describes every record and
option; this is the workflow. Run headless, under `timeout -k 5`, in the
game folder. It needs a CONscripter newer than 0.18.0 (move the `engine`
submodule forward first).

**Checker first.** After every new build, make a trace and run
`cons_trace_check.py` before digging by hand; go deeper only for what it
can't explain.

```sh
cd out/<game>-cons
SDL_VIDEODRIVER=dummy SDL_AUDIODRIVER=dummy timeout -k 5 3600 \
    ../../engine/build/conscripter --root . --save-dir /tmp/<game>-saves \
    --trace /tmp/t.jsonl --trace-ignore <UI sprites> --auto-play 0
echo $?   # 0 end, 1 auto-play stuck, 2 script error, 3 startup failure, 128+n signal
python3 -I ../../engine/tools/cons_trace_check.py /tmp/t.jsonl --script 0.txt
```

No window or Xvfb is needed, so runs can go in parallel, each with its own
`--save-dir` (always; never `out/saiima-flow`) and trace.

- `--trace-ignore` takes the game's UI sprites (the document's sprite map):
  left out of the trace, never pressed by auto-play. `--auto-choice
  first|cycle|random` and `--seed n` pick the route.
- The checker ranks what looks wrong (errors, crashes, missing files,
  sprites left up, filters left over a CG, sounds left looping, voices cut
  off or missing, leaked text, long waits, memory growth, labels never
  reached) at the script line `*label:line`; its last line is a
  `--shot-pages` option for pictures of just those pages. UI ranges, CG
  name pattern and known false positives go in `cons_trace_check.json`
  beside the trace (or `--config FILE`); add a false positive only after
  looking at the page.
- A `crash` record has the signal, script line, gosub chain and the last
  50 commands: read those before the core dump; a script error has an
  `error` record with the same place.
- Over someone else's full build use `research/tools/scratch.sh` and
  `research/tools/scene_ab.py` (a scene under two engines or scripts).

**Every route, not one** (最果てのイマ's first runs missed 1,062 labels).
`engine/tools/cons_explore.py` reruns with `--auto-choice untaken` until no
run reaches a new label. `--replay t.jsonl` repeats an earlier run's
choices and seed (fresh `--save-dir` each: globals stay in the save
folder); a `replay diverged` warning marks where a new build went another
way; compare traces with `cons_trace_check.py --diff before after`. A
route simulator (step 11.4) can write the `--replay` file: the engine reads
only `choice` records (`{"t":"choice","at":"*label:line","picked":2}`, or
`"result"` for a `btnwait`), places being `0.txt` line numbers.

**Start anywhere.** `--goto <label> --vars <file>` starts at a label with
`%`/`$` variables preset (`%312=876`, `$5=text`, one per line) instead of a
patched `0.txt`; `*start`'s setup runs first. Vars files come from a
simulator, `--dump-vars file` (F8 and exit), or `engine/tools/
cons_trace_vars.py t.jsonl --page N|--at '*label:line'`; globals from the
save folder need an F8 dump.

**Long runs.** Other agents stop engines with `pkill -x conscripter`: run
long traces through a renamed symlink (`ln -s conscripter
out/conscripter-<game>`) and builds and runs under
`research/tools/capped.sh <GiB> <cmd>`. `--checkpoint-every N` and
`--load n` restart a deep run (slots 900 up); keep the bar in
`--trace-ignore`. `--roundtrip-every N` saves, loads and compares every N
pages: a `save/load mismatch` means state (filters, `pushscreen`,
`spmotion` keys, variables, Lua) doesn't survive a save.

**Other options** (`engine/DEBUG.md`): `--trace-perf`, `--dump-frames`;
`--trace-layers` (what is drawn front to back); `--audio-stats` (needs
`--real-time`); `--trace-vars 300-899` (compare with the simulator's
variables via `cons_trace_vars.py --diff a.jsonl b.jsonl`); `--shot-pages
412,980-990`, `--shot-every n`, `--shot-at '*label:line+ms'`, `--shot-after
ms` (under `SDL_VIDEODRIVER=dummy` the clock is virtual: same frames every
run); F8 writes the state; the web build takes `?trace`, trace via F7.

**Older recipe** when the trace can't do it: patch a copy of `0.txt`
(`textbtnwait` in `*conv_wait` → `mov %9,0`; comment out `click` in
`*conv_click_wait`; `mov %101,1:goto *label` after `erasetextwindow 0` in
`*start`); `research/xuse/smoke.py` does scripted clicks and screenshots
(click queue `%1000+`).

## Test saves at chosen scenes

To hand over saves at moments worth testing (battles, movies, minigames,
route splits, endings), list them in a slots file, one `slot label
description` a line, and run:

```sh
python3 -I research/tools/test_saves.py out/<game>-cons <slots.txt> -o <game>-test-saves.zip
```

For each line the engine starts at the label (`--goto`) and saves before
the scene runs (`--checkpoint-start --checkpoint-slot <slot>`), then plays
to the first page for the thumbnail (`--shot-pages 1`, or `--shot-after
3000` when stuck). A checkpoint is a bare save, so the script adds a
160×90 `savethumbN.bmp` and the slot's text, which `savegame2` appends as
`"<date>|<play time>|<text>"*` and `getsavestr` reads from the end. The
zip holds the files flat and uncompressed, as the web player's Export
Saves writes them (`save16.dat`, `savethumb16.bmp`); Import Saves takes
that, and so does a native save folder. Leave `gloval.sav` and `kidoku.dat`
out: they would replace the player's unlocks and read text.

With `--vars-dir <dir>`, each label's `<dir>/<label>.txt` (a `--vars`
file) sets the variables first and the save holds them; without it the
saves carry none of the route's earlier choices (Muramasa:
`research/nitroplus/route_vars.py` writes those files from the route
paths). Globals (`%800` up) live in `gloval.sav`, not in the zip.

## Film list and shot index

Running the original is the last resort and costs a session each time, so
doubts are batched:

- Search first: `research/tools/shots.py find <words>` looks through every
  `research/*/local/shots/INDEX.tsv` (git-ignored; columns: file, game,
  scene, page, what it shows, date). Add each new shot with
  `shots.py add <file> <game> <scene> <page> <what>`.
- If only the original can settle a doubt, append a line to
  `research/<engine>/local/film-list.md` (git-ignored; `| game | scene /
  page | question | asked by | date |`) and carry on; do not start
  wine/Vita3K/Ryujinx for it. Exception: a quick check that a generated
  save or patched script reaches directly (a few frames) may run at once
  through `headless-run.sh`; mind its token cost.
- Run the original only through `research/tools/headless-run.sh` or a script that
  re-runs itself under it (`xtrace.sh`, `ntrace.sh`, `vita-emu.sh`,
  `switch-emu.sh`); `.claude/hooks/emu_run.py` refuses direct `wine`,
  `Vita3K`, `Ryujinx`, `Xvfb`.
