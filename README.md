# vn2ons

Converts Japanese visual novels you own into games that run on
[ONScripter](https://onscripter.osdn.jp/onscripter.html), the open-source
NScripter clone. The output is a `0.txt` script plus `arc.nsa` archives and a
font, playable in a stock ONScripter build (including WebAssembly builds) with
no engine patches.

The first supported game is **最果てのイマ COMPLETE** (Saihate no Ima, Xuse
engine). The design is meant to take more engines later; see [`PLAN.md`](PLAN.md).

> vn2ons ships no game data. It reads files from your own installation.

## Status

Work in progress. For 最果てのイマ COMPLETE:

| Area | Status |
|---|---|
| Archives, images, BGM, sound effects, voices | ✅ Decoded |
| Script text, speaker names, encyclopedia links | ✅ Converted |
| Backgrounds, sprites, BGM, sound effects, voices in scripts | ✅ Converted |
| Control flow (variables, conditions, goto, gosub/return) | ✅ Converted; routes are reached through clickable links |
| Transitions (cross-fades, mask wipes) | ✅ Converted with the original durations |
| ADV text box and name plate | ✅ Matches the original layout |
| Full-screen text modes | 🚧 In progress |
| System menus (title, save/load, settings, backlog) | ❌ Not built yet |
| Movies | ❌ Not converted |
| Special text displays and ~35 rarer opcodes | ❌ Not decoded |

Instructions vn2ons can't convert yet stay in `0.txt` as `;` comments naming
their source script and instruction number, and each conversion prints a
ranked list of them.

## Building

Needs a Rust toolchain with edition 2024 support (Rust 1.85 or newer).

```sh
cargo build --release
```

The binary is `target/release/vn2ons`.

## Converting a game

```sh
vn2ons convert-game "/path/to/最果てのイマ COMPLETE" \
  -o out/saiima \
  --font /usr/share/fonts/OTF/ipagp.ttf \
  --title "Saihate no Ima COMPLETE"
```

The game directory is the installation folder containing `HD/` and `CD/`.
ONScripter needs a Japanese-capable font; `--font` copies it into the
output as `default.ttf`.

Then run ONScripter on the output directory:

```sh
onscripter --root out/saiima
```

Options:

| Option | Effect |
|---|---|
| `--loose` | Write assets as loose files instead of packing them into `.nsa` archives |
| `--linear` | Play the story scripts in file order with no jumps (debugging aid) |

To convert a single scene for testing:

```sh
vn2ons convert-script "/path/to/game" --script 3 -o out/scene-0003 --font …
```

## Format tools

Low-level subcommands for inspecting the Xuse engine's files. Run
`vn2ons <command> --help` for details.

| Command | Files | Subcommands |
|---|---|---|
| `vn2ons xarc` | `HD/*.arc` archives | `list`, `verify` (CRC check), `extract` |
| `vn2ons gaf` | `*.004` GAF4 image containers | `list`, `names`, `extract` (PNG) |
| `vn2ons xsa` | `CD/*.xsa` voice banks, `HD/*.bin` sound banks | `list`, `extract` (Ogg) |
| `vn2ons cd` | `HD/CDFiles/*.cd` compiled scripts | `info`, `dump`, `messages`, `strings`, `symbols`, `opstats` |

## Project layout

| Path | Contents |
|---|---|
| `crates/vn2ons-core` | Engine-neutral intermediate representation (IR) |
| `crates/vn2ons-engines` | Source engine readers; `xuse/` holds the archive, image, sound and script decoders and the script-to-IR conversion |
| `crates/vn2ons-ons` | ONScripter output: IR → `0.txt`, NSA archive writer |
| `crates/vn2ons-cli` | The `vn2ons` command |
| `research/` | Python scripts used while reverse-engineering the formats (see [`research/README.md`](research/README.md)) |
| `PLAN.md` | Design, architecture and format notes |

The pipeline is: detect engine → unpack archives → decode assets and
decompile scripts → IR → NScripter script and archives. Only `vn2ons-ons`
knows about NScripter, so other output targets could be added later.

## Testing

```sh
cargo test
cargo clippy --all-targets
```

## License

[GPL-3.0-or-later](LICENSE).
