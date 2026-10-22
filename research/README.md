# Research scripts

Throwaway Python for reverse-engineering formats. What holds up moves
into the Rust crates; the scripts stay as the record of how it was found.
Paths are arguments, no game data is inside. Run `python3 -I` from
outside the game directory. Shared helpers (traces, scratch runs, saves,
diffs): `research/tools/`, see `docs/tools.md`. `local/` under each
engine is git-ignored.

Listed below: the scripts to start from. Most others in each folder open
with a docstring; `ls` the folder.

## tools/ghidra (any game)

`ghidra/ghidra_export.sh <binary> <outdir> [--names <names.tsv>] [--rtti] [-- <import options>]`
decompiles a binary with Ghidra (headless; `analyzeHeadless` on `PATH` or
in `$GHIDRA_INSTALL_DIR/support`) into one pseudo-C file per function
with callers, callees and cp932 strings, plus `functions.tsv` and
`strings.tsv`. A names file is applied first (`ApplyNames.java`), so
identified functions keep their names; later runs reuse the analyzed
project.

## xuse/ (最果てのイマ COMPLETE)

| Script | What it does |
|---|---|
| `xarc_check.py <arc…>` | Tests CRC hypotheses on XARC archives (found CRC-16/XMODEM) |
| `xsa_probe.py <xsa…>` | Checks the .xsa slot table across files |
| `cdlib.py` | Helpers for .cd scripts: load sections, scan pool strings |
| `decrypt_0cd.py <0.cd> <out>` | Decrypts 0.cd with the cipher found in the exe |
| `gafkey.py` | GAF4 key generator and XOR from the exe |
| `gaf_extract.py <outdir> <file.004…>` | Extracts the PNGs from GAF4 files (needs `gafkey.py`) |
| `nstrace.py <0.txt> [steps] [--show]` | Runs the NScripter subset conscripter-convert emits (labels, goto, gosub/return, arithmetic, if) to find loops or trace the path to the first text page |
| `smoke.py` | Runs a converted game headless in the built engine with scripted clicks and screenshots (docstring; expects the title and menus `conv-cons` writes) |
| `voice_align.py <game dir> <out.json>` | Aligns voice ids to .xsa clips statistically, text length vs clip length (needs numpy) |
| `names.tsv` | Names for `Farthest2015.exe`'s functions and globals (`ghidra_export.sh --names`) |

## nitroplus/ (装甲悪鬼村正, 続・殺戮のジャンゴ)

| File | What it is |
|---|---|
| `npa.py <exe> <npa> [out [glob]]` | Lists or extracts the encrypted NPA archives |
| `builtins.md` | What each engine built-in does, with use counts (`conscripter-convert nss builtins`) and the NScripter mapping |

## oretsuba/ (俺たちに翼はない, PS Vita)

| Script | What it does |
|---|---|
| `cpk.py <file> [out [glob]]` | Lists or extracts CRI archives: CPK (TOC with names, or ITOC with IDs only, as in the `.AWB` sound banks) and AFS2 (`VOICE.AWB`); decompresses CRILAYLA |
| `atlas.py <file.png> [first_row]` | Prints the boxes of the opaque parts of a SystemData atlas (menu labels, buttons), to cut them out |
| `nstrace.py <0.txt> <path>...` | Runs a converted 0.txt's control flow along choice paths (`S01_02C:2,…,=ending scene`) and reports the ending each run reaches |
