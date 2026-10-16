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
| `allpac.{hed,mrg,nam}` (16 GB) | Main asset archive: `.hed` index, `.mrg` data, `.nam` names (`*.mzp`, `*.NXZ`) |
| `allpaccg`, `allpachd`, `allpacml`, `allpachdml` (same triple) | CG; others not yet looked at (`ml` likely the multi-language copies) |
| `allui.{hed,mrg,nam}` (71 MB) | UI; entries start `NXGX` + a gzip stream |
| `parts.{hed,mrg}` | not yet looked at |
| `allscr.mrg` (9 MB), `script_text.mrg` (5 MB) | Scripts and their text; `mrgd00` archives without a separate index |
| `stream/*.opus` (16,601) | Voices (`A00_01_00_0000_0.opus`: scene, line) and songs |
| `movie/*.mp4` (46) | Openings, endings, credits, phase titles |
| `ja*/` | Fonts (TT_TsukuMin-D, TT_TsukuGo-D, FOT-SeuratPro-M) |
| `knife1.bfsar` (386 MB) | Nintendo sound archive (BGM/SE, presumably) |
| `shader/*.bnsh`, `exfont_nx01.bffnt` | Switch shaders and font, not needed |

Existing work to check before reversing anything (step 0.3): the
fan translation tools for this release (deepLuna, for
`script_text.mrg`) and the 魔法使いの夜 remake tools, which read the
same `mrgd00`/MZP formats.

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
| Archives (`hed`/`mrg`/`nam`, MZP, NXGX/NXZ pictures) | ❌ |
| `knife1.bfsar` sound | ❌ |
| Scripts | ❌ |
| Everything after (text, pictures, screens, extras) | ❌ |

### Next

1. Survey the existing tools for the `mrgd00`/MZP formats and
   `script_text.mrg`; write down what they cover.
2. The archive reader (`crates/conv-engines/src/typemoon/`) with
   `list`/`extract`, checked over every entry.
3. Picture decoding (NXGX, NXZ) and `knife1.bfsar`.
4. Decode `allscr.mrg`, measure its commands.
5. Write the walkthrough.

## History

- 2026-10-16: NSP unpacked, engine identified, this document started.
