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
  --romfsdir=<game-en> patch/<the patch's Program NCA>
```

It changes every archive but `allpachd`/`allpacml`/`allpachdml`:
`allscr.mrg` grows from 9 to 19 MB (553 entries), `allpac` gains 10
entries.

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
| `stream/*.opus` (16,601) | Voices (`A00_01_00_0000_0.opus`) and songs, 30.1 hours; Nintendo's Opus framing (`typemoon/nxopus.rs`): 48 kHz, 16,468 mono, 133 stereo, pre-skip 120, no sample count or loop points in the file. `typemoon sounds` repacks them into Ogg Opus (no re-encode). The same files in both RomFS copies |
| `movie/*.mp4` (46) | H.264 1920×1080 + AAC 48 kHz: ffmpeg reads them |
| `ja*/` | Fonts (TT_TsukuMin-D, TT_TsukuGo-D, FOT-SeuratPro-M) |
| `knife1.bfsar` (386 MB) | Nintendo sound archive (`typemoon/bfsar.rs`, `fwav.rs`): 1,297 wave sounds (`WSD_moon_se02093`, `WSD_woh_*`, `WSD_fate_*`, …; 1,300 in the patched copy, which adds `WSD_mix_se032`–`034`), all DSP ADPCM 48 kHz, 195 looping (107 from inside the wave); and 16,596 stream sounds `STRM_*`, whose files are the `stream/*.opus` above (their names are in the archive's file table). `typemoon sounds` writes each wave sound as `se/<name>.wav` |
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
  Sprites are atlases (`ARU_A_18_00_N01_10_10`: 886×4160, the body on
  top), to cut up later from the script's or the exe's numbers.

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
  takes a number (104 distinct), not a name. The table from numbers to
  `stream/MOON01_BGM*.opus` is not in the scripts or `BGML_TOP.MZP`:
  the exe is next. Arguments after a backquote (`` `001:4000``,
  `` `017:50``) look like options, number:value (time in ms, volume).
  Loop points are not in the scripts.

Japanese: 549 scripts, 1,079,001 commands, 93 command names. English
(the patched copy): 549 scripts, 1,082,931 commands, 96 names.

| Command | Uses | Scripts | First use (Japanese) | What it looks like |
|---|---|---|---|---|
| `_WKST` | 486528 | 534 | `_WKST(F823,eq,0)` | set a variable (`F`, `G`, `C` + number; `eq` = assign) |
| `_STCC` | 62606 | 519 | `_STCC(0)` |  |
| `_WNTY` | 58596 | 521 | `_WNTY(0)` |  |
| `_STCH` | 54334 | 521 | `_STCH(1,WHITE,240,136,1000,255,2,,CC)` | set a picture on a layer (layer, picture, x, y, …) |
| `_STCP` | 53915 | 510 | `_STCP(1,1)` |  |
| `_WKAD` | 45688 | 516 | `_WKAD(F823,1)` | add to a variable |
| `_ZM` | 42600 | 516 | `_ZM01602($000107)` | show text line `$n` (the `_Z?` commands carry a 5-hex-digit id) |
| `_FADS` | 34452 | 521 | `_FADS(1,0,0)` | fade / transition? |
| `_STZ4` | 32305 | 512 | `_STZ4(4,984,984,984,984,0,0,0)` |  |
| `_STMA` | 24309 | 485 | `_STMA(3,1)` |  |
| `_STRT` | 21829 | 516 | `_STRT(0,0,0,0,0)` |  |
| `_PGST` | 17202 | 519 | `_PGST(-1)` | page start? |
| `_VPLY` | 16690 | 483 | `_VPLY(D10_00_00_0000_0,03fb5)` | play voice `stream/<name>.opus` |
| `_STBR` | 14988 | 460 | `_STBR(4,23)` |  |
| `_SEPL` | 14451 | 480 | `_SEPL(0,1,MOON_SE03001,,`001:4000,,`017:50)` | play sound effect (channel, loop, `WSD_` name less its prefix, options) |
| `_WTTM` | 14424 | 508 | `_WTTM(1,1)` | wait (ms, ?) |
| `_STEC` | 12877 | 437 | `_STEC(12,48)` |  |
| `_CLO3` | 11627 | 504 | `_CLO3()` |  |
| `_SNX4` | 8787 | 435 | `_SNX4(4,11,11)` |  |
| `_STGS` | 7240 | 481 | `_STGS(0,0,0,0,0)` |  |
| `_STCF` | 5085 | 475 | `_STCF(0,8)` |  |
| `_SCH2` | 4800 | 425 | `_SCH2(7,img2343,,,,0,,,CC,,,,,,,,,,2)` |  |
| `_SEFD` | 3401 | 412 | `_SEFD(1,,,`001:2000)` | sound effect fade |
| `_FANX` | 2924 | 319 | `_FANX(1,0,0)` |  |
| `_ZZ` | 2569 | 549 | `_ZZ00a01()` | label (its name the argument) |
| `_JUMP` | 1766 | 544 | `_JUMP(240,START)` | jump (number, label) |
| `_SNX2` | 1654 | 249 | `_SNX2(35,27)` |  |
| `_SEVL` | 1353 | 210 | `_SEVL(0,,,,`001:3000,,`017:30)` | sound effect volume |
| `_ZY` | 1272 | 510 | `_ZY02100(LV00000)` | local label for `_IF__` (`LV…`) |
| `_SQK2` | 1228 | 236 | `_SQK2(-1,1,4,0,600,0,0,0,0)` |  |
| `_MSAD` | 1178 | 250 | `_MSAD($000120)` | add to the message log? |
| `_IF__` | 1164 | 507 | `_IF__(F026,ne,0,LV00000)` | if (variable, `eq`/`ne`/…, value) jump to local label |
| `_MPLY` | 1158 | 452 | `_MPLY(52,,`017:100)` | play music by number |
| `_CLO4` | 957 | 294 | `_CLO4(1000)` |  |
| `_SBAS` | 933 | 121 | `_SBAS(14,13)` |  |
| `_SHOK` | 865 | 165 | `_SHOK(1,3,100,18)` |  |
| `_SNX3` | 842 | 166 | `_SNX3(21,32)` |  |
| `_SELR` | 823 | 100 | `_SELR(0,$042569)` | choice option (index, text line) |
| `_MVOL` | 780 | 220 | `_MVOL(,,`001:5000,,`017:80)` | music volume |
| `_MFAD` | 724 | 364 | `_MFAD(,,`001:4000,,`011:0)` | music fade |
| `_STPV` | 712 | 173 | `_STPV(6,459,178,)` |  |
| `_CLO5` | 591 | 203 | `_CLO5(2000)` |  |
| `_STRA` | 584 | 84 | `_STRA(12,1)` |  |
| `_RDED` | 550 | 507 | `_RDED(486)` |  |
| `_RTM_` | 544 | 539 | `_RTM_()` | return |

The other 48: `_RDAS` 530; `_RDST` 530; `_RTST` 527; `_STRR` 447; `_ATSV` 398; `_SHAF` 397; `_SSW2` 205; `_SPMA` 195; `_STNS` 194; `_GOTS` 170; `_SELD` 170; `_SQS2` 132; `_SHK2` 120; `_GOTO` 101; `_SGMA` 100; `_SACL` 95; `_SOXY` 78; `_VCWT` 71; `_FADE` 62; `_IFNX` 53; `_IFP4` 52; `_TOMP` 48; `_TOMW` 48; `_SPRD` 41; `_SPRC` 40; `_STCG` 39; `_FCSP` 31; `_MSQK` 30; `_WTSV` 28; `_FCLN` 26; `_SELA` 26; `_STTR` 25; `_WKOR` 21; `_VPL2` 16; `_SESP` 15; `_STCL` 8; `_CLO2` 5; `_STQK` 5; `_SHS2` 3; `_IFSC` 2; `_TRPY` 2; `_XRSM` 2; `_XSTP` 2; `_IFUR` 1; `_RTM2` 1; `_RTMN` 1; `_SELS` 1; `_SSS2` 1.

**How the patched `allscr.mrg` differs.** It has 553 entries: the same
549 names and an empty 553rd entry. 28 scripts are byte-identical and
521 differ. It adds `_END_()` at the end of 519 scripts, `_SPFC` (487
uses in 16 scripts, `_SPFC(29,aki_a_30_01_n01,,,,,12,)`) and one
`_NOP_`. It drops 2,347 `_STCP` and adds `_STGS` +874, `_STCC` +654,
`_WTTM` +640, `_WKST` +636, `_CLO3` +581, `_SQS2`/`_SHS2`/`_SSS2` about
+400 each, and `_SACL` +355. The patch is an update NSP, so some of
this may be the official update rather than the translation. The text
numbering is the same.

## Story structure and walkthrough

### Walkthrough

Not written yet: two routes (Arcueid, Ciel) and their endings; the
source is still to be picked.

## Progression variables

Not mapped yet.

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
| Sound (`stream/*.opus`, `knife1.bfsar`) | ✅ every sound of both RomFS copies converts |
| Scripts | 🔶 read: every command of both languages splits (one malformed in the original) and its text and voices join; meanings not worked out yet |
| Everything after (text, pictures, screens, extras) | ❌ |

### Next

1. Work out the top commands' arguments (the picture layer commands
   `_STCH`/`_STCP`/`_STCC`/`_STZ4`/`_STMA`, `_FADS`, `_WNTY`), from the
   scripts and the exe (an NSO, Ghidra). In the exe, also find the
   music number table (`_MPLY`) and the loop points.
2. Write the walkthrough.
3. Pictures for the build: cut the sprite atlases (body and faces in
   one texture) by the numbers the script or the exe gives.
4. Sound for the build: the looping wave sounds' loop points
   (SDL_mixer loops a WAV whole), and where music loops (not in the
   Opus files or the scripts: the bfsar's stream info or the exe).

## History

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
