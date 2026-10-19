# conscripter-convert command reference

Every subcommand and option of the converter. `conscripter-convert <command> --help`
prints the same from the program itself; a test (`cargo test`) fails when a
subcommand or `--flag` is missing from this page.

Most players want only `build` (or the GUI, [`gui/README.md`](../gui/README.md)).

## Building a game

### `conscripter-convert build`

```sh
conscripter-convert build <game folder> -o <output> [options]
```

Recognises the game in the folder (as `conscripter-convert games <folder>`
does) and runs that game's convert command (`convert-game`, `convert-nss`,
`mages convert`, `typemoon convert`) with its title and recommended
options. Any convert option after `-o` is passed on and replaces the game's
own (`--title`, `--file`, ...). Each game's document in `docs/games/` gives
the command it runs.

An English edition is built on the Japanese game:
`build <english folder> --japanese <japanese folder> -o <output>` (Full Metal
Daemon Muramasa on the Japanese 装甲悪鬼村正; Tsukihime's fan patch RomFS on
1.0.2's).

### `conscripter-convert games`

`games` lists the games the converter builds; `games <folder>` lists the ones
found in a folder; `games --json` prints the list as JSON.

### `conscripter-convert lint`

`lint <dir>` checks a converted game folder's `0.txt`: missing labels, effect
numbers past 255, half-width characters in vertical text, files that are not
in the archives.

### The convert commands

`build` runs one of these; call them directly to choose every option.

| Command | Converts | Its own options |
|---|---|---|
| `conscripter-convert convert-game <game>` | Xuse (最果てのイマ): the whole game | `--title`, `--linear` (play the story scripts in file order with no jumps; a debugging aid), `--original-archives` |
| `conscripter-convert convert-script <game>` | one Xuse script (`--script N`, the NNNN of `CDFiles/NNNN.cd`), for testing a scene | `-o`, `--font`, `--title`, WebP and audio options |
| `conscripter-convert convert-nss <game>` | Nitroplus NSS (装甲悪鬼村正, 続・殺戮のジャンゴ, the English Muramasa) | `--file`, `--scene`, `--chain N`, `--movies <dir>`, `--no-movies`, `--english <dir>`, `--title`, `--original-archives` |
| `conscripter-convert mages convert <game>` | the decrypted PS Vita 俺たちに翼はない | `--scenes A,B`, `--title` |
| `conscripter-convert typemoon convert <RomFS>` | 月姫 (Switch) | `--exe`, `--english <RomFS>`, `--title` |

All but `convert-script` also take the build options below.

### Build options

| Option | Effect |
|---|---|
| `-o`, `--output <dir>` | The folder to write |
| `--font <ttf>` | The font copied into the output as `default.ttf` (default: the bundled IPA Gothic). Use a fixed-width font: the engine puts each character in a full-width cell, so a proportional font leaves gaps after narrow kana |
| `--loose` | Write assets as loose files instead of packing them into `.nsa` archives |
| `--dry-run` | Compile and write `0.txt` only, no assets (seconds instead of minutes; the same `0.txt` as a full build) |
| `--source-map <file>` | Also write where each `0.txt` line came from in the game's scripts (`<0.txt line>` tab `<script>:<instruction>`); only engines that record it |
| `--small` | A smaller download: `--webp` and `--audio-codec opus` together (an explicit `--audio-codec` or `--webp-quality` still applies). `version.txt` names the media flags a build used |
| `--webp` | Store images as near-lossless WebP, about half the size of PNG. CONscripter's web build reads it; native builds need an SDL_image with libwebp. Pictures wider or taller than WebP allows (16,383 pixels) stay PNG |
| `--webp-quality <N>` | With `--webp` (or `--small`): lossy WebP at quality N (0-100, as `cwebp -q`; transparency stays lossless). 80 makes the pictures about a quarter of the near-lossless size and is hard to tell apart |
| `--audio-codec <vorbis\|opus>` | Voices and sound effects as Vorbis (the game's own files, the default) or Opus (re-encoded with ffmpeg; needs CONscripter 0.21.0 or later). Music stays Vorbis |
| `--audio-quality <N>` | Opus kbit/s per channel (6-256, default 32); Vorbis ffmpeg `-q:a` (-1 to 10), which re-encodes the game's own Vorbis |

`--original-archives` (`convert-game`, `convert-nss`) leaves the pictures and
sounds out of the output; CONscripter reads them from the game's own archives,
which the output's `cons_archives.txt` names. Background and how to build
one: [`docs/original-archives.md`](original-archives.md).

### `conscripter-convert strtab`

`strtab <0.txt> -o <dir>` splits a built game's `0.txt` into the script with
text IDs and one text table per language
([`docs/string-tables.md`](string-tables.md)). Options: `--map <file>` (the
build's `--source-map`), `--english <0.txt> <map>` (the English build and its
map), `--scene-prefix <prefix>` (labels starting with it begin a scene).

## Format tools

Low-level subcommands for inspecting each engine's files. `conscripter-convert <command> <subcommand> --help` lists the arguments.

| Command | What it does |
|---|---|
| `conscripter-convert xarc list` | List entries of a `*.arc` |
| `conscripter-convert xarc verify` | Check all stored CRCs |
| `conscripter-convert xarc extract` | Extract all entries (`-o <dir>`) |
| `conscripter-convert gaf list` | Header fields and image count of `*.004` files |
| `conscripter-convert gaf names` | Each image's index and original name |
| `conscripter-convert gaf extract` | Extract the embedded PNGs (`-o <dir>`) |
| `conscripter-convert xsa list` | List clips of a `*.xsa` or `HD/*.bin` |
| `conscripter-convert xsa extract` | Extract clips as `.ogg` (`-o <dir>`) |
| `conscripter-convert cd info` | Header fields and MD5 footer of `*.cd` scripts |
| `conscripter-convert cd dump` | Label table and instructions |
| `conscripter-convert cd messages` | Decoded messages and hyperlinks |
| `conscripter-convert cd strings` | Strings in the operand pool (heuristic) |
| `conscripter-convert cd symbols` | Label symbol table (`CDFiles/0.cd`); `--script <id>` |
| `conscripter-convert cd opstats` | Count opcodes over many scripts |
| `conscripter-convert npa list` | Files of a Nitroplus `*.npa`: `<game exe> <archive>` |
| `conscripter-convert npa extract` | Extract decrypted files (`-o <dir>`, `--prefix`) |
| `conscripter-convert nss check` | Parse `*.nss` scripts and report failures |
| `conscripter-convert nss refs` | Check scenes, chapters, files and functions exist; list built-ins used |
| `conscripter-convert nss builtins` | Argument counts and common values of each built-in (`--top`) |
| `conscripter-convert nss route` | Route map: each story file's next files and conditions |
| `conscripter-convert mages list` | List a `*.cpk` or `*.AWB` |
| `conscripter-convert mages extract` | Extract files (`-o <dir>`, `--prefix`) |
| `conscripter-convert mages convert` | Convert the decrypted Vita game (`--scenes`, `--title`, build options) |
| `conscripter-convert mages check` | Parse every script in `media/SCRIPT.cpk`, check file IDs |
| `conscripter-convert typemoon list` | List an MRG or MZP archive |
| `conscripter-convert typemoon extract` | Extract entries (`-o <dir>`, `--prefix`, `--png`, `--max-bytes`) |
| `conscripter-convert typemoon check` | Read every archive of a RomFS, decode every picture, count failures |
| `conscripter-convert typemoon sounds` | Convert the RomFS sound to `.ogg`/`.wav` |
| `conscripter-convert typemoon script` | Rank script commands by use, check references; `-o` writes each script as text |
| `conscripter-convert typemoon exe` | Read the exe's tables (`--romfs`, `--motion`) |
| `conscripter-convert typemoon media` | Write the build's pictures, movies and sound (`--english`, WebP and audio options) |
| `conscripter-convert typemoon convert` | Convert the RomFS (`--exe`, `--english`, `--title`, build options) |
