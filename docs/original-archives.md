# Playing from the original archives

Today every build re-encodes the game's pictures and sounds into
`arc.nsa` (1.3 GB for 続・殺戮のジャンゴ, 1.7 GB for 俺たちに翼はない,
2.4 GB of pictures for 月姫). The plan here: CONscripter reads the
player's own archives in place, so a converted game is its script, a
mapping file and the few pictures the converter makes. Builds take
seconds, we never hand out game data, an update is a few hundred KB,
and saves stay valid because `0.txt` is the same either way.

Phase 1 (done): the design, and Django's NPA read in place
(`--original-archives`). Phase 2: the other formats, one PR each
(order at the end); done: Muramasa's zlib NPAs.

## The model

A converted game folder holds:

- `0.txt`: the same script as today, byte for byte. Asset names stay
  the converter's paths (`cg/bg/x.jpg`, `sound/se/x.ogg`).
- `cons_archives.txt`: the mapping: which original archive answers
  which script folder, with what key. Small and made at build time
  from the player's install.
- `arc.nsa`: only what the converter **makes**: composites, pre-scaled
  zooms, mirrored pictures, colour cards, the engine screens' pictures
  and warp maps, and converted movies.
- The player's archives, copied or linked into the folder (`cg.npa`,
  `sound.npa`, …), or picked in a browser (below).

The engine looks a name up in this order: loose file, original archive,
`arc.nsa`. A missing original is a missing file (the trace's `missing`
record), as a missing loose file is today.

### The mapping file

One line per archive (implemented: `npa`; the rest is the phase-2
plan):

```
npa  <folder> <file> <512 hex: decryption table>     Nitroplus NPA
npk3 <folder> <file> <64 hex: AES-256 key>           Nitroplus Diesel
mrg  <folder> <hed> <mrg> <nam>                      TYPE-MOON MRG/MZP
cpk  <folder> <file>                                 CRI CPK / AWB (AFS2)
xarc <folder> <file>                                 Xuse XARC / XSA / GAF4
=    <script name> <archive>:<entry>[|<transform>]   one entry by name or ID
```

