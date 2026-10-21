# Converting a new game

A checklist for an agent (or lead) starting a new game: what to do, when
it is done, where the detail is (`docs/new-game-reference.md` by heading,
`docs/tools.md`). House rules: `CLAUDE.md`; lessons: `docs/lessons.md`.

**The goal**: `CLAUDE.md`, "Goal". Matching the original frame by frame
is not a step: only for a bug seen in play and with the user's yes
(`docs/tools.md`, "Matching the original"). **Order of evidence** for any
question about the original: its code (scripts, decompile, formats), the
oracle (one exe function, memory dumps), then wine or an emulator, last.

## For the lead

Lead notes: `docs/lead.md`. Set up the game in one PR: its id and branch
prefix, its row in `CLAUDE.md`'s table, `docs/games/<game>.md` from
`docs/games/TEMPLATE.md`; then its worktree (`git worktree add
../conscripter-convert-<game> -b <game>/start origin/main`).

## Steps

### Start

1. **Scope.** Ask the user (or read the lead's prompt): everything or
   story first, platforms (web is the main one), what to skip. Done: the
   answers are in the game document's "Scope".
2. **Learn the game as a player.** A wiki or playthrough for routes,
   choice mechanic, endings, extras; the manual for its screens
   (`docs/tools.md`, "Manuals"). Done: a "Walkthrough" section with its
   source.
3. **Identify the engine** from exe strings, RTTI, archive magic, config
   files; for a known engine, find its format docs and tools first. A
   Windows game: run the first-contact kit. Done: engine named in the
   document; for a family we already convert, the report names the
   nearest game and the code that carries over. Detail: reference,
   "First contact".
4. **Start the game document** from `docs/games/TEMPLATE.md` (public: no
   local paths, `<install>`/`<game>` placeholders). Keep "Status" to the
   open state, current in every commit. Done: committed with Scope,
   Walkthrough, Progression variables and Status sections.

### Formats and scripts

5. **Archives and assets.** A reader and a decoder per format in
   `crates/conv-engines/src/<engine>/`, each with a `conscripter-convert
   <format> list|extract` subcommand; keys from the exe. Audio Ogg passes
   through (loop points as `LOOPSTART`/`LOOPEND` tags); movies to H.264/AAC
   MP4. Done: every archive verifies, every image decodes, every sound
   plays over the whole corpus; empty or 1×1 assets counted. Detail:
   reference, "A new engine's code".
6. **Decode the scripts** fully (header, code, constant pool, labels)
   with a dump subcommand; rank opcodes or built-ins by use and work from
   the top; work out every index space a jump uses. Decompile with
   `research/tools/ghidra/ghidra_export.sh`, every identified name in
   `research/<engine>/names.tsv`. Done: every script parses, every jump
   target exists. Detail: reference, "Decompiling the exe".
7. **Convert what the corpus uses.** Decode, convert what the corpus
   uses, and keep the ranked gaps in Status. Done: gaps ranked in Status.

### Converting

8. **Lower to the IR** (`crates/conv-core/src/ir/`; only `crates/conv-cons`
   knows NScripter). What isn't converted yet becomes a `;` comment
   naming the script and instruction; game flags go in the free ranges
   of `crates/conv-core/src/alloc.rs`; every
   left-out feature a `// dropped: <reason>` (`research/tools/dropped_lint.py`).
   Done: `--dry-run` writes a `0.txt`; what is left out is a ranked
   list in Status.
9. **Register the game**: `crates/conv-engines/src/registry.rs`, the
   same in `gui/games.py` with a case in `gui/tests/test_games.py`, the
   snapshot baseline (`tests/snapshot.sh --update <id>`), a lowering
   test on a tiny synthetic input. Done: `tests/snapshot.sh` covers the
   game. Detail: reference, "A new engine's code".
10. **After every feature**, dry-run the whole game, `conscripter-convert
    lint <dir>`, and a debug trace checked with `cons_trace_check.py`
    (`cons_explore.py` covers every route). Done: checker clean, every
    ending reached. Detail: reference, "Checking a build with the debug
    trace".
11. **Text and voices.** Text box and name plate from the original's
    layout; fixed-width font; the exact voice rule; backlog with voice
    replay; ruby, size, colour tags. Done: every voiced line plays; the
    longest message (name and ruby included) fits the window.
12. **Pictures, sound, transitions** with their real durations; sprite
    references checked against asset names. An effect ONScripter can't
    draw: existing commands first (masks, `effect` numbers, `spfilter`),
    then step 13. Done: no missing files in the trace, transitions timed
    from the code.
13. **Engine additions**, only for what the engine's general pieces
    (`spmotion` keys and `spcurve` tables, `cons_rule.dll` masks, Lua)
    can't take as data. In `engine/src/cons/`, not opt-in, working in the
    web build, kept in saves, listed in the engine's `COMMANDS.md`; the
    engine PR goes first (`CLAUDE.md`, "Rules"). Done: submodule on a
    CONscripter `master` commit. Detail: reference, "Engine pieces".

### Screens and play

14. **System screens**: title, save/load, settings, backlog, button bar,
    confirmations, from the IR's screens (`SaveMenu`, `SaveGrid`,
    `SettingsMenu`, `PictureSettings`, `ButtonBar`, `CommandMenu`,
    `Backlog`, `ConfirmDialog`, `PictureScreen`; mechanics in
    `crates/conv-cons/src/menu/mod.rs`), changed only through what the
    game puts in the IR. Picture-drawn originals use the picture family,
    others the classic one; a third needs the user's yes. State the
    engine could keep (play time, counters) goes in the engine. Boot
    logos via `Script::boot`. Done: every screen opens and closes by
    mouse, keyboard and touch.
15. **Extras** (gallery, scene replay, music room), tables from the exe
    or system archives. Done: locked and unlocked states right, from a
    fresh start and from a 100 % save.
16. **Route and progression.** Map the progression variables (meaning,
    where set and read, save or system data) into the document; follow
    the walkthrough through the converted branches with `--trace-vars`;
    settle any disagreement from the original's code. Check when every
    button may be shown (locked on a fresh start, shown once unlocked),
    every setting against the original's, every input on every screen
    (touch has no right-click or wheel), and that system data survives a
    relaunch. Done: Status says "progression verified: yes, how".
    Detail: reference, "Route simulation".
17. **Saves survive a rebuild.** Load the previous build's saves and
    system data into the new one, natively and on the web. Global
    variables, sprite numbers, setting switches and slots never move once
    a build is out. Done: an older build's saves load at the same page.
18. **The web build**, loose files, played in a browser at every
    milestone. Done: loads, sound plays, saves survive a reload, screens
    work by touch; size and file count in the document. Detail:
    reference, "The web build".
19. **Play it through** in the web build (test saves at chosen scenes
    help: reference, "Test saves at chosen scenes"); each bug found is a
    Status item, matched against the original only as far as the bug
    needs. Done: the user agrees; then a release (`docs/releasing.md`), the
    game in `README.md`'s table and `gui/README.md`, new lessons in
    `docs/lessons.md`.
