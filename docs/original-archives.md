# Playing from the original archives

With `--original-archives`, CONscripter reads the player's own archives
in place instead of a re-encoded `arc.nsa`: a converted game is its
script, a mapping file and the few pictures the converter makes. Builds
are faster, no game data is handed out, an update is a few hundred KB,
and saves stay valid (`0.txt` is byte-identical either way).

Supported: Django's NPA, Muramasa's zlib NPA (Japanese only:
`--english` is refused, since the engine looks in the NPAs before
`arc.nsa`), 最果てのイマ's GAF4/XSA banks, and the web player.

## What a build holds

- `0.txt`: the same script as a normal build. Asset names stay the
  converter's paths (`cg/bg/x.jpg`, `sound/se/x.ogg`).
- `cons_archives.txt`: the mapping (below), made at build time from the
  player's install.
- `arc.nsa`: only what the converter **makes**: composites, pre-scaled
  zooms (`system/zoom`, most of it), mirrored pictures, colour cards,
  the engine screens' pictures, converted movies (browsers play neither
  MPEG-1 nor USM).
- The player's archives, copied or linked into the folder, or picked in
  the browser.

The engine looks a name up in this order: loose file, original archive,
`arc.nsa`. A missing original is a missing file (the trace's `missing`
record). Saves, `gloval.sav` and `kidoku.dat` hold script positions and
variables, never asset bytes: a folder can switch between the two kinds
of build and keep its saves. `--webp` and `--audio-codec opus` are
refused with `--original-archives`.

## The mapping file

`cons_archives.txt`, one line per archive or entry; names are relative
to the game folder:

```
npa  <folder> <file> <512 hex: decryption table>       Nitroplus NPA (plain or zlib)
range<TAB><name><TAB><file><TAB><offset><TAB><length>  bytes as they lie in a file
xor<TAB><file><TAB><hex key>                           the file XORed, key cycled by absolute offset
```

- **Folder lines** (`npa`): the script's paths are the archive's paths
  under a prefix (`cg/bg/x.jpg` is `bg\x.jpg` in `cg.npa`). Names match
  without regard to case (ASCII and full-width Latin: Django's scripts
  write `ｂ` where the archive has `Ｂ`) and of `/` against `\`. The
  engine inflates zlib entries with its own decoder (zlib is not linked
  on every platform).
- **`range` lines** (tab-separated: names may hold spaces): any entry
  stored whole and unpacked inside a bigger file; the converter knows
  each offset, so the engine needs no format code. 最果てのイマ uses
  them for its GAF4 pictures and XSA voices and effects, with an `xor`
  line per GAF4 bank.
- **Keys** come from the player's exe at build time.

## How to build one

```sh
target/release/conscripter-convert convert-nss <game> -o out/django-orig \
    --font <ipag.ttf> --title 続・殺戮のジャンゴ --original-archives
ln -s <game>/{cg,cg2,sound,voice}.npa out/django-orig/   # or copy them
```

最果てのイマ: `convert-game --original-archives`, then copy or link the
install's `HD/` and `CD/` into the game folder. BGM with a loop start
stays converted (the loop tag is written into the Ogg).

Check: `0.txt` byte-identical to a normal build (`cmp`), a headless
auto-play trace with no `missing` records.

## Platforms

- **Desktop, Kobo**: `fopen` in the game folder; 64-bit seeks
  (`fseeko` / `_fseeki64`) for archives over 2 GB.
- **Web**: the player picks their install as a second folder (the
  site's `addArchives` asks when the game folder lacks the archives
  `cons_archives.txt` names) and mounts the files as lazy `File` nodes;
  nothing is uploaded. The engine asks the page for every range before
  reading it (`webPreload` → `self.consPreload(fd, offset, length)`), as
  for `arc.nsa` entries.
- **Android**: the Storage Access Framework gives a tree URI; open each
  archive once through `ContentResolver.openFileDescriptor` and read
  the fd behind the `fopen_ons` hook, or copy the archives into the
  app's folder.
- **libretro**: archives copied or linked into the content folder.

## Planned (not built)

In order; each a PR of its own:

1. The converter reads zoom and composite sources from the archive
   instead of extracting them; then the engine scales zooms itself,
   which drops `system/zoom` (Django's folder would go under 60 MB).
2. Muramasa EN (NPK3: AES-256-CBC + zstd per segment; WebP inside):
   an `npk3 <folder> <file> <64 hex key>` line; decoders tiny-AES-c
   (Unlicense) and zstd's `zstddeclib.c` (BSD).
3. Version check: each line records the archive's size and an index
   hash, so a different release gives one clear message at startup.
4. Paused games, when they resume: 俺たちに翼はない (`cpk` lines and `=`
   ID lines for CPK/AWB; ADX in the engine, HCA voices converted or
   clHCA); 月姫 (pictures stay converted, BC7 at 4K is too slow to
   decode per expression change; text, Opus voices via an `|nxopus`
   transform in place).

Engine decoders must be GPLv2-compatible. The mapping carries only what
the player's exe holds.