- **Folder lines** cover games whose script paths are the archive's
  paths under a prefix (both Nitroplus games: `cg/bg/x.jpg` is `bg\x.jpg`
  in `cg.npa`). Names match without regard to case (ASCII and
  full-width Latin: Django's scripts write `ｂ` where the archive has
  `Ｂ`) and of `/` against `\`.
- **`=` lines** name entries one by one where the script names aren't
  the archive's: 俺たちに翼はない's numbers (`CG.cpk` ID 412 →
  `cg/412.png`), Xuse's sound-bank slots, a sub-picture of a GAF4. A
  `|transform` asks the engine to change the bytes on the way:
  `|nxgz` (inflate), `|bntx` (deswizzle + BC7 decode), `|nxopus`
  (wrap Nintendo Opus frames in Ogg), `|adx`, `|hca`.
- **Keys** come from the player's exe at build time. They could instead
  be named as `exe:<file>@<offset>` so the mapping carries none (phase
  2, if keys in a shared mapping are a concern); Django's table is not
  in its exe as plain bytes, so its 256 bytes stay in the mapping.
- **Version check** (phase 2): each line also records the archive's
  size and a hash of its index, so a different release (Django 1.00
  against 1.01's `cg2.npa`) gives one clear message at startup instead
  of missing pictures later.

### Saves and kidoku

Nothing changes: `0.txt` is identical (checked: `cmp` of both Django
builds), and saves, `gloval.sav` and `kidoku.dat` hold script
positions and variables, never asset bytes. A player can switch a
folder between the two kinds of build and keep their saves.

## The formats

Decode cost is per file, paid when the engine loads it (pictures are
cached as today). "Weak" is a 2016-era phone or a Kobo; "web" is wasm
in a browser, about half native speed, single-threaded. The speeds are
published figures for the libraries, not measured here.

| Format (game) | Container, crypto, packing | Inside | Run-time cost | Verdict |
|---|---|---|---|---|
| NPA, uncompressed (Django: `cg`, `cg2`, `sound`, `voice`) | index with encrypted names; first `0x1000 + len(name)` bytes of each entry through a 256-byte table | PNG, JPEG, BMP, Ogg Vorbis | 4 KB table lookups: nothing | **done** |
| NPA, zlib (Muramasa: every archive) | as above, then zlib per entry | PNG, JPEG, Ogg (zlib gains little on them) | inflate: ~300 MB/s desktop, ~50 MB/s weak, ~150 MB/s web; a 1 MB picture 3-20 ms | **done**: the engine's own inflater (puff-style, no library) |
| NPK3 (Muramasa EN) | AES-256-CBC index and data segments, then zstd per segment | lossless WebP (`.png` names), Ogg, TTF | AES ~1 GB/s with AES-NI, ~30-100 MB/s software (weak, web); zstd ~500 MB/s; WebP lossless decode is the larger cost (~30 ms for 1024×576 desktop) | run time; AES + zstd decoder |
| MRG/MZP + NXGZ/NXCX + BNTX (月姫, Switch) | `.hed`/`.mrg`/`.nam` index, MZP sectors, gzip/deflate | BNTX: Tegra block-linear swizzled BC7 (BC4 masks) at up to 886×4160 | inflate + deswizzle + BC7: ~100 Mpx/s desktop, 10-20 weak/web; a 3.7 Mpx sprite 40 ms desktop, 200-400 ms weak/web, per expression change; 16 GB of archives | **convert** the pictures (or a decode-once disk cache, phase 3); read scripts' text, Opus voices and movies in place |
| `stream/*.opus` (月姫) | loose files, Nintendo Opus framing | Opus 48 kHz | rewrap the header in Ogg: nothing (decode as today) | run time (`|nxopus`) |
| `knife1.bfsar` (月姫) | Nintendo sound archive | DSP ADPCM, loop points | ADPCM decode is trivial | run time possible; convert first (loops) |
| CPK, AWB/AFS2 (俺たちに翼はない, Vita) | TOC with names and IDs; no CRILAYLA (checked: `ExtractSize` = `FileSize` everywhere) | PNG; ADX music and effects; HCA voices | ADX trivial; HCA ~ Vorbis decode cost | pictures run time now; sound run time with an ADX/HCA decoder, or convert voices once |
| USM (俺たちに翼はない) | Sofdec2 | video + audio | — | **convert** (movies, below) |
| XARC, XSA, GAF4 (最果てのイマ) | XARC: plain entries with CRC-16; XSA: 256 slots; GAF4: PNG frames | PNG, Ogg Vorbis | nothing | run time; `=` lines for slots and frames |
| Movies (all) | MPEG-1 PS (Django, Muramasa `.ngs`, Xuse `.bin`), H.264 MP4 (月姫), USM | | the web plays movies with `<video>` (MP4/WebM only); native through ffmpeg | **convert**, as today (Django: 46 MB of `movie/`) |

What must still be converted, whatever the engine reads:

- **Movies**: browsers don't play MPEG-1 or USM; MP4 (H.264/AAC) or
  WebM stay converter output. Only the movie folder, and it is small
  next to the pictures (Django 46 MB, Muramasa about 350 MB before
  re-encoding).
- **Pictures the converter makes**: composites, zooms, mirrors, strips,
  colour cards, the engine screens. In Django's prototype these are
  all of the remaining `arc.nsa`: 477 MB, of which 429 MB are 1,467
  pre-scaled zooms (`system/zoom`). Drawing those zooms with the
  engine's own scaling instead would bring the folder under 60 MB; a
  separate item.
- **Switch textures** (月姫): BC7 at 4K sizes is too slow on weak
  devices and the web per expression change. Converted pictures, or a
  cache: decode once on first use and keep a PNG/WebP in the save folder
  (IndexedDB/OPFS on the web).
- **Lossy-to-lossy choices** (`--webp`, `--audio-codec opus`): they
  only make sense for the hosted web build, which is the case this
  design replaces; `--original-archives` refuses them.

## Platforms

- **Desktop, Kobo**: `fopen` of the files in the game folder. Large
  archives (Muramasa `voice.npa` 1.5 GB) need 64-bit seeks: the reader
  uses `fseeko` / `_fseeki64`.
- **Web**: the player gives the browser their install: a folder picker
  (`showDirectoryPicker`, File System Access, Chromium; the handle kept
  in IndexedDB so the next visit asks only for permission) or a drop of
  the files (every browser; dropped again each visit, or copied into
  OPFS once). Nothing is uploaded. Each `File` reads ranges with
  `file.slice(a, b).arrayBuffer()`, which is asynchronous; the engine
  already asks the page for every range before it reads it
  (`webPreload` → `self.consPreload(fd, offset, length)`, as for
  `arc.nsa` entries) and waits for it, so the page fills the bytes in
  and the read is served from memory. The NPA reader calls it for the
  header, the index and each entry. Phase 2: the page side (a picker
  on the player page and a `consPreload` that reads `File` ranges);
  the hosted game then ships `0.txt`, the mapping and a small
  `arc.nsa`.
- **Android**: the Storage Access Framework gives a tree URI, not a
  path; the app opens each archive once through
  `ContentResolver.openFileDescriptor` and the engine reads the fd
  (`fdopen`), behind the existing `fopen_ons` hook. Or the player
  copies the archives into the app's folder.
- **libretro**: the content is the game folder with `0.txt`; the
  archives are copied or linked in (RetroArch's VFS reads them as
  files), or the mapping names an absolute path.

## Licences

The engine is GPLv2. Decoders it would take in, all compatible:

| Need | Candidate | Licence |
|---|---|---|
| inflate (NPA zlib, NXGZ) | zlib (already linked by SDL_image's libpng on every platform; `-sUSE_ZLIB` on the web), or miniz single file | zlib / MIT |
| zstd (NPK3) | zstd's single-file decoder (`zstddeclib.c`) | BSD (dual BSD/GPLv2) |
| AES-256-CBC (NPK3) | tiny-AES-c | Unlicense |
| BC7/BC4 (BNTX), if decoded at run time | bcdec | MIT / Unlicense |
| HCA (AWB voices) | clHCA (as in vgmstream) | ISC-style |
| ADX | a few dozen lines of our own | — |

The archive formats themselves and their keys are facts about files the
player owns; the mapping carries only what the player's exe holds.

## Today against reading in place (Django, measured)

Whole story, the default build (`arc.nsa`, PNG/Ogg as in the game, no
`--webp`), the same machine, one run each:

| | Today | `--original-archives` |
|---|---|---|
| Build time | 118 s | 95 s (the prototype still extracts every picture to make the zooms from, then deletes the copies) |
| Converter peak memory | 496 MB | 474 MB |
| `arc.nsa` | 1,289 MB, 7,104 files | 477 MB, 1,838 files (all converter-made) |
| Game folder | 1.3 GB | 510 MB (+ the player's 862 MB of `cg`, `cg2`, `sound`, `voice` `.npa`) |
| `0.txt` | | byte-identical |
| Engine, 569 pages of auto-play | 20.5 s, 121.5 MB | 20.1 s, 119.3 MB |
| 60 page screenshots | | AE 0 at fuzz 0 against today's, every one |

The page records of the two traces differ only where two runs of
today's build differ from each other (a fading sound's volume sampled
at the page, 20 pages).

Phase 2 for the converter: read the zoom and composite sources straight
from the archive instead of extracting and deleting, which takes the
build to the time of making only its own pictures (the dry run is 3.5 s);
with the engine scaling zooms itself, under a minute and under 60 MB.

## How to build one (Django)

```sh
target/release/conscripter-convert convert-nss <game> -o out/django-orig \
    --font <ipag.ttf> --title 続・殺戮のジャンゴ --original-archives
ln -s <game>/{cg,cg2,sound,voice}.npa out/django-orig/   # or copy them
```

`cons_archives.txt` names the archives by file name, relative to the
game folder; it needs CONscripter with the reader (engine PR, see the
converter PR).

## Phase 2, per format, in order

1. **Muramasa NPA** (zlib), done: the engine inflates with its own
   decoder (zlib isn't linked on libretro, MSVC or Android). Whole
   story: `0.txt` byte-identical; folder 3.6 GB → 889 MB (737 MB of it
   `system/zoom`); build 381 → 307 s; 90 s headless auto-play reached
   more pages (4,552 against 4,104), 80 page shots AE 0, engine memory
   402 → 360 MB. `--english` is refused for now (the engine looks in the
   NPAs before `arc.nsa`, so the Japanese pictures would win). Left: the
   converter reads its zoom sources from the archive.
2. **Web player**: folder picker / drop, `consPreload` over `File`
   ranges, tried headless in Chromium with `web_run.py`.
3. **最果てのイマ** (XARC, XSA, GAF4): `xarc` lines and `=` lines for
   slots and frames; nothing to decode.
4. **俺たちに翼はない** (CPK, AWB): `cpk` lines with `=` ID names for
   the pictures; ADX in the engine; HCA voices converted once or a
   decoder (clHCA); movies stay converted.
5. **Muramasa EN** (NPK3): AES + zstd; WebP pictures as they are.
6. **月姫** (MRG/MZP/BNTX): pictures stay converted (or a decode-once
   cache); text, Opus voices (`|nxopus`) and `knife1.bfsar` sounds in
   place.
7. Version check lines; engine-side zoom scaling (drops `system/zoom`).
