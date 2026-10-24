"""月姫's motion values (FUN_00197710, 1.0.2 main) run in the exe under Unicorn (research/tools/exe_call.py).

    research/tools/.venv/bin/python -I research/typemoon/spline_exe.py <main NSO> M [M ...] [--used] [--keys] [--fields=3,5]

For each motion M, calls FUN_00197710(t, current = 0, field, first, last, k,
flag) as FUN_00197b80 calls it (first: the motion's first key, last: its last,
k: the first key at or after t, flag: M == 9999, FUN_00199950) for exe fields
3-27 at each key's time and 7 times between, and prints one line per motion:
`(M, [(field, t, value), ...])`. Mode 1 (key field 2) runs FUN_001972c0's
cubic spline; typemoon/exe.rs's tests check `Motion::value_at` against these.
memset (an import) is stubbed to really zero: exe_call would skip it and leave the last
call's spline in the arrays. --fields: only those exe fields. --keys: the motion's keys too, as `Key { time_ms, fields }` rows. --used:
the 48 mode-1 motions the scripts run (found by a dry-run convert).
Values to 4 places; t is whole ms (value_at takes u32).
"""
import math
import os
import struct
import sys

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "tools"))
from exe_call import Exe  # noqa: E402
from unicorn import arm64_const as A64  # noqa: E402

USED = [1692, 1905, 2577, 3925, 4250, 4251, 4252, 4253, 4301, 4306, 4343, 4352, 4353, 4354, 4637, 4638, 4639, 6920,
        7833, 7843, 8383, 8384, 8421, 8432, 11296, 11297, 15654, 20359, 20386, 21330, 21332, 22825, 22826, 22829, 22830,
        22831, 22832, 22974, 22975, 23080, 23081, 23082, 23985, 24125, 24126, 24137, 24138, 24142]
KEY_END, KEYS = 0x4B1940, 0x75CE18
args = [a for a in sys.argv[2:] if not a.startswith("--")]
FIELDS = next((list(map(int, a[9:].split(","))) for a in sys.argv if a.startswith("--fields=")), range(3, 28))
exe = Exe(sys.argv[1])
Q = [A64.UC_ARM64_REG_Q0, A64.UC_ARM64_REG_Q1]


def math_stub(fn, n, single):
    fmt = "<f" if single else "<d"
    size = 4 if single else 8

    def run(e, _):
        a = [struct.unpack(fmt, e.uc.reg_read(Q[i]).to_bytes(16, "little")[:size])[0] for i in range(n)]
        e.uc.reg_write(Q[0], int.from_bytes(struct.pack(fmt, fn(*a)), "little"))
        return 0
    return run


for name, fn, n in [("pow", math.pow, 2), ("sin", math.sin, 1), ("cos", math.cos, 1), ("sqrt", math.sqrt, 1),
                    ("exp2", lambda x: 2.0 ** x, 1)]:
    exe.stub_import(name, math_stub(fn, n, False))
    if name != "exp2":
        exe.stub_import(name + "f", math_stub(fn, n, True))


def memset(e, _):  # an import: exe_call would skip it, leaving the last call's spline in the zeroed arrays
    dst, val, n = (e.uc.reg_read(r) for r in (A64.UC_ARM64_REG_X0, A64.UC_ARM64_REG_X1, A64.UC_ARM64_REG_X2))
    e.uc.mem_write(dst, bytes([val & 0xff]) * n)
    return dst


exe.stub_import("memset", memset)


def u32(a):
    return struct.unpack("<I", exe.read(a, 4))[0]


for m in [int(a) for a in args] + (USED if "--used" in sys.argv else []):
    first, end = u32(KEY_END + 4 * (m - 2)), u32(KEY_END + 4 * (m - 1))
    raw = [exe.read(KEYS + 0x40 * k, 0x40) for k in range(first, end)]
    times = [struct.unpack_from("<I", r)[0] for r in raw]
    ts = sorted({a + (b - a) * s // 8 for a, b in zip(times, times[1:]) for s in range(8)} | set(times))
    out = []
    for f in FIELDS:
        for t in ts:
            k = first + next(i for i, x in enumerate(times) if x >= t)
            v = exe.call(0x197710, [float(t), 0.0, f, first, end - 1, k, int(m == 9999)], ret="float")[0]
            out.append(f"({f}, {t}, {v:.4f})")
    if "--keys" in sys.argv:
        for r in raw:
            print(f"    Key {{ time_ms: {struct.unpack_from('<I', r)[0]}, fields: {list(struct.unpack_from('<30h', r, 4))} }},")
    print(f"    ({m}, &[{', '.join(out)}]),")
