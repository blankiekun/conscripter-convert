# Research scripts

Throwaway Python used while reverse-engineering formats. Findings that hold
up move into the Rust crates; these stay as a record of how they were found.

They read files from your own install (paths are arguments) and contain no
game data. Run them with `python3 -I` from outside the game directory.

## xuse/ (最果てのイマ COMPLETE)

| Script | What it does |
|---|---|
| `xarc_check.py <arc…>` | Tests CRC hypotheses on XARC archives (found CRC-16/XMODEM) |
| `xsa_probe.py <xsa…>` | Checks the .xsa slot table across files |
| `cdlib.py` | Helpers for .cd scripts: load sections, scan pool strings |
| `decrypt_0cd.py <0.cd> <out>` | Decrypts 0.cd with the cipher found in the exe |
| `gafkey.py` | GAF4 key generator and XOR from the exe |
| `nstrace.py <0.txt> [steps] [--show]` | Runs the NScripter subset conscripter-convert emits (labels, goto, gosub/return, arithmetic, if) to find loops or trace the path to the first text page |
| `gaf_extract.py <outdir> <file.004…>` | Extracts the PNGs from GAF4 files (needs `gafkey.py`) |
| `voice_align.py <game dir> <out.json>` | Statistically aligns voice ids to .xsa clips by text length vs clip length (needs numpy) |

## nitroplus/ (装甲悪鬼村正)

| File | What it is |
|---|---|
| `npa.py <exe> <npa> [out [glob]]` | Lists or extracts the encrypted NPA archives |
| `builtins.md` | What each engine built-in does, with use counts (`conscripter-convert nss builtins`) and the NScripter mapping |

## oretsuba/ (俺たちに翼はない, PS Vita)

| Script | What it does |
|---|---|
| `cpk.py <file> [out [glob]]` | Lists or extracts CRI archives: CPK (TOC with names, or ITOC with IDs only, as in the `.AWB` sound banks) and AFS2 (`VOICE.AWB`); decompresses CRILAYLA |
| `atlas.py <file.png> [first_row]` | Prints the boxes of the opaque parts of a SystemData atlas (menu labels, buttons), to cut them out |
