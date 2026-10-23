"""月姫's easing (FUN_00130c80, 1.0.2 main) run in the exe under Unicorn (research/tools/exe_call.py).

    research/tools/.venv/bin/python -I research/typemoon/ease_exe.py <main NSO> [--all]

Calls FUN_00130c80(t, b = 0, c = 1, d = 16, code) for code 0-41 and t 0-16
(floats in s0-s3, code in w0, result in s0) and prints the Rust table
`EXE_EASES` typemoon/convert.rs's tests check `ease` (conv-core `eased`)
against. The math imports (pow, sin, cos, sqrt, exp2 and their float forms)
run as Python's: exe_call returns 0 from an import. --all: code 0-48.
"""
import math
import os
import struct
import sys

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "tools"))
from exe_call import Exe  # noqa: E402
from unicorn import arm64_const as A64  # noqa: E402

exe = Exe(sys.argv[1])
Q = [A64.UC_ARM64_REG_Q0, A64.UC_ARM64_REG_Q1]


def math_stub(fn, n, single):
    fmt = "<f" if single else "<d"
    size = 4 if single else 8

    def run(e, _):
        args = [struct.unpack(fmt, e.uc.reg_read(Q[i]).to_bytes(16, "little")[:size])[0] for i in range(n)]
        e.uc.reg_write(Q[0], int.from_bytes(struct.pack(fmt, fn(*args)), "little"))
        return 0
    return run


for name, fn, n in [("pow", math.pow, 2), ("sin", math.sin, 1), ("cos", math.cos, 1), ("sqrt", math.sqrt, 1),
                    ("exp2", lambda x: 2.0 ** x, 1)]:
    exe.stub_import(name, math_stub(fn, n, False))
    if name != "exp2":
        exe.stub_import(name + "f", math_stub(fn, n, True))

N = 16
for code in range(49 if "--all" in sys.argv else 42):
    v = [exe.call(0x130c80, [float(k), 0.0, 1.0, float(N), code], ret="float")[0] for k in range(N + 1)]
    print(f"    ({code}, [{', '.join(f'{x:.7}' for x in v)}]),")
