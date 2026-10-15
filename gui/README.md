# CONscripter-convert GUI

A small window around the `conscripter-convert` command for players:
drop the game folder on it, and it finds which game it is and builds it
with that game's recommended settings (the commands in
[`docs/games/`](../docs/games/)). It runs the converter as a child process
and shows its output; it converts nothing itself.

Releases ship it as one file per platform with the converter and the
fonts inside: `conscripter-convert-gui_<version>_x64_win.exe`,
`…_x64_linux`, `…_arm64_linux`, and `…_aarch64_darwin.zip` /
`…_x64_darwin.zip` for Apple silicon / Intel Macs (a `.app`, unsigned:
open it with right click → Open the first time). The bare converter is
attached for each of them too (`conscripter-convert_<version>_<platform>`).

## Using it

1. Drop the game folder on the window, or click it to choose the folder.
   The folder above the game data works too (the app looks two levels
   down). The box says "Detecting the game…" while it looks, then which
   game it found and where; with several, choose one under **Game**.
2. Check the output folder. The default is a new folder beside the game
   folder. The app asks before using a folder that isn't empty, because
   the converter replaces what it writes there and deletes the folders it
   packs into archives.
3. Optionally open **Advanced options** (below). Without it the game's
   recommended settings are used.
4. Click **Build**. A build takes several minutes and a few GB of disk;
   **Cancel** stops the converter and the ffmpeg processes it started.
   The bar follows how much the build has written against how much it
   usually writes (packing the archives briefly needs twice the finished
   size), so it moves unevenly; it fills when the converter ends.
5. Play it: after the build the app puts CONscripter into the game folder
   (below), so starting it there plays the game.

**CONscripter in the game folder**: after a build (not a dry run) the app
downloads the [CONscripter release](https://github.com/blankiekun/CONscripter/releases)
the build was made for (the version in its `version.txt`) for this
computer, keeps it in a cache folder (`%LOCALAPPDATA%`, `~/Library/Caches`
or `~/.cache`, under `conscripter-convert`), and puts it beside `0.txt`
named after the game's title, with the game's own icon (from the
original's `.exe`, or the Vita's `sce_sys/icon0.png`):

| System | What it writes |
|---|---|
| Windows | `<title>.exe`, its icon replaced |
| Linux | `<title>`, and `<title>.desktop` with `icon.png` (a program file has no icon; the launcher also starts it in the game folder, by absolute paths, so it needs a new build if the folder moves) |
| macOS (Apple silicon; Intel Macs have no CONscripter build) | `<title>.app`, which starts CONscripter in the folder the `.app` is in |

Without a connection the build still finishes, and the status says where
to download CONscripter.

| Game | Recognised by | Settings | Needs |
|---|---|---|---|
| 最果てのイマ COMPLETE | `Farthest2015.exe`, `HD/CDFiles/*.cd`, `CD/` | `convert-game`, title "Saihate no Ima COMPLETE", IPA Gothic | ffmpeg (movies) |
| 装甲悪鬼村正 | `Muramasa.exe` and `nss.npa`, `cg.npa`, `sound.npa`, `voice.npa`, `system.npa` | `convert-nss --file ma00_000.nss --chain 2000`, title 装甲悪鬼村正, IPA Mincho | |
| 俺たちに翼はない (PS Vita) | `media/SCRIPT.cpk`, `media/CG.cpk`, `sce_sys/param.sfo`; the CPK must be decrypted | `mages convert`, IPA Gothic | ffmpeg (sounds, movies) |

**Advanced options** are the converter flags that make sense for each
game; the line under them shows the exact command. **Recommended settings**
puts them back.

| Option | Games | Flag |
|---|---|---|
| Window title | all | `--title` |
| WebP pictures, WebP quality (empty: near-lossless; 0-100: lossy, 80 is about a quarter of the size) | 最果てのイマ | `--webp`, `--webp-quality` |
| Loose files | all | `--loose` |
| Script only (dry run) | 村正, 翼 | `--dry-run` |
| Start at the script, story files to follow | 村正 | `--file`, `--chain` |
| Only these scenes | 翼 | `--scenes` |
| Voices and sound effects (vorbis/opus), their quality | all | `--audio-codec`, `--audio-quality` |

**ffmpeg** is not included (its usual builds are GPL with libx264, and
large). Install it so `ffmpeg` is on PATH (Windows: for example
`winget install ffmpeg`; macOS: `brew install ffmpeg`; Linux: the
distribution's package), or point the app at `ffmpeg(.exe)` with
**Locate…**. The app checks that it has the libx264, aac and libvorbis
encoders.

**Fonts**: every build needs a Japanese font, which the converter copies
into the game as `default.ttf`. The release includes IPA Gothic
(`ipag.ttf`) and IPA Mincho (`ipam.ttf`, for 村正, whose text is
Mincho), with their license: the IPA Font License allows passing the
fonts on unchanged with the license, and bundling them means nothing to
find or install. **Change…** picks another font.

## Running from source

Python 3.10 or newer with tkinter, and a built converter:

```sh
cargo build --release
python -m venv .venv && .venv/bin/pip install -r gui/requirements.txt
.venv/bin/python gui/fetch_fonts.py      # IPA fonts into gui/fonts/ (else system fonts are used)
.venv/bin/python gui/app.py [<game folder>]
```

The app finds the converter bundled, next to itself, in
`target/release/`, or on PATH (`CONSCRIPTER_CONVERT=<path>` overrides).
Without `tkinterdnd2` it works without drag and drop.

- `python gui/app.py --detect <folder>` prints the game found and the
  command it would run, without a window.
- `python -m unittest discover -s gui/tests` tests the detection on fake
  folders, the commands against the converter's `--help`, the output
  folder checks and cancelling.

## Building the executable

```sh
.venv/bin/python gui/build_exe.py [--converter <path>]
```

writes `gui/dist/conscripter-convert-gui` (one file; a `.app` folder on
macOS). `dist/conscripter-convert-gui --self-test report.json` checks that
the bundled converter runs and the fonts and drag and drop are there. The
release workflow (`.github/workflows/release.yml`) does all of this on
each `v*` tag.

## Adding a game

Add a `Game` entry to `GAMES` in `games.py`: its marker files, an optional
`check` for problems the markers can't see, the converter arguments from
the game's document, its `options` (each a converter flag; their defaults
are the recommended settings), its font, whether it needs ffmpeg and the
finished build's size (the progress bar and the free-space check use it). Add a fake layout and the expected command to
`tests/test_games.py`.
