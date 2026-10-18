"""muramasa_en.exe's tempo curves, run in the exe under Unicorn (research/tools/exe_call.py).

    research/tools/.venv/bin/python -I research/nitroplus/en_tempo_exe.py <muramasa_en.exe> [--all]

Calls table[id](&from, &to, &time, &elapsed) from the 161-entry table at
0x5e40a0 (docs/games/muramasa-en.md, "Eased fades") with from 0, to 1,
time 16 and elapsed 0-16, and prints the ids the scripts use as the Rust
table `EXE_TEMPOS` in muramasa.rs's tests checks `diesel_tempo` against;
--all prints every id's shares at t = 1/4, 1/2, 3/4.
"""
import os
import sys

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "tools"))
from exe_call import Exe  # noqa: E402

TABLE = 0x5e40a0
USED = [("null", 0), ("Axl1", 1), ("Dxl1", 2), ("Axl2", 3), ("Dxl2", 4), ("Axl3", 5), ("Dxl3", 6),
        ("AxlDxl", 17), ("DxlAxl", 18), ("AxlAuto", 129), ("DxlAuto", 130)]

exe = Exe(sys.argv[1])


def share(i, k, n=16):
    return exe.call(exe.ptr(TABLE + 4 * i), [("&", "f", 0.0), ("&", "f", 1.0), ("&", "f", float(n)), ("&", "f", float(k))], ret="float")[0]


if "--all" in sys.argv:
    for i in range(161):
        print(i, hex(exe.ptr(TABLE + 4 * i)), *(f"{share(i, k, 4):.6f}" for k in (1, 2, 3)))
else:
    for name, i in USED:
        print(f'    ("{name}", [{", ".join(f"{share(i, k):.7}" for k in range(17))}]),')
assert not exe.hits, exe.hits
