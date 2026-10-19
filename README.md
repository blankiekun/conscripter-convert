# CONscripter-convert

Web player and more: [blankiekun.github.io](https://blankiekun.github.io/?app=cons#cons)

Converts Japanese visual novels you own into games that run on
[CONscripter](https://github.com/blankiekun/CONscripter), our fork of the
open-source ONScripter-Yuri engine (an NScripter clone). The converted game
plays on a PC, a phone or a handheld, and in a web browser. The converter
reads a game's archives, images, audio and compiled scripts and writes a
`0.txt` script plus `.nsa` archives.

> CONscripter-convert ships no game data. It reads files from your own
> installation.

## Games

| Game | You need | State | Details |
|---|---|---|---|
| 最果てのイマ COMPLETE | the PC install (`Farthest2015.exe`, `HD/`, `CD/`) | Playable | [`docs/games/saihate-no-ima.md`](docs/games/saihate-no-ima.md) |
| 装甲悪鬼村正 | the PC install (`Muramasa.exe` and the `.npa` archives) | Playable | [`docs/games/muramasa.md`](docs/games/muramasa.md) |
| Full Metal Daemon Muramasa (English, Steam) | the English install, plus the Japanese 装甲悪鬼村正 install | Early | [`docs/games/muramasa-en.md`](docs/games/muramasa-en.md) |
| 続・殺戮のジャンゴ -地獄の賞金首- | the PC install, updated to 1.01 | Story plays | [`docs/games/django.md`](docs/games/django.md) |
| 俺たちに翼はない (PS Vita) | the Vita package with its CPK files decrypted | Story plays | [`docs/games/oretsuba.md`](docs/games/oretsuba.md) |
| 月姫 -A piece of blue glass moon- (Switch) | the RomFS and `exefs/main` of version 1.0.2, or the English fan patch's RomFS on top of it | In progress | [`docs/games/tsukihime.md`](docs/games/tsukihime.md) |

States: Playable (start to end, system screens done), Story plays (the whole
story, some screens missing), In progress, Early. Each game's document says
what converts, what doesn't and why.

## Get a game playing

### With the GUI

Download `conscripter-convert-gui_<version>_<platform>` from the
[releases](https://github.com/blankiekun/conscripter-convert/releases),
drop the game folder on it and click **Build**. It finds the game, builds it
with that game's recommended settings and puts CONscripter beside it. See
[`gui/README.md`](gui/README.md).

### From the command line

```sh
conscripter-convert build /path/to/game -o out/mygame
```

`build` recognises the game (`conscripter-convert games /path/to/game`
lists what it finds). Add `--small` for a smaller download; an English
edition is built on the Japanese game with `--japanese <folder>`. All
options: [docs/cli.md](docs/cli.md). The output holds `0.txt`,
`arc.nsa`, `default.ttf` (IPA Gothic, a fixed-width Japanese font) and
`ons_args`; keep them together.

### Play it

| You have | Convert with | Play with |
|---|---|---|
| Windows | `conscripter-convert-gui_<version>_x64_win.exe` | the `<title>.exe` the GUI writes beside the game |
| macOS | `conscripter-convert-gui_<version>_aarch64_darwin.zip` or `_x64_darwin.zip` | the `<title>.app` the GUI writes |
| Linux | `conscripter-convert-gui_<version>_x64_linux` or `_arm64_linux` | the `<title>` and `.desktop` file the GUI writes |
| Android | convert on a computer | `conscripter_<version>_android.apk`, then copy the folder over |
| Any browser | convert on a computer | [the web player](https://blankiekun.github.io/?app=cons#cons): open the folder |
| RetroArch, Kobo | convert on a computer | the matching file from the [CONscripter releases](https://github.com/blankiekun/CONscripter/releases) |

Step by step for each: [`docs/play.md`](docs/play.md).

## What doesn't convert

Anything the converter can't convert yet stays in `0.txt` as a `;` comment
naming its source script and instruction, and each conversion prints a
ranked list of what was left out. Games built around custom systems
(minigames, battle systems) won't convert fully.

## Reporting a problem

Every build names itself: `version.txt` in the output folder and a small line
in the bottom-left corner of the settings screen read like `build 2026-10-14,
converter 562cbe2, CONscripter 0.17.0` (the day it was converted, the
converter's commit, `-dirty` if built from uncommitted changes, and the
CONscripter version it pins). Quote it in an
[issue](https://github.com/blankiekun/conscripter-convert/issues).

## For contributors

```sh
git clone --recurse-submodules https://github.com/blankiekun/conscripter-convert
cd conscripter-convert
cargo build --release              # Rust 1.85 or newer; target/release/conscripter-convert
research/tools/build-engine.sh     # CONscripter (needs SDL2 and CMake); engine/build/conscripter
cargo test
```

Every change goes in through a pull request ([`CLAUDE.md`](CLAUDE.md),
"Rules"; engine changes need their own PR to CONscripter first). CI runs
`tests/snapshot.sh` and `tests/pr-report.sh`; see
[`docs/tools.md`](docs/tools.md) for those and the rest of the tooling.

- [`docs/design.md`](docs/design.md): architecture, IR, output stage, crate layout
- [`docs/new-game.md`](docs/new-game.md), [`docs/new-engine.md`](docs/new-engine.md): adding a game or an engine
- [`docs/games/`](docs/games/): one document per game
- [`docs/lessons.md`](docs/lessons.md), [`docs/tools.md`](docs/tools.md), [`research/README.md`](research/README.md)
- [`engine/COMMANDS.md`](engine/COMMANDS.md), [`engine/DEBUG.md`](engine/DEBUG.md): the engine's commands, debug trace and auto-play
- [`docs/releasing.md`](docs/releasing.md)

## License

[GPL-3.0-or-later](LICENSE). CONscripter is GPL-2.0-or-later (see
[`engine/LICENSE`](engine/LICENSE)). The bundled IPA Gothic font is under the
IPA Font License.
