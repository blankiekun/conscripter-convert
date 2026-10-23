"""Muramasa.exe's camera projection, run in the exe under Unicorn (research/tools/exe_call.py).

    research/tools/.venv/bin/python -I research/nitroplus/camera_exe.py <Muramasa.exe>

Calls FUN_00489570 (an object's vertices to screen shorts; EAX the vertex
record: [0] count, doubles x y z at +8 + 0x18 i, the depth at +0xa0, turn
angles +0xa8..+0xb8 and +0xc0..+0xc8, zoom per mille +0xcc..+0xd4, offset
+0xd8 +0xe0; screen x shorts out at +0x80, y at +0x88) with arg 1 the camera
record (+0x14 x, +0x18 y, +0x1c z, +0x20..+0x28 angles), arg 3 4 the
object's place. One vertex at the object's place: prints (x, y, camera x, y,
z, depth, screen x, screen y) rows as the Rust table `EXE_CAMERA` in
convert/drawing.rs's tests.
"""
import os
import struct
import sys

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "tools"))
from exe_call import Exe, HEAP  # noqa: E402

PROJECT = 0x489570
VERT, CAM = HEAP + 0x80000, HEAP + 0x70000
exe = Exe(sys.argv[1])
rows = []
for (x, y, cx, cy, z, p) in [(0, 0, 0, 0, 1000, 0), (100, 50, 0, 0, 500, 0), (100, 50, 0, 0, 1000, 500),
                             (700, 400, 30, -20, 1200, 200), (-300, 900, 100, 100, 800, -500),
                             (512, 288, 0, 0, 3000, 900), (1, 2, 0, 0, 999, 1), (250, 100, -40, 60, 700, 300)]:
    v = bytearray(0x8000)
    struct.pack_into("<i", v, 0, 1)
    struct.pack_into("<d", v, 0xa0, p)
    struct.pack_into("<3i", v, 0xcc, 1000, 1000, 1000)
    exe.uc.mem_write(VERT, bytes(v))
    exe.uc.mem_write(CAM, bytes(0x14) + struct.pack("<3i", cx, cy, z) + bytes(0x20))
    exe.call(PROJECT, [CAM, 0x240, x, y], regs={"eax": VERT})
    sx, sy = struct.unpack("<hh", exe.read(VERT + 0x80, 2) + exe.read(VERT + 0x88, 2))
    rows.append(f"({x}, {y}, {cx}, {cy}, {z}, {p}, {sx}, {sy})")
print("    const EXE_CAMERA: [(i64, i64, i64, i64, i64, i64, i64, i64); %d] = [%s];" % (len(rows), ", ".join(rows)))
