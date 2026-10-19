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

`conscripter-convert build <game-102> -o <output>` recognises the game and
runs `typemoon convert` with `--title "月姫 -A piece of blue glass moon-"`;
`build <game-en> --japanese <game-102>` runs it with `--english <game-en>`
and `--title "Tsukihime -A piece of blue glass moon-"` (README, "Converting a game"); the
convert options (`--dry-run`, `--font`, …) go after `-o`.

The converter is `conscripter-convert typemoon convert <game> -o out/tsukihime-cons --font <ttf>`
(see "Status"). Getting `<game>` (the RomFS) out of the NSP, with
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
`pic/<name>.png` (lowercase), and the gallery's CGs (`allpaccg`'s
1,023 `IMG*`, the same way) as `cg/<name>.png`, or `.webp` with `--webp` (near-lossless)
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
default), 110 MB as Vorbis q4; `cg/` (1,023) 302 MB at
`--webp-quality 85` (from 2 GB, 100 s); `pic/` (14,763) 2.37 GB at
`--webp-quality 85`. On a sample of 57 pictures (12 sprites, 9 masks,
36 large backgrounds; 167 MB as PNG) near-lossless WebP is 45 % of PNG
and lossy q85 10 %: PNG would be about 23 GB, `--webp` about 10 GB.

