"""The eboot's EffectQuake (effect_quake_a 0x810089a4 starts a record,
effect_quake_b 0x81008a3c gives each update's offset), run under Unicorn
(research/tools/exe_call.py), printed as the Rust table `EBOOT_QUAKE` that
crates/conv-engines/src/mages/convert.rs's tests check `quake_frames`
against.

    research/tools/.venv/bin/python -I research/oretsuba/quake_exe.py <eboot.elf>

effect_quake_a takes its arguments as scene_load case 99 (0x8102840a) passes
them: the record, `:Time` (made positive), the fade (`:Time` with `:Fade 1`,
else 0), the loop flag, then `:Param` 0-5 as floats (core registers and the
stack: raw 4-byte words). Imports: memcpy (0x81131b54) and powf
(0x81131394, through ease_value) run as stubs; any other call is reported.
"""
import os
import struct
import sys

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "tools"))
import exe_call  # noqa: E402
from exe_call import Exe  # noqa: E402
from unicorn import arm_const as A  # noqa: E402

QUAKE_A, QUAKE_B = 0x810089a4, 0x81008a3c
POW, MEMCPY = 0x81131394, 0x81131b54
REC = exe_call.HEAP + 0x80000


def bits(x):
    return struct.unpack("<I", struct.pack("<f", x))[0]


def powf(exe, _):
    u = exe.uc
    get = lambda r: struct.unpack("<f", struct.pack("<I", u.reg_read(r)))[0]  # noqa: E731
    x, p = get(A.UC_ARM_REG_S0), get(A.UC_ARM_REG_S1)
    r = struct.unpack("<f", struct.pack("<f", x ** p))[0]
    u.reg_write(A.UC_ARM_REG_S0, bits(r))
    return 0


def memcpy(exe, _):
    u = exe.uc
    d, s, n = (u.reg_read(r) for r in (A.UC_ARM_REG_R0, A.UC_ARM_REG_R1, A.UC_ARM_REG_R2))
    u.mem_write(d, bytes(u.mem_read(s, n)))
    return d


exe = Exe(sys.argv[1])
exe.stubs[POW | 1] = powf
exe.stubs[MEMCPY | 1] = memcpy

# (time, fade, loop, [:Param 0-5], updates)
CASES = [
    (60, 0, 0, [10, 6, 0, 0, 0, 0], 62),
    (60, 60, 0, [10, 6, 0, 0, 0, 0], 62),
    (90, 0, 0, [0, 0, 8, 4, 3, 4], 92),
    (90, 90, 0, [0, 0, 8, 4, 3, 4], 92),
    (40, 0, 1, [0, 0, 5, 5, 2, 3], 100),
    (40, 0, 1, [0, 0, 6, 0, 1, 0], 100),
    (50, 50, 0, [12, 0, 6, 3, 2, 1], 52),
]
print("    // (time, fade, loop, [:Param 0-5], [(x, y) each update while it runs])")
for time, fade, loop, p, n in CASES:
    exe.uc.mem_write(REC, bytes(0x40))
    exe.call(QUAKE_A, [REC, time, fade, loop] + [bits(float(v)) for v in p])
    out = []
    for _ in range(n):
        r, (bx, by) = exe.call(QUAKE_B, [REC, ("buf", 4), ("buf", 4)])
        if r & 0xff == 0:
            break
        x, y = (struct.unpack("<i", exe.read(b[0], 4))[0] for b in (bx, by))
        out.append((x, y))
    print(f"    ({time}, {fade}, {loop}, {p}, &{out}),".replace(" ", " "))
assert all(h[0] in (POW, MEMCPY) for h in exe.hits), exe.hits[:5]
