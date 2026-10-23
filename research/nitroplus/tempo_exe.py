"""Muramasa.exe's tempo curves, run in the exe under Unicorn (research/tools/exe_call.py).

    research/tools/.venv/bin/python -I research/nitroplus/tempo_exe.py <Muramasa.exe>

apply_tempo 00449f40 (fastcall, the tempo name) returns the table builder
for a name; the builder (cdecl: &table, steps, distance) fills table[0..steps]
with distance * curve(i / steps). Its allocation FUN_0045a0b0 is stubbed and
the table pointer preset. Prints the builders' shares for steps 16 as the
Rust table `EXE_TEMPOS` in convert.rs's tests checks `tempo` + `eased` against.
"""
import os
import struct
import sys

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "tools"))
from exe_call import Exe, HEAP  # noqa: E402

APPLY_TEMPO, ALLOC = 0x449f40, 0x45a0b0
TABLE, STEPS, DIST = HEAP + 0x80000, 16, 1_000_000
NAMES = ["null", "Axl1", "Axl2", "Axl3", "Dxl1", "Dxl2", "Dxl3", "AxlAuto", "DxlAuto", "AxlDxl", "DxlAxl", "Linear"]

exe = Exe(sys.argv[1])
exe.stubs[ALLOC] = 1
for name in NAMES:
    builder = exe.call(APPLY_TEMPO, [("&", f"{len(name) + 1}s", name.encode())], cc="fastcall")[0]
    exe.uc.mem_write(TABLE, bytes(8 * (STEPS + 1)))
    exe.call(builder, [("&", "I", TABLE), STEPS, DIST])
    shares = struct.unpack(f"<{STEPS + 1}d", exe.read(TABLE, 8 * (STEPS + 1)))
    print(f'        ("{name}", [{", ".join(f"{s / DIST:.7}" for s in shares)}]),  // {builder:#x}')
