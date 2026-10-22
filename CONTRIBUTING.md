# Contributing

Most contributions here are a new game: you own a visual novel, you want
it to play in CONscripter (and in a browser), and the converter can't
read it yet. This page is the way in. The detailed playbook is
[`docs/new-game.md`](docs/new-game.md); what follows tells you what to
read, in what order, and the few rules that keep several games' work
from colliding.

## Ground rules

- **No game data in the repository.** No pictures, scripts, audio,
  extracted text, screenshots or disassembly of a game's executable.
  Code reads everything from the user's own installation, given as a
  path. Scratch output goes in `out/` or `research/<engine>/local/`, both
  git-ignored.
- **Only games you own.** You will need the install (and, for comparing,
  the original running under wine or on its platform).
- **Open an issue first** naming the game, its platform and, if you know
  it, its engine. Someone may already be on it, or on another game with
  the same engine.

## 1. Build what's there

Clone, build the converter and the engine, and run the tests as in the
[README](README.md#for-contributors). If you own one of the supported
games, convert it once (README, "Get a game playing") and play it: it
shows what a finished port looks like and what the output stage already
does for you.

## 2. Read, in this order

1. [`docs/design.md`](docs/design.md), "Architecture": the pipeline and the crates.
2. [`docs/new-game.md`](docs/new-game.md), steps 1-4: what to settle before
   writing code. The whole checklist is the playbook; it is written for
   AI agents but every step applies to people too.
3. A finished game's document, as the model for yours:
   [`docs/games/saihate-no-ima.md`](docs/games/saihate-no-ima.md).
   For a game on an engine already supported, read that engine's game
   document instead (the table in the README).
4. The rest of `docs/design.md` (the IR, the output stage),
   [`docs/lessons.md`](docs/lessons.md) (lessons from earlier games) and
   [`engine/COMMANDS.md`](engine/COMMANDS.md) (what CONscripter adds to
   ONScripter). They are long: read the headings
   (`grep -n '^##' <file>`) and look up the rest when you need it.

## 3. Find out where your game fits

Identify the engine from the install (`docs/new-game.md` step 3). Then:

- **An engine the converter already reads** (Xuse, Nitroplus NSS,
  MAGES TaskList on the Vita): you reuse its reader and add your game's
  own rules in a module of their own, as 続・殺戮のジャンゴ does
  beside 装甲悪鬼村正 (`crates/conv-engines/src/nitroplus/convert/django/`).
- **A new engine**: you write a reader in
  `crates/conv-engines/src/<engine>/` that turns the game into the IR
  (`crates/conv-core/src/ir/mod.rs`, top type `Script`), and one subcommand
  for it in `crates/conv-cli/src/main.rs`. Only `crates/conv-cons`
  knows about NScripter: you shouldn't need to write `0.txt` yourself.
  Throwaway reverse-engineering scripts go in `research/<engine>/` (see
  [`research/README.md`](research/README.md)); what holds up moves into
  Rust.

Either way, start `docs/games/<game>.md` before the code
(`docs/new-game.md` step 4) and keep its "Status" section current as you
go. It is public: no local paths (write `<game>` for the install), only
commands anyone with the game can run.

Follow the order of `docs/new-game.md`, and get one scene playing (text,
a picture, a voice) before going wide.

## 4. Check your build

- `--dry-run` writes `0.txt` only, in seconds: use it for anything about
  logic.
- The engine plays a build by itself and writes a trace of what it did;
  `engine/tools/cons_trace_check.py` lists what looks wrong (missing
  files, script errors, sprites left up, labels never reached). Recipes:
  [`docs/new-game-reference.md`](docs/new-game-reference.md), "Checking a build with the
  debug trace"; reference: [`engine/DEBUG.md`](engine/DEBUG.md).
- Before sending changes: `cargo test` and `cargo clippy --all-targets`
  pass, and every other game still builds the same if you touched shared
  code.

## Shared code

Every game's work meets in a few files. Keep your changes there small
and in their own place:

| File | How to change it |
|---|---|
| `crates/conv-core/src/ir/` (`mod.rs`, `screens.rs`, `picture.rs`) | Add new types and `Op` variants at the end of their block, with a doc comment; don't reorder existing ones |
| `crates/conv-cons/src/lib.rs`, `ops.rs` | A new feature's emitter goes in its own module, with one `match` arm in `ops.rs`. Sprite ranges are the constants at the top of `lib.rs`; a new variable range goes into `crates/conv-core/src/alloc.rs` |
| `crates/conv-cons/src/menu/` | Shared by every game's menus: change it through what your game passes in the IR, not with game-specific branches |
| `crates/conv-cons/src/switches.rs` | A new settings switch is one entry in `def` with the next free index (every game may use any switch) |

A game's own rules never go in shared files.

## Changing CONscripter

When the original does something ONScripter's commands can't reproduce,
the feature goes into the engine, not an approximation in the converter
(`docs/design.md`, "Target engine: CONscripter"). Look for an existing NScripter command first.

- Engine code is in the `engine/` submodule
  ([blankiekun/CONscripter](https://github.com/blankiekun/CONscripter)),
  ours in `engine/src/cons/`, preferably `ONScripter_cons.cpp`.
- Nothing is opt-in: a new behaviour or bug fix applies to every script,
  with no flag to turn it on. Check that it doesn't change another
  game's output.
- It must work in the web build (`engine/script/cross_web.sh`).
- Document it in the engine's `COMMANDS.md`.
- Send the engine change to CONscripter first; the converter change that
  uses it then moves the submodule pointer in the same commit.

## Commits and pull requests

- One commit per item, each one building and passing tests. The message
  starts with what it touches: `docs/games/django.md: ...`,
  `SaveGrid: ...`, `Django choices: ...`.
- Update the game document's "Status" in the same commit as the work it
  describes. When an item is done, move its write-up from "Status" to
  git log.
- Branch from `main`; to bring later `main` in, `git rebase origin/main`
  (no merge commits). Pull requests are squash-merged.
- Keep commit subjects under 72 characters.
- Stage files by name. Don't commit `out/`, local paths or anything from
  the game.

## Working with an AI agent

Much of this repository was written with AI agents. If you use one,
[`CLAUDE.md`](CLAUDE.md) holds the working rules and recipes they follow
(headless runs, wine, keeping context small); point your agent at it and
at this page.

## License

By contributing you agree your work is licensed under
[GPL-3.0-or-later](LICENSE); engine contributions follow CONscripter's
GPL-2.0-or-later.
