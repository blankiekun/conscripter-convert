# 月姫 -A piece of blue glass moon- (TYPE-MOON)

The 2021 remake of 月姫, Nintendo Switch release (title ID
`01001DC01486A000`, v0 base). TYPE-MOON's own engine, the family of
the 魔法使いの夜 remake.

## Scope

The user owns the Switch release and dumped it themselves (NSP plus
their console's `prod.keys`). The user asked for (2026-10-16): the
whole game **with every extra**, on **every platform** (web, Linux,
Windows, macOS), readable in **both Japanese and English**: the
Japanese from the release, the English from the Tsukihimates fan
patch (`tsukihimates_patch_2023.09.05.5c86a2e06.nsp`, which the user
downloaded). Nothing has been dropped yet.

## Build

No converter yet. Getting `<game>` (the RomFS) out of the NSP, with
[hactool](https://github.com/SciresM/hactool) and the console's keys:

```sh
hactool -k prod.keys -t pfs0 --pfs0dir=nsp <the .nsp>
hactool -k prod.keys --exefsdir=exefs --romfsdir=<game> nsp/<the Program NCA>
```

hactool 1.4 stops at a key line whose value isn't a multiple of 32 hex
digits (some newer `mariko_*` entries); give it a copy without them
(`awk -F' *= *' 'NF==2 && length($2)%32==0' prod.keys`).

The fan patch is an update NSP (title `01001DC01486A800`, version
131072) whose RomFS is a BKTR patch over the base game's, so it is
applied with the base Program NCA; the result is a full second RomFS
(`<game-en>`, 21 GB):

```sh
hactool -k prod.keys -t pfs0 --pfs0dir=patch <the patch .nsp>
hactool -k prod.keys --basenca=nsp/<the base Program NCA> \
  --exefsdir=<game-en>/../exefs --romfsdir=<game-en> patch/<the patch's Program NCA>
```

It changes every archive but `allpachd`/`allpacml`/`allpachdml`:
`allscr.mrg` grows from 9 to 19 MB (553 entries), `allpac` gains 10
entries.

The official update 1.0.2 (`[01001DC01486A800][v131072]`, 137 MB) is
applied the same way. Its Program NCA uses title-key crypto: the key is
the 16 bytes at 0x180 of its `.tik`. The result is `<game-102>`:

```sh
hactool -k prod.keys -t pfs0 --pfs0dir=upd <the 1.0.2 .nsp>
hactool -k prod.keys --titlekey=$(xxd -p -s 0x180 -l 16 upd/*.tik) \
  --basenca=nsp/<the base Program NCA> --exefsdir=<game-102>/../exefs \
  --romfsdir=<game-102> upd/<the update's Program NCA>
```

**Which copy to convert**: Japanese from `<game-102>` (base + 1.0.2),
English from `<game-en>` (the fan patch, which is 1.0.2 plus the
translation).

| File | base → 1.0.2 | 1.0.2 → fan patch |
|---|---|---|
| `main` (exe) | differs (a new build) | 65 bytes in 6 places |
| `allscr.mrg`, `script_text.mrg`, `allpac`, `allpaccg`, `allui` | differ | differ |
| `knife1.bfsar` | differs (adds `WSD_mix_se032`–`034`) | same |
| `allpac.nam` | differs (the 10 new entries) | same |
| `parts` | same | differs |
| `allpachd*`, `allpacml*`, `sdk`, `subsdk*`, fonts, `stream/` | same | same |

**The media folder** (pictures, movies, sound; Japanese from
`<game-102>`, English from `<game-en>`), about 15 minutes:

```sh
conscripter-convert typemoon media <game-102> -o out/tsukihime-cons \
  [--webp [--webp-quality 85]] [--audio-codec opus | --audio-quality 4]
```

The flags are the other games' (`convert` takes the same, and its
`0.txt` names the files they write). It writes every `allpac` picture
once, from its full art (the `.JPG` for a bare background, else the
larger of a name's two `.NXGZ`), at the texture's full size as
`pic/<name>.png` (lowercase), or `.webp` with `--webp` (near-lossless)
or `--webp-quality N` (lossy); the 98 `RULE_*` masks always PNG. The
movies as they are (H.264 1920 × 1080, remuxed with the index first, not
re-encoded) as `movie/<name>.mp4`, at their own size as the other
games' movies. The sound as `typemoon sounds` does: `stream/*.ogg` (the
game's Opus, repacked, loops as tags) and the wave sounds as
`se/*.wav` (PCM, loops as `smpl`), or with `--audio-codec opus` /
`--audio-quality N` (Vorbis) re-encoded as `se/*.ogg` with
`LOOPSTART`/`LOOPEND`. Pictures and movies already written are kept,
so a stopped run goes on.

Sizes for the Japanese copy: `stream/` 1.3 GB; `movie/` 1.7 GB; `se/`
1,395 MB as WAV, 56 MB as Opus (32 kbit/s a channel, the shared
default), 110 MB as Vorbis q4; `pic/` (14,763) 2.37 GB at
`--webp-quality 85`. On a sample of 57 pictures (12 sprites, 9 masks,
36 large backgrounds; 167 MB as PNG) near-lossless WebP is 45 % of PNG
and lossy q85 10 %: PNG would be about 23 GB, `--webp` about 10 GB.

**Both languages** in one build: `typemoon convert <game-102> --english
<game-en>` puts both copies of the scripts in one `0.txt` with a
language switch ("Languages" under "Status"). `--english` on `typemoon
media` (and `typemoon convert`) also writes the fan patch's pictures and movies that differ
from `<game-102>`'s, as `pic/en/<name>` and `movie/en/<name>`: 155 of
the 14,763 pictures (21 MB as WebP at 80) and 34 of the 46 movies
(1.4 GB: the 31 chapter titles, the endings with subtitles, the
openings and prologue, `TM_CI` with the patch's own logo). 38 movies'
bytes differ, but the patch re-encoded the four title backgrounds and
they show the same: a movie counts as the same when, at 200 sampled
times (grey, 480 × 270, each frame against the other's nearest), the
worst 16 × 16 block's mean difference stays at most 4 of 255
(`titlebg1`–`4`: at most 0.9; the next lowest, `ed02_ciel`, 42). The sound is the same in both copies
(`knife1.bfsar` and `stream/`, checked at each run) and comes from
`<game-102>`.

```sh
conscripter-convert typemoon media <game-102> --english <game-en> -o out/tsukihime-cons
conscripter-convert typemoon convert <game-102> --english <game-en> -o out/tsukihime-cons [--webp …]
```

**The game** (`0.txt` and, unless `--dry-run`, the media folder below
in the same output; `--loose` keeps the folders, else they are packed
into `arc*.nsa`):

```sh
conscripter-convert typemoon convert <game-102> -o out/tsukihime-cons
```

The exe is read from `<RomFS>/../exefs/main` (`--exe` for another
place), so each RomFS sits beside its own ExeFS, as the commands above
extract them: 1.0.2's `main` for Japanese, the fan patch's for English
(both have 1.0.2's tables). The base game's `main` is an older build
whose tables sit elsewhere; the converter refuses it.

The Program NCA is the 22 GB one (the others: Meta, Control,
Manual). The ExeFS holds `main` (3.3 MB NSO) and `subsdk0`/`subsdk1`.

## Files and formats

The RomFS, 21 GB, 16,729 files:

| Files | What |
|---|---|
| `allpac.{hed,mrg,nam}` (16 GB) | Main asset archive: `.hed` index, `.mrg` data, `.nam` names (29,476 names, 32 bytes each): 20,895 `.NXGZ`, 7,290 `.NXZ`, 1,279 `.JPG`, a few `.MZP`/`.PNG` |
| `allpaccg`, `allpachd`, `allpacml`, `allpachdml` (same triple) | CG; others not yet looked at (`ml` likely the multi-language copies) |
| `allui.{hed,mrg,nam}` (71 MB) | UI pictures, NXGX |
| `parts.{hed,mrg}` | not yet looked at |
| `allscr.mrg` (9 MB) | Scripts: an MZP (magic `mrgd00`, no `.hed`) of 552 entries. Entry 0: 550 script names, 32 bytes each (`START_SCRIPT`, `00_00`, `01_00_ARC01_1`, …, `JUMPQA`); entries 1 and 2 unknown; 3 onwards: one MZX0-compressed script per name |
| `script_text.mrg` (5 MB) | The text: an MZP of 10 entries, in pairs (big-endian u32 offset table, UTF-8 strings). The first pair is the Japanese text (3.4 MB); the other four pairs are small, not yet looked at |
| `stream/*.opus` (16,601) | Voices (`A00_01_00_0000_0.opus`) and songs, 30.1 hours; Nintendo's Opus framing (`typemoon/nxopus.rs`): 48 kHz, 16,468 mono, 133 stereo, pre-skip 120, no sample count or loop points in the file (the music's loops are in `knife1.bfsar`). `typemoon sounds` repacks them into Ogg Opus (no re-encode). The same files in both RomFS copies |
| `movie/*.mp4` (46) | H.264 1920×1080 + AAC 48 kHz: ffmpeg reads them |
| `ja*/` | Fonts (TT_TsukuMin-D, TT_TsukuGo-D, FOT-SeuratPro-M) |
| `knife1.bfsar` (386 MB) | Nintendo sound archive (`typemoon/bfsar.rs`, `fwav.rs`): 1,297 wave sounds (`WSD_moon_se02093`, `WSD_woh_*`, `WSD_fate_*`, …; 1,300 in the patched copy, which adds `WSD_mix_se032`–`034`), all DSP ADPCM 48 kHz, 195 looping (107 from inside the wave); and 16,596 stream sounds `STRM_*`, whose files are the `stream/*.opus` above (their names are in the archive's file table). `typemoon sounds` writes each wave sound as `se/<name>.ogg` (Opus, its loop as tags) |
| `shader/*.bnsh`, `exfont_nx01.bffnt` | Switch shaders and font, not needed |

How the formats nest, checked over every entry of both RomFS copies
(`conscripter-convert typemoon check <game>`, 8 min each with every
picture decoded; also
`typemoon list|extract <archive>`):

- **MRG** (`.hed`/`.mrg`/`.nam`): the `.hed` gives each entry's place
  in the `.mrg`, the `.nam` its name.
- **MZP** (`mrgd00`, u16 entry count, then 8-byte entries
  `u16 sector, u16 byte, u16 size_sectors, u16 size_low`, sector 0x800;
  size = `(size_sectors*0x800 & ~0xFFFF) | size_low`; data starts after
  the table).
- **MZX0**: an LZ-like compression on 16-bit words (the scripts;
  `typemoon/mzx.rs` has the commands). The fan patch's scripts were
  written by mangetsu's compressor, which leaves an odd last literal
  chunk one byte short; the reader accepts that when it still fills
  the unpacked size.
- **NXGX/NXCX** (`.NXGZ`/`.NXZ`): a 16-byte header (magic, sizes),
  then gzip (NXGX) or deflate (NXCX). Every picture inflates to
  **BNTX**, Nintendo's texture container (`typemoon/bntx.rs` has the
  header fields): one or more `BRTI` textures, each one 2D layer and
  one mip, Tegra block-linear swizzled (GOBs of 64 bytes × 8 rows,
  stacked `2^n` high, `n` in the header). Formats: BC7 sRGB (`0x2006`)
  for nearly all, BC4 (`0x1D01`, one channel) for the 98 `RULE_*`
  transition masks, R8G8B8A8 sRGB (`0x0B06`) for a few UI parts.
  `typemoon extract --png <archive>` writes `<entry>/<texture>.png`.
  Sprites are not atlases: every expression is a whole picture
  (`ARU_A_18_00_N01_10_10`: 886×4160, the full figure with its face),
  named `<pose>_<a>_<b>` from `_STCH`'s arguments 9 and 10 (see "The
  exe"). The bare `<pose>` (`ARU_A_18_00_N00`, 143×688) is a small
  copy of the figure.

### Existing tools (survey, 2026-10-16)

None of these are vendored or run; their code was read to check the
formats above. All three licences allow reuse.

| Tool | Licence | Covers | Use to us |
|---|---|---|---|
| [mangetsu](https://github.com/rschlaikjer/mangetsu) (C++, 2024) | MIT | Written for this release: MRG (`hed`/`nam`) list/extract/pack, MZP, MZX compress and decompress, NXGX/NXCX inflate, `script_text` to JSON, and the 魔法使いの夜 remake's HFA. Its `doc/script_commands.md` guesses at 17 script commands (`_VPLY` voice, `_STCH` picture, `_SEPL` sound, `_PGST` page, `_ZM` text, …) | The reference for the archive reader and MZX in Rust |
| [deepLuna](https://github.com/Hakanaou/deepLuna) (Python, 2023) | BSD-2 | Fan translation tool for this release: reads `allscr.mrg` and `script_text.mrg`, splits scripts into commands (`_NAME(arg,…);`, UTF-8), and finds text references (`$NNNNNN` = string index in `script_text`, `@x` modifiers) in `_ZM*`, `_MSAD` and `_SELR` (choices); `_PGST` sets the page number | The reference for the script syntax and how text is looked up |
| [PS-HuneX_Tools](https://github.com/Hintay/PS-HuneX_Tools) (Python, 2024) | MIT | HuneX's older MRG/MZX engines (PSP/PS2 era): `hed`, MZP, MZX0 specs, MZP picture tiles | Format specs as a cross-check; tile code is for older games |

Also: [MahoyoHDRepack](https://github.com/nike4613/MahoyoHDRepack)
(the 魔法使いの夜 remake: MRG/MZP/NXZ notes), and forum posts that open
the inflated NXGZ textures as BNTX in Switch Toolbox. Nobody documents
the script semantics beyond mangetsu's guesses: that is ours to work
out (the exe `main` is an NSO, to load in Ghidra).

Not covered by anything found, and ours now: Nintendo Opus to Ogg
Opus, the BFSAR chain to each wave sound's FWAV, DSP ADPCM. BNTX: our deswizzle (a few lines) and
the `bcdec_rs` crate for BC blocks.

## Executables

The builds the decompile addresses in this document belong to (the Ghidra addresses are those of 1.0.2).

| File | Version | sha256 |
|---|---|---|
| `main` (NSO), base game | 1.0.0 | `6e41c7094303a48b3223147b0fe52a2995c48f59af9d480b7e523ed4a1f103c7` |
| `main` (NSO), official update | 1.0.2 | `c128d79d3abd02b5581348618aa6403fdd651e77f4955f99ae43b34b90395faa` |
| `main` (NSO), fan patch | fan patch 2023.09.05 | `d28cb7c2391e74b2e3e2add120c208933aa314724efd2e47bfdf6c5c13aa6174` |

## The exe

`main` (an NSO) decompiles with Ghidra 12.1's AArch64 decompiler.
`research/tools/ghidra/nso2elf.py <main> <main.elf>` decompresses the
segments and writes an ELF with the module's dynamic section, so Ghidra
applies the relocations. Ghidra loads the ELF at 0x100000. Then:

```sh
research/tools/ghidra/ghidra_export.sh <main.elf> research/typemoon/local/ghidra \
  --names research/typemoon/names.tsv -- -postScript CodePointerFunctions.java
```

The export takes 5 minutes for 5,284 functions: 4 fail to decompile,
and none is left as an unresolved `func_0x…` call. Without
`CodePointerFunctions.java`, 1,420 functions are missing. The
addresses in `research/typemoon/names.tsv` are those of the 1.0.2
`main`.

- `script_command` (0x177790) runs one command. It switches on the
  first letter after `_`, then compares the next three letters packed
  into an int (`0x504c59` = `PLY`). That is why no command name
  appears as a string in the exe.
- `cmd_MPLY` (0x1649e0): `_MPLY(n, …)` reads the backquote options
  (`` `001:4000`` is option 1 with value 4000; option 1 sets the
  fade-in, as 1666.67 / value). It plays music `n` through
  `FUN_0019c470`, and for `n < 0x418` sets a heard-music flag bit
  `n + 0xC80`. Options: 1 fade-in, 11 a second fade time, 17 volume
  (default 100), 49 a flag passed on (default 1), 69 a start offset
  (in samples, to the sound library). An option's number is its three
  digits read as decimal. `n < 0` stops the music.
- **Music number → sound**: `FUN_0019c470` calls `FUN_001006f0(n)`,
  which reads a table of 132 u32 at 0x433F90: entry 0 is 0, entry `n`
  is `0x01000000 + n − 1`, the sound library's item ID of sound
  `n − 1` (item type 1 in the top byte). So `_MPLY(n)` plays the
  bfsar's sound `n − 1`: sounds 0–130 are the 131 music streams
  (`STRM_MOON01_BGM01` … `STRM_REONA_SEIMEISEN_APFARR_20191223`), and
  the scripts use `n` from 2 to 123.
- **Music loops** are in `knife1.bfsar`, not the exe: each stream
  sound's details (INFO, reference kind 0x2201) hold at +0x1C a
  reference (kind 0x2210) to 12 bytes: u8 stream type (3 = Opus),
  u8 loop flag, u8, u8, u32 loop start, u32 loop end, in samples at
  48 kHz. All 131 music streams have the loop flag set; 26 loop from
  0; every loop end is within the decoded length (49 within a second
  of it, the others leave a tail that never plays). Nothing in the
  start call (`FUN_001ee200`) overrides them.
- **The tables are read** by `typemoon/exe.rs` from the NSO itself
  (`typemoon/nso.rs`: the three segments, LZ4-unpacked, at their
  memory offsets; addresses below are Ghidra's, image at 0x100000).
  It checks for 1.0.2's layout (the base game's `main` is refused; the
  fan patch's reads the same). `conscripter-convert typemoon exe
  <exefs-102>/main [--romfs <game-102>]` prints the counts, and with
  `--romfs` checks every picture's size against its texture (see
  below).
- **The picture table**: 14,713 names of 32 bytes at 0x6E02A4 (an
  empty name ends it; the lookup's limit is 23,000): `BLACK`, `BLUE`,
  …, `WHITE`, `YELLOW` (plain colours), `IMG0000`…, the sprites
  `ARU_A_18_00_N00_02_02`…. A name's picture is `<name>.NXGZ` in
  `allpac`. Beside each name, a 0x48-byte record at 0x56F830 (all
  else 0, 0x20–0x27 0xFF):
  - f32 width at 0x2C and height at 0x30, in record units (1280 × 724
    is the screen, see "The stage"): half the texture's width and
    724/1440 of its height (IMG0024_01_01: texture 2576 × 1456, record 1288 × 732.04).
    6,141 names have a second, third-size copy in `allpac` too
    (IMG0006: 427 × 240 beside 2560 × 1440). `typemoon exe --romfs`
    (70 s): every expression (7,286, names ending `_a_b`) and 12,083
    pictures in all fit their full texture to half a stage pixel
    (tall sprites' heights are 0.2 short); 1,304 more to 3 px; 1,326,
    all bare sprite poses (`ARU_L_90_00_N00`: 590 × 878.5 for a
    1181 × 1711 texture), are further off, and so are the plain
    colours (640 × 362 for 1920 × 1080).
  - **A bare pose's record** is its expressions' box less the
    expression textures' padding: for 3,091 of the 3,092 bare names
    with expressions, some expression's record is the bare one grown by
    the same border on both sides (4 units a side for 2,111, 8 for 320,
    16 for 152, 12 for 108, some 4 across and 8 down), so both put the
    same point of the art at x, y. The odd one is `IMG1481` (3733 × 2514
    against 1879 × 1261). When `_STCH` swaps the expression on a
    layer, it moves the layer's centre by the difference of the two
    records' centres (`FUN_00160840`, the copy-from-layer branch).
  - f32 centre at 0x34 and 0x38, mostly half the size: `_STCH` puts
    it at its x, y (`FUN_00155d30` looks the name up).
  - byte 0x41: 3 for the plain colours, else 0 (bit 0: `_STCH` draws
    it opaque); byte 0x43: 1 for an expression; u16 at 0x46 a
    sprite's shot (`_A_` 0, `_L_` 1, `_M_` 2, `_S_` 3); u16 at 0x44
    per pose, not known.
- **The motion table**: `_STMA(layer, m)` starts motion `m`
  (1–24,192; `FUN_00197b80` runs it each frame). u32 at 0x4B1940 +
  4(m−1): one past its last keyframe (motion 1 starts at key 0;
  71,905 keys); u32 at 0x499F40 + 4(m−1): its length in ms (the last
  key's time, for all 24,191 with keys); u64 at 0x46AB38 + 8m: the
  fields it drives (a field is applied only under its bits: x 0x40880,
  y 0x5100000, alpha 0x4000, zoom 0x80200000/0x100400000, turn 0x88000,
  priority 1; 50-odd `_STMA` calls have keys for a field outside their
  mask); `FUN_001999a0(m)`: where it loops from (0: no loop; 1: the
  start; else a time in ms; motions over 9,859, a jump table: 2,037
  `_STMA` calls loop). Keyframes are 0x40 bytes at 0x75CE18
  (`FUN_00199ff0` reads field f): u32 time in ms, then s16 0x04 easing
  (`FUN_001961b0`'s set, as `FUN_00130c80`: 0, 1 straight, 2 quad in,
  3 quad out, 4 quad in-out, …, up to 41), 0x06 mode (2: each value
  holds until the next key, then jumps; 1: a curve, `FUN_001972c0`), 0x08
  x, 0x0A y, 0x0C alpha, 0x0E/0x10 (the centre?), an i32 at 0x14 the
  turn in 1/100° (as `_STRT`'s; −100001 unchanged, −100000 no key
  here), 0x18/0x1A zoom ‰ (the `_STZ4` zoom), 0x1C–0x22 (four shorts,
  1,920 × 1,080 values: a clip?), 0x24, 0x26, 0x28 and 0x2A (0x28,
  0x2A: shader and effect?), 0x2C–0x30 the effect colour (r, g, b), 0x32/0x34
  the expression, 0x36 (/255 + 1) and 0x38 (·0.01 + 1) to the state's
  0x1454/0x1458, 0x3A, 0x3C the priority (applied only to a layer whose
  priority isn't 0); fields 0x24 and 0x28–0x3C step (no in-between
  values). −32767 is "unchanged", −32766 "no key here":
  `FUN_00197710` (a field's value at a time) skips such keys, so the
  field runs straight on between the keys around it (5,926 fields: x
  1,015, y 1,258, alpha 1,229, zoom 2,360; 48 with no key after them
  hold the last value). The way to key j (its easing and mode) is set
  by the last key before j that sets it; before the first key's time
  the layer stays. The values are absolute (x, y in script units).
  Motion 229: zoom 584‰ → 566‰ and y 130 → 136 over 5 s. In the
  `_STMA` calls (24,333): 8,215 drive the turn, 10,801 ways to a key
  hold (mode 2), 18,775 keys ease (12,239 quad out, 2,881 quad in).
- **Scene numbers**: `_JUMP(n[,label])` goes to the script whose
  scene number is `n`, not by name: 551 s16 at 0x450D58, one per
  script in `allscr` order (`START_SCRIPT` 110, `00_00` 240,
  `01_00_ARC01_1` 300, …, `QA_01`–`QA_25` 1000–1024, `QACALL` 3100,
  `JUMP00`–`JUMPQA` 8001–8017), read by the `_JUMP` case of
  `script_command`. The label is a place inside that script; the same
  label names recur in many scripts.
- **The stage** (the draw list `FUN_001884f0`, the draw
  `FUN_00189ad0`): script coordinates are 480 × 272 for the whole
  screen, whatever its resolution. `_STCH` keeps x·640/1280 and
  y·480/720 (a 240 × 181.33 layer space); the draw turns it back,
  x_screen = W·x_layer/240 and y_screen = H·y_layer/181.33, so
  x_screen = W·x/480, y_screen = H·y/272 (a full-screen background
  sits at 240, 136, the middle). Screen shake is converted the same
  way, from 1920 × 1080 pixels to 480 × 272. A child layer's place is
  its parent's plus the parent's zoom times (its own x − the parent's
  centre·0.375, y − centre·0.5) (`FUN_001892b0`, `FUN_00189420`),
  and the draw turns a centre into 1920 × 1080 pixels as ·1920/640
  and ·1080/362: both work in 640 × 362 units, half the record stage
  below (not understood; the plain colours' records are 640 × 362).
  The scale passed to the
  draw is the zoom (`_STZ4` ‰ × `_STCH` argument 4 ‰) · W/320 and
  H/180. A child's offsets are scaled by its record's u16 at 0x46
  (`FUN_0012dfd0`: 0 → ×1, 1 → ×½, 2 → ×¼, 3 → ×⅛), the sprites'
  shot letter (`_A_`, `_L_`, `_M_`, `_S_`).
  **How big a picture is drawn: its record size on a 1280 × 724
  stage, the screen** (settled 2026-10-16 by the gallery). A
  background's whole texture fills the screen at zoom 1000: its
  record is 1280 × 724 (bare backgrounds' full art is a 2560 × 1440
  JPG in `allpac`, `IMG0020.JPG` beside its 427 × 240 `IMG0020.NXGZ`),
  and its centre (640, 362) lands on script (240, 136) × 8/3. Evidence:
  `research/typemoon/cg_framing.py` compares the gallery's CGs
  (`allpaccg`, 2560 × 1440, shown on the whole gallery screen) with
  the story pictures' thumbnails, whole and middle half. Of 235
  16:9 CGs, 147 match a story picture whole (correlation ≥ 0.95,
  most 0.997: IMG3005 is IMG0020, IMG3001 is IMG0005), none its
  middle half, 88 nothing. At full size IMG3005 against IMG0020.JPG:
  RMSE 0.085 whole, 0.33 for the middle half. So a texel is
  W/2560 of the screen for 2× art, and on screen x, y and sizes are
  script × 8/3 and record units on 1280 × 724. The plain colours
  (`BLACK` …, records 640 × 362, textures 1920 × 1080) are drawn to
  fill the screen; the converter can treat them as full-screen
  fills. The decompile alone didn't give it: `FUN_001dfa6c`/`88`
  return a texture's registered size (`FUN_001df790`; for pictures
  `FUN_001a3180`: the BRTI size ÷ 1 or ÷ 2/3); the quad (±size/2
  about a normalised pivot, `FUN_001ba89c`) and the scale
  (zoom · W/320) go to the vertex shaders as constants; those
  shaders (two BNSH in the exe at ELF file offsets 0xAC3000 and
  0xAD8000, with their NV assembly as text, `!!NVvp5.0`) only do
  projection × model × vertex. `shader/*.bnsh` are pixel shaders
  (Maxwell, readable with envytools' `envydis -m gm107`).
- **Picture layers**: 40 (0–39), staged by the commands below and shown
  by `_FADS`. Arguments are expressions (`FUN_00102d3c`); `FUN_00102b24`
  splits at most 20 of them.
  - `_STCH` (`FUN_00160840`): 0 layer; 1 picture (empty: clear the
    layer); 2 x and 3 y (kept as x·640/1280 and y·480/720; one left out
    keeps the layer's, or the copied layer's); 4 → the
    layer's 0x324, a zoom in ‰ (default 1000) multiplied with
    `_STZ4`'s at the draw; 5 alpha 0–255 (default 255); 6
    priority (draw order; default layer × 10, as `_STCP`); 7 and 8
    (`CC`) never read (see History, 2026-10-17); 9 and 10 the expression: unless both are 0,
    `_a_b` (two digits each) is added to the name; one left out is 0,
    or with argument 18 the copied layer's (read from its picture's
    name, `_xx_yy` at its end), even with neither given; 11 a parent layer
    (−1 none) whose clip and settings it copies; 12–15 four shorts
    (a clip rectangle?), 16 and 17 two more; 18 a layer to copy every
    setting from; 19 rotation in 1/100°.
  - `_STCC(n)` (`FUN_00162140`) clears layer `n` (no argument: 0,
    negative: all 40), its priority to 0 (so a `_SCH2` after it without
    argument 6 isn't drawn).
  - `_STCP(layer, p)` (`FUN_00162a40`) sets the layer's priority.
  - `_STZ4(layer, x0, y0, x1, y1, ms, e, f)` (`FUN_00162d20`): zoom in
    ‰ from (x0, y0) (default: the current zoom) to (x1, y1) over `ms`
    (turned into frames, ·60/1000); `e` is the easing (`FUN_00132030`,
    for `FUN_00130c80`) and `f` a per-layer byte that keeps the zoom
    off the picture's size (see History, 2026-10-17).
  - `_STMA(layer, m, mode, p)` (`FUN_00199620`): motion `m` (above;
    0 stops it), `p` a start parameter; `mode` 1 hides the text
    window until the next text (0x89E, the flag `_CLO2` sets and the
    next text clears, fading the window in over 18 frames); 2 hides it
    while the settings' byte 99 is 1 (its default, `FUN_0012c810`), none
    or −1 while it is 0; 0 never (117 uses of 2, all converted).
  - `_FADS(type, ms, rule, mode)` (inline in `script_command`, then
    `FUN_00160610`): shows the staged layers. Type 0 cross-fades over
    `ms` (12,240 uses); type 2 wipes through rule mask `rule` (947),
    the `rule`-th entry after `MASK_TOP.MZP` in `allpachd` (that is
    `RULE_<rule>`; `FUN_0014b4c0`); type 1 (20,842, always `ms` 0)
    keeps a running mask and returns at once. `mode` as in `_STMA`,
    2 and none only for a fade of 2 ms or more (53 uses of 2, 1 of 1).
- `_WNTY(n)` (`FUN_00138ea0`) picks the text window's type: 0 (42,820
  uses) or 1 (15,756); the text drawer (`FUN_001391a0`) offsets a
  line by 5.5 for type 1 and 3.5 otherwise (what that changes on
  screen is not worked out).
- `_WTTM(ms, …)` waits `ms`, turned into frames as ms·3/50.
- `_SEPL(channel, loop, sound, …)` (`FUN_0016a340`) plays a sound
  effect on its channel, in place of the one there; the same sound
  already looping there at a volume above 0 goes on (not started
  again, its options unread). The converter stops a loop on the
  channel before the next sound (83 replays of the same loop played it
  twice over).

## Script language

Read by `typemoon/script.rs`; `conscripter-convert typemoon script
<romfs> [-o <dir>]` ranks the commands, checks every text and sound
reference, and with `-o` writes each script as `<name>.txt`, one
command a line with its text after it (under a second).

- **`allscr.mrg`** (an MZP): entry 0 holds 549 script names
  (`START_SCRIPT`, `00_00`, `01_00_ARC01_1`, …, `JUMPQA`). Entry 1 is
  an MZP of 549 label tables, one per script, in CRLF text lines
  (`LBL_000              016,0002e`: the label, then two hex numbers,
  the second a byte offset into the script). Entry 2 is an MZP of two
  small tables (`0x80000000` words, and 0xFF fill), not read yet. Entry
  `3 + i` is script `i`, MZX-compressed.
- **A script** is one line of ASCII: `_NAME(arg,arg,…);` again and
  again. Text is not in it: `$000107` is line 107 of
  `script_text.mrg`. The `_Z?` commands carry a 5-hex-digit id
  (`_ZM01602`); the ranking strips it (`_ZM`). The original has one
  malformed command, `_STCP)21,0)` in `05_00_ARC05_9_2B`.
- **`script_text.mrg`**: pair 0 holds the game's 43,961 lines
  (Japanese; English in the patched copy, same numbering); the four
  other pairs are blank slots. Markup in the lines: ruby `<base|ruby>`
  (3,476 in Japanese, 140 in English), `@g`/`@b`/`@k`/`@t` codes at
  the start of a line (135), inline pictures `[ber00]` (52) and
  `[zap00]` (8), `\n` breaks inside a line. 42,016 lines are used by
  the Japanese scripts and 42,224 by the English ones.
- **Variables**: `F…`, `G…`, `C…` plus a number (`_WKST(F823,eq,0)`
  sets, `_WKAD` adds, `_IF__(F026,ne,0,LV00000)` tests).
- **Sound**: `_VPLY(<voice>,<hex id>)` names a `stream/` file
  (all 16,690 uses do). `_SEPL(channel, loop, <name>, …)` names a wave
  sound without its `WSD_` prefix, in any case (`MOON_SE03001`). In
  Japanese, 14,444 of 14,451 uses match one; the other 7 name
  `MIX_SE007`–`012`, which no copy has. In English, 14,512 of 14,543
  match; the 31 misses are `MIX_SE007`–`038`. Music: `_MPLY(52, …)`
  takes a number (104 distinct), not a name: the bfsar's sound
  `n − 1` (see "The exe"). Arguments after a backquote (`` `001:4000``,
  `` `017:50``) are options, number:value (time in ms, volume). Loop
  points are in the bfsar ("The exe", music loops).

Japanese: 549 scripts, 1,079,001 commands, 93 command names. English
(the patched copy): 549 scripts, 1,082,931 commands, 96 names.

| Command | Uses | Scripts | First use (Japanese) | What it looks like |
|---|---|---|---|---|
| `_WKST` | 486528 | 534 | `_WKST(F823,eq,0)` | set a variable (`F`, `G`, `C` + number; `eq` = assign) |
| `_STCC` | 62606 | 519 | `_STCC(0)` | clear a picture layer |
| `_WNTY` | 58596 | 521 | `_WNTY(0)` | text window type (0 or 1) |
| `_STCH` | 54334 | 521 | `_STCH(1,WHITE,240,136,1000,255,2,,CC)` | set a picture on a layer (layer, picture, x, y, …; "The exe") |
| `_STCP` | 53915 | 510 | `_STCP(1,1)` | layer priority |
| `_WKAD` | 45688 | 516 | `_WKAD(F823,1)` | add to a variable |
| `_ZM` | 42600 | 516 | `_ZM01602($000107)` | show text line `$n` (the `_Z?` commands carry a 5-hex-digit id) |
| `_FADS` | 34452 | 521 | `_FADS(1,0,0)` | show the staged layers (type, ms, rule) |
| `_STZ4` | 32305 | 512 | `_STZ4(4,984,984,984,984,0,0,0)` | zoom a layer (‰, from, to, ms) |
| `_STMA` | 24309 | 485 | `_STMA(3,1)` | start a motion from the exe's table |
| `_STRT` | 21829 | 516 | `_STRT(0,0,0,0,0)` |  |
| `_PGST` | 17202 | 519 | `_PGST(-1)` | the page a load replays to (left out, see History) |
| `_VPLY` | 16690 | 483 | `_VPLY(D10_00_00_0000_0,03fb5)` | play voice `stream/<name>.opus` |
| `_STBR` | 14988 | 460 | `_STBR(4,23)` |  |
| `_SEPL` | 14451 | 480 | `_SEPL(0,1,MOON_SE03001,,`001:4000,,`017:50)` | play sound effect (channel, loop, `WSD_` name less its prefix, options) |
| `_WTTM` | 14424 | 508 | `_WTTM(1,1)` | wait (ms) |
| `_STEC` | 12877 | 437 | `_STEC(12,48)` |  |
| `_CLO3` | 11627 | 504 | `_CLO3()` |  |
| `_SNX4` | 8787 | 435 | `_SNX4(4,11,11)` |  |
| `_STGS` | 7240 | 481 | `_STGS(0,0,0,0,0)` |  |
| `_STCF` | 5085 | 475 | `_STCF(0,8)` |  |
| `_SCH2` | 4800 | 425 | `_SCH2(7,img2343,,,,0,,,CC,,,,,,,,,,2)` |  |
| `_SEFD` | 3401 | 412 | `_SEFD(1,,,`001:2000)` | sound effect fade |
| `_FANX` | 2924 | 319 | `_FANX(1,0,0)` |  |
| `_ZZ` | 2569 | 549 | `_ZZ00a01()` | label (its name the argument) |
| `_JUMP` | 1766 | 544 | `_JUMP(240,START)` | jump to scene number n ("The exe"), at the label |
| `_SNX2` | 1654 | 249 | `_SNX2(35,27)` |  |
| `_SEVL` | 1353 | 210 | `_SEVL(0,,,,`001:3000,,`017:30)` | sound effect volume |
| `_ZY` | 1272 | 510 | `_ZY02100(LV00000)` | local label for `_IF__` (`LV…`) |
| `_SQK2` | 1228 | 236 | `_SQK2(-1,1,4,0,600,0,0,0,0)` |  |
| `_MSAD` | 1178 | 250 | `_MSAD($000120)` | add to the message log? |
| `_IF__` | 1164 | 507 | `_IF__(F026,ne,0,LV00000)` | if (variable, `eq`/`ne`/`lt`/`le`/`gt`/`ge`, value) holds, run on; else skip to the local label |
| `_MPLY` | 1158 | 452 | `_MPLY(52,,`017:100)` | play music by number |
| `_CLO4` | 957 | 294 | `_CLO4(1000)` |  |
| `_SBAS` | 933 | 121 | `_SBAS(14,13)` |  |
| `_SHOK` | 865 | 165 | `_SHOK(1,3,100,18)` |  |
| `_SNX3` | 842 | 166 | `_SNX3(21,32)` |  |
| `_SELR` | 823 | 100 | `_SELR(0,$042569)` | choice option (index, text line or a name written as it is, 1 = shown greyed and skipped by the cursor); `_GOTS` then jumps to the index-th label |
| `_MVOL` | 780 | 220 | `_MVOL(,,`001:5000,,`017:80)` | music volume |
| `_MFAD` | 724 | 364 | `_MFAD(,,`001:4000,,`011:0)` | music fade |
| `_STPV` | 712 | 173 | `_STPV(6,459,178,)` |  |
| `_CLO5` | 591 | 203 | `_CLO5(2000)` |  |
| `_STRA` | 584 | 84 | `_STRA(12,1)` |  |
| `_RDED` | 550 | 507 | `_RDED(486)` |  |
| `_RTM_` | 544 | 539 | `_RTM_()` | return |

The other 48: `_RDAS` 530; `_RDST` 530; `_RTST` 527; `_STRR` 447; `_ATSV` 398; `_SHAF` 397; `_SSW2` 205; `_SPMA` 195; `_STNS` 194; `_GOTS` 170; `_SELD` 170; `_SQS2` 132; `_SHK2` 120; `_GOTO` 101; `_SGMA` 100; `_SACL` 95; `_SOXY` 78; `_VCWT` 71; `_FADE` 62; `_IFNX` 53; `_IFP4` 52; `_TOMP` 48; `_TOMW` 48; `_SPRD` 41; `_SPRC` 40; `_STCG` 39; `_FCSP` 31; `_MSQK` 30; `_WTSV` 28; `_FCLN` 26; `_SELA` 26; `_STTR` 25; `_WKOR` 21; `_VPL2` 16; `_SESP` 15; `_STCL` 8; `_CLO2` 5; `_STQK` 5; `_SHS2` 3; `_IFSC` 2; `_TRPY` 2; `_XRSM` 2; `_XSTP` 2; `_IFUR` 1; `_RTM2` 1; `_RTMN` 1; `_SELS` 1; `_SSS2` 1.

**How the scripts differ, base → 1.0.2 → fan patch** (`typemoon
script -o` on each copy, the dumps compared without their text):

- **The official update 1.0.2** changes 521 of the 549 scripts and
  leaves 28 alone (1,082,420 commands). It adds `_END_()` at the end of
  519 scripts and a new command `_SPFC` (487 uses in 16 scripts). The
  largest other additions: `_WKST` +1,280, `_STGS` +1,004, `_STCC`
  +858, `_CLO3` +608, `_WTTM` +431, `_SQS2`/`_SHS2`/`_SSS2` about +400
  each. The largest removals: `_STCP` −2,396, `_WKST` −672, `_MSAD`
  −520, `_FADS` −477. Its text has the same 43,961 lines, of which it
  uses 41,467. It brings a new malformed command, `_STTR,0,26)` in
  `14_01A_CIEL14A_2`; the base's `_STCP)21,0)` is fixed.
- **The fan patch** changes 213 scripts of 1.0.2 and leaves 336 alone.
  Every script keeps the same text references in the same order. It
  turns 303 `_ZM` into `_MSAD` (+814 `_MSAD` in all), swaps 238 `_WKAD`
  for `_WTTM` (+236), fixes the `_STTR` typo, and adds an empty 553rd
  entry and `_NOP_`.

## Story structure and walkthrough

### Walkthrough

Sources: the scripts of 1.0.2 (the English copy's control flow is the
same), read with `research/typemoon/routes.py`, which prints every
script's choices with the conditions on each option, its flag tests
and where its jumps go; checked against NookGaming's walkthrough of
the English release
(<https://nookgaming.com/tsukihime-a-piece-of-blue-glass-moon-walkthrough-guide/>),
whose choices and dead ends match the scripts'.

```sh
conscripter-convert typemoon script <game-102> -o <dump>
conscripter-convert typemoon extract <game-102>/allscr.mrg -o <raw>
python3 -I research/typemoon/routes.py <dump> <raw>/00000.bin <main-102.elf>
```

**Layout.** `START_SCRIPT` → `00_00` (the prologue), then one script
group per day: `NN_00_ARCNN_*` (Arcueid's route, 「月姫」, days 1–14)
and `NN_01_CIELNN_*` (Ciel's, 「夜の虹」, days 1–14, then
`15_01A`/`15_01B`). Days 1–3 are common; the scenes jump between the
two groups by choice, and scenes both routes pass through branch on
`F058` (1 once Ciel's route is taken). 82 story choices. `JUMP00`–`JUMP15`, `ARCHIVE` and
`JUMPQA` are the chapter select (they jump to the scene number in
`G008`); `*_FLAG0`/`*_FLAG1` set `F069` and jump on, as entries for
it.

**The first play is Arcueid's route.** Ciel's route opens on day 3,
and its option is greyed until Arcueid's ending is seen (`G003`):

1. Ciel's points, day 1–2 (`F050` → `F075` → `F076` → `F077`, each
   copied into the next and added to): day 1, 「ちょっとした疑問がわいて
   しまった。」 or 「せっかくなのでじっくり観賞するしかなかった。」 (1);
   lunch, 「廊下に出てから考えよう。」 (+1, only with the first);
   after school, 「学校をぶらつく。」 (+1 when the first was taken;
   sets `F054`); day 2 lunch, 「とりあえず、廊下に出て考えよう。」 (+1,
   only with `F054`).
2. Day 3 lunch (`03_00_ARC03_4`): with all four (`F077` ≥ 4) a third
   option appears, 「茶道室に行ってみようか……？」, which sets `F057`.
3. Day 3 night (`03_00_ARC03_5B`): 「……いや、俺の起こした現実だ。」
   is greyed unless `F057` = 1 and `G003` = 1. Taking it goes to
   `03_01_CIEL03_3_2`; day 4 (`04_01_CIEL04_1`) sets `F058`.

Day 7's 「……二十七祖ってなんだ？」 (`07_00_ARC07_3_1`) is also greyed
until `G003`.

**Endings.**

| Ending | Last scenes | Sets | Then |
|---|---|---|---|
| Arcueid (the only one of her route) | `13_00_ARC13_6` → `14_00_ARC14_1` | `G003` | lesson `QA_12` |
| Ciel, normal | day 14 `14_01_CIEL14_1C`: 「……アルクェイドには従えない。」 → `14_01A_*` → `15_01A_CIEL15A_1` | `G004` | `QA_23` |
| Ciel, extra | the same choice: 「……アルクェイドを、嫌いにはなれない。」 → `14_01B_*` → `15_01B_CIEL15B_1` | | `QA_24` |

The extra ending's option is greyed unless `G004` = 1 and `F081` ≥ 5
(the walkthrough: save at the start of day 14, see the normal ending,
load). `F081` is the Arcueid points of Ciel's route (`F051` →
`F078` → `F079` → `F080` → `F081`); there it gets +1 on day 4 for
「ホテルに戻ろうと思う。」, +2 on day 8 for 「アルクェイドの方が、ほっと
けない。」, +1 on day 9 always (`09_01_CIEL09_4_1`) and +1 on day 9
for 「『それ以上進んだら嫌いになるからなっ！』」: 5 needs all of them.
At `F081` ≥ 7 two lines of `14_01B_03_CIEL14B_5B` differ; whether
that can be reached isn't checked. `QA_25` is reached only from the
gallery's `JUMPQA` (the walkthrough: Gallery > Ending List, the icon at
the bottom right).

**Dead ends.** 21, each followed by a 「教えて！シエル先生」 lesson
(`_JUMP(3100,QA_nn)` into `QACALL`). The walkthrough's dead end n is
`QA_n` up to 11 and `QA_(n+1)` from 12.

| Lesson | Day | Scene | Reached by |
|---|---|---|---|
| `QA_01` | 4 | `04_00_ARC04_6_1` | 「……酔狂はここまでだ。逃げよう。」, then 「屋敷に戻ろうと思う。」 |
| `QA_02` | 4 | `04_00_ARC04_9_2` | 「……部屋で外の様子をうかがう。」 |
| `QA_03` | 5 | `05_00_ARC05_3_1` | 「……今すぐヴローヴを捜すべきだ。」 with `F058` = 0 (Arcueid's route) |
| `QA_04` | 5 | `05_00_ARC05_6_1` | 「下りていく。」 |
| `QA_05` | 5 | `05_00_ARC05_9_1` | 「……投擲だ。」 |
| `QA_06` | 5 | `05_00_ARC05_9_3` | 「……牽引だ。」 |
| `QA_07` | 8 | `08_00_ARC08_7_2` | 「……嘘をついて誤魔化す。」 (`F060` = 0; 「……正直に話す。」 sets it) |
| `QA_08` | 9 | `09_00_ARC09_3_2_1` | 「……槙久の書斎を調べてみる。」 |
| `QA_09` | 9 | `10_00_ARC10LIVING` | 「アルクェイドに会いに行く。」 |
| `QA_10` | 10 | `10_00_ARC10_7_3` | 「……学校に戻っているかも？」 |
| `QA_11` | 10 | `10_00_ARC10_8_1` | 「ただ、欲しいだけなんだ。」 |
| `QA_13` | 5 | `05_01_CIEL05_2_1` | 「―――このまま押し切る！」 |
| `QA_14` | 5 | `05_01_CIEL05_2_2_1` | 「先輩に駆け寄る。」 |
| `QA_15` | 10 | `10_01_CIEL10_5_2` | 「自分は足手まといだ。ここで待っていよう。」 |
| `QA_16` | 11 | `11_01_CIEL11_2_2` | day 9's 「じゃあ、痛くて効かないものを。」 (`F061`) |
| `QA_17` | 13 | `13_01_01_CIEL13_3_2` | 「後方に逃げる。」 |
| `QA_18` | 13 | `13_01_01_CIEL13_4_1` | 「……眼鏡を外す。」; forced when 「……眼鏡は外さない。」 is greyed: `F069` = 1, set on day 12 when day 9's 「……因果応報だ。女を手本にして処理しよう。」 (`F063`) was taken |
| `QA_19` | 13 | `13_01_02_CIEL13_5_1` | 「最短で、あの吸血鬼を殺す。」 |
| `QA_20` | 13 | `13_01_02_CIEL13_6_1` | 「面倒だ。殺してしまえ。」 |
| `QA_21` | 14 (extra) | `14_01B_02_CIEL14B_3_1` | 「諦めない。」 |
| `QA_22` | 14 (extra) | `14_01B_03_CIEL14B_4_2` | 「もう一度上へ。何か見落としがある筈だ。」 |

Options shown only under a condition, besides those above: day 2's
「ファミレスに行ってみる。」 (`F056`, set by 「パンを買って教室で済ませよう。」)
in place of 「寄り道をして帰る。」; day 10's questions to Ciel
(`10_01_CIEL10_3_2`), each gone once asked (`F065`–`F068`).

## Progression variables

The route gates (above); the many `G0nn |= 2` are gallery unlocks,
not mapped yet. `G…` are presumably system data and `F…` save data
(not checked).

| Variable | Meaning | Set | Read |
|---|---|---|---|
| `G003` | Arcueid's ending seen | `14_00_ARC14_1` | day 3 Ciel option, day 7 option |
| `G004` | Ciel's normal ending seen | `15_01A_CIEL15A_1` | day 14 extra option |
| `F050`, `F075`–`F077` | Ciel's points, days 1–2 | see the walkthrough | `F077` ≥ 4 on day 3 |
| `F054` | took day 1's 「学校をぶらつく。」 into `01_01_CIEL01_3_3` | there | day 2 lunch |
| `F057` | went to the tea room on day 3 | `03_01_CIEL03_2_3` | day 3 night |
| `F058` | on Ciel's route | `04_01_CIEL04_1`, `04_01_CIEL04_1C` | shared scenes |
| `F051`, `F078`–`F081` | Arcueid points | see "Endings" | `F081` ≥ 5 on day 14 |
| `F060` | told the truth on day 8 | `08_00_ARC08_5_1` | `08_00_ARC08_7` |
| `F061` | chose 「痛くて効かないもの」 on day 9 | `09_01_CIEL09_1_2` | `11_01_CIEL11_1D` |
| `F063` | chose 「因果応報」 on day 9 | `09_01_CIEL09_3_1` | `10_01_CIEL10_8`, `12_01_CIEL12_1`, day 13 |
| `F069` | day 12–13 variant (from `F063`) | `12_01_CIEL12_1_A`/`_B`, the `*_FLAG*` entries | day 12–13 scenes, the glasses choice |
| `G008` | scene number for the chapter select | `JUMP*`, `QACALL` | `JUMP*` |
| `_RDST(n)` flags (global %1200 + n) | scene n read (`FUN_00192630`, n < 0x240); 560, 383, 562 choose the title's movie | each scene's start | the title |

## Testing

Nothing yet.

## Status

| Area | State |
|---|---|
| Getting the files out of the NSP | ✅ |
| Engine identified | ✅ (TYPE-MOON, `mrgd00`/MZP) |
| Survey of existing tools | ✅ (see "Existing tools") |
| Archives (`hed`/`mrg`/`nam`, MZP, MZX, NXGX/NXCX) | ✅ every entry of both RomFS copies |
| Pictures (BNTX) | ✅ every texture of both RomFS copies decodes |
| Sound (`stream/*.opus`, `knife1.bfsar`) | ✅ every sound of both RomFS copies converts, with its loop (music and wave sounds) |
| Exe (`main`) in Ghidra | ✅ decompiles; dispatcher, the top commands' handlers, the music, picture and motion tables named |
| Exe tables (pictures, motions, scene numbers) | ✅ read from the NSO (`typemoon exe`); picture sizes checked against the textures |
| Scripts | 🔶 read: every command of both languages splits (one malformed in the original) and its text and voices join; the top commands' arguments read from the exe, the rest not yet |
| Media folder (`typemoon media`) | ✅ `allpac`'s pictures at full size (PNG, or WebP with the shared `--webp` flags), the movies as they are, the sound (WAV wave sounds, or Opus/Vorbis with the shared `--audio-*` flags), loops kept in every mode |
| Script conversion (`typemoon convert`) | 🔶 the flow (labels, `_JUMP`, `_IF__`, flags), text (pages, ruby, voices; the fan patch's styled letters, private-use U+E000–U+E2FE drawn by its own `ja/TT_TsukuMin-D.ttf`, as ASCII: italic and serif italic as italic (`!i1`), mirrored and upside-down plain (mirrored lines, stored right to left, put back in reading order), U+E200 as `^-35`; its accents (é ï ç â à ê, 46 uses) and em dashes kept, as the `0.txt` is UTF-8; a `^` break in a choice's option as a space), pictures (`_STCH`/`_STCC`/`_FADS`: record size and place, fades and rule wipes), sound effects and music; plays from the start through `00_00` into the first arc headless. Layers drawn in priority order (`_STCP`, `_STCH` argument 6), changed keeping their settings (`_SCH2`) or copying another layer's (argument 18), zoomed (`_STZ4`), faded (`_STGS`) and moved (`_STMA`, the exe's motions, a −32766 field interpolated across its key, turned by its turn keys, held where a key's mode 2 says, only the fields its mask drives, each key eased as the exe's (`spcurve` tables of its Penner set), looping where the exe loops until `_STMA(n, 0)`, `_STCH` or another motion stops it; curves, priority and expression keys not followed), children placed in their parent layers, orbiting with their turn and following their motions (`_STCH` argument 11), clipped to their panels with their soft edges (`_STCH` arguments 12–17, CONscripter's `spclip` feather), turned (`_STRT`), drawn with their shader as far as CONscripter's blends and filters go (`_STBR`) and drawn with their `_STEC` effect (levels, greyscale and colour layers; effect 1 a colour fill, CONscripter's `spfilter` 9 and 10). Choices (`_SELR`, `_SELD`, `_GOTS`; options under a flag offered by it, greyed ones shown greyed and not selectable (`seldisable`); the chapter-jump menus), `_GOTO` and the Switch's own branches (`_IFNX`/`_IFP4`). The text window hidden and faded (`_CLO2`, `_CLO3`/`_CLO4`/`_CLO5` with CONscripter's `windowfade`, the script going on; the next text fades it back in). Not yet: the other layer commands, screens |
| Screens, extras | 🔶 the title; the system menu's Save and Load tabs (see History, 2026-10-17), opened in game by a right click at a text wait (on its Save tab); Config (for now the language switch), FlowChart, the backlog not yet |
| Languages (`--english`) | 🔶 one `0.txt` with both copies (below); the switch is the system menu's **Config** tab for now (in game; at rest on the title), until the settings tab with its 日本語/English option is converted |

#### Languages

With `--english <game-en>` the build holds both copies of the
scripts (`language.rs`): the Japanese as before (labels `tm_<script>…`)
and the fan patch's (`tm_en_<script>…`, `pic/en/` for its own 155
pictures, the English text layout). The flags keep the Japanese
numbering (none is the English copy's alone). The language is the
copy the script runs in, so a save keeps it; the engine's `language`
is set at a new game's start, on each switch and after each load
(`loadgosub`). A new game starts in the language last chosen (global
%1800).

The switch goes into the other copy at once:

- At the same page when the script's ops up to it are the same in both
  copies but for the text (screen, sound and flags then are too): 29,197
  of 42,532 Japanese pages; 330 of the 515 scripts with text line up to
  the end. The
  rest differ from some point on (the fan patch splits and joins pages
  with `_MSAD`, counts them in `F823`, and has its own waits).
- Else where the scene was entered (a script's start or a label another
  script's `_JUMP` goes to, 1,088 of them), put back as it was there:
  each entry saves the game into slot 897, and the switch loads it,
  then goes on in the other copy. After a load from the save screen
  that slot is of another visit, so the entry is played again from a
  cleared screen (sprites, music, every sound channel) with the flags
  as they are (a flag the scene added to before the switch is added to
  twice).

Checked headless (trace, `--goto conv_title_start`, the switch
reached by a patched `0.txt` that `return *tm_switch`s from a `gosub`
at a page's wait as the menu does from the right click's `rgosub`):
Japanese page 5 of `00_00` → English page 5 (same sprites and sound);
the same with the page forced unaligned → slot 897 loaded, English from
`00_00`'s `START`; %1800 = 1 → a new game in English; an English save
loaded → `language "en"`, English pages; there a switch at an unaligned
page → the reset path, Japanese from the entry, the old loops stopped.
The Japanese-only build (no `--english`) is unchanged but for the right
click.


### Next

1. The script conversion, next part: the layer commands the opening
   leans on, in order of use. `typemoon convert --dry-run` prints the
   counts. Splitting sprites into body and face crops is out unless
   the user asks.
2. The other picture archives in the media folder: `allpaccg` (the
   gallery's CGs, 2 GB), `allui` (71 MB).
3. The menus: the settings tab (the defaults in "Accuracy", the
   日本語/English switch: move `SystemPage::Jump(language::SWITCH)`
   from the Config tab to its option), the backlog; then the save list's route picture and chapter labels
   (by the phase), and Config and Gallery on the title.
4. Still open in the exe: the motion keyframes' other fields.
5. The metrics (`tests/snapshots/tsukihime.metrics.json`) count each
   argument a converted command passes and doesn't read as a gap
   (`<command> argument <i> ignored`, `option <n>` for a backquote
   option); the list is in that file. The biggest: `_STZ4`'s 2 timed uses (and 2 on
   layers with no picture), `_WTTM` 1 (with it, the wait less
   `DAT_00be0a00`, a frame count `FUN_00181b30` adds to; when it is
   reset not found), `_SEPL` option 17.

### Accuracy

The converter's hand-typed values carry a provenance comment (`src:`,
`guess:`, `ours`). Open guesses, each settled by one frame of the
original (Japanese, default text size):

- The text window's line pitch (45 px: 28 of the drawer's 448, the step
  it uses for its extra lines; the exe measures a line of the font) and
  so its 14 rows, and whether the glyph sits 16 px below its line's
  y (`FUN_001a7790` adds 14 + 10 of 1080 inside the line). Place,
  width and glyph size are the exe's (`FUN_001391a0`: from 88, 65 on
  a 640 × 448 stage, 464 wide; glyphs of the screen width / 32 × 0.76).
- The text speed: 35 ms a character, the exe's default for unread
  text (the settings' defaults at 0x56b080, the same as the reset in
  `FUN_0012c810`: +0xe unread 35 ms, +0x10 read 15 ms; `FUN_0013b2d0`
  shows 16.67 / ms characters a frame, all at once at 0). Read text is
  typed at the same 35 ms here (CONscripter has one speed); the other
  defaults (volumes +4/+8/+0xa = 40, 40, 10; auto wait +0x1c = 2200)
  wait for a settings screen.
- `_STBR` 11 (gauss) drawn as a blur of 8.
- Zooms are whole percents (`spzoom`): one under 10 % is coarse
  (`spzoom 95,5,9` may be 4.5–5.5 %), and one under 1 % is placed at
  1 %, not 0, with a motion's keys scaled from what was placed (a line
  drawn at 1‰ across that a motion opens to 1933‰, `05_00_ARC05INTERLUDE`).
  Exact would need a finer `spzoom` in the engine.

### Open questions

The biggest commands not converted, looked up in `script_command`
(0x177790):

- `_SNX4(…)` (8,762), **seen**: `script_command` has no case for it
  (`_SNX2` and `_SNX3` have), so the Switch exe does nothing. Left
  out.

The mapped commands, argument by argument and branch by branch, are in
`docs/games/tsukihime.findings.tsv` (seen, inferred or unknown, and the
cheapest check for each; redone 2026-10-17 against docs/new-game.md's
steps 6–17: 67 seen, 16 inferred, 6 unknown). Open there, by size:

- A motion field set on keys apart, with keys between that pass it
  through (−32766, 5,926 fields): the exe eases it once across them, the
  converter per key (its value at the key between straight).
- Motion keys' priority (0x3C, 23,115 calls) and expression (0x32/0x34,
  5,254), and the stepped fields 0x24–0x2A (shader, effect and colour
  changes mid-motion?).
- A loop whose layer goes to another IR layer at a `_FADS` (layers
  drawn below it come or go: 1,916 times in the Japanese build) or whose
  IR layers change (324) starts again from its start; a stopped loop
  goes back to its place where the exe leaves it as it is (`spmotion n`
  drops the offset).
- `_SEPL`/`_MPLY` volume (option 17) and `_SEVL` (not converted, 1,285):
  every sound at full volume.
- Zooms under 10 % are coarse (`spzoom` takes whole percents).

## History

- 2026-10-17: looping motions, **seen**: `FUN_001999a0(m)` (a switch
  compiled to jump tables, so `exe.rs` runs it: `run_switch`, a few
  AArch64 instructions) gives where motion m loops from: 0 not at all
  (22,165), 1 the start (1,805), else a time in ms (222; 206 at a key).
  `FUN_00197b80` wraps the time past the length back to it. The
  converter writes `spmotion` loop 1, or 4 from the key nearest that
  time. The loop stops (`FUN_00199620`, `FUN_00160840`): `_STMA(n, 0)`
  (4 of its 92 uses stop a loop; the rest stop motions that have
  ended), `_STCH`/`_SCH2` on the layer (at once; with argument 18 the
  copy's loop goes on, here started again), another motion, the layer
  leaving the screen. Japanese build: 4,211 looping `spmotion`s (with
  the children's and the restarts), 4,057 stops. Checked: tests, the
  dry run, a trace from `tm_start_script`.

- 2026-10-17: motion easing, **seen** (`FUN_00130c80` = `FUN_001961b0`,
  `FUN_00197710`): a key's easing 0x04 (2–41: quad, cubic, quart, quint,
  sine, circ, expo, elastic, back, bounce, each in, out, in-out,
  out-in; Penner's, with the exe's constants: expo's 1.001, elastic's
  periods 0.3/0.45, back's 1.70158) shapes the way from it to the next
  key, until a key sets another; 0, 1 and the rest are straight. Each
  `MotionKey` carries its curve (the shared IR change, #364) and the
  emitter writes an `spcurve` table per curve used (31 in the Japanese
  build): of 87,146 `spmotion` keys, 19,524 are eased (13,065 quad
  out). `_STGS`'s easing (argument 4, 27 uses) the same. Checked: the
  curves against the decompile's formulas (`ease.rs` tests), the dry
  run, a trace from `tm_start_script` (3,097 pages, no script error,
  the same findings as before).

- 2026-10-17: `_STEC` exact, **seen** in
  `nexl_nxshader_pre_alter.bnsh`: the sampled texel (the sRGB texture,
  decoded by the sampler) is raised to 0x3EE8BA2E = 0.454545 and × 255
  before the levels (`lg2`, `ex2`), the output only clamped and ÷ 255;
  mode 3 (effect 82) keeps G and B at 0 where the texel's are. Converted
  with CONscripter's `spfilter` kinds 12–14 (engine PR 123: the levels
  on 255·srgb⁻¹(v/255)^(1/2.2) in float, one rounding; kind 13 the grey
  of those; 14 the zero rule): every effect 2–82 now decodes first, even
  with identity levels (an identity effect lifts 10 to 18, 30 to 35).
  Checked in the engine on test ramps against the formula in float64:
  exact, but for 30 of 7,680 grey pixels a level off at a .5 boundary.
- 2026-10-17: coloured panel frames (`_STCH` argument 17 1–6, 263
  uses), **seen** in `FUN_00189ad0`: the picture is cut at the frame's
  inner edge (the scissor), then the corner and edge pieces are drawn
  over it in the colour (alpha, r, g, b to `FUN_001b03e0`: 1 and the
  default black, 2 white, 3 red, 4 0.18, 0.07, 0, 5 0.97, 0.96, 0.91,
  6 1, 0.91, 0.82; case 1's r and g are a register Ghidra shows as a
  handle, read as 0): `alphagradation` from the rectangle's edge, then
  `_inv` one frame in; the scissor keeps only the `_inv` part, the
  colour fading from the cut inward over the frame width. Converted
  with CONscripter's `spclip` feather from a colour (engine PR 122):
  1,119 `spclip`s of the Japanese `0.txt`. Checked on page 14 from
  `tm_01_00_ARC01_1__LV00000`: the panel's right edge at x 664 goes
  black over 20 px (x 655: 57 of 149 without the frame; 3t² − 2t³ of
  8.5/20 gives 58).
- 2026-10-17: panel frames and `_FADS` type 1, **seen**.
  `FUN_00189ad0` picks a frame piece size by the frame width (≥ 10,
  32, 40, 64, 128, 256 px: tables 0x4528E0, 0x75BE00 (each size's
  3 × 3 cuts), textures 338–343, which `FUN_0013b9a0` loads from
  `allui`'s `BORDER_PARTS.NXGZ`: `alphagradation020`–`512`, then
  `_inv` and `_2`). With no colour (argument 17 0) it draws the layer's
  mask in blend 0 (replace): the inner rectangle, then the corner and
  edge pieces, so the picture fades in from the rectangle's edge over
  the frame width (the ramp is near 3t² − 2t³: 0.16, 0.52, 0.87 at a
  quarter, half, three quarters). Converted with CONscripter's new
  `spclip` feather (engine PR 118): 6,846 `spclip`s of the Japanese
  `0.txt` are feathered, and an axis as wide as the screen gets no
  frame, as the exe's. With a colour (1–6, 263 uses) the pieces
  (normal, then `_inv` one frame in) are drawn in that colour (1 and 7
  black, 2 white, 3–6 tints): not drawn yet; the picture is cut at the
  frame's inner edge as before. `_FADS` type 1 (20,842 uses, always
  0 ms and rule 0) is a cut that switches the rule mask off
  (`FUN_0014b4c0(-1)`) and goes on at once (`FUN_00160610` returns
  −1), as converted; its mode 0 (417 uses) sets nothing.

- 2026-10-17: **the system menu's Save and Load tabs**
  (`typemoon/menu.rs`, the shared `SystemMenu`), from the exe:
  `FUN_0014c370` draws the bar (`menu_window` at (18.67, 1), the
  buttons Save, Load, Config, FlowChart, Title at (26.67, 43.56 + 31.11
  a row), hit box 74 × 29 at table 0x691438; `system_titlebg`,
  `system_scenebg` at (250.67, 0), `underline` at (0, 29.87)),
  `FUN_0014f240` the save list (`config_innerbg` at (120, 0); 4 rows at
  (133.67, 36.92 + 93.75 a row) of `save_win`'s (32, 32)-(992, 304), and
  (31, 336)-(991, 608) for the row chosen; the thumbnail at (151, row +
  23.23); the route panel at (453.33, 39.41)), all on the 640 × 448
  stage. 100 slots, 25 pages turned by the wheel and Left/Right; a
  click saves (Save only in game) or loads, asking first over a save;
  Title goes back to the title. The title's Load opens the Load tab.
  Ours: `menu_btn`'s cells cut 240 × 104 from (24, 26), centred on the
  hit box. Guesses: the game dimmed under the menu, the thumbnail 192 ×
  108, the date and the slot's last line in the font where the picture
  digits and the slot text are, the fade 200 ms. Left out: the header's
  chapter title and scene name, the cursor's line and light, the route
  picture by the save's phase (the unknown route's is drawn), each
  save's chapter label, the 39 autosaves. Config and FlowChart are shown
  at rest. Engine: CONscripter #117 (a tab reached from a held tab
  keeps the game as its thumbnail), #119 (`hover.under`).

- 2026-10-17: **the title** (`typemoon/title.rs`), from the exe:
  `FUN_0018d2e0` sets it up, `FUN_0018e200` runs it, `FUN_0018d5d0`
  draws it, the parts from `allui`'s `TITLE_PARTS` (part ids 0x138–0x143
  map to its textures by the tables at 0x75BFC8, one per movie). The
  backdrop is one of four looping movies (table 0x50B890), chosen in
  `FUN_0018d1c0` by the scene-read flags `_RDST` sets: `titlebg2` after
  560 (Arcueid's ending), `titlebg3` after 383 too, `titlebg4` after 562
  too (with music `_MPLY 72` in place of 2). The logo at (340, 36.27),
  the copyright at (436, 507.73) and an added light pulsing over 4 s at
  (360, 278.04) on a 960 × 544 stage; the menu on a 640 × 448 stage at
  x 272, rows 23.23 apart from y 298.67 with Continue (shown only with a
  save to resume), 312.36 without (table 0x452AF4). Sounds
  (`FUN_001acef0`, table 0x4341A0): `WSD_MOON_SE00001` on a move,
  `SE00002` on a choice, `SE00011` for New Game, which fades the music
  over 1.5 s and the screen to white. The fade in: 1,389 ms (0.012 a
  frame). Ours: the menu strip's cells cut 312 × 88 from (24, 22) so
  the widget's hit box (table 0x75CBD8) centres on each label; New Game
  and Load at rest, chosen and lit (the strip's three columns) as normal,
  hover and pressed. Left out: the boot logo and caution screens, the
  "PRESS ANY BUTTON" step, the logo's particles, its three bands and
  the menu's rows fading in one after another, and the selection line
  (`titlecursor`). Movies are no longer packed into `arc.nsa`
  (CONscripter plays them from loose files only). The registry and GUI
  list the game.
- 2026-10-17: numbers with a `.` (`_STCH(19,img1729,…,242.25,…)`, 24
  `_STCH` and with them 12 `_STZ4` and 4 `_STRT` on their layers),
  **seen**: the expression reader (`FUN_00103160`) takes the digits
  with `strtol` and stops at the `.`, so 242.25 is 242. `_VPLY`'s
  argument 1 (`_VPLY(B50_01_01_0007_0,03978)`, 16,690 uses) is never
  read (`script_command`, case "PLY": the name, then argument 2 when
  there are three). `_SELS` (1 use, `QACALL`) is a two-slot choice
  with no texts (`FUN_00192800`: slots 0 and 1, the select screen in
  mode 2, `FUN_00183b20`); its look is the select screen's, left for
  the screens.

- 2026-10-17: media flags as the other games' (the user's choice): no
  re-encode by default. Pictures PNG, `--webp`/`--webp-quality`
  WebP (was lossy q85 always); the `RULE_*` masks always PNG, exact.
  Movies remuxed at 1080p (were re-encoded at 720p, 182 MB; the
  title background `titlebg2` is 150 MB as is, 2.3 MB at 720p CRF 23).
  Wave sounds WAV with `smpl`, `--audio-codec opus` / `--audio-quality`
  re-encode with loop tags. Vorbis loops checked as Opus: CONscripter
  reads its `LOOPSTART`/`LOOPEND` comments (`soundLoop`); a test pulse
  decodes at shift 0; the disk-audio `dwaveloop` of
  `WSD_mix_se032.ogg` repeats exactly every 10,603 frames; one of the
  196 looping files (`WSD_moon_se02067`, a loop from 0) decodes 128
  frames short of its end, which the engine then loops whole.
- 2026-10-17: `_SCH2` (4,674 uses) and `_STCH` argument 18, **seen**:
  both are `FUN_00160840` (0x160840), `_SCH2` with its first parameter
  0, which keeps the layer's alpha (0 on a layer with no picture),
  priority and turn where `_STCH` sets 255, layer × 10 and 0, and
  skips the live layer's reset (`FUN_00131f00`) and the tint's.
  Argument 18 first copies another layer's settings (place 0xa4/0x144,
  alpha 0x1e4, priority 0x234, turn 0x284/0xbe4, zoom 0x324, `_STZ4`'s
  0x7d4/0x824/0x874, parent 0x89c, clip 0xa2c, tint 0x16da, motion
  0xc34), and every layer whose parent was that layer gets this one.
  The scripts change a character's expression this way (`_SCH2(19,
  ari_…,…,16)`, then `_STCP(16,0)` and a cross-fade). The 2,418
  `_STZ4` and 330 `_STRT` "on a layer not staged" were these layers:
  14 and 5 are left. The motion isn't copied (the converter's runs at
  its `_FADS`).

- 2026-10-17: `_STRT(layer, from, to, ms, ease, cx, cy)`
  (`FUN_00162b20`), **seen**: `FUN_001326e0` sets the live layer's turn
  to `to` with `from − to` left to run off over ms · 60 / 1000 frames
  (none at 0 frames), `FUN_001325d0` the easing; `cx`, `cy` go to the
  live layer's 0x72, 0x74. All 21,691 uses are untimed with from = to,
  so `from`, the time and the easing are read as meaning nothing
  (21,356 now fully converted; 0.txt unchanged). The 335 left are on
  layers the converter hasn't staged (see `_STZ4`).

- 2026-10-17: the chapter-jump menus and options under a flag,
  **seen** in `script_command` (0x177790, case "ELR"): `_SELR` adds a
  slot in the order it runs, keeping its index (for `_GOTS`'s label)
  and its text as written: a `$` line, else the name itself
  (`JUMP00`–`JUMPQA`'s `SEL_JUMPn` menus list script names; only
  `TEST_SCRIPT`, the debug menu, goes there; `ARCHIVE` uses their
  `ARCHIVEJUMP` part to go to a scene by `G008`). Argument 2 = 1 marks
  the slot greyed, and the cursor skips it (`FUN_00183e50`), so it
  can't be picked. Eight choices have options under an `_IF__` (or
  greyed under one flag, shown under another): each such option sets
  a bit of a local the choice reads, and the choice is one `select`
  per set of them; a greyed option is left out, as `select` has no
  disabled option. `_SELS` (1 use, `QACALL`'s yes/no) is not converted.
- 2026-10-17: greyed options (`_SELR(i, text, 1)`) are shown greyed
  and can't be picked, with CONscripter's `seldisable` (engine #116:
  the off colour at half alpha, no mouse area, the cursor keys pass
  over them). A greyed option sets a second bit (16 up) of the
  choice's local when run under an `_IF__`; an option offered anywhere
  is offered (offered wins over greyed). Four choices show one
  (day 3's Ciel route, 「……二十七祖ってなんだ？」, 「……眼鏡は外さない。」,
  「……アルクェイドを、嫌いにはなれない。」), each in the `select` for
  the flags that grey it.
- 2026-10-17: the 1,300 wave sounds as Ogg Opus (`se/<name>.ogg`,
  ffmpeg's libopus at 48 kbit/s a channel, VBR): 1,395 MB as WAV, 83 MB
  as Opus. Their loops go in as `LOOPSTART`/`LOOPEND` tags (start, and
  the FWAV's end, the sample count). Checked: each decoded file is as
  long as its wave (its `LOOPEND`), libopus's delay is undone (a test
  pulse decodes at shift 0), and under SDL's disk audio driver the
  engine's `dwaveloop` of `WSD_mix_se032.ogg` repeats exactly every
  10,603 frames at 44.1 kHz, as the WAV did. Opus plays through the
  engine's own decoder (`decodeOpus`, in every build, the web one too).

- 2026-10-17: sound loops, **seen** in `knife1.bfsar`
  (`bfsar::stream_loops`): a stream's details (INFO sound, kind 0x2201)
  hold at 0x18 a reference (kind 0x2210) to u32 stream type (byte 1:
  loops), u32 loop start, u32 loop end. 131 streams loop (all the
  `BGM`s and `SONG`s, 105 after an intro); voices don't. The positions
  count decoded samples after the Opus pre-skip: in 59 of the 80 tracks
  with enough after their end to compare, the decoded music repeats
  itself exactly from end − 1 (the shift with the least difference over
  0.1 s, searched ±400 samples; the rest end in a fade or noise), so
  the end is taken as end − 1. `typemoon sounds` writes them as Ogg
  Opus `LOOPSTART`/`LOOPEND` tags, which CONscripter loops music and
  `dwaveloop` by (`decodeOpus`); the 196 looping wave sounds get their
  FWAV loop as a WAV `smpl` chunk. Checked in the engine with SDL's disk
  audio driver: `dwaveloop` of `WSD_mix_se032` (loop 16352–27892 of
  27893) repeats exactly every 10,603 frames at 44.1 kHz (the loop's
  11,541 at 48 kHz). The songs' loops end 1–6 s before their files do.

- 2026-10-17: the English (fan patch) text layout, **seen** in the
  code: when `FUN_0012df20` (0x12df20, the system language) says
  English, the text drawer `FUN_001391a0` (0x1391a0) starts lines at x
  30 instead of 88, wraps them at 584 instead of 464, puts the first
  line at (short)(65 − 9.882353) = 55 (state +0x670), and
  `FUN_001a7790` (0x1a7790) draws glyphs × 0.9. The wrap table at
  0x44DEFC (30 floats, window types 3–32 from state +0x8a2, or +0x8aa
  in English when it is above 0) holds 299.8, 405.6, 509.8 for each
  pair of types, the same in both languages and in both exes; types
  0–2 take 464 or 584. The English build (detected by its text: most
  lines without kana) uses x 60, y 88, 41 columns (1168 px / 28) and
  27 px glyphs (1280 / 32 × 0.76 × 0.9); a headless page fills x 61
  to 1206 and wraps there. The fan patch's text carries its own `^`
  line breaks at about 55 letters, so few lines reach the wrap.
- 2026-10-17: `_STEC` effects 2–82 (9,193 uses), **seen**: the record
  (83 × 20 f32 at 0x4358B0, `stec.rs`) goes to the shader constants
  (`FUN_0019f6c0`, defaults from `FUN_001ec000`: gamma 1, low 0, high
  255) of `shader/nexl_nxshader_pre_alter.bnsh` (fragment program at
  file offset 0x1080, `envydis -m gm107`; its constant bank 1 is 1/255,
  −1/255, 255, 127.5 at 0x2300), which draws an altered texture: per
  texel (0–255) the greyscale (54 R + 183 G + 19 B)/256 in mode 1, then
  per channel `low + (high − low)·g/255` with g = 0 for a gamma ≤ 0, else
  255·(c/255)^(1/gamma); then each colour layer L (r, g, b) at amount a
  (0–1) by its blend: 2 normal `c + a(L − c)`, 3 add `min(c + aL, 255)`,
  5 multiply, 8 colour dodge `255c/(255 − aL)` (black for c ≤ 3), 14
  add then mixed by a, 17 screen, 18 overlay; others none. Effect 1
  (mode 2) fills the texel with the colour instead, and the layer is then
  drawn in that colour too (the normal shader multiplies by it), so a
  white `_STEC(n,1,0xFFFFFF)` is a white silhouette, not "none" as
  converted before. The shader's last step (brightness and contrast,
  `FUN_001a3180`'s first two arguments) is 1.0 for these. Converted
  with CONscripter's `spfilter` kinds 9/10 and blends 4/5 (#99).
- 2026-10-17: `_STCH` arguments 7 and 8, **seen** unused: `FUN_00160840`
  reads arguments 0 and 2–6 (`FUN_00102c38`/`FUN_00102d3c`), 1 as the
  name (`FUN_00102c6c`) and 9–19, never 7 or 8, and `script_command`
  passes the split arguments straight to it. Argument 8 is `CC` in
  54,502 of 54,503 calls (the other has it as argument 7). They no longer
  count as gaps.

- 2026-10-17: `_STZ4`'s other arguments, **seen** in `FUN_00162d20`,
  `FUN_00131f30` and the draws (`FUN_00132e30`/`FUN_00137f50` for the
  live layers, `FUN_00189ad0` for `FUN_001884f0`'s list). The time is
  ms · 60 / 1000 frames (integer); at 0 frames the zoom is the target
  and x0, y0 and the easing `e` are never used, so the 29,897 static
  calls now read every argument. Argument 7 (`f`; empty keeps the
  layer's, `_STCH` and `_STCC` clear it) is the layer's byte 0xA2
  (state 0x874): when set, the quad is drawn at zoom 1 and the zoom
  goes only to the shaders' constants (`FUN_001b3780`, draw flag 0x40;
  **inferred**: for the blurs, `_STBR` 11 or 17 in most of its 55
  uses). A child's place still uses its parent's zoom
  (`FUN_00138a70` doesn't read the byte). The converter draws such a
  layer without its `_STZ4` (or motion) zoom: 78 `spzoom` and 23
  `spmotion` of the Japanese `0.txt` changed. Left: the 2 timed uses
  (jump to their end) and 2,418 calls on layers not staged.

- 2026-10-17: `_CLO3`/`_CLO4`/`_CLO5` fade the text window while the
  script goes on, with CONscripter's new `windowfade` (engine #82):
  `_CLO3` → `windowfade 0,67` (4 frames), `_CLO4(ms)` → `windowfade
  0,ms/2` (drawn at 2 × (f − 0.5), so gone at half time), `_CLO5(ms)` →
  `windowfade 256,ms/2,0,ms/2:textclear:texton`, and the first text after
  `_CLO2`/`_CLO3`/`_CLO4` gets `windowfade 256,150,0,150` (18 frames,
  the same curve). Tracked in script order: a text reached by a jump
  from elsewhere may get a needless fade-in, or keep the plain show.
  Japanese `0.txt`: 13,186 `textoff` became `windowfade` (`_CLO2`'s 5
  stay), 12,624 fade-ins (596 of them `_CLO5`); metrics full +1,574.

- 2026-10-17: `_CLO2`/`_CLO3`/`_CLO4` hide the text window (`textoff`),
  all **seen** in `script_command` (0x177790) and `FUN_00138d40`
  (0x138d40, the window's alpha each frame; ticks from `FUN_001ad180`,
  1 per 60 Hz frame, 2 after a dropped one). `DAT_00be0550` is the
  shown flag, `DAT_00be0558` the window alpha, `DAT_00be05b0` a second
  fade multiplied into the window art (pictures 0x1d, 0x1e) and the
  text as max(0, 2 × (f − 0.5)) (`FUN_001391a0`). `_CLO2` calls
  `FUN_00153ab0(1)` (new page) and sets flag and alpha to 0. `_CLO3`,
  when shown, clears the flag and sets `DAT_00be0560`, so the alpha
  falls 0.2604 a tick (4 frames) instead of the normal 0.06 a tick.
  `_CLO4(ms)` sets the second fade to 1 and its step to −16.67/`ms` a
  tick (the earlier note that it sets the flag was wrong); `_CLO5(ms)`
  sets flag and alpha to 1 and fades the second one up from 0. The
  next text (`FUN_0016e0d0`) restarts that fade from 0 at 0.0556 a tick
  when it is below 1 or after `_CLO2`/`_CLO3`. 13,191 `textoff` in the
  Japanese `0.txt`.

- 2026-10-17: `_PGST(n)` left out, **seen**: `script_command` stores
  `n` in the state (`DAT_00be0290` + 0x8c4) unless a seek is running;
  the save writes it (`FUN_0015a910`) and the load reads it back
  (`FUN_0015baf0`), the backup copies at `DAT_00be0338` (`FUN_001178c8`,
  `FUN_00117af0`) keep it, and its only reader is the load:
  `FUN_00182330` calls `FUN_00182c60(page)`, which finds the scene's
  `_PGST(page)` in the script (for a few scenes, chosen by
  `DAT_00450d58` and a flag, another page), sets the seek flag
  `DAT_00be0900` and the sought page `DAT_00be09c0`, and runs commands
  until `_PGST` reaches that page. CONscripter's saves restore the state
  themselves. (The +0x8c4 in `FUN_0015baf0`'s copy into `DAT_00be0258`
  and in `FUN_00160840` is another table, `DAT_00be0250`.) 17,202 calls
  move from gaps to `left_out`.

- 2026-10-16: children follow their parent's motions. `_STMA` on a
  parent gives each layer placed in it (and theirs) an `spmotion` of
  its own, from where the parent's keys place it; the motions start
  after every place of the `_FADS`. A child's alpha is its own times its
  parent's (`FUN_001884f0`: the parent's at +0x1E4, `_STCH` argument 5).
  Not followed: `_STGS` fades on a parent, and a child's own motion
  beside its parent's.

- 2026-10-16: a parent's turn for its children. `FUN_001892b0` and
  `FUN_00189420` (the child's x, y) turn its offset in the parent by
  the parent's angle (`FUN_0018c480`: the angles summed up the chain,
  in 1/100°) with x' = u cos a − v sin a, y' = u sin a + v cos a, y down:
  clockwise on the screen. So a positive angle turns a picture
  clockwise (it was drawn counter-clockwise before): `spzoom`'s angle
  is now −a/100. 20,757 `spzoom` and 1,725 places of the Japanese
  `0.txt` changed.
  - **The sign, checked again** (the same day). Two places in the game's
    own code put things clockwise for a positive angle: the child math
    above and `FUN_00189ad0`'s frame corners (it turns the corner
    offsets with x' = dx cos − dy sin, y' = dy cos + dx sin, y down,
    and draws the corner pieces with the same angle). 1,725 child
    places sit in turned parents, which the original couldn't have
    shipped if the picture turned the other way. The renderer itself
    (`FUN_001b1890` → `FUN_001bacf0` → `FUN_001b8990`: Euler z = −angle,
    row-vector model matrix, y-down ortho in `FUN_001d83bc`) reads as
    counter-clockwise, but that read leans on the view matrix and the
    shader's layout, neither traced; so clockwise is inferred from the
    game's code, not seen at the vertex.
  - **A child's own picture turns by its own angle only**:
    `FUN_001884f0` copies the layer's angle alone into the draw record
    that `FUN_00189ad0` reads (record +0x1c, ×1.82 into 1/65536 turns
    for `FUN_001b0480`), and `FUN_0018c480` (the sum) is called only by
    the two position functions. The scripts agree: of 1,831 children
    in turned parents, 14 set the parent's angle on the child too
    (1740 and 1740), which summing would turn twice. 1,405 `spzoom`
    lines of the Japanese `0.txt` changed.
- 2026-10-16: panels (`_STCH` arguments 12–17). The draw
  (`FUN_00189ad0`) reads arguments 12–15 as a rectangle x, y, w, h on a
  1920 × 1080 screen: w or h 0, or 1920 × 1080, is no clip; otherwise
  it is scaled to the render target (× width / 1920, × height / 1080)
  and set as a scissor. Argument 16 is a frame's width (1920 units):
  the scissor is the rectangle less the frame on each side, across if
  narrower than the screen, down if lower. With argument 17 ≥ 1 the
  frame is drawn in a colour (1 and 7 black, 2 white, 3, 4, 5, 6 tints;
  261 uses), with corner pieces. A child draws in its parent's
  rectangle (copied at `_STCH` and again at the draw), and an argument
  left out keeps the layer's value. The uses fit: 2,210 rectangles,
  mostly wide bands past both sides (−428, 158, 2775, 765) or tall ones
  past the top and bottom (698, −136, 957, 1353), cut-in panels.
  Converted with `spclip` (9,065 clips, 3,422 back to the full screen).
  Not drawn: the frames, and a turned layer's rectangle isn't turned.
  Checked headless: `01_00_ARC01_8`'s bedroom strip shows from y 166 to
  554 on the 720 screen.

- 2026-10-16: choices. `_SELR(index, $text[, 1])` adds an option (1:
  greyed), `_SELD(_D000, …)` shows them and `_GOTS(_D000, label, …)`
  goes to the picked option's label; an option is written once per
  branch of the `_IF__`s around it (shown or greyed), and the converter
  keeps the first text of each index, so a flag-gated option is always
  offered. `_GOTO(label)` jumps in the script. `_IFNX(,L)` and
  `_IFP4(,L)` (also `_IFSC`) hold the Switch's and the PS4's own lines
  (a picture placed a few pixels apart): the Switch's run, the others
  are skipped. Headless (300 s, auto-play): 29 choices, from `00_00`
  through the first four days of Arcueid's route into a bad end's
  Q&A (`QACALL`).

- 2026-10-16: turns, shaders, tints.
  - `_STRT(layer, from, to, ms, ease)` (`FUN_00162b20`) turns the
    staged layer to `to` in 1/100° (clockwise on the screen, as the
    child-layer math turns; see the parent-turn entry above); `_STCH` argument 19 the same, and
    `_STCH` resets it. No use is timed.
  - `_STBR(layer, n)` (`FUN_00162490`) picks the layer's pixel shader;
    `_STCH` resets it (to a global default, else none). The draw maps
    the byte through a table at 0x44DD70 (`FUN_00132d20`; 1–39 map to
    themselves) into the shader list at 0xBD2308 (56 names, 16 bytes
    each, relocated): 0 default, 3 fulladd, 7 monotone, 8 negative,
    11 gauss, 15 haze, 18 add, 19 multi, 20 dodge, 21 burn, 22 darken,
    23 lighten, 24 screen, 25 add_alpha, 26 sub, 27 ps_add, 28 ps_sub,
    29 ps_mul, 30 ps_screen, 31 overlay, 32 hardlight, 33 softlight, …
    Uses: 17 (3,475), 11 (3,158), 23 (1,909), 24, 22, 7, 10, 19. Drawn
    with `spblend` (add, multiply, subtract, screen; darken and burn as
    multiply, lighten and dodge as screen) or a filter (greyscale,
    negative, blur); forcealpha, haze, wave, overlay and the lights as
    normal.
  - `_STEC(layer, effect, colour)` (`FUN_00103de0`): effect 1 tints the
    layer by `colour` (0x000000 954 uses, 0xFFFFFF 410, 0xE5FFFF, …;
    multiplied, white none), 0 ends it; `_STCH` resets it. Effects 2–9
    and 10–82 (2 is the most used, 1,338): see Open questions.
  1,001,727 commands fully converted.

- 2026-10-16: parent layers. `_STCH` argument 11 names the layer a
  picture is placed in (6,028 uses, mostly effect pictures over a
  scene). From `FUN_001892b0` and `FUN_00189190`: the child's x, y are
  in its parent's picture, from the parent record's top-left
  (child = parent's place + parent's zoom × (x · shot − the parent
  record's centre), the shot ½, ¼, ⅛ for a parent sprite's `_L_`,
  `_M_`, `_S_`); its zoom is the parent's times its own and the shot.
  Resolved at each `_FADS`, so a moved parent places its children
  again. Not followed: the parent's rotation and motions.

- 2026-10-16: motions. `_STMA(layer, m)` (`FUN_00199620`) starts motion
  `m` on the shown layer at once (and its staged copy), while the
  script goes on; on a layer not shown yet it waits for the `_FADS`
  that shows it. Its keys are absolute (x, y in script units, alpha,
  the `_STZ4` zoom); the converter turns them into `spmotion` offsets
  and zooms from the layer's place (the IR's `SpriteMotion2`, kept at
  the end) and moves the layer's state to the last key. The motion
  table was read one motion late (its entries are each motion's end,
  not its start; see "The exe"). `typemoon exe --motion m,…` prints
  motions' keys. 961,377 commands fully converted.

- 2026-10-16: zoom and alpha fades. `_STZ4(layer, x0, y0, x1, y1, ms,
  …)` (`FUN_00162d20`) sets the staged layer's zoom across and down in
  ‰ (to x1, y1; `_STCH` resets it to 1000), times `_STCH`'s zoom
  (32,317 of 32,319 uses are static; the 2 timed ones jump to their
  end). `_STGS(layer, from, to, ms, ease)` (`FUN_001637a0`) fades the
  layer's alpha from `from` (default its own) to `to` (default 0) over
  `ms`, running from the next `_FADS` while the script goes on (4,877
  of 8,114 timed; easing not followed): an `spmotion` from the IR's
  `SpriteMotion2`, started inside the transition so its first key
  shows. 937,479 commands fully converted.

- 2026-10-16: layer priority. The draw list (`FUN_001884f0`) keeps the
  layers with a picture and a priority above 0 (`+0x234` per layer,
  set by `_STCP` and `_STCH` argument 6, default layer × 10) and
  stable-sorts them by priority, lower first (drawn first, so under).
  Each `_FADS` gives them IR layers in that order from 39 up and draws
  again only the IR layers whose picture changed. 897,047 of 1,082,419
  commands fully converted (was 845,480).

- 2026-10-16: the script conversion's first part (`typemoon/convert.rs`,
  `build.rs`; `typemoon convert`). Every script is lowered, 845,480 of
  1,082,419 commands fully. A picture is drawn at its record's size
  from its file's (the full art's header, read without decoding:
  `typemoon::picture_size`), its record's centre at the script's x, y
  (480 × 272 for the screen); the layers' order follows their number.
  `_FADS` 0 is a cross-fade, 2 a wipe through `pic/rule_NN.webp`. Text:
  each `_ZM`/`_MSAD` is a line of the page, `_WNTY(1)` before one
  ends the page, `@n` joins the next line without a click; `^` is a
  line break, `@x` codes and `[ber00]` are dropped, ♡/♥ are kept
  (UTF-8 `0.txt`). Flags: `F`/`C` locals from %196, `G` globals %800 + n.
  Found on the way: the Ogg Opus files' OpusTags said a vendor length
  of 13 for "conscripter" (11); ffmpeg read them, opusfile (the
  engine's voices and sound effects) refused every one; and an opaque
  WebP has no alpha channel, so `lsp ":a;…"` took its right half as
  the mask on native builds (fixed in CONscripter, engine PR #55; the
  corner-pixel workaround in `typemoon media` is gone). Checked headless (trace, 200 s): from `START_SCRIPT` through
  `00_00` into `01_00_ARC01_*`, every voice and sound loads.

- 2026-10-16: the media folder (`typemoon media`, see "Build"). The
  user's picks: WebP at full size, not the BC7 textures (a sample of
  101 `allpac` pictures: 63.1 MB in the archive, WebP q85 at full size
  7.2 MB, at stage size 2.6 MB), smaller movies, the Opus as it is.
  Japanese copy: 14,763 pictures (one per name: 1,279 bare
  backgrounds from their 2560 × 1440 `.JPG`, the rest from the larger
  `.NXGZ`/`.NXZ`), 2.37 GB from 16 GB; movies 1.7 GB → 182 MB; 15 min
  on 8 threads, 5.7 GB peak (32 threads went over 10 GB). Checked: RMSE
  against the lossless decode 0.3 % for `ARU_A_18_00_N01_10_10`
  (886 × 4160), 1.6 % for `IMG0020` against its JPG; CONscripter
  (native) draws both, the sprite's alpha whole.

- 2026-10-16: how big a picture is drawn, settled by matching the
  gallery's CGs to the story pictures (`cg_framing.py`): a record
  is on a 1280 × 724 stage, the screen; a background's whole texture
  fills it. The shaders were read on the way (they don't hold it).

- 2026-10-16: the stage: script coordinates are 480 × 272 for the
  screen (the draw's vertex math), record units 640 × 362; a bare
  pose's record is its expressions' less their padding. How big a
  picture is drawn is left open (see "The stage"). The user picked
  full-size WebP for the media folder.

- 2026-10-16: the exe's tables read in Rust (`typemoon/nso.rs`,
  `exe.rs`, `typemoon exe`): 14,713 pictures, 24,192 motions (71,903
  keys), 551 scene numbers. Picture sizes checked against every
  texture; the record's byte meanings corrected (0x43 the expression
  flag, 0x46 the shot). A WebP sample for the build's media folder.

- 2026-10-16: the walkthrough, from the scripts (`routes.py`) and
  checked against a public one: the route gates (Ciel's points, the
  Arcueid points, `G003`/`G004`), the three endings, the 21 dead ends
  and their lessons. `_JUMP` goes by scene number, a table in the exe.
  See "Walkthrough" and "Progression variables".

- 2026-10-16: the exe's commands. `_MPLY(n)` plays the bfsar's sound
  `n − 1` (a table in the exe); the music loops are in the bfsar's
  stream details; the picture and motion tables are in the exe; the
  arguments of `_STCH`, `_STCC`, `_STCP`, `_STZ4`, `_STMA`, `_FADS`,
  `_WNTY` and `_WTTM` read from their handlers; sprites turned out to
  be whole pictures, one per expression, not atlases. See "The exe".

- 2026-10-16: the exe decompiles (`nso2elf.py`,
  `CodePointerFunctions.java`, `research/typemoon/names.tsv`); the
  script dispatcher and `_MPLY` found. The official update 1.0.2 was
  unpacked and compared with the base and the fan patch. The script
  changes are split between them in "Script language", and the copies
  to convert are chosen in "Build".

- 2026-10-16: scripts read (`typemoon/script.rs`, `typemoon script`).
  Both languages: 549 scripts, 93 (96) command names ranked, every
  text reference within the 43,961 lines, every voice found, the
  sound effect names matched to the archive (all but `MIX_SE*`). See
  "Script language".

- 2026-10-16: sound (`typemoon/nxopus.rs`, `bfsar.rs`, `fwav.rs`;
  `conscripter-convert typemoon sounds <romfs> -o <dir>`, 9 s). Both
  RomFS copies: 16,601 streams to Ogg Opus (identical in both), 1,297
  (Japanese) and 1,300 (English) wave sounds to WAV. Checked: ffmpeg
  decodes every Ogg with no error; for 910 of them (every 150th voice
  and every music file) the decoded length equals the granule count
  less the pre-skip. DSP ADPCM: the 107 (108) waves that loop from
  inside store the decoder state at the loop start, and our samples
  before it match it on every channel (a change in the rounding makes
  106 of them fail).

- 2026-10-16: pictures (`typemoon/bntx.rs`, `typemoon extract --png`):
  BNTX parse, Tegra block-linear deswizzle, BC1/BC2/BC3/BC4/BC7 blocks
  through the `bcdec_rs` crate. `typemoon check` decodes every texture;
  each one's swizzled size matches the `BRTI` header's. Base (Japanese):
  29,561 textures (29,451 BC7, 98 BC4, 12 RGBA8); patched (English):
  29,571 (29,463 BC7, 98 BC4, 10 RGBA8). Looked at a background, a
  sprite atlas, UI parts, the RGBA8 parts and transition masks: right
  way up, colours right.

- 2026-10-16: the archive reader (`crates/conv-engines/src/typemoon/`:
  `mrg`, `mzp`, `mzx`, `nxx`) and `conscripter-convert typemoon
  list|extract|check`. Every entry of both RomFS copies reads and
  unpacks, and each wrapped entry's size matches the `.hed`. Inside:
  29,475 `allpac` entries (28,185 BNTX, 1,279 JPEG, 5 PNG, 6 MZP),
  `allpaccg` 919 BNTX + 600 JPEG, `allpachd` 14,705 unknown (`bin`),
  `allui` 26 BNTX. MZP sizes: mangetsu's and deepLuna's formula is
  64 KiB too big when rounding to the sector crosses a 64 KiB boundary;
  ours isn't.
- 2026-10-16: tool survey (mangetsu, deepLuna, PS-HuneX_Tools);
  the nesting of MZP, MZX and NXGX→BNTX checked on our files.
- 2026-10-16: NSP unpacked, engine identified, this document started.
