# Adding an engine

What a new source engine needs in the code, once `docs/new-game.md`
steps 0-2 have identified its formats. Follow the shape the three
engines share (`xuse/`, `nitroplus/`, `mages/` in
`crates/conv-engines/src/`); a game's own rules go in a module of their
own (`nitroplus/convert/muramasa.rs`).

1. **Module**: `crates/conv-engines/src/<engine>/mod.rs`, one file per
   format (archives, scripts), `convert.rs` (lowering to the IR),
   `assets.rs` (an asset source behind a trait, as Nitroplus' and
   MAGES' `Assets`, so the lowering can be tested without the game),
   `build.rs`.
2. **`build.rs`**: `pub fn build(game: &Path, …, opts: &Opts) ->
   Result<Built>` reads the game, writes its assets through
   `output::Output` (its dry-run gate and error list) and returns the IR
   with the folders `--webp` and `--audio-codec` act on
   (`conv_engines::build`). Converting sounds or movies with ffmpeg goes
   through `media::ffmpeg`. An `OUTPUT_DIRS` constant lists the asset
   folders (packed into `arc.nsa`).
3. **CLI**: one subcommand in `crates/conv-cli/src/main.rs` (or a
   `<engine>.rs` beside it for several), its own options plus
   `#[command(flatten)] build: build::BuildArgs`, calling
   `build::run(build, &title, OUTPUT_DIRS, |opts| <engine>::build::build(…))`.
   Format tools (list, extract, check) go in `conv-cli/src/tools/<engine>.rs`.
4. **Variables**: the game's flags go in the free ranges listed at the
   top of `crates/conv-core/src/alloc.rs`; the build refuses a game
   variable in a range conv-cons uses. Say in the game's document where
   they landed.
5. **Screens**: start from the IR's screens (`docs/new-game.md` step
   7); add a screen type only when none can draw the original's, and
   make it general.
6. **Registry, GUI, snapshot**: an entry in
   `crates/conv-engines/src/registry.rs`, the same in `gui/games.py`
   (a test compares them), a case in `tests/snapshot.sh` and its
   baseline (`tests/snapshot.sh --update <id>`).
7. **Tests**: a lowering test on a tiny synthetic input (Nitroplus'
   `Fake` assets in `nitroplus/convert.rs` are the pattern), and the
   format readers' corner cases.
8. **Docs**: `docs/games/<id>.md` from `docs/games/TEMPLATE.md`, a row
   in CLAUDE.md's table, `PLAN.md` §6, and lessons for any engine in
   `PLAN.md` §13.
