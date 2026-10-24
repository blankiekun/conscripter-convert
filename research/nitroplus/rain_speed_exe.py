"""How fast Muramasa.exe's CreateEffect "Rain" streak falls, run in the exe under Unicorn.

    research/tools/.venv/bin/python -I research/nitroplus/rain_speed_exe.py <Muramasa.exe> [speed ...]

Runs the real Rain update FUN_00436390 on one drop whose path FUN_00414060
builds from (100, -200) to (130, 288), in the frame loop of FUN_0044b850:
each frame stamps the object (+0x9b4) with timeGetTime only after calling
the update, so the update's FUN_00449930 adds the time since the previous
frame (one frame, not the 42 ms since the last update) times speed / 16
(FUN_0048c5d0: frame period table, entry 4 = 16 ms; game speed 1.0 from
FUN_0047b400) to the drop's 16.16 path index. Prints, for frame periods
16 and 17 ms, the drop's y each update and the px per ms over the path.
"""
import os
import struct
import sys

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "tools"))
from exe_call import Exe  # noqa: E402

OBJ, RAIN, TEX, PIX, ROWS = 0x10000000, 0x10100000, 0x10800000, 0x10900000, 0x10a00000
W, H = 512, 288


def run(path, speed, period, frames=60):
    exe = Exe(path)
    uc = exe.uc
    for base, size in ((OBJ, 0x100000), (RAIN, 0x700000), (TEX, 0x100000), (PIX, 0x100000), (ROWS, 0x100000)):
        uc.mem_map(base, size)
    exe.call(0x48c5d0, [], regs={"esi": 0x5dcad8})  # frame period table and its index (4)
    # FUN_0047b400: game speed table entry 3 = 1.0, selected; state 0x100 (playing)
    uc.mem_write(0x5d4148, struct.pack("<d", 1.0))
    uc.mem_write(0x5d4074, struct.pack("<I", 3))
    uc.mem_write(0x5d4040, struct.pack("<I", 0x100))
    period_ms = struct.unpack("<I", exe.read(0x5dcb0c + 4 * exe.read(0x5dcaf0, 1)[0], 4))[0]
    # the drop: path points (x, y, z) at +0xc08, end marked -0x8000 (FUN_00435e60)
    uc.mem_write(RAIN + 0xc08, struct.pack("<i", -0x8000) * 3 * 4000)
    exe.call(0x414060, [100, -200, 0, 130, H, 0], regs={"eax": RAIN + 0xc08})
    ys = [struct.unpack("<3i", exe.read(RAIN + 0xc08 + 12 * k, 12))[1] for k in range(600)]
    npts = next(k for k, y in enumerate(ys) if y == -0x8000)
    uc.mem_write(RAIN, struct.pack("<II", 0, 1))  # last update, drop count
    uc.mem_write(RAIN + 8 + 0x400, struct.pack("<I", speed))
    uc.mem_write(RAIN + 0x5dcc08, struct.pack("<I", TEX))
    uc.mem_write(TEX + 8, struct.pack("<II", W, H))
    uc.mem_write(TEX + 0x28, struct.pack("<II", PIX, ROWS))
    uc.mem_write(ROWS, b"".join(struct.pack("<I", PIX) for _ in range(H)))  # all rows on one: only the index matters
    uc.mem_write(OBJ + 0x12b0, struct.pack("<I", RAIN))
    exe.call(0x48c5d0, [], regs={"esi": 0x5dcad8})
    now = [0]
    exe.stub_import("WINMM.dll!timeGetTime", lambda e, at: now[0])
    exe.stubs[0x449de0] = 0  # the draw request
    t0, prev, out = 1000, None, []
    for n in range(frames):
        t = t0 + n * period
        now[0] = t
        if prev is not None:
            uc.mem_write(OBJ + 0x9b4, struct.pack("<I", prev))
            last = struct.unpack("<I", exe.read(RAIN, 4))[0]
            exe.call(0x436390, [OBJ])
            if struct.unpack("<I", exe.read(RAIN, 4))[0] != last:
                idx = struct.unpack("<I", exe.read(RAIN + 8, 4))[0]
                out.append((t, idx >> 16, idx))
        prev = t
    return period_ms, npts, out


def main():
    path = sys.argv[1]
    for speed in [int(s) for s in sys.argv[2:]] or [152, 64]:
        for period in (16, 17):
            period_ms, npts, out = run(path, speed, period)
            # path points moved between updates (the index goes back to 0 past the path's end)
            steps = [(b[0] - a[0], b[1] - a[1]) for a, b in zip(out, out[1:]) if b[1] > a[1]]
            dt, dp = sum(s[0] for s in steps), sum(s[1] for s in steps)
            print(f"speed {speed} frame {period} ms (table {period_ms}): path {npts} points;"
                  f" updates at {[o[0] - 1000 for o in out[:5]]} ms, index {[o[1] for o in out[:5]]};"
                  f" {dp / dt:.4f} px/ms = speed/{speed * dt / dp:.2f}")


main()
