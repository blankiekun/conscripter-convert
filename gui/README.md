# CONscripter-convert GUI

A small window around the `conscripter-convert` command for players:
drop the game folder on it, and it finds which game it is and builds it
with that game's recommended settings (the commands in
[`docs/games/`](../docs/games/)). It runs the converter as a child process
and shows its output; it converts nothing itself.

Releases ship it as one file per platform with the converter and the
fonts inside: `conscripter-convert-gui_<version>_x64_win.exe`,
`…_x64_linux`, and `…_aarch64_darwin.zip` (a `.app`, unsigned: open it with
right click → Open the first time).

## Using it

1. Drop the game folder on the window, or click it to choose the folder.
   The folder above the game data works too (the app looks two levels
   down).
2. Check the output folder. The default is a new folder beside the game
   folder. The app asks before using a folder that isn't empty, because
   the converter replaces what it writes there and deletes the folders it
   packs into archives.
3. Click **Build**. A build takes several minutes and a few GB of disk;
   **Cancel** stops the converter and the ffmpeg processes it started.
4. Play the result with [CONscripter](https://github.com/blankiekun/CONscripter/releases).

| Game | Recognised by | Settings | Needs |
|---|---|---|---|
| 最果てのイマ COMPLETE | `Farthest2015.exe`, `HD/CDFiles/*.cd`, `CD/` | `convert-game`, title "Saihate no Ima COMPLETE", IPA Gothic | ffmpeg (movies) |
| 装甲悪鬼村正 | `Muramasa.exe` and `nss.npa`, `cg.npa`, `sound.npa`, `voice.npa`, `system.npa` | `convert-nss --file ma00_000.nss --chain 2000`, title 装甲悪鬼村正, IPA Mincho | |
| 俺たちに翼はない (PS Vita) | `media/SCRIPT.cpk`, `media/CG.cpk`, `sce_sys/param.sfo`; the CPK must be decrypted | `mages convert`, IPA Gothic | ffmpeg (sounds, movies) |

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
the game's document, its font, whether it needs ffmpeg and about how much
it writes. Add a fake layout and the expected command to
`tests/test_games.py`.
