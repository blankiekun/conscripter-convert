"""The eboot's ease_value (0x8103cc18: from, to, t, power, type), run under
Unicorn (research/tools/exe_call.py), printed as the Rust table
`EBOOT_EASE` that crates/conv-engines/src/mages/convert.rs's tests check
`eboot_ease` against.

    research/tools/.venv/bin/python -I research/oretsuba/ease_exe.py <eboot.elf>

The libm pow it calls (import thunk 0x81131394, SceLibm NID 0x6DEA815A)
runs as a stub doing powf(s0, s1) in single precision; every other call
would be reported.
"""
import os
import struct
import sys

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "tools"))
from exe_call import Exe  # noqa: E402
from unicorn import arm_const as A  # noqa: E402

EASE_VALUE = 0x8103cc18
POW = 0x81131394


def f32(x):
    return struct.unpack("<f", struct.pack("<f", x))[0]


def powf(exe, _):
    u = exe.uc
    get = lambda r: struct.unpack("<f", struct.pack("<I", u.reg_read(r)))[0]  # noqa: E731
    x, p = get(A.UC_ARM_REG_S0), get(A.UC_ARM_REG_S1)
    u.reg_write(A.UC_ARM_REG_S0, struct.unpack("<I", struct.pack("<f", f32(x ** p)))[0])
    return 0


exe = Exe(sys.argv[1])
exe.stubs[POW | 1] = powf
POWERS = [0.5, 1.0, 1.5, 2.0, 3.0]
print("    // (type, power, [share at t = 0, 0.1, ..., 1])")
for ty in range(4):
    for p in POWERS:
        v = [exe.call(EASE_VALUE, [0.0, 1.0, k / 10, p, ty], ret="float")[0] for k in range(11)]
        print(f"    ({ty}, {p!r}, [{', '.join(f'{x!r}' for x in v)}]),")
assert all(h[0] == POW for h in exe.hits), exe.hits
