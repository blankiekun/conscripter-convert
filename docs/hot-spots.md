# Hot spots

- **Hot spots**: files every game's work changes. Before editing one,
  check what main changed in it (`git log -p HEAD..origin/main -- <file>`).
  Keep each edit small and in its own place:

  | File | How to edit it |
  |---|---|
  | `crates/conv-core/src/ir/` (`mod.rs`: the ops and `Script`; `screens.rs`: the shared screens; `picture.rs`: the picture-drawn ones) | Add new types and `Op` variants at the end of their block, with a doc comment; don't reorder or reformat existing ones |
  | `crates/conv-cons/src/lib.rs`, `ops.rs` | Put a new feature's emitter in its own module (`puzzle.rs`, `replay.rs`) and add one `match` arm in `ops.rs` (`Emitter::op`). Sprite ranges are the constants at the top of `lib.rs`; a new variable range goes into `conv-core/src/alloc.rs` first (a test checks every `%N` conv-cons names is there, and the build refuses a game variable inside one) |
  | `crates/conv-cons/src/menu/` (one file per screen; the shared constants in `mod.rs`) | Shared by every game's menus: change it only through what a game passes in its IR (`SaveMenu`, `ButtonBar`, …), not with game-specific branches |
  | `crates/conv-cons/src/switches.rs` | A new settings switch is one entry in `def` (its index and what applying it does), with the next free index; nothing in `menu/` |
  | `crates/conv-cli/src/main.rs` | One subcommand per engine; keep the engine's own code in `crates/conv-engines/src/<engine>/` |
  | `engine` (submodule pointer) | Merge the engine PR first, then point the submodule at a commit on CONscripter `master` that contains every engine commit `origin/main` points at (rebase onto `origin/main` first, so the pointer only moves forward) |
  | `README.md`, `docs/new-game.md`, `docs/lessons.md`, `docs/design.md`, this file | Edit only your game's rows and paragraphs; general lessons go in `docs/lessons.md` |
  | `Cargo.toml`, `Cargo.lock` | Add dependencies in their own commit; on a conflict, take both sides and let `cargo build` rewrite the lock file |

- **Working beside other agents**: start every item from a fresh
  `origin/main` and run `cargo test`; rebase again before pushing a change
  to a hot spot and just before handing the PR back. When you change the
  shape of shared code (a helper's signature, a split or moved function, a
  renamed IR type, renumbered numbers), open that as its own PR and get it
  merged before building on it. A single game's own rules live in a module
  of their own (`nitroplus/convert/muramasa.rs`), chosen by the CLI.
