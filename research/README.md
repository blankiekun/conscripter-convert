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
| `decrypt_0cd.py <0.cd> <out>` | Recovers the 256-byte XOR key of 0.cd from byte frequencies and decrypts it |
| `voice_align.py <game dir> <out.json>` | Statistically aligns voice ids to .xsa clips by text length vs clip length (needs numpy) |
