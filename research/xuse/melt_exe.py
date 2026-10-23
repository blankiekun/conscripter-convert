"""Saihate's MeltDown transition (Trans_MeltDown), run in the exe under Unicorn
(research/tools/exe_call.py), printed as the Rust tables warp.rs's tests check
its port against.

    research/tools/.venv/bin/python -I research/xuse/melt_exe.py <Farthest2015.exe>

- 0x4451e0: the per-row acceleration (16.16) of the byte 0, 128, 255 for a
  600 px screen;
- 0x445310: the melting edge, a spline through 11 points `rand() & h`, scaled
  to -h..0, for w x h = 800 x 600 with MSVC's rand() from seed 1 (the exe
  seeds it from the clock, srand(_time64) at 0x407150);
- 0x4458e0: the draw, on a 16 x 12 picture whose pixel holds its row + 1
  (0: the new picture shows), edge from seed 1, acceleration of 255, at
  progress 0, 1/4, 1/2, 3/4 (16.16).
"""
import struct
import sys
import os

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "tools"))
from exe_call import Exe, HEAP  # noqa: E402

exe = Exe(sys.argv[1])
MINE = HEAP + 0x40000   # buffers of ours (exe_call's arguments start at HEAP)
ALLOC = [HEAP + 0x80000]
state = [1]


def arg(sp, i):
    return struct.unpack("<I", exe.read(sp + 4 * i, 4))[0]


def new(e, sp):
    a = ALLOC[0]
    ALLOC[0] += (arg(sp, 0) + 15) & ~15
    e.uc.mem_write(a, bytes(arg(sp, 0)))
    return a


def memset(e, sp):
    e.uc.mem_write(arg(sp, 0), bytes([arg(sp, 1) & 255]) * arg(sp, 2))
    return arg(sp, 0)


def memcpy(e, sp):
    e.uc.mem_write(arg(sp, 0), e.read(arg(sp, 1), arg(sp, 2)))
    return arg(sp, 0)


def rand(e, sp):  # MSVC
    state[0] = (state[0] * 214013 + 2531011) & 0xffffffff
    return state[0] >> 16 & 0x7fff


exe.stubs[0x4e7726] = new                 # operator new
exe.stub_import("MSVCR90.dll!memset", memset)
exe.stub_import("MSVCR90.dll!memcpy", memcpy)
exe.stub_import("MSVCR90.dll!rand", rand)
exe.uc.mem_write(0x4e840a, b"\xd9\xfa\xc3")  # the _CIsqrt thunk: fsqrt; ret


def i32s(addr, n):
    return list(struct.unpack(f"<{n}i", exe.read(addr, 4 * n)))


def accel(byte, h):
    return exe.call(0x4451e0, [h, byte], cc="thiscall")[0]


def edge(w, h, seed):
    state[0] = seed
    exe.uc.mem_write(MINE, bytes(4 * w))
    exe.call(0x445310, [MINE, w, h])
    return i32s(MINE, w)


print("    // research/xuse/melt_exe.py: 0x4451e0 of 0, 128, 255 at h 600")
print(f"    const EXE_ACCEL: [u32; 3] = {[accel(b, 600) for b in (0, 128, 255)]};")
e = edge(800, 600, 1)
print("    // 0x445310 at 800 x 600, seed 1: every 25th column")
print(f"    const EXE_EDGE: [i32; 32] = {e[::25]};")

W, H = 16, 12
off = edge(W, H, 1)
a = accel(255, H)
print(f"    // 0x4458e0 on {W} x {H} (edge {off}, accel {a}): source row + 1, 0 the new picture")
print(f"    const EXE_DRAW: [[[u8; {W}]; {H}]; 4] = [")
SRC, DST, OFF, ST, BOX = MINE + 0x1000, MINE + 0x2000, MINE + 0x3000, MINE + 0x3800, MINE + 0x3c00
src = b"".join(bytes([y + 1, 0, 0]) for y in range(H) for _ in range(W))
exe.uc.mem_write(OFF, struct.pack(f"<{W}i", *off))
for q in (32768, 40960, 49152, 57344):
    exe.uc.mem_write(SRC, src)
    exe.uc.mem_write(DST, bytes(len(src)))
    exe.uc.mem_write(ST, bytes(8 * W))
    exe.uc.mem_write(BOX, struct.pack("<7I", W, H, DST, 3 * W, SRC, 3 * W, 3))
    exe.call(0x4458e0, [BOX, q, a, OFF, ST])
    d = exe.read(DST, len(src))
    rows = [[d[3 * (y * W + x)] for x in range(W)] for y in range(H)]
    print("        [" + ", ".join(str(r) for r in rows) + "],")
print("    ];")
bad = [h for h in exe.hits if h[1] not in ("MSVCR90.dll!memset", "MSVCR90.dll!memcpy", "MSVCR90.dll!rand", "MSVCR90.dll!??3@YAXPAX@Z", "MSVCR90.dll!??_V@YAXPAX@Z") and h[0] != 0x4e7726]
assert not bad, bad
