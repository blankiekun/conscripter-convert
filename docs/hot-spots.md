# Hot spots

Hot spots are files every game's work changes. Before editing one, read
what main changed in it (`git log -p HEAD..origin/main -- <file>`), then
keep the edit small and in its own place:

| File | How to edit it |
|---|---|
| `crates/conv-core/src/ir/` (`mod.rs`: ops and `Script`; `screens.rs`: shared screens; `picture.rs`: picture-drawn ones) | Append new types and `Op` variants at the end of their block, with a doc comment; leave existing ones in order and format |
| `crates/conv-cons/src/lib.rs`, `ops.rs` | A new feature's emitter gets its own module (`puzzle.rs`, `replay.rs`) plus one `match` arm in `ops.rs` (`Emitter::op`). Sprite ranges are the constants at the top of `lib.rs`; a new variable range goes into `conv-core/src/alloc.rs` first (a test checks every `%N` conv-cons names is there, and the build refuses a game variable inside one) |
| `crates/conv-cons/src/menu/` (one file per screen; shared constants in `mod.rs`) | Shared by every game's menus: change it through what a game passes in its IR (`SaveMenu`, `ButtonBar`, ...); game-specific branches belong in the game's module |
| `crates/conv-cons/src/switches.rs` | A new settings switch is one entry in `def` (its index and what applying it does) at the next free index; `menu/` stays untouched |
| `crates/conv-cli/src/main.rs` | One subcommand per engine; the engine's own code lives in `crates/conv-engines/src/<engine>/` |
| `engine` (submodule pointer) | Merge the engine PR first, then point the submodule at a CONscripter `master` commit that contains every engine commit `origin/main` points at (rebase onto `origin/main` first, so the pointer only moves forward) |
| `README.md`, `docs/new-game.md`, `docs/lessons.md`, `docs/design.md`, this file | Edit your game's rows and paragraphs; general lessons go in `docs/lessons.md` |
| `Cargo.toml`, `Cargo.lock` | Dependencies get their own commit; on a conflict take both sides and let `cargo build` rewrite the lock file |

**Working beside other agents**
- Start every item from a fresh `origin/main` and run `cargo test`; rebase
  again before pushing a hot-spot change and just before handing the PR
  back.
- A change to the shape of shared code (a helper's signature, a split or
  moved function, a renamed IR type, renumbered numbers) is its own PR,
  merged before anything builds on it.
- A single game's own rules live in a module of their own
  (`nitroplus/convert/muramasa.rs`), chosen by the CLI.
