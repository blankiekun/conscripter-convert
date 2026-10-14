# CONscripter-convert

Converts Japanese visual novels you own into games that run on
[CONscripter](https://github.com/blankiekun/CONscripter), our fork of the
open-source ONScripter-Yuri engine (an NScripter clone that also builds for
the web). CONscripter-convert reads a game's archives, images, audio and
compiled scripts, translates them into an engine-neutral intermediate form,
and writes a `0.txt` script plus `.nsa` archives that CONscripter plays,
including its WebAssembly build.

The converter and the engine are developed together. When a feature of the
original game can't be reproduced with the commands ONScripter already has,
it is added to CONscripter rather than approximated (see
[`PLAN.md`](PLAN.md) §4.1).

> CONscripter-convert ships no game data. It reads files from your own
> installation.

## Supported engines

| Engine | Status | Tested with | Details |
|---|---|---|---|
| Xuse | Complete for its first game | 最果てのイマ COMPLETE | [`docs/games/saihate-no-ima.md`](docs/games/saihate-no-ima.md) |
| Nitroplus NSS | In progress (branch `nitroplus`) | 装甲悪鬼村正 | |
| 5pb./MAGES TaskList (PS Vita, CRI archives) | In progress (branch `oretsuba`): the story plays, all endings reachable | 俺たちに翼はない (Vita) | [`docs/games/oretsuba.md`](docs/games/oretsuba.md) |

Each engine is a separate reader feeding the same output stage (see
[`PLAN.md`](PLAN.md)). Each game's document says what converts, what
doesn't and why. How a new game is approached is in
[`docs/new-game.md`](docs/new-game.md).

Anything CONscripter-convert can't convert yet stays in `0.txt` as a `;`
comment naming its source script and instruction, and each conversion
prints a ranked list of what was left out. Games built around custom systems
(minigames, battle systems, menus scripted in the engine's own language)
won't convert fully.

## Getting the code

The engine is a git submodule in `engine/`:

```sh
git clone --recurse-submodules https://github.com/blankiekun/conscripter-convert
# or, in an existing checkout:
git submodule update --init engine
```

## Building

### The converter

Needs a Rust toolchain with edition 2024 support (Rust 1.85 or newer).

```sh
cargo build --release
```

The binary is `target/release/conscripter-convert`.

### The engine

CONscripter needs SDL2, SDL2_ttf, SDL2_image, SDL2_mixer, libjpeg, bzip2 and
Lua, plus CMake. On Arch Linux:

```sh
sudo pacman -S --needed cmake sdl2-compat sdl2_ttf sdl2_image sdl2_mixer libjpeg-turbo bzip2 lua
cmake -B engine/build -S engine -DCMAKE_BUILD_TYPE=RelWithDebInfo
make -C engine/build -j"$(nproc)"
```

The binary is `engine/build/conscripter`. For other platforms, including the
WebAssembly build (`engine/script/cross_web.sh`, needs emsdk), see
[`engine/README.md`](engine/README.md).

## Converting a game

```sh
conscripter-convert convert-game /path/to/game -o out/mygame \
  --font /path/to/japanese-font.ttf --title "My Game"
```

The game path is the installation directory. The engine needs a font that
covers Japanese; `--font` copies it into the output as `default.ttf`. Use a
fixed-width font (e.g. IPA Gothic, `ipag.ttf`): the engine puts each
character in a full-width cell, so a proportional font such as IPA P
Gothic leaves gaps after narrow kana.

The output folder holds `0.txt`, `arc.nsa`, `default.ttf` and `ons_args`.
`ons_args` holds the engine's startup options (`--enc:sjis`: the script is
Shift-JIS); CONscripter reads it from the game folder, so keep it with the
other files.

Every build names itself, for players reporting a problem: `version.txt`
in the output folder and a small line in the bottom-left corner of the
settings screen read like `build 2026-10-14, converter 562cbe2,
CONscripter 0.17.0`: the day it was converted, the converter's commit
(`-dirty` when built from uncommitted changes) and the CONscripter
version the repository pins. `version.txt` also names the source when
the converter knows it (the Vita package's ID and version).

Then run CONscripter on the output directory:

```sh
engine/build/conscripter --root out/mygame
```

| Option | Effect |
|---|---|
| `--loose` | Write assets as loose files instead of packing them into `.nsa` archives |
| `--linear` | Play the story scripts in file order with no jumps (debugging aid) |
| `--webp` | Store images as near-lossless WebP, about half the size of PNG. CONscripter's web build reads it; native builds need an SDL_image with libwebp |

`conscripter-convert convert-script <game> --script N -o <dir>` converts a
single script, which is handy for testing one scene.

## Format tools

Each engine reader comes with low-level subcommands for inspecting its
files (`list`/`extract` and the like). Run
`conscripter-convert <command> --help` for details. For Xuse:

| Command | Files | Subcommands |
|---|---|---|
| `conscripter-convert xarc` | `*.arc` archives | `list`, `verify`, `extract` |
| `conscripter-convert gaf` | `*.004` GAF4 image containers | `list`, `names`, `extract` |
| `conscripter-convert xsa` | `*.xsa` voice banks and `*.bin` sound banks | `list`, `extract` |
| `conscripter-convert cd` | `*.cd` compiled scripts | `info`, `dump`, `messages`, `strings`, `symbols`, `opstats` |

## How it works

```
game dir → unpack archives → decode assets, decompile scripts → IR → 0.txt + .nsa → CONscripter
```

| Path | Contents |
|---|---|
| `crates/conv-core` | The engine-neutral intermediate representation (IR) |
| `crates/conv-engines` | Source engine readers, one module per engine (`xuse/`, …) |
| `crates/conv-cons` | CONscripter output: IR → `0.txt`, NSA archive writer |
| `crates/conv-cli` | The `conscripter-convert` command |
| `engine/` | CONscripter, the target engine (git submodule) |
| `research/` | Scripts used while reverse-engineering formats (see [`research/README.md`](research/README.md)) |
| `PLAN.md` | Design, IR, output stage, engine additions, general lessons |
| `docs/new-game.md` | Steps and planning for converting a new game |
| `docs/games/` | One document per game: formats, opcodes, progression variables, status |
| `CLAUDE.md` | Working rules and recipes for AI agents working on this repo |

Adding a source engine means writing a reader in `conv-engines` that
produces the IR; only `conv-cons` knows about NScripter and CONscripter.

### Adding features to CONscripter

When a game needs something the engine can't do:

1. Add it to `engine/src/onsyuri/` as a new, opt-in command or option, so
   scripts that don't use it behave as before (a fix to something the
   engine does wrong applies to every script instead). It must work in
   the web build too.
2. Document it in [`engine/ADDITIONS.md`](engine/ADDITIONS.md) and in
   [`PLAN.md`](PLAN.md) §4.2.
3. Commit and push it in the engine repository, then commit the new
   submodule commit here together with the converter change that uses it.

## Testing

```sh
cargo test
cargo clippy --all-targets
```

`research/xuse/smoke.py` runs a converted game headless in
`engine/build/conscripter` with scripted clicks and screenshots (see its
docstring; it expects the title and menus `conv-cons` writes); `research/xuse/nstrace.py` traces an emitted `0.txt` without
the engine. Per-game checks (route simulations) are listed in each
game's document.

The engine can also play a converted game by itself and write a debug
trace (`--trace t.jsonl --auto-play 0`, headless with
`SDL_VIDEODRIVER=dummy`). `engine/tools/cons_trace_check.py` lists what
looks wrong in the trace: script errors and crashes, missing files,
sprites left up, pictures a few pixels off, filters left over a CG,
sounds left looping, garbled text, slow spots and labels never reached.
See [`engine/DEBUG.md`](engine/DEBUG.md), and "Checking a build with the
debug trace" in [`docs/new-game.md`](docs/new-game.md) for the recipes.

## License

[GPL-3.0-or-later](LICENSE). CONscripter is GPL-2.0-or-later (see
[`engine/LICENSE`](engine/LICENSE)).
