# Playing a converted game

A converted game is a folder holding `0.txt`, `arc.nsa` (and `arc1.nsa`, ...),
`default.ttf` and `ons_args`. [CONscripter](https://github.com/blankiekun/CONscripter)
plays it. Pick the way that matches your device. To make the folder, see
[README.md](../README.md) ("Get a game playing"); for every option,
[the converter reference](cli.md).

CONscripter releases:
<https://github.com/blankiekun/CONscripter/releases>. File names carry the
version (`<version>`). The engine's own per-device pages are
[PLATFORMS.md](https://github.com/blankiekun/CONscripter/blob/master/PLATFORMS.md)
and the options list in its README.

## With the converter's GUI (Windows, macOS, Linux)

After a build the GUI puts CONscripter into the game folder, so starting it
there plays the game ([`gui/README.md`](../gui/README.md), "CONscripter in the game
folder"):

| System | Start |
|---|---|
| Windows | `<title>.exe` in the game folder |
| Linux | `<title>`, or `<title>.desktop` (it starts the game in its folder) |
| macOS | `<title>.app`; unsigned, so open it with right click, then Open, the first time |

Without a connection the build still finishes and the status says where to
download CONscripter; then use the next section.

## CONscripter by hand (Windows, Linux, macOS)

1. Download the file for your system from the releases:
   `conscripter_<version>_x64_win.exe` (or `x86_win`), `_x64_linux`
   (`x86_linux`, `arm64_linux`, `arm32_linux`), or `_universal_darwin`
   (`_aarch64_darwin` for Apple silicon only).
2. Run it with the game folder: `conscripter --root <game folder>`.
   Options in an `ons_args` file in the game folder also apply.
3. On a Mac the binary is unsigned: run `chmod +x` on it and
   `xattr -d com.apple.quarantine` on it once.
4. Alt+Enter goes fullscreen, F10 stretches the game to the window, a long
   click opens the menu.

`conscripter --help` lists the options.

## In the browser

The site <https://blankiekun.github.io/?app=cons#cons> has a CONscripter window.
What its code shows (the page itself is the reference; this is not a
promise about its look):

1. Choose **Open Folder...** and pick the converted game folder (the one with
   `0.txt` in it), or drop the folder, or a `.zip` of it (**Open Zip...**),
   on the window. **Play the Demo** needs no game.
2. The game is read from your disk and is not uploaded.
3. Saves stay in your browser; clearing the site's data erases them. **File >
   Export Saves** writes them to a `.zip` and **Import Saves** loads it back.
   The site can also copy saves to your Google Drive (**File > Sync with
   Google Drive**) when that is offered.
4. **Recent games** lists the last games opened. In Chrome and Edge the
   browser can keep the folder, so a game opens again without choosing it;
   other browsers ask for the folder again. **File > Settings**, "Keep a copy of
   games I open", lets games open again offline.

Saves are kept per game under the folder's name.

If you host the files yourself instead, see the engine's
[PLATFORMS.md](https://github.com/blankiekun/CONscripter/blob/master/PLATFORMS.md)
("Web").

## Android

Install `conscripter_<version>_android.apk`. Put the game folder in
`/storage/emulated/0/Android/data/io.github.blankiekun.conscripter/files` or
pick any folder through the file picker. A three-finger tap or a long touch
opens the menu; four fingers skip text.

## RetroArch and handhelds

Each console has its own file in the releases and its own install steps
(Switch, PS Vita, Miyoo Mini, Wii U, 3DS): see the engine's
[PLATFORMS.md](https://github.com/blankiekun/CONscripter/blob/master/PLATFORMS.md).
Big games may not fit a console's memory; the page says what fits.

## Kobo (KOReader)

Unzip `conscripter_<version>_kobo_koreader.zip` at the root of the Kobo's
storage, copy `play.sh` next to the game's archives, then run it from
KOReader's file browser (long-press, "Execute shell script"). Details:
PLATFORMS.md, "Kobo".

## If a game does not start

- The game needs a Japanese font: `default.ttf` in the folder, or `--font`.
- After force-quitting, remove `envdata` from the save folder.
- Report a problem with the line from `version.txt` (see the README).
