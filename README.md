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

| Engine | Status | Tested with |
|---|---|---|
| Xuse | Complete for its first game | 最果てのイマ COMPLETE |

Other engines are planned (Nitroplus is in progress on its own branch);
the pipeline is built so each one is a separate reader feeding the same
output stage (see [`PLAN.md`](PLAN.md)). How a new game is approached is
in [`docs/new-game.md`](docs/new-game.md).

### Xuse support

最果てのイマ COMPLETE converts completely: every instruction the story
uses is converted (only the unreachable debug menu is left out).

| Area | Status |
|---|---|
| Archives, images, BGM (with loop points), sound effects, voices, movies | ✅ Decoded |
| Script text, speaker names, ruby, text sizes and colours, hyperlinks (the route choices) | ✅ Converted |
| Backgrounds, sprites, colour filters, music, sound effects, voices in scripts | ✅ Converted |
| Control flow (variables, conditions, goto, gosub/return, link scenes) | ✅ Converted |
| Transitions | ✅ Every kind drawn as in the original (some via CONscripter effect plugins) |
| Screen filters, shakes, ripples, pulses, quakes, flashes, full-screen animations | ✅ Converted |
| ADV text box, name plate and full-screen text modes | ✅ Converted |
| System screens (title, save/load, settings, backlog with names and voice replay, button bars, confirmations) | ✅ Built as in-script menus |
| Extras (CG gallery, scene replay, BGM room) | ✅ Built as in-script menus |
| Movies | ✅ Web, Windows and Android; skipped by the Linux and macOS builds |
| Not converted | The system-voice room (WSS), the chapter history screen, the blog bar's buttons, autosave, some settings of the original's settings dialog |

Other Xuse games may convert but haven't been tried.

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

Low-level subcommands for inspecting Xuse game files. Run
`conscripter-convert <command> --help` for details.

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
| `crates/conv-engines` | Source engine readers, one module per engine (`xuse/`) |
| `crates/conv-cons` | CONscripter output: IR → `0.txt`, NSA archive writer |
| `crates/conv-cli` | The `conscripter-convert` command |
| `engine/` | CONscripter, the target engine (git submodule) |
| `research/` | Scripts used while reverse-engineering formats (see [`research/README.md`](research/README.md)) |
| `PLAN.md` | Design notes, format documentation, and the rules for engine additions |
| `docs/new-game.md` | Steps and planning for converting a new game |
| `CLAUDE.md` | Working rules and recipes for AI agents working on this repo |

Adding a source engine means writing a reader in `conv-engines` that
produces the IR; only `conv-cons` knows about NScripter and CONscripter.

### Adding features to CONscripter

When a game needs something the engine can't do:

1. Add it to `engine/src/onsyuri/` as a new, opt-in command or option, so
   scripts that don't use it behave as before. It must work in the web
   build too.
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
docstring).

## License

[GPL-3.0-or-later](LICENSE). CONscripter is GPL-2.0-or-later (see
[`engine/LICENSE`](engine/LICENSE)).
