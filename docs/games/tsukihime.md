# 月姫 -A piece of blue glass moon- (TYPE-MOON)

The 2021 remake of 月姫, Nintendo Switch release (title ID
`01001DC01486A000`, v0 base). TYPE-MOON's own engine, the family of
the 魔法使いの夜 remake.

## Scope

The user owns the Switch release and dumped it themselves (NSP plus
their console's `prod.keys`). The user asked for (2026-10-16): the
whole game **with every extra**, on **every platform** (web, Linux,
Windows, macOS). Nothing has been dropped yet.

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
| `stream/*.opus` (16,601) | Voices (`A00_01_00_0000_0.opus`) and songs; Nintendo's Opus framing (header word `0x80000001`), not Ogg: ffmpeg can't read them as they are |
| `movie/*.mp4` (46) | H.264 1920×1080 + AAC 48 kHz: ffmpeg reads them |
| `ja*/` | Fonts (TT_TsukuMin-D, TT_TsukuGo-D, FOT-SeuratPro-M) |
| `knife1.bfsar` (386 MB) | Nintendo sound archive: 1,298 embedded FWAV, no FSTM (sound effects, maybe BGM) |
| `shader/*.bnsh`, `exfont_nx01.bffnt` | Switch shaders and font, not needed |

How the formats nest, checked on our files (2026-10-16):

- **MRG** (`.hed`/`.mrg`/`.nam`): the `.hed` gives each entry's place
  in the `.mrg`, the `.nam` its name.
- **MZP** (`mrgd00`, u16 entry count, then 8-byte entries
  `u16 sector, u16 byte, u16 size_sectors, u16 size_low`, sector 0x800;
  size = `(size_sectors*0x800 & ~0xFFFF) | size_low`; data starts after
  the table).
- **MZX0**: an LZ-like compression (the scripts).
- **NXGX/NXCX** (`.NXGZ`/`.NXZ`): a 16-byte header (magic, sizes),
  then gzip (NXGX) or deflate (NXCX). An `allui` picture inflates to
  **BNTX**, Nintendo's texture container: swizzled (Tegra block-linear)
  and probably BCn-compressed, so pictures need a deswizzle and a BC
  decoder.

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

Not covered by anything found: Nintendo Opus to Ogg Opus (simple:
repack the frames into Ogg pages, as vgmstream reads them),
FWAV extraction from `knife1.bfsar` (vgmstream and Switch Toolbox read
it; a Rust reader is small), BNTX deswizzle and BC decoding (Rust
crates exist: `tegra_swizzle` and a BCn decoder; adding them is its own
commit).

## Script language

Not decoded yet.

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
| Archives (`hed`/`mrg`/`nam`, MZP, NXGX/NXZ pictures) | ❌ |
| `knife1.bfsar` sound | ❌ |
| Scripts | ❌ |
| Everything after (text, pictures, screens, extras) | ❌ |

### Next

1. The archive reader (`crates/conv-engines/src/typemoon/`): MRG, MZP,
   MZX, NXGX/NXCX, with `list`/`extract`, checked over every entry
   (mangetsu as the reference).
2. Pictures: BNTX deswizzle and BC decode, every picture decoding.
3. Sound: Nintendo Opus to Ogg Opus; FWAV out of `knife1.bfsar`.
4. Decode `allscr.mrg` (MZX, split into commands), join the text from
   `script_text.mrg`, rank the commands by use.
5. Write the walkthrough.

## History

- 2026-10-16: tool survey (mangetsu, deepLuna, PS-HuneX_Tools);
  the nesting of MZP, MZX and NXGX→BNTX checked on our files.
- 2026-10-16: NSP unpacked, engine identified, this document started.
