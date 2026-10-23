# Converting a new game

A checklist; each step ends on a "Done:" criterion. Detail: `docs/new-game-reference.md` (by heading),
`docs/tools.md`. House rules: `CLAUDE.md`; lessons: `docs/lessons.md`.

**Order of evidence** about the original: its code (scripts, decompile, formats), the oracle (one exe function,
memory dumps), then wine or an emulator. Frame-by-frame matching: a bug seen in play, with the user's yes
(`docs/tools.md`, "Matching the original").

## For the lead

Setup and dispatch: `docs/lead-new-game.md`, sections 3-4.

## Steps

### Start

1. **Scope.** Ask the user (or read the lead's prompt): everything or story first, platforms (web is the main
   one), what to skip. Done: answers are in the game document's "Scope".
2. **Learn the game as a player**: wiki or playthrough for routes, choices, endings, extras; the manual for
   screens (`docs/tools.md`, "Manuals"). Done: "Walkthrough" section with its source.
3. **Identify the engine** from exe strings, RTTI, archive magic, config files (a Windows game: the
   first-contact kit; a known engine: its format docs first). Done: engine named in the document; for a family
   we convert, the nearest game and the code that carries over; prior work (fan-translation tools, engine
   reimplementations, extractors) listed with what each answers. Detail: reference, "First contact".
4. **Start the game document** from `docs/games/TEMPLATE.md` (public: `<install>`/`<game>` placeholders, no
   local paths). "Status" holds the open state, current in every commit. Done: committed with Scope,
   Walkthrough, Progression variables and Status.

### Formats and scripts

5. **Archives and assets**: per format a reader and decoder in `crates/conv-engines/src/<engine>/` with a
   `conscripter-convert <format> list|extract` subcommand; keys from the exe. Ogg passes through (loop points as
   `LOOPSTART`/`LOOPEND` tags); movies become H.264/AAC MP4. Done: every archive verifies, every image decodes,
   every sound plays over the whole corpus; empty and 1x1 assets counted. Detail: reference, "A new engine's
   code".
6. **Decode the scripts** fully (header, code, constant pool, labels) with a dump subcommand; rank opcodes or
   built-ins by use, work from the top, resolve every index space a jump uses. Decompile with
   `research/tools/ghidra/ghidra_export.sh`, each identified name in `research/<engine>/names.tsv`; tables and
   self-contained functions are lifted, not read by hand. Done: every script parses, every jump target exists.
   Detail: reference, "Decompiling the exe", "Lifting from the exe".
7. **Convert what the corpus uses**, most-used opcodes first. Done: the ranked gaps are in Status.

### Converting

8. **Lower to the IR** (`crates/conv-core/src/ir/`; only `crates/conv-cons` knows NScripter). Unconverted
   instructions become a `;` comment naming script and instruction; game flags take free ranges of
   `crates/conv-core/src/alloc.rs`; each left-out feature gets `// dropped: <reason>`
   (`research/tools/dropped_lint.py`). Done: `--dry-run` writes a `0.txt`; the left-out list is ranked in Status.
9. **Register the game**: `crates/conv-engines/src/registry.rs`; `gui/games.py` with a case in
   `gui/tests/test_games.py`; the snapshot baseline (`tests/snapshot.sh --update <id>`); a lowering test on a
   tiny synthetic input. Done: `tests/snapshot.sh` covers the game. Detail: reference, "A new engine's code".
10. **After every feature** dry-run the whole game, run `conscripter-convert lint <dir>` and a debug trace
    through `cons_trace_check.py` (`cons_explore.py` covers every route). Done: checker clean, every ending
    reached. Detail: reference, "Checking a build with the debug trace".
11. **Text and voices**: text box and name plate from the original's layout; fixed-width font; the exact voice
    rule; backlog with voice replay; ruby, size, colour tags. Done: every voiced line plays; the longest message
    (name and ruby included) fits the window.
12. **Pictures, sound, transitions** with real durations; sprite references checked against asset names. For an
    effect ONScripter can't draw, try existing commands first (masks, `effect` numbers, `spfilter`), then step
    13. Done: no missing files in the trace, transitions timed from the code.
13. **Engine additions**, only for what the engine's general pieces (`spmotion` keys, `spcurve` tables,
    `cons_rule.dll` masks, Lua) can't take as data. Code in `engine/src/cons/`, working in the web build, kept
    in saves, listed in `COMMANDS.md`; the engine PR goes first (`CLAUDE.md`, "Rules"). Done: submodule on a
    CONscripter `master` commit. Detail: reference, "Engine pieces".

### Screens and play

14. **System screens** (title, save/load, settings, backlog, button bar, confirmations) from the IR's screens
    (mechanics in `crates/conv-cons/src/menu/mod.rs`), changed only through what the game puts in the IR. Picture-drawn originals use the picture family, others the classic one; a
    third needs the user's yes. Engine-kept state (play time, counters) lives in the engine. Boot logos via
    `Script::boot`. Done: every screen opens and closes by mouse, keyboard and touch.
15. **Extras** (gallery, scene replay, music room): tables from the exe or system archives. Done: locked and
    unlocked states right, from a fresh start and from a 100 % save.
16. **Route and progression**: document the progression variables; follow the walkthrough through the converted
    branches with `--trace-vars`; settle disagreements from the original's code; check button unlocks, settings
    and every input on every screen (touch has no right-click or wheel) against the original, and that system
    data survives a relaunch. Done: Status says "progression verified: yes, how".
    Detail: reference, "Route simulation".
17. **Saves survive a rebuild**: load the previous build's saves and system data into the new one, natively and
    on the web. Global variables, sprite numbers, setting switches and slots keep their numbers once a build is
    out. Done: an older build's saves load at the same page.
18. **The web build**, loose files, played in a browser at every milestone. Done: loads, sound plays, saves
    survive a reload, screens work by touch; size and file count in the document. Detail: reference, "The web
    build".
19. **Play it through** in the web build (test saves at chosen scenes help: reference, "Test saves at chosen
    scenes"); each bug is a Status item, matched against the original only as far as the bug needs. Done: the
    user agrees; then release (`docs/releasing.md`), add the game to `README.md`'s table and `gui/README.md`,
    new lessons to `docs/lessons.md`.