**Both languages** in one build: `typemoon convert <game-102> --english
<game-en>` puts both copies of the scripts in one `0.txt` with a
language switch ("Languages" under "Status"). `--english` on `typemoon
media` (and `typemoon convert`) also writes the fan patch's pictures and movies that differ
from `<game-102>`'s, as `pic/en/<name>`, `cg/en/<name>` and
`movie/en/<name>`: 155 of the 14,763 pictures (21 MB as WebP at 80), 7
of the 1,023 CGs (3.9 MB at 85) and 34 of the 46 movies
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
|---|---|---|
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
- **Two music slots** (2026-10-19, `research/typemoon/names.tsv`
  `music_*`): music plays on two sound players, one per 0x3c-byte slot
  record (state 0 idle, 1 playing, 2 fading out; volume, target, step
  per frame). `sound_update` (0x19b7f0) steps both every frame and stops
  a fading slot at volume 0. `_MFAD` (`music_fadeout` 0x19c9b0) only sets
  the current slot fading. A new `_MPLY` flips to the other slot
  (`music_slot_flip` 0x19b550), so a fading track could play on under
  it, but `cmd_MPLY` decides first (decompile lines 203-218): **without
  option 1** it stops the current player at once when it holds a sound
  (`music_player_active` 0x19ca50 = 3 → `music_stop_now` 0x19c610,
  `Stop(0)`), fading or not: a cut. **With option 1** it re-fades the old
  slot over option 11's ms (else option 1's; `_MFAD`'s rate is replaced)
  while the new one fades in on the other player (`music_play_fade`
  0x19c6d0): a cross-fade.
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
- **vgmstream agrees** (r2117 `vgmstream-cli -m`, 2026-10-18; a release
  binary, no loops are read from our own output): 50 random FWAVs cut
  raw out of `knife1.bfsar` (every `FWAV` magic, 1,297 found, as many as
  the wave sounds) give the same sample count as one of our decoded WAVs
  and the same loop (start, end within a sample, or none) in 50 of 50.
  50 random `stream/*.opus`: vgmstream reads no loop (as noted, the
  loops are only in the bfsar, so the 131 music loops stay unchecked by
  a second decoder) and its length is ours less the 120-sample pre-skip
  in 50 of 50 (it counts our Ogg's granule, which includes the pre-skip).
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
|---|---|---|
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
Gallery's Ending List (the walkthrough: the icon at the bottom right),
shown once the other 24 lessons are read (`_RDED`).

**Dead ends.** 21, each followed by a 「教えて！シエル先生」 lesson
(`_JUMP(3100,QA_nn)` into `QACALL`). The walkthrough's dead end n is
`QA_n` up to 11 and `QA_(n+1)` from 12.

| Lesson | Day | Scene | Reached by |
|---|---|---|
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
| `QA_15` | 10 | `10_01_CIEL10_5_2` | 「自分は足手まといだ。ここで待っていよう。」, or before it 「問題はない。話は放課後にすればいいし。」 (`10_01_CIEL10_3B` → `10_01_CIEL10_3_1` → the scene's `LBL_029`) |
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
|---|---|---|
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

Manual: none for the game. The NSP's `Manual` NCA holds only legal and
support HTML (`legalinfo.xml`), so reference screens must come from the
emulator trace or footage.

### Route runs in ours (2026-10-19)

The three endings and the 21 dead ends played headless in one save
folder, so the global flags carry from route to route as in a player's
game. Recipe (a `--dry-run` build over a full build's media, the
engine with `--until`/`--max-pages` saving the globals at the stop,
CONscripter #182):

```
python3 -I research/typemoon/route_choose.py <out>/0.txt <out>/text.ja.txt \
  research/typemoon/routes/<route>.txt > <route>.choose
conscripter --root <out> --save-dir <save> --auto-play 0 --trace <route>.jsonl \
  --goto tm_START_SCRIPT --until conv_title $(sed 's/^/--choose /' <route>.choose)
```

in the order `arcueid`, `ciel-normal`, `ciel-extra` (each needs the
flags of the one before: `G003` ungreys Ciel's day-14 option, `G004`
the extra ending's), then each dead end alone, `--goto tm_<scene>
--until conv_title` with the `button tm_QACALL 2` line (yes to the
lesson question). A run ends at the title when the scene's `_RTM_`
returns (before, QACALL's "no" fell through into the staff roll and
the test script).

| Run | Pages | Reached |
|---|---|---|
| arcueid | 16346 | `QA_12`, movie `ed01_arc`, the title |
| ciel-normal | 22043 | `QA_23`, `ed02_ciel`, the title (Ciel's option enabled by `G003`) |
| ciel-extra | 24034 | day 14's choice with option 2 enabled, `15_01B`, `QA_24`, `ed03_ciel`, the title |

| Dead end | Lesson | Pages |
|---|---|---|
| `04_00_ARC04_6_1` | `QA_01` | 118 |
| `04_00_ARC04_9_2` | `QA_02` | 64 |
| `05_00_ARC05_3_1` | `QA_03` | 105 |
| `05_00_ARC05_6_1` | `QA_04` | 65 |
| `05_00_ARC05_9_1` | `QA_05` | 100 |
| `05_00_ARC05_9_3` | `QA_06` | 65 |
| `08_00_ARC08_7_2` | `QA_07` | 76 |
| `09_00_ARC09_3_2_1` | `QA_08` | 143 |
| `10_00_ARC10LIVING` | `QA_09` | 243 |
| `10_00_ARC10_7_3` | `QA_10` | 71 |
| `10_00_ARC10_8_1` | `QA_11` | 111 |
| `05_01_CIEL05_2_1` | `QA_13` | 55 |
| `05_01_CIEL05_2_2_1` | `QA_14` | 47 |
| `10_01_CIEL10_5_2` | `QA_15` | 126 |
| `11_01_CIEL11_2_2` | `QA_16` | 82 |
| `13_01_01_CIEL13_3_2` | `QA_17` | 53 |
| `13_01_01_CIEL13_4_1` | `QA_18` | 131 |
| `13_01_02_CIEL13_5_1` | `QA_19` | 242 |
| `13_01_02_CIEL13_6_1` | `QA_20` | 108 |
| `14_01B_02_CIEL14B_3_1` | `QA_21` | 104 |
| `14_01B_03_CIEL14B_4_2` | `QA_22` | 183 |

All 21 reach their lesson and, after it, the title.

### Memory dumps through Ryubing's GDB stub (2026-10-19)

`--enable-gdb-stub --gdb-stub-port 55555` on the `switch-emu.sh` command
line (headless reads the flag, not `Config.json`'s `enable_gdb_stub`).
Then `research/tools/gdbdump.py 127.0.0.1:55555 <prefix> --get-info` writes
a dump in our format. The stub has no memory map, but `monitor get info`
lists the modules (here `nnrtld`, `knife1.nss` 1.3 MB, `glslc`,
`multimedia`, `nnSdk`) and the layout (heap 0x655a800000-0x66da7fffff).
Measured: the modules, 16.2 MB, read at 1.2 MB/s in 4 KB packets (with
TCP_NODELAY: without it each packet waited ~130 ms on the delayed ACK).
**The heap is not read yet.** A read of unmapped memory makes the emulator
log an invalid access, and the stub then stops answering (one run) or
the emulator aborts when the client goes (another). The heap's first
probe at its start hung it, and there is no command that lists the
mapped pages. Left: find the mapped heap pages first, from pointers in
the threads' registers (`qXfer:threads`, `g`) and the modules' data,
before reading.

### The original under a Switch emulator (Ryubing)

Step 1, the boot check (2026-10-17): **boots to the title** at full
speed (60 frames a second on the title's moving clouds). Ryubing's own headless front end is built into its main
binary (`Ryujinx --no-gui`, the old `Ryujinx.Headless.SDL2`), so the
flatpak `io.github.ryubing.Ryujinx` (1.3.3) runs it; no source build.
It still opens an SDL window, so it runs on its own Xvfb. A bare
`flatpak run io.github.ryubing.Ryujinx` (even with `--help` or
`--headless`) opens the GUI on the desktop: run it only through
`research/typemoon/switch-emu.sh`.

Setup, in a data dir of its own (`<emu>/data`, never the flatpak's
`~/.var/app/...`, which the user plays with):

- Firmware: each `<id>.nca` / `<id>.cnmt.nca` of the firmware zip goes
  to `<emu>/data/bis/system/Contents/registered/<id>.nca/00` (what
  the GUI's firmware install does; headless has no install option).
- Keys: `<emu>/data/system/prod.keys` and `title.keys` (the full
  file; Ryubing loads it as it is, without hactool's filtering).
  **The firmware must not need a newer master key than the keys
  have**: check each NCA's "Master Key Revision" with hactool first.
- Update 1.0.2: `<emu>/data/games/01001dc01486a000/updates.json` =
  `{"selected": "<update nsp>", "paths": ["<update nsp>"]}`; the log
  then says `Application Loaded: ... v1.0.2`.
- Run: `SHOT=15 research/typemoon/switch-emu.sh <emu>/data 240
  --graphics-backend Vulkan --skip-user-profiles-manager "<base nsp>"`
  (Xvfb by `-displayfd`, `SDL_AUDIODRIVER=dummy`; log and a
  root-window shot every `SHOT` s in `<emu>/logs`). Vulkan runs on the
  host GPU under Xvfb. Never the desktop: no Wayland socket, an empty
  `XDG_RUNTIME_DIR` (SDL finds a Wayland socket there even without
  `WAYLAND_DISPLAY`), `SDL_VIDEODRIVER=x11`, and the run stops unless
  its window is on its own Xvfb. The `capped.sh` 8 GiB scope does not
  hold it: flatpak moves the emulator into a scope of its own, so the
  script stops only the Ryujinx on its display (SIGKILL after 10 s:
  it outlives SIGTERM), never one the user runs.

- Input: `KEYS="40:z 41:z"` presses keys at those seconds, held
  150 ms (Ryubing's default keyboard: `z` = A, `x` = B); `FPSAT=<s>`
  counts the frames differing in any pixel over 5 s.

What a first boot does: with no save data the game plays the
prologue (`00_00`–`00_02`, with the credits), about 6 minutes with A
pressed every second, then the title, where the presses pick New
Game. The second boot (the game has written its save data by then)
opens on the title about 35 s in: `SHOT=5 FPSAT=100
research/typemoon/switch-emu.sh <emu>/data 130 ...`. Against our
title's places: the menu rows (New Game, Load Game, Config, Gallery;
no Continue) are 37–38 px apart in both, and the copyright sits 18 px
below our picture's top, as the rows sit 18–19 px below theirs (the
glyphs inside the parts; checking those needs a full build's
`sys/title`).

A first try with firmware 23.0.1 and keys up to master key 0x15
failed: 96 of its 238 NCAs need 0x16 (`MissingKeyException:
key_area_key_application_16`; with those left out, the game dies 8 s
in without the shared font, `0100000000000811`). Firmware 22.5.0 with
its own keys works.

Step 2, the trace (2026-10-17): an ExeFS mod logs every script
command the original runs, and `research/tools/trace_diff.py` compares
it with our `--trace`. No GDB stub: one stop a command would stall the
game, and a log line costs nothing.

- The hook (`research/typemoon/switchtrace/hook.s`): `script_command`
  (0x177790) runs commands in a loop, and its head (0x177828, `mov x8,
  xzr`, x0 = the next command's text) branches to it. It writes
  `TMT <script index> <offset> <command text>` through
  `svcOutputDebugString`, which Ryubing logs as a warning with its
  clock (`00:01:23.456`), then goes back. The script index is the u16
  behind the GOT slot 0xbe04f0 (`FUN_00159ad0` sets it: entry 3 + i of
  `allscr.mrg`), the offset is from the loaded script's buffer
  (0x1473e04, `FUN_00157390`). A command that waits (a fade, `_TOMW`)
  is run again each frame, after an empty one (its `;`).
- `mkmain.py` puts it in 1.0.2's `main`: the text segment lengthened
  over the zeros before the rodata page (0x1542c0 to 0x155000, mapped
  executable already; Ryubing writes only a segment's own bytes, so an
  IPS patch there would be lost), written uncompressed, the build ID
  kept. Ryubing replaces `main` with it from
  `<emu>/data/mods/contents/01001dc01486a000/tmtrace/exefs/main`
  (`ModLoader ApplyExefsMods: NSO 'main' replaced`).

```sh
clang --target=aarch64-none-elf -c research/typemoon/switchtrace/hook.s -o hook.o
ld.lld -Ttext=0x2542c0 --defsym=SCRIPT_INDEX_PTR=0xbe04f0 --defsym=SCRIPT_BUF=0x1473e04 \
  --defsym=RETURN=0x17782c -e hook hook.o -o hook.elf && llvm-objcopy -O binary hook.elf hook.bin
python3 -I research/typemoon/switchtrace/mkmain.py <exefs-102>/main hook.bin \
  <emu>/data/mods/contents/01001dc01486a000/tmtrace/exefs/main
SHOT=60 IDLEA=1 research/typemoon/switch-emu.sh <emu>/data 780 --graphics-backend Vulkan \
  --skip-user-profiles-manager --disable-ptc "<base nsp>"
conscripter-convert typemoon extract <game-102>/allscr.mrg -o <scr>
conscripter-convert typemoon extract <game-102>/script_text.mrg -o <txt>
python3 -I research/typemoon/switchtrace/tmtrace.py <emu>/logs/ryu.log <scr> <txt> -o orig.jsonl
```

`IDLEA=1` is the original's auto mode: A once the last command has
stayed a second on a text, the opening's `_TOMW` or a script's end (the
title then takes A as New Game). `IDLEA=0.3` (checked every 0.2 s) is
the fast run: 1.4 s a page instead of 2.5, the times unchanged (the
emulator's clock is untouched). A custom refresh rate does not scale
the game's frames by a fixed factor (400 %: about 1.7x), so its times
would not compare (`docs/new-game.md`, "Trace speeds"). `tmtrace.py` turns the log into
`trace_diff.py`'s records: "at" `<script>:<command>` counted as
`script::parse` counts them (the converter's `--source-map` gives the
same), "ms" the log's clock, one record for a command run again, a
page per `_ZM`/`_MSAD` with the `@n` lines before it. The data dir
needs `bis/user` from a first boot: with none, the game aborts in
`nn::fs::MountSaveData` (headless does not make the save). From that
save, run 5 went through the opening's `00_01` (`_TOMP(4)`, its
pages drawn by the exe, not by commands), the title, New Game,
`START_SCRIPT` and 297 pages of `00_00` in 13 minutes. Ours:

```sh
conscripter-convert typemoon convert <game-102> -o <ours> --dry-run --source-map <ours>/map.tsv
# in <ours>, the full build's pic, se, stream, movie, sys, default.ttf linked in:
conscripter --root . --save-dir saves --goto tm_START_SCRIPT --trace ours.jsonl \
  --auto-play 0 --real-time --max-pages 297
python3 -I research/tools/trace_diff.py orig.jsonl ours.jsonl --map map.tsv --ms --pages 297 --max 10000 --merge-into _FADS
```

Step 3, the full-route reference trace (2026-10-18): **started, stopped
in `00_00`** to free memory (Ryubing holds 5.5 GB and flatpak takes it
out of the `MemoryMax` scope; the machine fell under 4 GB free beside
the wine traces): 208 pages of `00_00` in 17 minutes, from the save at
the title. The route is Arcueid's (the first play): ours takes every
first option except the 11 dead ends of her route (the lessons
`QA_01`–`QA_11`, table above) and reaches `14_00_ARC14_1` at page
16334, about 6.4 h of original at `IDLEA=0.3`. The original takes the
same options by `switch-emu.sh`'s `AVOID`: at a choice (`_SELD` the
last command) it reads the `_SELR`s before it from the trace and
presses Down past the greyed and avoided options, then A (not yet
reached in a run: the first choice is day 1's `01_00_ARC01_1`).

```sh
SHOT=600 IDLEA=0.3 AVOID="042606 042614 042615 042617 042621 042623 042655 042665 042668 042679 042680" \
  research/typemoon/switch-emu.sh <emu>/data 7200 --graphics-backend Vulkan \
  --skip-user-profiles-manager --disable-ptc "<base nsp>"
python3 -I research/typemoon/switchtrace/tmtrace.py <emu>/logs/ryu.log <scr> <txt> -o orig.jsonl
# ours, in <ours> as above; <choose> = the 10 selects holding an avoided option, `*label:line=N`
conscripter --root . --save-dir saves --goto tm_START_SCRIPT --trace ours.jsonl --auto-play 0 \
  $(sed 's/^/--choose /' <choose>)
python3 -I research/tools/trace_diff.py orig.jsonl ours.jsonl --map map.tsv --ms --pages 10000 --merge-into _FADS
```

The `AVOID` ids are the `$` string of each dead-end option's `_SELR`
(`typemoon script` prints them: `_SELR(1,$042614)  « ……部屋で外の様子をうかがう。 »`);
the selects for `--choose` are the 0.txt `select`s naming those texts.
Diff over the 208 pages: no text findings; 4 missing (`_STMA` at
`00_00:212` and `1121`, `_STGS` at `00_00:2243`, `2244`), 21 merged
into `_FADS`; the 172 time findings mean nothing (ours ran without
`--real-time`).

## Milestones

The date each phase was reached, and its PR (`-`: none yet, or made
before PRs). Fill a row in at the end of its phase, so the next port
can be compared with this one.

| Milestone | Date | PR |
|---|---|---|
| First commit | 2026-10-16 14:12 | - |
| First dry run | - | - |
| Story plays | 2026-10-16 20:10 (opening) | - |
| Title, save, settings | 2026-10-17 14:35-14:43; settings 2026-10-18 11:52 | - |
| Extras | 2026-10-18 03:08 | - |
| Progression verified | 2026-10-19 (3 endings, all 21 dead ends, in ours) | PR_N |
| Finished | - | - |

## Status

Working: files out of the NSP; all archives, pictures (BNTX) and
sounds (with loops) of both RomFS copies; the exe in Ghidra with its
tables read (`typemoon exe`); `typemoon media`; scripts read (every
command splits, text and voices join); `typemoon convert` plays from
the start through `00_00` into the first arc (flow, text, pictures,
sound, layers, motions, choices, text window fade, the lessons' yes/no
question); the 45 story movies (`_TOMP`: prologue, openings, day
cards, endings, credits; History, 2026-10-18, "story movies"); boot logos, title,
the system menu's Save and Load tabs, Config (the settings, also on
the title; History, 2026-10-18, "the settings");
the Gallery's CG gallery (its other modes drawn, not open);
`--english <game-en>` builds both languages in one `0.txt`, switching
in game; the original boots under Ryubing and its trace diffs against
ours over `00_00`'s first 297 pages; route runs reach the 3 endings and
21 lessons (Testing). How each was done: History ("Status as it stood
before the 2026-10-18 prune": the full table, Languages, Trace diff).

Not yet: the other layer commands, FlowChart, the backlog, the Gallery's Music Player,
Flowchart and Movie. The Ending List replays the lessons (History,
2026-10-18, "the Ending List"); its pictures' places are partly guessed.

### Next

1. The script conversion, next part: the layer commands the opening
   leans on, in order of use. `typemoon convert --dry-run` prints the
   counts. Splitting sprites into body and face crops is out unless
   the user asks.
2. The Gallery's other modes (Music Player, Movie, Flowchart; drawn
   in its menu, not clickable). None is small: each
   needs its table, its `allui` parts and its screen read from the
   exe, then an IR screen (`SoundRoom` fits the music and movie
   lists). `FUN_00107f38` opens them by the menu row: 1
   `cGalleryMusic` (`FUN_00119518`, its list `cGalleryMusicList`
   and panel `cGalleryMusicPanel`), 2 `cFlowchart` (`FUN_0010f8fc`,
   `cFlowchartScene`; the 495 `GALLERY_FC_TH*` thumbnails), 3
   `cGalleryMovie` (`FUN_00117cd0`, the player `FUN_00104f28`
   `cGalleryMoviePlayer`). In the CG gallery
   what is left out (History, 2026-10-18, "the CG gallery"): the
   viewer's zoom and pan (`FUN_0010c56c`: the right stick or a pinch
   zooms between fitted and 1:1, a button toggles fitted and filling
   the screen, the left stick or a drag pans, left and right step
   through the slot's CGs; ours: a click steps on). It would need the
   viewer's input in the script (`btnwait` keys, `spzoom` and `amsp`
   on the CG). 926 CGs, in neither unlock table,
   show only after flag 557 (`_RDED(557)`, the last lesson), as the
   exe reads (`FUN_00156720` scans the first 0xA8C picture names).
   `allui` (71 MB) is still not in the media folder.
3. The menus: the
   rows waiting for engine features (below), the backlog; then the
   save list's route picture and chapter labels (by the phase). The
   settings rows drawn at the exe's defaults with no effect: the read
   text speed (+0x10, 15 ms) and the unread/read samples, the two
   "ページ末まで一気に表示" boxes (+0x5A, +0x5B), the auto page wait
   (+0x5C, 3000 ms; the engine has one auto wait), 既読シーンのオート
   スキップ and its question box (+0x5E, +0x60), the text fade-ins
   (+0x61, +0x62), the system sound's mute (+0x66), the SE test, the
   values' digits (`config_number`), the characters' volume sliders
   (50 %) and voice tests; the 推奨設定 question is not asked. The
   characters' rows show all 19 from the start (the exe shows a row once
   its flag, 0x44D4B4, says the character was met).
4. Still open in the exe: the motion keyframes' other fields.
5. The full-route reference trace (Testing, step 3): rerun it whole
   once the machine has 8 GB free (Ryubing takes 5.5 GB), from the
   title; so far `00_00`'s 208 pages, text clean; look at the
   missing `_STMA` (`00_00:212`, `1121`) and `_STGS` (`00_00:2243`,
   `2244`).
6. The metrics (`tests/snapshots/tsukihime.metrics.json`) count each
   argument a converted command passes and doesn't read as a gap
   (`<command> argument <i> ignored`, `option <n>` for a backquote
   option); the list is in that file. The biggest: `_STZ4`'s 2 timed uses (and 2 on
   layers with no picture).
6. Every kind has a row in `tsukihime.findings.tsv` (2026-10-17, from
   the decompile; 45 rows open). The open ones that matter most, by
   use: the layer quakes `_SQK2`/`_SHK2`/`_SSW2`, `_STPV` (the layer's pivot), `_SNX2`/`_SNX3`
   and `_STNS`/`_STRA`/`_STRR`/`_SACL`/`_SBAS`/`_SOXY` (layer fields
   whose drawing is not found yet), `_VCWT` (wait for the voice),
   `_VPL2`, `_SESP`, `_WKOR`, `_STTR`, `_STCL`, `_SGMA`, `_SPMA`, the
   chapter start (`_SELA`, `_ATSV`/`_WTSV`), the
   scenes' read flags (`_RDED(1..532)`, for the Flowchart; the
   lessons' are done, History 2026-10-18 "lesson read flags"), the
   `_SELD`'s choice number. (`_FANX`, `_STQK`, `_TRPY`, `_NOP_` and the
   picture cache `_SPFC`/`_SPRC` are in `LEFT_OUT`: the exe draws
   nothing for them.)
7. The first launch's prologue (ours: straight to the title). Leads:
   flag 485 unread sends the title's setup (`FUN_00190180`) down the
   first-launch branch (`DAT_00be0e98`) and the game mode
   (`FUN_00148ac0`) to scene 0x104 instead of 0x6E (`00_00`).
7. Trace diff leftovers (`trace_diff.py --merge-into _FADS`): 2
   `missing` (`_STMA` `00_00:212` and `1121`, never drawn in the exe)
   and 72 `time` (chiefly `_FADS` after a new
   picture: the original's 2-10 frames loading it, left out;
   `_MPLY(52)` at `00_00:4296`).

### Accuracy

The converter's hand-typed values carry a provenance comment (`src:`,
`guess:`, `ours`). Open guesses, each settled by one frame of the
original (Japanese, default text size):

- The text window's line pitch (45 px: 28 of the drawer's 448, the step
  it uses for its extra lines; the exe measures a line of the font) and
  so its 14 rows, and whether the glyph sits 16 px below its line's
  y (`FUN_001a7790` adds 14 + 10 of 1080 inside the line).
- The text speed: 35 ms a character, the exe's default for unread
  text (+0xe unread 35 ms, +0x10 read 15 ms). Read text is typed at the
  same speed here (CONscripter has one). The settings' sliders step in
  11 levels where the exe's go by 1 ms, 50 ms or 1 %: the text speed
  25 ms a level (35 at level 9, its knob 10 % from the left, the exe's
  14 %), the auto wait 1100 ms a level (2200 at level 8, 0-11000 for
  the exe's 0-10000), the volumes 10 % a level (40, 40, 10 exact).
- `_STBR` 11 (gauss) drawn as a blur of 8.
- Motions (`_STMA`): the place moves a pixel at a time (a 5 s pan of
  16 px steps every 300 ms; the exe draws it at sub-pixel places).
- Zooms are whole percents (`spzoom`): one under 10 % is coarse, and
  one under 1 % is placed at 1 %, not 0 (`05_00_ARC05INTERLUDE`).
  Exact would need a finer `spzoom` in the engine.

### Open questions

Mapped commands, argument by argument, are in
`docs/games/tsukihime.findings.tsv` (67 seen, 16 inferred, 6 unknown).
Open there, by size:

- Motion keys' priority (0x3C, 23,115 calls) and expression (0x32/0x34,
  5,254), and the stepped fields 0x24-0x2A (shader, effect and colour
  changes mid-motion?).

## History

Done items: [docs/history/tsukihime.md](../history/tsukihime.md).
