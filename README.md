# vn2ons

Converts Japanese visual novels you own into games that run on
[ONScripter](https://onscripter.osdn.jp/onscripter.html), the open-source
NScripter clone. vn2ons reads a game's archives, images, audio and compiled
scripts, translates them into an engine-neutral intermediate form, and writes
a `0.txt` script plus `.nsa` archives that a stock ONScripter build (including
WebAssembly builds) can play with no engine patches.

> vn2ons ships no game data. It reads files from your own installation.

## Supported engines

| Engine | Status | Tested with |
|---|---|---|
| Xuse | Partial (see below) | 最果てのイマ COMPLETE |

Only the Xuse engine is supported for now. Other engines are planned; the
pipeline is built so each one is a separate reader feeding the same output
stage (see [`PLAN.md`](PLAN.md)).

### Xuse support

| Area | Status |
|---|---|
| Archives, images, BGM, sound effects, voices | ✅ Decoded |
| Script text, speaker names, hyperlinks | ✅ Converted |
| Backgrounds, sprites, music, sound effects, voices in scripts | ✅ Converted |
| Control flow (variables, conditions, goto, gosub/return) | ✅ Converted |
| Transitions (cross-fades, mask wipes) | ✅ Converted with the original durations |
| ADV text box and name plate | ✅ Converted |
| Full-screen text modes | 🚧 In progress |
| System menus (title, save/load, settings, backlog) | ❌ Not built yet |
| Movies | ❌ Not converted |
| Special text displays and ~35 rarer opcodes | ❌ Not decoded |

Other Xuse games may convert but haven't been tried.

Anything vn2ons can't convert yet stays in `0.txt` as a `;` comment naming
its source script and instruction, and each conversion prints a ranked list
of what was left out. Games built around custom systems (minigames, battle
systems, menus scripted in the engine's own language) won't convert fully.

## Building

Needs a Rust toolchain with edition 2024 support (Rust 1.85 or newer).

```sh
cargo build --release
```

The binary is `target/release/vn2ons`.

## Converting a game

```sh
vn2ons convert-game /path/to/game -o out/mygame \
  --font /path/to/japanese-font.ttf --title "My Game"
```

The game path is the installation directory. ONScripter needs a font that
covers Japanese; `--font` copies it into the output as `default.ttf`.

Then run ONScripter on the output directory:

```sh
onscripter --root out/mygame
```

| Option | Effect |
|---|---|
| `--loose` | Write assets as loose files instead of packing them into `.nsa` archives |
| `--linear` | Play the story scripts in file order with no jumps (debugging aid) |

`vn2ons convert-script <game> --script N -o <dir>` converts a single script,
which is handy for testing one scene.

## Format tools

Low-level subcommands for inspecting Xuse game files. Run
`vn2ons <command> --help` for details.

| Command | Files | Subcommands |
|---|---|---|
| `vn2ons xarc` | `*.arc` archives | `list`, `verify`, `extract` |
| `vn2ons gaf` | `*.004` GAF4 image containers | `list`, `names`, `extract` |
| `vn2ons xsa` | `*.xsa` voice banks and `*.bin` sound banks | `list`, `extract` |
| `vn2ons cd` | `*.cd` compiled scripts | `info`, `dump`, `messages`, `strings`, `symbols`, `opstats` |

## How it works

```
game dir → unpack archives → decode assets, decompile scripts → IR → 0.txt + .nsa
```

| Path | Contents |
|---|---|
| `crates/vn2ons-core` | The engine-neutral intermediate representation (IR) |
| `crates/vn2ons-engines` | Source engine readers, one module per engine (`xuse/`) |
| `crates/vn2ons-ons` | ONScripter output: IR → `0.txt`, NSA archive writer |
| `crates/vn2ons-cli` | The `vn2ons` command |
| `research/` | Scripts used while reverse-engineering formats (see [`research/README.md`](research/README.md)) |
| `PLAN.md` | Design notes and format documentation |

Adding an engine means writing a reader in `vn2ons-engines` that produces
the IR; only `vn2ons-ons` knows about NScripter.

## Testing

```sh
cargo test
cargo clippy --all-targets
```

## License

[GPL-3.0-or-later](LICENSE).
