"""Muramasa.exe's SetBlur trail: which motion frame each copy shows, run in the exe under Unicorn
(research/tools/exe_call.py).

    research/tools/.venv/bin/python -I research/nitroplus/trail_exe.py <Muramasa.exe>

Calls FUN_00460530 (a motion step's trail update) for a Move (arg 2 FUN_00456b30) on an object
record: +0xdc0 passes, +0xdd8 lag (ms), +0xa56 the motion's frame (ushort), +0xa64 its frame
count, +0xa60 its eased-frame doubles, +0xa6c its (x, y, -) ints per frame. Copy i lags
_ftol(i * lag * speed / period) frames (period the int at 005dcb0c + 4 * [005dcaf0], FUN_0048c5d0
fills them first) and takes that frame's x into +0xde4 + 4 i; before frame 0 it keeps what it
had. Prints (lag, passes, frame, [copy frames, -1 = kept]) rows as the Rust table `EXE_TRAIL` in
convert/muramasa/opening.rs's tests.
"""
import os
import struct
import sys

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "tools"))
from exe_call import Exe  # noqa: E402

STEP, MOVE = 0x460530, 0x456b30
MEM = 0xd0000000  # our records, mapped here
OBJ, CTX, COPIES, EASE, POS = MEM, MEM + 0x10000, MEM + 0x20000, MEM + 0x100000, MEM + 0x110000
FRAMES = 200
exe = Exe(sys.argv[1])
exe.uc.mem_map(MEM, 0x200000)
exe.call(0x48c5d0, regs={"esi": 0x5dcad8})  # frame tables: 005dcaf0 = 4
exe.stub_import("WINMM.DLL!timeGetTime", 0)
# the speed FUN_0047c250 gives: the double at 005d4130 + 8 * [005d4074]; the settings' init
# (0047b579, 0047b5e0) sets index 3 and 005d4148 = 1.0
exe.uc.mem_write(0x5d4074, struct.pack("<i", 3))
exe.uc.mem_write(0x5d4148, struct.pack("<d", 1.0))
period = struct.unpack("<i", exe.read(0x5dcb0c + 4 * exe.read(0x5dcaf0, 1)[0], 4))[0]
exe.uc.mem_write(EASE, b"".join(struct.pack("<d", j) for j in range(FRAMES)))
exe.uc.mem_write(POS, b"".join(struct.pack("<3i", j, 0, 0) for j in range(FRAMES)))
rows = []
for lag, passes, frame in [(50, 2, 0), (50, 2, 2), (50, 2, 3), (50, 2, 5), (50, 2, 6), (50, 2, 30),
                           (16, 4, 20), (17, 4, 20), (40, 3, 20), (100, 8, 60), (1, 2, 10), (33, 5, 40)]:
    o = bytearray(0x2000)
    struct.pack_into("<I", o, 0x1200, CTX)
    struct.pack_into("<I", o, 0x11f0, COPIES)
    struct.pack_into("<i", o, 0xdc0, passes)
    struct.pack_into("<i", o, 0xdd8, lag)
    struct.pack_into("<I", o, 0xde4, 0xffff)  # copies set before (not the first step)
    struct.pack_into("<H", o, 0xa56, frame)
    struct.pack_into("<i", o, 0xa64, FRAMES)
    struct.pack_into("<I", o, 0xa60, EASE)
    struct.pack_into("<I", o, 0xa6c, POS)
    for i in range(1, passes + 1):
        struct.pack_into("<i", o, 0xde4 + 4 * i, -1)
        struct.pack_into("<i", o, 0xfe8 + 4 * i, 0)
    exe.uc.mem_write(OBJ, bytes(o))
    exe.uc.mem_write(CTX, bytes(0x100))
    exe.call(STEP, [OBJ, MOVE])
    got = [struct.unpack("<i", exe.read(OBJ + 0xde4 + 4 * i, 4))[0] for i in range(1, passes + 1)]
    rows.append(f"({lag}, {passes}, {frame}, &{got})")
print(f"    // frame period {period} ms")
print("    const EXE_TRAIL: [(u32, u32, u32, &[i32]); %d] = [%s];" % (len(rows), ", ".join(rows)))
